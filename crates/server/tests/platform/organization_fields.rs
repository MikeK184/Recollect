use super::*;
use recollect_server::{knowledge_mapping::fields, worker};
use review::ok;

async fn setup() -> (Harness, Login, Uuid, String, Value, Value) {
    let h = Harness::new().await;
    let owner = h.login().await;
    let b = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Canonical field origins"}),
    )
    .await;
    let brain = b["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}");
    let prod = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"PROD"}),
    )
    .await;
    let dev = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"DEV"}),
    )
    .await;
    (h, owner, brain, base, prod, dev)
}
async fn source(h: &Harness, owner: &Login, base: &str, title: &str, env: &Value) -> Value {
    let s=ok(h,"POST",&format!("{base}/sources"),owner,json!({"title":title,"media_type":"text/plain","content":"billing-api uses PostgreSQL.\n","retain_content":true})).await;
    ok(
        h,
        "PUT",
        &format!("{base}/sources/{}/groups", s["id"].as_str().unwrap()),
        owner,
        json!({"group_ids":[env["id"]]}),
    )
    .await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    s
}
async fn drain(h: &Harness) {
    for _ in 0..80 {
        let n = fields::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("field schedule {}", e.1));
        let mut worked = false;
        for _ in 0..40 {
            if !worker::run_once(&h.state, "heavy").await.unwrap() {
                break;
            }
            worked = true;
        }
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM knowledge_field_outbox WHERE revision>handled_revision",
        )
        .fetch_one(&h.admin)
        .await
        .unwrap();
        if pending == 0 && n == 0 && !worked {
            return;
        }
    }
    let state: Vec<(i32, String, i64, i64)> =
        sqlx::query_as("SELECT phase,state,revision,handled_revision FROM knowledge_field_outbox")
            .fetch_all(&h.admin)
            .await
            .unwrap();
    panic!("Canonical field refresh did not drain: {state:?}");
}
async fn view(h: &Harness, owner: &Login, base: &str, environment: Value) -> Value {
    ok(h,"POST",&format!("{base}/organization/fields/view"),owner,json!({"scope":{"kind":"knowledge","selection":{"repository_ids":[],"area_ids":[],"environment_id":environment}},"entity_id":null,"cursor":null})).await
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; deterministic field metadata makes zero model calls"]
async fn canonical_fields_preserve_exact_origins_scope_refresh_and_current_labels() {
    let (h, owner, brain, base, prod, dev) = setup().await;
    let s = source(&h, &owner, &base, "Production source", &prod).await;
    let independent = source(&h, &owner, &base, "Independent development source", &dev).await;
    let mut proposal = review::proposal(&s["version"]["id"], "billing-api", "PostgreSQL");
    proposal["content"]["selection"]["environment_id"] = prod["id"].clone();
    proposal["content"]["validity"] =
        json!({"kind":"unknown","from":null,"to":null,"precision":"unknown"});
    let c = ok(&h, "POST", &format!("{base}/claims"), &owner, proposal).await;
    let c=ok(&h,"POST",&format!("{base}/claims/{}/review",c["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":c["id"],"action":"accept","reason":"Synthetic original evidence reviewed.","revalidation_basis":null,"content":null})).await["claims"][0]["revision"].clone();
    drain(&h).await;
    let status = ok(
        &h,
        "POST",
        &format!("{base}/organization/view"),
        &owner,
        json!({"scope":{"kind":"knowledge"}}),
    )
    .await;
    assert_eq!(status["producer_status"], "unconfigured");
    assert_eq!(status["community_status"]["state"], "unconfigured");
    assert!(status["topics"].as_array().unwrap().is_empty());
    let all = view(&h, &owner, &base, Value::Null).await;
    assert_eq!(all["coverage"]["refresh_state"], "ready");
    let entities = all["entities"].as_array().unwrap();
    assert_eq!(
        entities.len(),
        2,
        "Only typed environment UUIDs; free subject/value never become service/technology"
    );
    let prod_entity = entities
        .iter()
        .find(|e| e["reference_id"] == prod["id"])
        .unwrap()["id"]
        .clone();
    assert!(
        all["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["origin_kind"] == "source_context"
                && o["field_pointer"] == "/environments/0"
                && o["value"] == prod["id"])
    );
    assert!(
        all["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["origin_kind"] == "claim_revision"
                && o["revision_id"] == c["id"]
                && o["field_pointer"] == "/content/selection/environment_id"
                && o["value"] == prod["id"])
    );
    for o in all["origins"].as_array().unwrap() {
        assert!(
            all["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["key"] == o["evidence_key"])
        );
        assert!(o.get("byte_start").is_none());
    }
    let scoped = view(&h, &owner, &base, prod["id"].clone()).await;
    assert_eq!(scoped["entities"].as_array().unwrap().len(), 1);
    assert_eq!(scoped["entities"][0]["reference_id"], prod["id"]);
    let before_ids = all["origins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].clone())
        .collect::<Vec<_>>();
    drain(&h).await;
    assert_eq!(
        view(&h, &owner, &base, Value::Null).await["origins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["id"].clone())
            .collect::<Vec<_>>(),
        before_ids,
        "Idempotent drain preserves immutable contributor IDs"
    );
    ok(
        &h,
        "PATCH",
        &format!("{base}/evidence/groups/{}", prod["id"].as_str().unwrap()),
        &owner,
        json!({"name":"Production","description":""}),
    )
    .await;
    let renamed = view(&h, &owner, &base, prod["id"].clone()).await;
    assert_eq!(renamed["entities"][0]["label"], "Production");
    assert_eq!(
        renamed["entities"][0]["id"], prod_entity,
        "Current names hydrate without replacing canonical reference identity"
    );
    ok(
        &h,
        "PUT",
        &format!("{base}/sources/{}/groups", s["id"].as_str().unwrap()),
        &owner,
        json!({"group_ids":[dev["id"]]}),
    )
    .await;
    let stale = view(&h, &owner, &base, Value::Null).await;
    assert!(!stale["origins"].as_array().unwrap().iter().any(|o|o["origin_kind"]=="source_context"&&o["revision_id"]==s["version"]["id"]),"Changed context is hidden before worker refresh");
    drain(&h).await;
    let refreshed = view(&h, &owner, &base, Value::Null).await;
    assert!(
        refreshed["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["origin_kind"] == "source_context"
                && o["revision_id"] == s["version"]["id"]
                && o["value"] == dev["id"])
    );
    assert!(
        refreshed["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["revision_id"] == independent["version"]["id"])
    );
    ok(&h,"POST",&format!("{base}/claims/{}/review",c["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":c["id"],"action":"reject","reason":"Synthetic field qualification correction.","revalidation_basis":null,"content":null})).await;
    assert!(
        !view(&h, &owner, &base, Value::Null).await["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["origin_kind"] == "claim_revision" && o["revision_id"] == c["id"]),
        "A rejected current claim cannot keep an older field contributor visible before refresh"
    );
    drain(&h).await;
    let foreign = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Foreign field namespace"}),
    )
    .await;
    let foreign_view = view(
        &h,
        &owner,
        &format!("/api/brains/{}", foreign["id"].as_str().unwrap()),
        Value::Null,
    )
    .await;
    assert!(foreign_view["entities"].as_array().unwrap().is_empty());
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; old manifests must close field contributors"]
async fn canonical_fields_old_manifest_source_erasure_removes_exact_contributors() {
    let (h, owner, brain, base, prod, _) = setup().await;
    let erased = source(&h, &owner, &base, "Erased source context", &prod).await;
    let independent = source(&h, &owner, &base, "Independent same environment", &prod).await;
    drain(&h).await;
    let before = view(&h, &owner, &base, Value::Null).await;
    let entity = before["entities"][0]["id"].clone();
    assert_eq!(before["entities"][0]["contributor_count"], 2);
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(
            sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM brains WHERE id=$1")
                .bind(brain)
                .fetch_one(&mut *tx)
                .await
                .unwrap()
                .to_string(),
        )
        .execute(&mut *tx)
        .await
        .unwrap();
    let target: Uuid = erased["id"].as_str().unwrap().parse().unwrap();
    let m: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT recollect_privacy_preview($1,'source',$2)")
            .bind(brain)
            .bind(target)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(!m.0["knowledge_field_inputs"].as_array().unwrap().is_empty());
    let mut old = m.0;
    old.as_object_mut()
        .unwrap()
        .remove("knowledge_field_inputs");
    let request = Uuid::new_v4();
    sqlx::query("INSERT INTO privacy_requests(id,brain_id,actor_id,target,cause,manifest) VALUES($1,$2,recollect_actor(),$3,'erase',$4)").bind(request).bind(brain).bind(json!({"kind":"source","id":target})).bind(old).execute(&mut *tx).await.unwrap();
    sqlx::query("SELECT recollect_privacy_apply($1)")
        .bind(request)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM knowledge_field_inputs WHERE brain_id=$1 AND revision_id=$2",
    )
    .bind(brain)
    .bind(
        erased["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
    let after = view(&h, &owner, &base, Value::Null).await;
    assert_eq!(after["entities"][0]["id"], entity);
    assert_eq!(after["entities"][0]["contributor_count"], 1);
    assert_eq!(
        after["origins"][0]["revision_id"],
        independent["version"]["id"]
    );
    drain(&h).await;
    assert_eq!(
        view(&h, &owner, &base, Value::Null).await["entities"][0]["contributor_count"],
        1,
        "Refresh cannot resurrect an erased field context"
    );
    h.finish().await;
}

async fn canonical_repository(
    h: &Harness,
    brain: Uuid,
    origin: &str,
    revision: char,
) -> (Uuid, Uuid) {
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let repo = Uuid::new_v4();
    let snapshot = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)",
    )
    .bind(repo)
    .bind(brain)
    .bind(origin)
    .bind(actor)
    .execute(&h.admin)
    .await
    .unwrap();
    sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by) VALUES($1,$2,$3,$4,'fixture','fixture','fixture','{}','{}',1,1,0,$5)").bind(snapshot).bind(brain).bind(repo).bind(revision.to_string().repeat(40)).bind(actor).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) VALUES($1,$2,$3,0,$4)").bind(Uuid::new_v4()).bind(brain).bind(snapshot).bind(json!({"kind":"unknown-open-kind","name":"untyped structural evidence","file":"main.rs","line":1})).execute(&h.admin).await.unwrap();
    (repo, snapshot)
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; canonical repo/manifest fields and old closure"]
async fn canonical_fields_repository_manifest_and_original_pointer_order() {
    let (h, owner, brain, base, prod, _) = setup().await;
    let (a, sa) = canonical_repository(&h, brain, "example.test/fields/a", 'a').await;
    let (b, sb) = canonical_repository(&h, brain, "example.test/fields/b", 'b').await;
    let manifest=ok(&h,"POST",&format!("{base}/revision-manifests"),&owner,json!({"name":"Exact canonical manifest","environment_id":prod["id"],"kind":"committed","entries":[{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]},{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]}],"notes":"","base_revision":null})).await;
    drain(&h).await;
    let v = view(&h, &owner, &base, Value::Null).await;
    assert_eq!(
        v["entities"].as_array().unwrap().len(),
        3,
        "Only two canonical repositories and one canonical environment, regardless of unknown extractor kinds"
    );
    let origins = v["origins"].as_array().unwrap();
    assert!(
        origins
            .iter()
            .any(|o| o["origin_kind"] == "repository_snapshot"
                && o["revision_id"] == json!(sa)
                && o["field_pointer"] == "/repository_id"
                && o["value"] == json!(a)),
        "Actual repository snapshot contributes its declared UUID: {v}"
    );
    let entries = manifest["entries"].as_array().unwrap();
    for (n, entry) in entries.iter().enumerate() {
        assert!(
            origins
                .iter()
                .any(|o| o["origin_kind"] == "manifest_revision"
                    && o["revision_id"] == manifest["id"]
                    && o["field_pointer"] == json!(format!("/entries/{n}/repository_id"))
                    && o["value"] == entry["repository_id"]),
            "Field index must preserve the actual canonical manifest order: {v}"
        );
    }
    for entity in v["entities"].as_array().unwrap() {
        assert_eq!(
            entity["label_pointer"],
            if entity["kind"] == "repository" {
                "/canonical_origin"
            } else {
                "/name"
            }
        );
    }
    let scoped=ok(&h,"POST",&format!("{base}/organization/fields/view"),&owner,json!({"scope":{"kind":"knowledge","selection":{"repository_ids":[a],"environment_id":null,"area_ids":[]}}})).await;
    assert!(
        scoped["origins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["value"] != json!(b)),
        "Eligible multi-repo manifests cannot leak another repository reference"
    );
    // An old journal manifest has no new field derivative IDs. Canonical IDs
    // still close the exact repository and dependent manifest contributors.
    let mut tx = h.admin.begin().await.unwrap();
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(actor.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let old: sqlx::types::Json<Value> = sqlx::query_scalar(
        "SELECT recollect_privacy_preview($1,'snapshot',$2)-'knowledge_field_inputs'",
    )
    .bind(brain)
    .bind(sa)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    let rid = Uuid::new_v4();
    sqlx::query("INSERT INTO privacy_requests(id,brain_id,actor_id,target,cause,manifest) VALUES($1,$2,$3,$4,'erase',$5)").bind(rid).bind(brain).bind(actor).bind(json!({"kind":"snapshot","id":sa})).bind(old).execute(&mut *tx).await.unwrap();
    sqlx::query("SELECT recollect_privacy_apply($1)")
        .bind(rid)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let after = view(&h, &owner, &base, Value::Null).await;
    assert!(
        !after["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["revision_id"] == json!(sa) || o["revision_id"] == manifest["id"])
    );
    assert!(
        after["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["revision_id"] == json!(sb)),
        "Independent canonical repository survives exact erasure"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; queued actor revocation must fail closed"]
async fn canonical_fields_queued_writer_loss_cannot_publish() {
    let (h, owner, brain, base, prod, _) = setup().await;
    drain(&h).await;
    let (member, writer) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{member}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    let s = source(&h, &writer, &base, "Writer-origin field context", &prod).await;
    let prior: i64 =
        sqlx::query_scalar("SELECT revision FROM knowledge_field_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    sqlx::query("UPDATE accounts SET enabled=false WHERE id=$1")
        .bind(member)
        .execute(&h.admin)
        .await
        .unwrap();
    let disabled = view(&h, &owner, &base, Value::Null).await;
    assert_eq!(disabled["coverage"]["refresh_state"], "blocked");
    assert_eq!(
        fields::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("field schedule {}", e.1)),
        0
    );
    let changed: i64 =
        sqlx::query_scalar("SELECT revision FROM knowledge_field_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(
        changed > prior,
        "Account authority changes durably invalidate field publication"
    );
    sqlx::query("UPDATE accounts SET enabled=true WHERE id=$1")
        .bind(member)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        fields::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("field schedule {}", e.1)),
        1
    );
    let job = worker::claim(&h.state.pool, "heavy")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.kind, "knowledge.fields");
    assert_eq!(job.actor_id, member);
    // Canonical authority change by the owned admin connection has no
    // originating application writer; it cannot silently adopt the owner.
    sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
        .bind(brain)
        .bind(member)
        .execute(&h.admin)
        .await
        .unwrap();
    let failure = worker::execute(&h.state, &job).await.unwrap_err();
    assert_eq!(failure, worker::Failure::Revoked);
    assert!(worker::fail(&h.state.pool, &job, failure).await.unwrap());
    let phase: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM jobs WHERE id=$1")
            .bind(job.id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(phase.0, "cancelled");
    let v = view(&h, &owner, &base, Value::Null).await;
    assert_eq!(v["coverage"]["refresh_state"], "blocked");
    assert!(
        !v["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["revision_id"] == s["version"]["id"])
    );
    assert_eq!(
        fields::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("field schedule {}", e.1)),
        0,
        "No valid originating writer means no publication"
    );
    // A later authorized canonical change gives the refresh a real current
    // writer; a system identity never bypasses the revoked writer boundary.
    ok(
        &h,
        "PATCH",
        &format!("{base}/evidence/groups/{}", prod["id"].as_str().unwrap()),
        &owner,
        json!({"name":"PROD current owner","description":""}),
    )
    .await;
    drain(&h).await;
    assert!(
        !view(&h, &owner, &base, Value::Null).await["entities"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j; bounded field details, cursor expiry and retention"]
async fn canonical_fields_paging_and_retention_cannot_replay_old_labels() {
    let (h, owner, brain, base, _, _) = setup().await;
    let mut snapshots = vec![];
    for n in 0..85 {
        let (_, snapshot) =
            canonical_repository(&h, brain, &format!("example.test/field-page/{n}"), 'a').await;
        snapshots.push(snapshot);
    }
    drain(&h).await;
    let first = view(&h, &owner, &base, Value::Null).await;
    assert_eq!(first["origins"].as_array().unwrap().len(), 80);
    assert_eq!(first["coverage"]["qualified_inputs"], 85);
    assert_eq!(first["coverage"]["counts_are_lower_bounds"], true);
    let cursor = first["next_cursor"].clone();
    assert!(cursor.is_object());
    let second = ok(
        &h,
        "POST",
        &format!("{base}/organization/fields/view"),
        &owner,
        json!({"scope":first["scope"],"cursor":cursor}),
    )
    .await;
    assert_eq!(second["origins"].as_array().unwrap().len(), 5);
    assert!(second["next_cursor"].is_null());
    let unique: std::collections::BTreeSet<_> = first["origins"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["origins"].as_array().unwrap())
        .map(|o| o["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        unique.len(),
        85,
        "Native detail paging neither duplicates nor loses references"
    );
    let mut expired = cursor.clone();
    expired["expires_at"] = json!("2000-01-01T00:00:00Z");
    let (status, body, _) = h
        .call(
            "POST",
            &format!("{base}/organization/fields/view"),
            Some(&owner),
            json!({"scope":first["scope"],"cursor":expired}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "organization_changed");
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["repository_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '2 days' WHERE id=$1").bind(snapshots[0]).execute(&h.admin).await.unwrap();
    let current = view(&h, &owner, &base, Value::Null).await;
    assert!(
        !current["expires_at"].is_null(),
        "Every retained repository representation carries its actual deadline"
    );
    assert!(
        !current["origins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["revision_id"] == json!(snapshots[0]))
    );
    let stale = h
        .call(
            "POST",
            &format!("{base}/organization/fields/view"),
            Some(&owner),
            json!({"scope":first["scope"],"cursor":cursor}),
        )
        .await;
    assert_eq!(
        stale.0,
        StatusCode::CONFLICT,
        "Policy epoch changes invalidate old cursor labels"
    );
    drain(&h).await;
    let removed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM knowledge_field_inputs WHERE brain_id=$1 AND revision_id=$2",
    )
    .bind(brain)
    .bind(snapshots[0])
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        removed, 0,
        "Automatic refresh physically removes an expired contributor"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL; exact canonical chunk preflight, no model calls"]
async fn canonical_embedding_fixture_chunk_preflight_preserves_all_twenty_documents() {
    canonical_chunk_preflight(
        include_str!(
            "../../../../docs/research/graph-organization-adversarial-fixtures-2026-10-18.json"
        ),
        "canonical-chunk-preflight.json",
        20,
        21,
    )
    .await;
}
#[tokio::test]
#[ignore = "Requires owned PostgreSQL; exact fresh held-out chunk preflight, no model calls"]
async fn canonical_embedding_fresh_heldout_preflight_preserves_all_twelve_documents() {
    canonical_chunk_preflight(
        include_str!(
            "../../../../docs/research/graph-organization-fresh-heldout-v1-2026-10-18.json"
        ),
        "canonical-fresh-chunk-preflight.json",
        12,
        12,
    )
    .await;
}
async fn canonical_chunk_preflight(raw: &str, receipt: &str, documents: usize, chunks: usize) {
    let (h, owner, brain, base, _, _) = setup().await;
    let corpus: Value = serde_json::from_str(raw).unwrap();
    assert_eq!(corpus["documents"].as_array().unwrap().len(), documents);
    let mut manifest = vec![];
    for document in corpus["documents"].as_array().unwrap() {
        let source=ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":format!("Frozen vector source {}",document["id"].as_str().unwrap()),"media_type":"text/plain","content":document["text"],"retain_content":true})).await;
        while worker::run_once(&h.state, "capture").await.unwrap() {}
        let version: Uuid = source["version"]["id"].as_str().unwrap().parse().unwrap();
        let rows:Vec<(i32,i32,i32,String,String)>=sqlx::query_as("SELECT ordinal,byte_start,byte_end,content,encode(sha256(convert_to(content,'UTF8')),'hex') FROM source_chunks WHERE brain_id=$1 AND version_id=$2 ORDER BY ordinal").bind(brain).bind(version).fetch_all(&h.admin).await.unwrap();
        let mut joined = String::new();
        let mut previous = 0;
        for (ordinal, start, end, text, hash) in rows {
            assert_eq!(start, previous);
            assert_eq!(end - start, text.len() as i32);
            assert!(text.len() <= 4096);
            assert_eq!(
                document["text"]
                    .as_str()
                    .unwrap()
                    .get(start as usize..end as usize),
                Some(text.as_str())
            );
            joined.push_str(&text);
            previous = end;
            manifest.push(json!({"document_id":document["id"],"partition":document["partition"],"ordinal":ordinal,"byte_start":start,"byte_end":end,"chunk_sha256":hash}));
        }
        assert_eq!(
            joined,
            document["text"].as_str().unwrap(),
            "Every original byte, including late anchors and unsupported language, remains in canonical chunks"
        );
    }
    assert_eq!(manifest.len(), chunks);
    let h06: Vec<_> = manifest
        .iter()
        .filter(|m| m["document_id"] == "h06")
        .collect();
    if documents == 20 {
        assert_eq!(h06.len(), 2);
        assert_eq!(h06[0]["byte_start"], 0);
        assert_eq!(h06[0]["byte_end"], 4096);
        assert_eq!(h06[1]["byte_start"], 4096);
        assert_eq!(h06[1]["byte_end"], 5647);
    }
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(calls, 0);
    let hash: String = sqlx::query_scalar("SELECT encode(sha256($1::bytea),'hex')")
        .bind(serde_json::to_vec(&manifest).unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let source_hash: String = sqlx::query_scalar("SELECT encode(sha256($1::bytea),'hex')")
        .bind(std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/src/evidence.rs")).unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.cache/automatic-mapping")
        .join(receipt);
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(out,serde_json::to_vec_pretty(&json!({"status":"passed","canonical_documents":documents,"canonical_chunks":chunks,"model_calls":0,"database":"owned_disposable_fixture","manifest_sha256":hash,"source_splitter_sha256":source_hash,"manifest":manifest,"embedding_vectors":"not_requested_not_measured"})).unwrap()).unwrap();
    h.finish().await;
}
