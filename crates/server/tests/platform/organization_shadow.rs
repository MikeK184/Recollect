//! Real loopback gateway binding and retained-only unpublished evaluation.
//! Synthetic finite vectors prove plumbing, never extractor/grouping quality.
use super::*;
use recollect_server::{
    auth::Auth,
    graph::analytics::shadow::{self, Mode, ShadowResult, ShadowSource},
    knowledge_mapping::source_features::HybridParameters,
};
#[path = "../../examples/mapping_qualification/support/evaluation_failure.rs"]
mod failure_cleanup;

fn versions(docs: &[Value]) -> Vec<ShadowSource> {
    docs.iter()
        .map(|d| ShadowSource {
            source_version_id: d["version"]["id"].as_str().unwrap().parse().unwrap(),
            candidate_id: None,
        })
        .collect()
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; actual deliberately failed native version and26 subsequent native trials, zero provider calls"]
async fn organization_shadow_requeued_experiment_is_cancelled_without_poisoning_next_grid_trial() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    sqlx::query("DELETE FROM knowledge_organization_heads WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let inputs = versions(&docs);
    let mut trials = vec![];
    for neighbors in [3, 5, 8] {
        for minimum_cosine in [0.65, 0.75, 0.85] {
            for vector_scale in [0.25, 0.5, 1.0] {
                let parameters = HybridParameters {
                    neighbors,
                    minimum_cosine,
                    vector_scale,
                };
                let first = trials.is_empty();
                let report = shadow::queue(
                    &h.state,
                    &auth,
                    brain,
                    &inputs,
                    Mode::EligibleVector,
                    parameters,
                    if first {
                        "deliberately-wrong-test-version"
                    } else {
                        "2026.08.1"
                    },
                )
                .await
                .unwrap_or_else(|e| panic!("queue:{}", e.1));
                let job = worker::claim(&h.state.pool, "heavy")
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    (job.brain_id, job.target_id, job.actor_id, job.kind.as_str()),
                    (brain, report, auth.user.id, "graph.analyze")
                );
                let expected = job.id;
                let state = h.state.clone();
                tokio::spawn(async move { worker::execute(&state, &job).await })
                    .await
                    .unwrap()
                    .unwrap();
                if first {
                    let before: (String, Option<String>) =
                        sqlx::query_as("SELECT state,error_code FROM jobs WHERE id=$1")
                            .bind(expected)
                            .fetch_one(&h.admin)
                            .await
                            .unwrap();
                    assert_eq!(
                        before,
                        (
                            "queued".into(),
                            Some("organization_engine_version_changed".into())
                        )
                    );
                    let cleanup = failure_cleanup::cancel_failed(
                        &h.state, &h.admin, &auth, brain, report, expected,
                    )
                    .await
                    .expect(
                        "Exact failed native attempt must be closed and original job cancelled",
                    );
                    assert_eq!(cleanup["before"], "queued");
                    assert_eq!(cleanup["after"], "cancelled");
                    assert_eq!(cleanup["pending_native_attempts"], 0);
                    trials.push(json!({"parameters":parameters,"state":"failed","source_denominator":3,"pair_denominator":3,"failed_source_outcomes":3,"cleanup":cleanup}));
                } else {
                    let result = shadow::result(&h.state, &auth, brain, report)
                        .await
                        .unwrap_or_else(|e| panic!("next result:{}", e.1));
                    assert_eq!(result.sources.len(), 3);
                    assert_eq!(result.groups.len(), 1);
                    assert_eq!(
                        result
                            .sources
                            .iter()
                            .filter(|s| s.state == "assigned")
                            .count(),
                        2
                    );
                    trials.push(json!({"parameters":parameters,"state":"ready","source_denominator":result.sources.len(),"pair_denominator":3,"result":result}));
                }
            }
        }
    }
    assert_eq!(trials.len(), 27);
    assert_eq!(trials.iter().filter(|t| t["state"] == "failed").count(), 1);
    assert_eq!(
        trials
            .iter()
            .map(|t| t["source_denominator"].as_u64().unwrap())
            .sum::<u64>(),
        81
    );
    let pending:i64=sqlx::query_scalar("SELECT count(*) FROM jobs WHERE brain_id=$1 AND kind='graph.analyze' AND state IN('queued','running')").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(pending, 0);
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    let receipt = json!({"state":"passed","scenario":"exactrequeuednativejobcancelled_next26trialscomplete","trials":trials,"native_model_calls":0,"paid_calls":0,"quality_measured":false,"pending_own_analytics_jobs":0});
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../.cache/automatic-mapping/shadow-requeue-proof-{}.json",
        Uuid::new_v4()
    ));
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    server.abort();
    h.finish().await;
}
async fn calculate(
    h: &Harness,
    auth: &Auth,
    brain: Uuid,
    inputs: &[ShadowSource],
    mode: Mode,
) -> (ShadowResult, Value) {
    let id = shadow::queue(
        &h.state,
        auth,
        brain,
        inputs,
        mode,
        HybridParameters::default(),
        "2026.08.1",
    )
    .await
    .unwrap_or_else(|e| panic!("queue: {}", e.1));
    let projection: Value = sqlx::query_scalar(
        "SELECT projection FROM knowledge_shadow_calculations WHERE report_id=$1",
    )
    .bind(id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (job.kind.as_str(), job.brain_id, job.target_id, job.actor_id),
        ("graph.analyze", brain, id, auth.user.id)
    );
    let state = h.state.clone();
    tokio::spawn(async move { worker::execute(&state, &job).await })
        .await
        .unwrap()
        .unwrap();
    let status: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM analytics_reports WHERE id=$1")
            .bind(id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(status, ("ready".into(), None));
    (
        shadow::result(&h.state, auth, brain, id)
            .await
            .unwrap_or_else(|e| panic!("result: {}", e.1)),
        projection,
    )
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j, two actual loopback extraction calls; zero paid calls"]
async fn organization_shadow_five_modes_scoped_metadata_and_exact_cohort_invariance() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, profile) = organization_hybrid::prepare(&h, &owner, &base).await;
    // An experiment must work without selecting a production recipe. The
    // retained profile remains actual canonical data owned by this fixture.
    sqlx::query("DELETE FROM knowledge_organization_heads WHERE brain_id=$1")
        .bind(brain)
        .execute(&h.admin)
        .await
        .unwrap();

    allow(&h, &owner, &base, |p| {
        p["purposes"] = json!(["extraction", "embedding"])
    })
    .await;
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let repo = Uuid::new_v4();
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/shadow/canonical',$3)").bind(repo).bind(brain).bind(auth.user.id).execute(&h.admin).await.unwrap();
    let environment = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Actual scoped shadow PROD"}),
    )
    .await;
    let mut inputs = versions(&docs);
    for (n, input) in inputs.iter_mut().take(2).enumerate() {
        ok(
            &h,
            "PUT",
            &format!("{base}/sources/{}/groups", docs[n]["id"].as_str().unwrap()),
            &owner,
            json!({"group_ids":[environment["id"]]}),
        )
        .await;
        sqlx::query(
            "INSERT INTO source_import_scopes(version_id,brain_id,selection) VALUES($1,$2,$3)",
        )
        .bind(input.source_version_id)
        .bind(brain)
        .bind(json!({"repository_ids":[repo],"area_ids":[],"environment_id":environment["id"]}))
        .execute(&h.admin)
        .await
        .unwrap();
        input.candidate_id = Some(
            organization_shadow_privacy::actual_candidate(
                &h,
                &auth,
                brain,
                input.source_version_id,
                &p,
            )
            .await,
        );
    }
    let mut hybrid = None;
    for mode in [
        Mode::NativeHybrid,
        Mode::NativeWeighted,
        Mode::EntityOverlap,
        Mode::Metadata,
        Mode::EligibleVector,
    ] {
        let (result, projection) = calculate(&h, &auth, brain, &inputs, mode).await;
        assert_eq!(result.sources.len(), 3);
        assert_eq!(
            result
                .sources
                .iter()
                .filter(|s| s.ner_feature_available)
                .count(),
            2
        );
        assert_eq!(
            result
                .sources
                .iter()
                .filter(|s| s.vector_feature_available)
                .count(),
            if matches!(mode, Mode::NativeHybrid | Mode::EligibleVector) {
                3
            } else {
                0
            }
        );
        if mode == Mode::Metadata {
            assert_eq!(
                result.groups.len(),
                2,
                "Unknown metadata must not manufacture a third shared group"
            );
            assert!(result.groups.iter().all(|g| g.source_versions.len() == 2
                && g.label_provenance.kind == "canonical_scope"
                && g.label_provenance.byte_start.is_none()
                && g.label_provenance.quote.is_none()));
            assert!(
                result
                    .groups
                    .iter()
                    .any(|g| g.label == "example.test/shadow/canonical"
                        && g.label_provenance.field_id == Some(repo))
            );
            assert!(
                result
                    .groups
                    .iter()
                    .any(|g| g.label == "Actual scoped shadow PROD")
            );
        }
        if mode == Mode::NativeHybrid {
            hybrid = Some(projection);
        }
        if mode == Mode::EntityOverlap {
            assert!(result.groups.iter().all(|g| g.kind == "entity_facet"));
            assert!(result.gds_version.is_none());
        }
        if mode == Mode::EligibleVector {
            assert!(
                result
                    .groups
                    .iter()
                    .all(|g| g.kind == "native_community" && g.source_versions.len() >= 2)
            );
        }
        let direct = h
            .call(
                "POST",
                &format!("{base}/graph/analytics/{}/view", result.report_id),
                Some(&owner),
                json!({"offset":0}),
            )
            .await;
        assert_eq!(
            direct.0,
            StatusCode::NOT_FOUND,
            "Known unpublished report IDs cannot enter public analytics: {}",
            direct.1
        );
    }
    let listing = ok(
        &h,
        "GET",
        &format!("{base}/graph/analytics"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(listing["reports"], json!([]));
    // An unrelated same-Brain vector is deliberately close enough to change
    // neighbors if cohort restriction were applied only after top-k/centroids.
    let outside = source(
        &h,
        &owner,
        &base,
        "Outside experimental cohort retained source.",
    )
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    organization_hybrid::retained(
        &h,
        brain,
        profile,
        outside["version"]["id"].as_str().unwrap().parse().unwrap(),
        &[1., 0., 0., 0., 0., 0., 0., 0.],
    )
    .await;
    let (_, projection) = calculate(&h, &auth, brain, &inputs, Mode::NativeHybrid).await;
    assert_eq!(
        Some(projection),
        hybrid,
        "An unrequested source cannot enter IDF, centroid, top-k, or node ordering"
    );
    let stored:i64=sqlx::query_scalar("SELECT count(*) FROM knowledge_shadow_sources WHERE report_id=(SELECT id FROM analytics_reports WHERE purpose='organization_shadow' AND state='ready' ORDER BY created_at DESC LIMIT 1)").fetch_one(&h.admin).await.unwrap();
    assert_eq!(stored, 3);
    let production:(i64,i64,i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_mapping_origins WHERE brain_id=$1),(SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$1),(SELECT count(*) FROM knowledge_entities WHERE brain_id=$1),(SELECT count(*) FROM knowledge_topics WHERE brain_id=$1),(SELECT count(*) FROM knowledge_source_inputs WHERE brain_id=$1),(SELECT count(*) FROM knowledge_organization_heads WHERE brain_id=$1)").bind(brain).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        production,
        (0, 0, 0, 0, 0, 0),
        "Unpublished modes cannot create production provenance, entities, topics, source inputs, or heads"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; one actual loopback extraction, zero paid calls"]
async fn organization_shadow_binding_duplicates_no_access_and_empty_features() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    allow(&h, &owner, &base, |p| {
        p["purposes"] = json!(["extraction", "embedding"])
    })
    .await;
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let mut inputs = versions(&docs);
    let candidate = organization_shadow_privacy::actual_candidate(
        &h,
        &auth,
        brain,
        inputs[0].source_version_id,
        &p,
    )
    .await;
    let request: Uuid =
        sqlx::query_scalar("SELECT request_id FROM knowledge_shadow_candidates WHERE id=$1")
            .bind(candidate)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let cx = gateway::Context {
        brain,
        actor: auth.user.id,
        device: None,
    };
    let changed = json!({"language":"en","mentions":[{"key":"architecture","kind":"concept","span":{"quote":"architecture","line_from":1,"line_to":1,"occurrence":null},"confidence":0.5}],"relations":[],"aliases":[]});
    assert!(
        shadow::admit(
            &h.state,
            &auth,
            cx,
            request,
            inputs[0].source_version_id,
            &changed
        )
        .await
        .is_err(),
        "Caller cannot substitute a separately valid candidate for the actual returned mapping"
    );
    assert!(
        shadow::admit(
            &h.state,
            &auth,
            cx,
            request,
            inputs[1].source_version_id,
            &changed
        )
        .await
        .is_err(),
        "Input0 binding is exact"
    );
    assert!(
        shadow::queue(
            &h.state,
            &auth,
            brain,
            &[],
            Mode::NativeHybrid,
            Default::default(),
            "2026.08.1"
        )
        .await
        .is_err()
    );
    assert!(
        shadow::queue(
            &h.state,
            &auth,
            brain,
            &[inputs[0].clone(), inputs[0].clone()],
            Mode::NativeHybrid,
            Default::default(),
            "2026.08.1"
        )
        .await
        .is_err()
    );
    let (_, foreign) = h.fixture_member().await;
    let foreign = organization_shadow_privacy::evaluator(&h, &foreign).await;
    assert!(
        shadow::queue(
            &h.state,
            &foreign,
            brain,
            &inputs,
            Mode::EligibleVector,
            Default::default(),
            "2026.08.1"
        )
        .await
        .is_err()
    );
    inputs[0].candidate_id = Some(candidate);
    let (result, _) = calculate(&h, &auth, brain, &inputs, Mode::NativeHybrid).await;
    sqlx::query("UPDATE model_requests SET detail_expired=true WHERE id=$1")
        .bind(request)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(
        shadow::result(&h.state, &auth, brain, result.report_id)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_shadow_candidates WHERE id=$1"
        )
        .bind(candidate)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    inputs[0].candidate_id = None;
    // Failed/missing NER does not exclude independent retained vectors.
    let (source_only, _) = calculate(&h, &auth, brain, &inputs, Mode::NativeHybrid).await;
    assert!(source_only.sources.iter().all(|s| !s.ner_feature_available));
    assert_eq!(source_only.groups.len(), 1);
    sqlx::query("UPDATE semantic_entries SET state='pending',embedding=NULL,request_id=NULL WHERE brain_id=$1").bind(brain).execute(&h.admin).await.unwrap();
    let (empty, projection) = calculate(&h, &auth, brain, &inputs, Mode::NativeHybrid).await;
    assert!(empty.groups.is_empty());
    assert!(empty.gds_version.is_none());
    assert_eq!(projection["nodes"], json!([]));
    assert!(
        empty.sources.iter().all(|s| s.groups.is_empty()
            && !s.vector_feature_available
            && !s.ner_feature_available)
    );
    let attempts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM analytics_attempts WHERE report_id=$1")
            .bind(empty.report_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        attempts, 0,
        "Empty features must not allocate native scratch"
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j, two real loopback calls and actual expiry journal"]
async fn organization_shadow_concurrent_admission_and_independently_retained_excerpt() {
    let (h, owner, base, p, server) = setup().await;
    let (brain, docs, _) = organization_hybrid::prepare(&h, &owner, &base).await;
    allow(&h, &owner, &base, |p| {
        p["purposes"] = json!(["extraction", "embedding"])
    })
    .await;
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let settings = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = settings["policy"].clone();
    policy["document_days"] = json!(1);
    policy["support_excerpt_days"] = Value::Null;
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":settings["change_id"],"policy":policy}),
    )
    .await;
    let child = ok(&h,"POST",&format!("{base}/excerpts"),&owner,json!({"source_id":docs[0]["id"],"version_id":docs[0]["version"]["id"],"first_line":1,"last_line":1,"title":"Independent shadow excerpt"})).await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    let version: Uuid = child["version"]["id"].as_str().unwrap().parse().unwrap();
    let mapping = json!({"language":"en","mentions":[{"key":"architecture","kind":"concept","span":{"quote":"architecture","line_from":1,"line_to":1,"occurrence":null},"confidence":1.0}],"relations":[],"aliases":[]});
    *p.candidates.lock().unwrap() = json!({"claims":[],"mapping":mapping});
    let cx = gateway::Context {
        brain,
        actor: auth.user.id,
        device: None,
    };
    let response = gateway::invoke(&h.state, cx, gateway::extraction(Uuid::new_v4(), version))
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let (a, b) = tokio::join!(
        shadow::admit(&h.state, &auth, cx, response.request.id, version, &mapping),
        shadow::admit(&h.state, &auth, cx, response.request.id, version, &mapping)
    );
    let a = a.unwrap_or_else(|e| panic!("first: {}", e.1));
    assert_eq!(a, b.unwrap_or_else(|e| panic!("concurrent: {}", e.1)));
    let control = organization_shadow_privacy::actual_candidate(
        &h,
        &auth,
        brain,
        docs[1]["version"]["id"].as_str().unwrap().parse().unwrap(),
        &p,
    )
    .await;
    let parent: Uuid = docs[0]["version"]["id"].as_str().unwrap().parse().unwrap();
    let manifest = json!({"source_versions":[parent]});
    let (natural,erased):(Vec<Uuid>,Vec<Uuid>)=sqlx::query_as("SELECT recollect_shadow_candidates_affected($1,$2,false),recollect_shadow_candidates_affected($1,$2,true)").bind(brain).bind(manifest).fetch_one(&h.admin).await.unwrap();
    assert!(natural.is_empty());
    assert_eq!(erased, vec![a]);
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '3 days' WHERE id=$1",
    )
    .bind(parent)
    .execute(&h.admin)
    .await
    .unwrap();
    recollect_server::privacy_journal::run_once(&h.state)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT privacy_state FROM source_versions WHERE id=$1")
            .bind(parent)
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        "expired"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_shadow_candidates WHERE id=ANY($1)"
        )
        .bind(vec![a, control])
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        2,
        "Natural parent expiry must preserve independently retained candidate payloads"
    );
    let (result, _) = calculate(
        &h,
        &auth,
        brain,
        &[ShadowSource {
            source_version_id: version,
            candidate_id: Some(a),
        }],
        Mode::EntityOverlap,
    )
    .await;
    assert_eq!(result.groups.len(), 1);
    let target = json!({"kind":"source","id":docs[0]["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_shadow_candidates WHERE id=$1"
        )
        .bind(a)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM knowledge_shadow_candidates WHERE id=$1"
        )
        .bind(control)
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        1
    );
    assert!(
        shadow::result(&h.state, &auth, brain, result.report_id)
            .await
            .is_err()
    );
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}
