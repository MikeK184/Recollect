//! Semantic recall over scopes beyond the exact bound: the shortlist tier's
//! recall against exact cosine, plan timings as the application role,
//! isolation, paging, erasure and migration behavior, on deterministic
//! synthetic vectors. The recall numbers describe these synthetic distributions
//! only; they are not a quality claim about any embedding model. Repository
//! facts supply many distinct identities cheaply.
use super::*;
use axum::routing::get;
use recollect_protocol::ScopeSelection;
use recollect_server::retrieval::{
    EXACT_DIMENSIONS, EXACT_LIMIT, ProofTier, SCOPE_LIMIT, SHORTLIST, SHORTLIST_PAGES,
    semantic_proof,
};
use std::{collections::BTreeSet, time::Instant};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn gaussian(&mut self) -> f64 {
        let u = self.uniform().max(1e-12);
        let v = self.uniform();
        (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
    }
    fn unit(&mut self, dims: usize) -> Vec<f64> {
        let v: Vec<f64> = (0..dims).map(|_| self.gaussian()).collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.into_iter().map(|x| x / norm).collect()
    }
}

/// A vector at cosine `cosine` to `query` (both unit length).
fn at_cosine(rng: &mut Rng, query: &[f64], cosine: f64) -> Vec<f32> {
    let dims = query.len();
    let noise = rng.unit(dims);
    let dot: f64 = noise.iter().zip(query).map(|(a, b)| a * b).sum();
    // Remove the component along the query so the cosine is exact.
    let orthogonal: Vec<f64> = noise.iter().zip(query).map(|(n, q)| n - dot * q).collect();
    let norm = orthogonal.iter().map(|x| x * x).sum::<f64>().sqrt();
    let rest = (1.0 - cosine * cosine).sqrt();
    query
        .iter()
        .zip(&orthogonal)
        .map(|(q, o)| (cosine * q + rest * o / norm) as f32)
        .collect()
}
fn f32s(v: &[f64]) -> Vec<f32> {
    v.iter().map(|x| *x as f32).collect()
}

struct Embedder {
    vector: Mutex<Vec<f32>>,
    calls: AtomicUsize,
}
async fn embeddings(State(p): State<Arc<Embedder>>, Json(body): Json<Value>) -> Json<Value> {
    p.calls.fetch_add(1, Ordering::SeqCst);
    let vector = p.vector.lock().unwrap().clone();
    assert_eq!(vector.len() as u64, body["dimensions"].as_u64().unwrap());
    let data: Vec<Value> = body["input"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, _)| json!({"index":i,"embedding":vector}))
        .collect();
    Json(json!({"model":body["model"],"data":data,"usage":{"prompt_tokens":6,"total_tokens":6}}))
}
async fn catalogue() -> Json<Value> {
    Json(json!({"data":[
        {"id":"gpt-5.6-luna"},{"id":"gpt-4.1-mini"},{"id":"gpt-4.1"},
        {"id":"text-embedding-3-large"},{"id":"text-embedding-3-small"}]}))
}

struct Fixture {
    h: Harness,
    owner: Login,
    base: String,
    brain: Uuid,
    actor: Uuid,
    profile: Uuid,
    dims: usize,
    embedder: Arc<Embedder>,
    server: tokio::task::JoinHandle<()>,
}

/// One repository snapshot and the facts it holds.
#[derive(Clone, Copy)]
struct Repo {
    repository: Uuid,
    snapshot: Uuid,
}

impl Fixture {
    async fn new(dims: usize) -> Self {
        Self::with(Harness::new().await, dims).await
    }

    async fn with(mut h: Harness, dims: usize) -> Self {
        let embedder = Arc::new(Embedder {
            vector: Mutex::new(vec![0.0; dims]),
            calls: AtomicUsize::new(0),
        });
        let router = Router::new()
            .route("/v1/models", get(catalogue))
            .route("/v1/embeddings", axum::routing::post(embeddings))
            .with_state(embedder.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let mut config = (*h.state.config).clone();
        config.models.endpoint = format!("http://{address}/v1");
        config.models.key = Some("synthetic-provider-credential".into());
        h.state = AppState::new(h.state.pool.clone(), config).unwrap();
        h.router = app(h.state.clone());
        let owner = h.login().await;
        let brain = ok(
            &h,
            "POST",
            "/api/brains",
            &owner,
            json!({"name":"Synthetic large semantic scope"}),
        )
        .await;
        let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
        let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
        Self::configure(&h, &owner, &base, dims).await;
        let actor: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
            .fetch_one(&h.admin)
            .await
            .unwrap();
        let profile = Self::profile(&h, brain, dims).await;
        Self {
            h,
            owner,
            base,
            brain,
            actor,
            profile,
            dims,
            embedder,
            server,
        }
    }

    /// Enable query embedding of `dims` dimensions on a Brain's policy.
    async fn configure(h: &Harness, owner: &Login, base: &str, dims: usize) {
        let settings = ok(
            h,
            "GET",
            &format!("{base}/models/policy"),
            owner,
            Value::Null,
        )
        .await;
        let mut policy = settings["current"]["policy"].clone();
        let rebuild = policy["embedding_dimensions"] != json!(dims);
        policy["enabled"] = json!(true);
        policy["purposes"] = json!(["embedding"]);
        policy["embedding_dimensions"] = json!(dims);
        policy["content_classes"] = json!([
            "document",
            "claim",
            "query",
            "raw_session",
            "support_excerpt"
        ]);
        policy["automatic_embedding"] = json!(false);
        policy["automatic_learning"] = json!(false);
        policy["autonomous_memory"] = json!(false);
        let mut body = json!({"base_change":settings["current"]["change_id"],"policy":policy});
        if rebuild {
            body["rebuild_embeddings"] = json!(true);
        }
        ok(h, "PUT", &format!("{base}/models/policy"), owner, body).await;
    }

    /// An active profile of `dims` dimensions on the Brain's current policy.
    async fn profile(h: &Harness, brain: Uuid, dims: usize) -> Uuid {
        let profile = Uuid::new_v4();
        sqlx::query("INSERT INTO semantic_profiles(id,brain_id,provider,model,dimensions,representation,created_by,policy_id) SELECT $1,$2,policy->>'provider',policy->>'embedding_model',$3,'engineering-text-1',created_by,p.id FROM model_policies p JOIN model_policy_heads h ON h.policy_id=p.id AND h.brain_id=p.brain_id WHERE h.brain_id=$2")
            .bind(profile).bind(brain).bind(dims as i32).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO semantic_heads(brain_id,profile_id) VALUES($1,$2) ON CONFLICT(brain_id) DO UPDATE SET profile_id=excluded.profile_id")
            .bind(brain).bind(profile).execute(&h.admin).await.unwrap();
        profile
    }

    /// A repository with one snapshot of `facts` facts. The first facts take
    /// the explicit `planted` vectors; the rest are independent noise.
    async fn repository(&self, name: &str, facts: usize, planted: &[Vec<f32>]) -> Repo {
        let repository = self.new_repository(self.brain, name).await;
        self.snapshot(
            self.brain,
            self.profile,
            repository,
            name,
            facts,
            planted,
            "active",
            0,
        )
        .await
    }

    async fn new_repository(&self, brain: Uuid, name: &str) -> Uuid {
        let repository = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)",
        )
        .bind(repository)
        .bind(brain)
        .bind(format!("https://example.test/{name}"))
        .bind(self.actor)
        .execute(&self.h.admin)
        .await
        .unwrap();
        repository
    }

    /// A snapshot `age_hours` old; only the newest snapshot of a repository is
    /// current for recall.
    #[allow(clippy::too_many_arguments)]
    async fn snapshot(
        &self,
        brain: Uuid,
        profile: Uuid,
        repository: Uuid,
        name: &str,
        facts: usize,
        planted: &[Vec<f32>],
        privacy: &str,
        age_hours: i32,
    ) -> Repo {
        let snapshot = Uuid::new_v4();
        let admin = &self.h.admin;
        sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by,privacy_state,created_at) VALUES($1,$2,$3,$7,'synthetic','1','1','{}','{}',0,$4,0,$5,$6,now()-$8*interval '1 hour')")
            .bind(snapshot).bind(brain).bind(repository).bind(facts as i64).bind(self.actor).bind(privacy)
            .bind(format!("r{age_hours}")).bind(age_hours)
            .execute(admin).await.unwrap();
        // One connection so that the seed makes the noise reproducible.
        let mut tx = admin.begin().await.unwrap();
        sqlx::query("SET LOCAL statement_timeout='30min'")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("SELECT setseed(0.2024)")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('kind','function','name','fact '||$3::text||' '||n,'file','src/m'||(n%40)||'.rs','line',n) FROM generate_series(0,$4-1) n")
            .bind(brain).bind(snapshot).bind(name).bind(facts as i32)
            .execute(&mut *tx).await.unwrap();
        for (ordinal, vector) in planted.iter().enumerate() {
            sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,fact_id,state,embedding) SELECT gen_random_uuid(),$1,$2,'repository_fact',f.id,f.id,'ready',$4::text::vector FROM repository_facts f WHERE f.snapshot_id=$3 AND f.ordinal=$5")
                .bind(brain).bind(profile).bind(snapshot).bind(serde_json::to_string(vector).unwrap()).bind(ordinal as i32)
                .execute(&mut *tx).await.unwrap();
        }
        sqlx::query("INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,fact_id,state,embedding) SELECT gen_random_uuid(),$1,$2,'repository_fact',f.id,f.id,'ready',(SELECT array_agg((random()-0.5)::real) FROM generate_series(1,$4::int) g WHERE f.id IS NOT NULL)::vector FROM repository_facts f WHERE f.snapshot_id=$3 AND f.ordinal>=$5")
            .bind(brain).bind(profile).bind(snapshot).bind(self.dims as i32).bind(planted.len() as i32)
            .execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        Repo {
            repository,
            snapshot,
        }
    }

    /// The fact at `ordinal` of a snapshot.
    async fn fact(&self, repo: Repo, ordinal: i32) -> Uuid {
        sqlx::query_scalar("SELECT id FROM repository_facts WHERE snapshot_id=$1 AND ordinal=$2")
            .bind(repo.snapshot)
            .bind(ordinal)
            .fetch_one(&self.h.admin)
            .await
            .unwrap()
    }

    async fn facts(&self, repo: Repo) -> BTreeSet<Uuid> {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM repository_facts WHERE snapshot_id=$1")
            .bind(repo.snapshot)
            .fetch_all(&self.h.admin)
            .await
            .unwrap()
            .into_iter()
            .collect()
    }

    async fn analyze(&self) {
        for table in ["semantic_entries", "repository_facts"] {
            sqlx::query(&format!("ANALYZE {table}"))
                .execute(&self.h.admin)
                .await
                .unwrap();
        }
    }

    fn scope(repositories: &[Uuid]) -> ScopeSelection {
        ScopeSelection {
            repository_ids: repositories.to_vec(),
            ..ScopeSelection::default()
        }
    }

    /// Run a scoring statement as the application role under a statement bound.
    async fn proof(
        &self,
        repositories: &[Uuid],
        vector: &[f32],
        tier: ProofTier,
        limit: usize,
        explain: bool,
        timeout: &str,
    ) -> recollect_server::retrieval::SemanticProof {
        let mut tx = recollect_server::db::actor_tx(&self.h.state.pool, self.actor)
            .await
            .unwrap();
        sqlx::query(&format!("SET LOCAL statement_timeout='{timeout}'"))
            .execute(&mut *tx)
            .await
            .unwrap();
        let proof = semantic_proof(
            &mut tx,
            self.brain,
            Self::scope(repositories),
            vector,
            tier,
            limit,
            explain,
        )
        .await
        .unwrap_or_else(|e| panic!("proof {tier:?}: {} {}", e.1, e.2));
        tx.rollback().await.unwrap();
        proof
    }

    /// A recall over the given repositories whose query vector is `vector`.
    async fn recall(&self, repositories: &[Uuid], vector: &[f32], extra: Value) -> Value {
        *self.embedder.vector.lock().unwrap() = vector.to_vec();
        let mut input = query("Which function matches this question", &["semantic"]);
        input["selection"] =
            json!({"repository_ids":repositories,"area_ids":[],"environment_id":null});
        input["limit"] = json!(20);
        for (key, value) in extra.as_object().into_iter().flatten() {
            input[key] = value.clone();
        }
        recall(&self.h, &self.owner, &self.base, input).await
    }

    async fn finish(self) {
        self.server.abort();
        if std::env::var_os("RECOLLECT_SCALE_KEEP").is_some() {
            eprintln!("KEPT database {}", self.h.database);
            return;
        }
        self.h.finish().await;
    }
}

fn execution_ms(plan: &[String]) -> f64 {
    plan.iter()
        .find_map(|line| line.trim().strip_prefix("Execution Time: "))
        .and_then(|rest| rest.trim_end_matches(" ms").parse().ok())
        .expect("plan execution time")
}

fn recall_at(exact: &[(Uuid, f32)], candidate: &[(Uuid, f32)], k: usize) -> f64 {
    let truth: BTreeSet<Uuid> = exact.iter().take(k).map(|r| r.0).collect();
    let found = candidate
        .iter()
        .take(k)
        .filter(|r| truth.contains(&r.0))
        .count();
    found as f64 / truth.len().max(1) as f64
}

fn reason_codes(result: &Value) -> Vec<String> {
    result["coverage"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap().to_string())
        .collect()
}

fn ids(result: &Value) -> Vec<Value> {
    result["context"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].clone())
        .collect()
}

const SIZES: [usize; 3] = [5000, 15000, 30000];

/// The best of several timed plans: the shared machine adds load spikes that
/// the statement bound itself would not forgive, so the report keeps the
/// capability and the median.
async fn timed(
    fx: &Fixture,
    scope: &[Uuid],
    vector: &[f32],
    tier: ProofTier,
    limit: usize,
    timeout: &str,
) -> (f64, f64) {
    let mut runs = vec![];
    for _ in 0..3 {
        let plan = fx.proof(scope, vector, tier, limit, true, timeout).await;
        runs.push(execution_ms(&plan.plan));
    }
    runs.sort_by(f64::total_cmp);
    (runs[0], runs[1])
}

/// Shared body of the per-dimension measurements: scope sizes 5,000, 20,000
/// and 50,000, a planted-neighbour query and an independent-noise query
/// (independent noise is the adversarial case: no neighbour is meaningfully
/// closer than the background).
async fn measure(dims: usize) {
    let fx = Fixture::new(dims).await;
    let mut rng = Rng(0xC0FFEE ^ dims as u64);
    let query = rng.unit(dims);
    // 150 planted neighbours at cosines from 0.9 down to 0.35.
    let planted: Vec<Vec<f32>> = (0..150)
        .map(|i| at_cosine(&mut rng, &query, 0.9 - 0.55 * i as f64 / 149.0))
        .collect();
    let started = Instant::now();
    let mut repositories = vec![];
    for (i, size) in SIZES.iter().enumerate() {
        let name = format!("scale-{i}");
        repositories.push(
            fx.repository(&name, *size, if i == 0 { &planted } else { &[] })
                .await,
        );
    }
    fx.analyze().await;
    eprintln!(
        "SCALE dims={dims} fixture of {} entries built in {:?}",
        SIZES.iter().sum::<usize>(),
        started.elapsed()
    );
    let planted_query = f32s(&query);
    let noise_query = f32s(&rng.unit(dims));
    let best = fx.fact(repositories[0], 0).await;
    let repository_ids: Vec<Uuid> = repositories.iter().map(|r| r.repository).collect();
    let exact_cap = if dims <= EXACT_DIMENSIONS as usize {
        SCOPE_LIMIT
    } else {
        EXACT_LIMIT
    };
    for scoped in 1..=SIZES.len() {
        let scope = &repository_ids[..scoped];
        let rows: usize = SIZES[..scoped].iter().sum();
        for (label, vector) in [("planted", &planted_query), ("noise", &noise_query)] {
            let exact = fx
                .proof(scope, vector, ProofTier::Exact, 100, false, "120s")
                .await;
            let mut line = format!("SCALE dims={dims} rows={rows} query={label} recall@100");
            for size in [1000, 2000, SHORTLIST, 8000] {
                let short = fx
                    .proof(scope, vector, ProofTier::Shortlist, size, false, "20s")
                    .await;
                let recall = recall_at(&exact.ranked, &short.ranked, 100);
                line += &format!(" M{size}={recall:.3}");
                if label == "planted" && size == SHORTLIST && dims > EXACT_DIMENSIONS as usize {
                    assert!(
                        recall >= 0.95,
                        "A planted neighbourhood must survive the shortlist: {recall}"
                    );
                }
            }
            eprintln!("{line}");
        }
        // The exact statement is timed over the whole scope (above its bound it
        // shows what the tier would cost); the shortlist as production runs it.
        let exact = timed(&fx, scope, &planted_query, ProofTier::Exact, 5000, "120s").await;
        let short = timed(
            &fx,
            scope,
            &planted_query,
            ProofTier::Shortlist,
            SHORTLIST,
            "20s",
        )
        .await;
        eprintln!(
            "SCALE dims={dims} rows={rows} exact_ms best/median={:.0}/{:.0} shortlist_ms best/median={:.0}/{:.0}",
            exact.0, exact.1, short.0, short.1
        );
        assert!(
            short.0 < 2000.0,
            "The shortlist statement exceeds its bound at {rows} rows"
        );
        if rows <= exact_cap {
            assert!(
                exact.0 < 2000.0,
                "The exact statement exceeds its bound at {rows} rows"
            );
        }
        // The same scope through the product, as one request.
        let result = fx.recall(scope, &planted_query, json!({})).await;
        let expected_exact = rows <= exact_cap;
        assert_eq!(result["semantic"]["scoped_entries"], rows);
        assert_eq!(
            result["semantic"]["method"],
            if expected_exact {
                "exact_cosine"
            } else {
                "binary_shortlist_rerank"
            }
        );
        // Exact ranking returns the exact best fact, whatever the dimension
        // (at 8 dimensions noise can beat a planted neighbour); the shortlist
        // must at least keep the planted best of a well-separated corpus.
        let expected_top = if expected_exact {
            let truth = fx
                .proof(scope, &planted_query, ProofTier::Exact, 5, false, "120s")
                .await;
            json!(truth.ranked[0].0)
        } else {
            json!(best)
        };
        assert_eq!(result["context"]["items"][0]["id"], expected_top);
        let reranked = result["semantic"]["reranked"].as_u64().unwrap() as usize;
        if expected_exact {
            assert_eq!(reranked, rows);
            assert!(!reason_codes(&result).contains(&"semantic_shortlist_approximate".to_string()));
        } else {
            assert!(reranked <= SHORTLIST * SHORTLIST_PAGES && reranked >= SHORTLIST.min(rows));
            assert!(reason_codes(&result).contains(&"semantic_shortlist_approximate".to_string()));
            assert!(
                result["context"]["items"][0]["qualifications"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("semantic_shortlist_approximate"))
            );
        }
        eprintln!(
            "SCALE dims={dims} rows={rows} request method={} reranked={reranked} elapsed_ms={}",
            result["semantic"]["method"], result["elapsed_ms"]
        );
    }
    fx.finish().await;
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; builds 50,000 synthetic vectors"]
async fn semantic_scale_3072_dimensions_up_to_fifty_thousand_entries() {
    measure(3072).await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; builds 50,000 synthetic vectors"]
async fn semantic_scale_1536_dimensions_up_to_fifty_thousand_entries() {
    measure(1536).await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; builds 50,000 synthetic vectors"]
async fn semantic_scale_384_dimensions_rank_exactly_over_the_whole_scope() {
    measure(384).await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; builds 50,000 synthetic vectors"]
async fn semantic_scale_8_dimensions_rank_exactly_over_the_whole_scope() {
    measure(8).await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_shortlist_returns_only_eligible_in_scope_vectors_above_the_exact_bound() {
    let dims = 512;
    let fx = Fixture::new(dims).await;
    let mut rng = Rng(7);
    let query = rng.unit(dims);
    let q = f32s(&query);
    // Legitimate neighbours: the best at cosine 0.9.
    let planted: Vec<Vec<f32>> = (0..20)
        .map(|i| at_cosine(&mut rng, &query, 0.9 - 0.02 * i as f64))
        .collect();
    // High scorers that must never appear: the query itself (cosine 1).
    let adversarial = vec![q.clone(); 3];
    let inside = fx.repository("inside", EXACT_LIMIT + 200, &planted).await;
    let stale = fx
        .snapshot(
            fx.brain,
            fx.profile,
            inside.repository,
            "stale",
            3,
            &adversarial,
            "active",
            48,
        )
        .await;
    let outside = fx.repository("outside", 3, &adversarial).await;
    let erased_repository = fx.new_repository(fx.brain, "erased").await;
    let erased = fx
        .snapshot(
            fx.brain,
            fx.profile,
            erased_repository,
            "erased",
            3,
            &adversarial,
            "erased",
            0,
        )
        .await;
    // Another Brain of the same owner holds perfect matches too.
    let other: Uuid = ok(
        &fx.h,
        "POST",
        "/api/brains",
        &fx.owner,
        json!({"name":"Other Brain with perfect matches"}),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    Fixture::configure(&fx.h, &fx.owner, &format!("/api/brains/{other}"), dims).await;
    let other_profile = Fixture::profile(&fx.h, other, dims).await;
    let other_repository = fx.new_repository(other, "other").await;
    let foreign = fx
        .snapshot(
            other,
            other_profile,
            other_repository,
            "other",
            3,
            &adversarial,
            "active",
            0,
        )
        .await;
    // Eligible facts without a vector: coverage must count them.
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('kind','function','name','unindexed '||n) FROM generate_series(100000,100006) n")
        .bind(fx.brain).bind(inside.snapshot).execute(&fx.h.admin).await.unwrap();
    fx.analyze().await;

    let result = fx
        .recall(&[inside.repository, erased_repository], &q, json!({}))
        .await;
    assert_eq!(result["semantic"]["method"], "binary_shortlist_rerank");
    assert_eq!(result["semantic"]["scoped_entries"], EXACT_LIMIT + 200);
    assert_eq!(result["semantic"]["unindexed_entries"], 7);
    assert_eq!(
        result["algorithm"],
        "pgvector-binary-shortlist-exact-rerank"
    );
    let reasons = reason_codes(&result);
    for expected in [
        "semantic_shortlist_approximate",
        "semantic_scoped_coverage_partial",
    ] {
        assert!(reasons.contains(&expected.to_string()), "{reasons:?}");
    }
    assert!(!reasons.contains(&"semantic_shortlist_exhausted".to_string()));
    let returned = ids(&result);
    assert!(!returned.is_empty());
    assert_eq!(returned[0], json!(fx.fact(inside, 0).await));
    let legitimate = fx.facts(inside).await;
    for id in &returned {
        assert!(
            legitimate.contains(&id.as_str().unwrap().parse().unwrap()),
            "Only the selected repository's current facts may be returned"
        );
    }
    for repo in [stale, outside, erased, foreign] {
        for id in fx.facts(repo).await {
            assert!(
                !result.to_string().contains(&id.to_string()),
                "An out-of-scope or ineligible high scorer was returned"
            );
        }
    }
    for item in result["context"]["items"].as_array().unwrap() {
        let qualifications = item["qualifications"].as_array().unwrap();
        assert!(qualifications.contains(&json!("semantic_similarity_not_truth")));
        assert!(qualifications.contains(&json!("semantic_shortlist_approximate")));
    }
    // An evaluation can force the exact tier and keeps the refusal above it,
    // before any provider call.
    let calls = fx.embedder.calls.load(Ordering::SeqCst);
    *fx.embedder.vector.lock().unwrap() = q.clone();
    let mut exact = query_input(&[inside.repository]);
    exact["semantic_exact"] = json!(true);
    let (code, body, _) =
        fx.h.call(
            "POST",
            &format!("{}/recall", fx.base),
            Some(&fx.owner),
            exact,
        )
        .await;
    assert_eq!(code, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "semantic_scope_too_large");
    assert_eq!(fx.embedder.calls.load(Ordering::SeqCst), calls);
    // The refusal needs the semantic channel, like the other semantic options.
    let (code, _, _) =
        fx.h.call(
            "POST",
            &format!("{}/recall", fx.base),
            Some(&fx.owner),
            json!({"query":"Which function","semantic_exact":true}),
        )
        .await;
    assert_eq!(code, StatusCode::BAD_REQUEST);
    fx.finish().await;
}

fn query_input(repositories: &[Uuid]) -> Value {
    let mut input = query("Which function matches this question", &["semantic"]);
    input["selection"] = json!({"repository_ids":repositories,"area_ids":[],"environment_id":null});
    input
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn semantic_shortlist_pages_for_missing_identities_then_reports_exhaustion() {
    let dims = 512;
    let fx = Fixture::new(dims).await;
    let mut rng = Rng(11);
    let query = rng.unit(dims);
    let q = f32s(&query);
    // Three vectors almost identical to the query; everything else is noise.
    let planted: Vec<Vec<f32>> = (0..3)
        .map(|i| at_cosine(&mut rng, &query, 0.999 - 0.0005 * i as f64))
        .collect();
    let rows = SHORTLIST * SHORTLIST_PAGES + 200;
    let repo = fx.repository("pages", rows, &planted).await;
    fx.analyze().await;
    // A minimum similarity keeps only the planted rows, far fewer than the 100
    // candidates: the scope is paged to the end of its allowance, no further.
    let result = fx
        .recall(
            &[repo.repository],
            &q,
            json!({"semantic_min_similarity":0.99}),
        )
        .await;
    assert_eq!(result["semantic"]["method"], "binary_shortlist_rerank");
    assert_eq!(result["semantic"]["scoped_entries"], rows);
    assert_eq!(
        result["semantic"]["reranked"],
        SHORTLIST * SHORTLIST_PAGES,
        "{result}"
    );
    assert_eq!(ids(&result).len(), 3);
    let reasons = reason_codes(&result);
    assert!(reasons.contains(&"semantic_shortlist_exhausted".to_string()));
    assert!(reasons.contains(&"semantic_shortlist_approximate".to_string()));
    // Within the allowance the identical request reaches every row and says so
    // by omitting the exhaustion note.
    let small = fx
        .repository("pages-small", EXACT_LIMIT + 100, &planted)
        .await;
    let result = fx
        .recall(
            &[small.repository],
            &q,
            json!({"semantic_min_similarity":0.99}),
        )
        .await;
    assert_eq!(result["semantic"]["reranked"], EXACT_LIMIT + 100);
    assert_eq!(ids(&result).len(), 3);
    assert!(!reason_codes(&result).contains(&"semantic_shortlist_exhausted".to_string()));
    fx.finish().await;
}
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn automatic_recall_keeps_semantic_search_between_the_exact_bound_and_the_cap() {
    let dims = 512;
    let fx = Fixture::new(dims).await;
    let mut rng = Rng(23);
    let query = rng.unit(dims);
    let q = f32s(&query);
    let planted: Vec<Vec<f32>> = (0..5)
        .map(|i| at_cosine(&mut rng, &query, 0.9 - 0.05 * i as f64))
        .collect();
    let repo = fx.repository("auto", EXACT_LIMIT + 300, &planted).await;
    fx.analyze().await;
    *fx.embedder.vector.lock().unwrap() = q;
    let mut input = json!({"query":"Which function handles the auto fixture?","strategy":"auto","semantic_request_id":Uuid::new_v4(),"selection":{"repository_ids":[repo.repository],"area_ids":[],"environment_id":null}});
    let result = recall(&fx.h, &fx.owner, &fx.base, input.clone()).await;
    let reasons = reason_codes(&result);
    assert!(
        !reasons.iter().any(|r| r.contains("scope_too_large")),
        "{reasons:?}"
    );
    assert_eq!(result["semantic"]["method"], "binary_shortlist_rerank");
    assert!(reasons.contains(&"semantic_shortlist_approximate".to_string()));
    assert_eq!(result["algorithm"], "identity-priority-rrf-k60");
    assert!(
        ids(&result).contains(&json!(fx.fact(repo, 0).await)),
        "{result}"
    );
    // The evaluation flag keeps today's refusal, which auto mode reports as a
    // dropped channel and not as a failed request.
    input["semantic_exact"] = json!(true);
    input["semantic_request_id"] = json!(Uuid::new_v4());
    let result = recall(&fx.h, &fx.owner, &fx.base, input).await;
    assert!(result["semantic"].is_null());
    assert!(reason_codes(&result).contains(&"automatic_semantic_scope_too_large".to_string()));
    fx.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn erasure_leaves_no_signature_and_the_signature_always_follows_the_vector() {
    let dims = 8;
    let fx = Fixture::new(dims).await;
    let keep = fx.repository("keep", 40, &[]).await;
    let doomed = fx.repository("doomed", 40, &[]).await;
    let count = |sql: &'static str, snapshot: Uuid| {
        let admin = fx.h.admin.clone();
        async move {
            sqlx::query_scalar::<_, i64>(sql)
                .bind(snapshot)
                .fetch_one(&admin)
                .await
                .unwrap()
        }
    };
    let signed = "SELECT count(*) FROM semantic_entries e JOIN repository_facts f ON f.id=e.fact_id WHERE f.snapshot_id=$1 AND e.signature IS NOT NULL AND bit_length(e.signature)=8 AND e.signature=binary_quantize(e.embedding)::bit varying";
    let cleared = "SELECT count(*) FROM semantic_entries e JOIN repository_facts f ON f.id=e.fact_id WHERE f.snapshot_id=$1 AND e.embedding IS NULL AND e.signature IS NULL AND e.state='removed'";
    assert_eq!(count(signed, doomed.snapshot).await, 40);
    // The product's erasure path clears the vector, and with it the signature.
    let target = json!({"kind":"snapshot","id":doomed.snapshot});
    let preview = ok(
        &fx.h,
        "POST",
        &format!("{}/erasures/preview", fx.base),
        &fx.owner,
        target.clone(),
    )
    .await;
    ok(
        &fx.h,
        "POST",
        &format!("{}/erasures", fx.base),
        &fx.owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert_eq!(count(cleared, doomed.snapshot).await, 40);
    assert_eq!(count(signed, keep.snapshot).await, 40);
    // Any writer: nulling the vector nulls the signature, a new vector replaces
    // it, and the table refuses a signature without a vector.
    let entry: Uuid = sqlx::query_scalar("SELECT e.id FROM semantic_entries e JOIN repository_facts f ON f.id=e.fact_id WHERE f.snapshot_id=$1 LIMIT 1")
        .bind(keep.snapshot).fetch_one(&fx.h.admin).await.unwrap();
    sqlx::query("UPDATE semantic_entries SET embedding='[1,-1,1,-1,1,-1,1,-1]' WHERE id=$1")
        .bind(entry)
        .execute(&fx.h.admin)
        .await
        .unwrap();
    let signature: String =
        sqlx::query_scalar("SELECT signature::text FROM semantic_entries WHERE id=$1")
            .bind(entry)
            .fetch_one(&fx.h.admin)
            .await
            .unwrap();
    assert_eq!(signature, "10101010");
    sqlx::query("UPDATE semantic_entries SET signature=B'00000000' WHERE id=$1")
        .bind(entry)
        .execute(&fx.h.admin)
        .await
        .unwrap();
    let signature: String =
        sqlx::query_scalar("SELECT signature::text FROM semantic_entries WHERE id=$1")
            .bind(entry)
            .fetch_one(&fx.h.admin)
            .await
            .unwrap();
    assert_eq!(
        signature, "10101010",
        "The signature is derived; a writer cannot set it"
    );
    sqlx::query("UPDATE semantic_entries SET state='removed',embedding=NULL WHERE id=$1")
        .bind(entry)
        .execute(&fx.h.admin)
        .await
        .unwrap();
    let signature: Option<String> =
        sqlx::query_scalar("SELECT signature::text FROM semantic_entries WHERE id=$1")
            .bind(entry)
            .fetch_one(&fx.h.admin)
            .await
            .unwrap();
    assert_eq!(signature, None);
    // The constraint is the last line: with triggers off a mismatch is refused.
    let mut tx = fx.h.admin.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    let refused = sqlx::query("UPDATE semantic_entries SET signature=B'1' WHERE id=$1")
        .bind(entry)
        .execute(&mut *tx)
        .await;
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("semantic_signature_matches_embedding")
    );
    tx.rollback().await.unwrap();
    fx.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn reindexing_to_other_dimensions_replaces_signatures_with_the_new_length() {
    let (h, owner, base, _p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    source(&h, &owner, &base, "A bounded synthetic reindex source.").await;
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let lengths = |h: &Harness| {
        let admin = h.admin.clone();
        async move {
            sqlx::query_as::<_, (Option<i32>, Option<i32>, String)>("SELECT bit_length(signature),vector_dims(embedding),state FROM semantic_entries ORDER BY created_at")
                .fetch_all(&admin).await.unwrap()
        }
    };
    let first = lengths(&h).await;
    assert!(first.iter().any(|r| r.2 == "ready"));
    for (bits, dims, _) in first.iter().filter(|r| r.2 == "ready") {
        assert_eq!((*bits, *dims), (Some(3072), Some(3072)));
    }
    let settings = ok(
        &h,
        "GET",
        &format!("{base}/models/policy"),
        &owner,
        Value::Null,
    )
    .await;
    let mut next = settings["current"]["policy"].clone();
    next["embedding_dimensions"] = json!(1024);
    ok(&h,"PUT",&format!("{base}/models/policy"),&owner,json!({"base_change":settings["current"]["change_id"],"policy":next,"rebuild_embeddings":true})).await;
    // The old generation's vectors and signatures are gone together.
    let old: i64 = sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE embedding IS NOT NULL OR signature IS NOT NULL")
        .fetch_one(&h.admin).await.unwrap();
    assert_eq!(old, 0);
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let second: Vec<_> = lengths(&h)
        .await
        .into_iter()
        .filter(|r| r.2 == "ready")
        .collect();
    assert!(!second.is_empty());
    for (bits, dims, _) in &second {
        assert_eq!((*bits, *dims), (Some(1024), Some(1024)));
    }
    let mismatches: i64 = sqlx::query_scalar("SELECT count(*) FROM semantic_entries WHERE embedding IS NOT NULL AND signature IS DISTINCT FROM binary_quantize(embedding)::bit varying")
        .fetch_one(&h.admin).await.unwrap();
    assert_eq!(mismatches, 0);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn migration_072_backfills_every_signature_in_batches_and_restores_the_timeout() {
    let position = db::MIGRATIONS
        .iter()
        .position(|(name, _)| *name == "072_semantic_signature")
        .unwrap();
    let h = Harness::new_through(position).await;
    let fx = Fixture::with(h, 8).await;
    // More rows than one backfill batch, some without a vector.
    fx.repository("backfill", 2600, &[]).await;
    sqlx::query("UPDATE semantic_entries SET state='removed',embedding=NULL WHERE id IN (SELECT id FROM semantic_entries ORDER BY id LIMIT 37)")
        .execute(&fx.h.admin).await.unwrap();
    let before: (i64, i64) = sqlx::query_as("SELECT count(*) FILTER (WHERE embedding IS NOT NULL),count(*) FILTER (WHERE embedding IS NULL) FROM semantic_entries")
        .fetch_one(&fx.h.admin).await.unwrap();
    assert_eq!(before, (2563, 37));
    // Apply it as the runner does: inside the runner's transaction and under its
    // ten-second statement default, which the migration must hand back.
    let mut tx = fx.h.admin.begin().await.unwrap();
    sqlx::raw_sql(db::MIGRATIONS[position].1)
        .execute(&mut *tx)
        .await
        .unwrap();
    let timeout: String = sqlx::query_scalar("SELECT current_setting('statement_timeout')")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(timeout, "10s");
    sqlx::query("INSERT INTO recollect_migrations(name) VALUES($1)")
        .bind(db::MIGRATIONS[position].0)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let after: (i64, i64, i64) = sqlx::query_as("SELECT count(*) FILTER (WHERE signature IS NOT NULL AND signature=binary_quantize(embedding)::bit varying),count(*) FILTER (WHERE embedding IS NULL AND signature IS NULL),count(*) FROM semantic_entries")
        .fetch_one(&fx.h.admin).await.unwrap();
    assert_eq!(after, (2563, 37, 2600));
    // The trigger and the constraint exist from the migration on.
    let constraint: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_constraint WHERE conname='semantic_signature_matches_embedding' AND convalidated")
        .fetch_one(&fx.h.admin).await.unwrap();
    assert_eq!(constraint, 1);
    db::migrate(&fx.h.admin).await.unwrap();
    db::ready(&fx.h.admin).await.unwrap();
    fx.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn shortlist_ranking_equals_exact_ranking_on_the_fixed_semantic_corpus() {
    // The corpus of the exact-cosine recall test: an exact match, a near match,
    // a distractor and an out-of-selection canary, with controlled vectors.
    let (h, owner, base, p, server) = setup().await;
    let cx = context(&h, &base).await;
    enable(&h, &owner, &base).await;
    p.mode.store(5, Ordering::SeqCst);
    for text in [
        "Operational executions are retained in the audit journal.",
        "NEAR_VECTOR: The previous runs view also exposes execution metadata.",
        "DISTRACTOR_VECTOR: Tea and coffee are available in the kitchen.",
        "OUTSIDE_SCOPE_CANARY: previous runs must never enter the selected collection.",
    ] {
        source(&h, &owner, &base, text).await;
    }
    discover(&h, cx).await;
    while worker::run_once(&h.state, "model").await.unwrap() {}
    let mut vector = vec![0.0f32; 3072];
    vector[0] = 1.0;
    let mut ranked = vec![];
    for tier in [ProofTier::Exact, ProofTier::Shortlist] {
        let mut tx = recollect_server::db::actor_tx(&h.state.pool, cx.actor)
            .await
            .unwrap();
        let proof = semantic_proof(
            &mut tx,
            cx.brain,
            ScopeSelection::default(),
            &vector,
            tier,
            SHORTLIST,
            false,
        )
        .await
        .unwrap_or_else(|e| panic!("proof {tier:?}: {} {}", e.1, e.2));
        tx.rollback().await.unwrap();
        ranked.push(proof.ranked);
    }
    assert_eq!(ranked[0].len(), 4);
    assert_eq!(recall_at(&ranked[0], &ranked[1], 100), 1.0);
    for (exact, short) in ranked[0].iter().zip(&ranked[1]) {
        assert_eq!(exact.0, short.0, "The shortlist keeps the exact order");
        assert!((exact.1 - short.1).abs() < 1e-6);
    }
    server.abort();
    h.finish().await;
}
