//! Independent native proof for the guarded qualification runner setup.
use super::*;
use axum::{Json, extract::FromRequestParts};
use recollect_server::{auth::Auth, graph::analytics::shadow, model_gateway as gateway};

#[path = "../../examples/mapping_qualification/support/reconciliation.rs"]
mod fixture;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; all provider requests are controlled loopback"]
async fn reconciliation_controls_use_real_prior_workflow_targets_and_canonical_gateway() {
    let h = Harness::new().await;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join(fixture::CONTROLS_PATH)).unwrap();
    let hash: String = sqlx::query_scalar("SELECT encode(sha256($1),'hex')")
        .bind(&bytes)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(hash, fixture::CONTROLS_SHA256);
    let controls: Value = serde_json::from_slice(&bytes).unwrap();
    let mut receipts = vec![];
    let mut offered_count = 0;
    let mut future_sizes = Value::Null;
    for case in controls["cases"].as_array().unwrap() {
        let p = fixture::setup_case(&h.state, &h.admin, case).await.unwrap();
        if future_sizes.is_null() {
            future_sizes = fixture::product_future_sizes(&h.state, &p);
            eprintln!("Product future-size diagnostic: {future_sizes}");
        }
        let probe = fixture::prove_current_input_boundary(&h.state, &h.admin, &p)
            .await
            .unwrap();
        assert_eq!(
            p.canonical_inputs.len(),
            if p.case_id == "rc06" { 1 } else { 2 }
        );
        assert_eq!(p.preview.policy_id, p.policy_id);
        assert_eq!(p.run.selection.environment_id, Some(p.environment_id));
        assert_eq!(p.run.actor_id, p.actor_id);
        let expected_enum = std::iter::once(Value::Null)
            .chain(p.offered.iter().map(|id| json!(id)))
            .collect::<Vec<_>>();
        assert_eq!(
            p.preview.body["text"]["format"]["schema"]["properties"]["claims"]["items"]["properties"]
                ["replaces_revision"]["enum"],
            json!(expected_enum)
        );
        assert!(!p.device_id.is_nil() && !p.operation_id.is_nil());
        let input0 = &p.canonical_inputs[0];
        assert_eq!(input0["data"], case["source"]["text"]);
        assert!(!input0.to_string().contains("Cassandra"));
        assert!(!p.preview.body.to_string().contains("claims_required"));
        assert!(!p.preview.body.to_string().contains("eligible_to_offer"));
        assert!(!p.preview.body.to_string().contains("held_out"));
        assert!(
            !p.preview
                .body
                .to_string()
                .contains("canonical_postconditions")
        );
        if p.case_id == "rc06" {
            assert!(p.offered.is_empty());
            assert_eq!(
                p.preview.body["text"]["format"]["schema"]["properties"]["retirements"]["maxItems"],
                0
            );
            assert!(p.prior_workflow_receipt["review_decision_id"].is_string());
            assert_eq!(p.prior_workflow_receipt["loopback_calls"], 0);
        } else {
            assert_eq!(p.offered, vec![p.prior_revision_id]);
            assert_eq!(p.prior_workflow_receipt["loopback_calls"], 2);
            assert!(p.prior_workflow_receipt["reviewer_id"].is_null());
            assert!(
                p.canonical_inputs[1]["data"]
                    .as_str()
                    .unwrap()
                    .contains("Cassandra")
            );
            offered_count += 1;
        }
        let schema_hash: String = sqlx::query_scalar("SELECT encode(sha256($1),'hex')")
            .bind(serde_json::to_vec(&p.preview.body["text"]["format"]["schema"]).unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
        let body_hash: String = sqlx::query_scalar("SELECT encode(sha256($1),'hex')")
            .bind(serde_json::to_vec(&p.preview.body).unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
        receipts.push(json!({"id":p.case_id,"brain_id":p.brain_id,"canonical_id_map":p.canonical_id_map,
            "source_version_id":p.run.source_version_id,"prior_revision_id":p.prior_revision_id,"offered":p.offered,
            "actual_input_count":p.canonical_inputs.len(),"prior_loopback_calls":p.prior_workflow_receipt["loopback_calls"],
            "current_loopback_calls":probe["loopback_current_calls"],"schema_sha256":schema_hash,"body_sha256":body_hash,
            "canonical_prior_unchanged":probe["canonical_prior_unchanged"],"source_role":p.source.provenance["role"],
            "capture_binding_id":p.source.provenance["capture"]["binding_id"]}));
        fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
    }
    assert_eq!(offered_count, 5);
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        requests, 16,
        "Five real prior extraction/support pairs plus six current empty boundary probes"
    );
    let receipt = json!({"controls_sha256":hash,"cases":receipts,"eligible_machine_memories_offered":5,"reviewed_manual_memories_offered":0,"product_future_sizes":future_sizes,
        "loopback_requests":requests,"paid_requests":0,"real_prior_pipeline":true,"actual_worker_preview_body_equality":true,
        "semantic_quality_measured":false,"normal_data_copied":false});
    let path = root.join(".cache/automatic-mapping/reconciliation-canonical-preflight.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    h.finish().await;
}

async fn selected(
    h: &Harness,
    run: &recollect_protocol::LearningRun,
) -> Result<Vec<Uuid>, &'static str> {
    let mut tx = recollect_server::db::preparation_tx(&h.state.pool, run.actor_id, run.device_id)
        .await
        .map_err(|e| e.1)?;
    let result = recollect_server::autonomous::targets(&h.state, &mut tx, run, 16384)
        .await
        .map_err(|e| e.1);
    tx.commit().await.unwrap();
    result
}
fn controls() -> Value {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    serde_json::from_slice(&std::fs::read(root.join(fixture::CONTROLS_PATH)).unwrap()).unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; controlled loopback only"]
async fn reconciliation_excerpt_lineage_preserves_exact_scope_current_head_and_privacy() {
    let corpus = controls();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "rc04")
        .unwrap();
    let h = Harness::new().await;
    let owner = h.login().await;
    let p = fixture::setup_case(&h.state, &h.admin, case).await.unwrap();
    eprintln!("Lineage negative phase: setup_ready");
    assert_eq!(
        selected(&h, &p.run).await.unwrap(),
        vec![p.prior_revision_id]
    );
    let mut foreign_scope = p.run.clone();
    foreign_scope.selection.environment_id = None;
    assert!(selected(&h, &foreign_scope).await.unwrap().is_empty());
    let mut foreign_brain = p.run.clone();
    foreign_brain.brain_id = Uuid::new_v4();
    assert!(selected(&h, &foreign_brain).await.is_err());
    eprintln!("Lineage negative phase: scope_and_brain_checked");
    let base = format!("/api/brains/{}", p.brain_id);
    review::ok(&h,"POST",&format!("{base}/claims/{}/review",p.prior_claim_id),&owner,
        json!({"base_revision":p.prior_revision_id,"action":"revalidate","reason":"Explicit human current-head control.","content":null,"revalidation_basis":"review_correction"})).await;
    eprintln!("Lineage negative phase: human_revalidation_ready");
    assert!(
        selected(&h, &p.run).await.unwrap().is_empty(),
        "Historical machine head and newly reviewed current head are both excluded"
    );
    fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
    eprintln!("Lineage negative phase: reviewed_case_archived");
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM jobs WHERE lane='model' AND state IN ('queued','running')",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        pending, 0,
        "Archive uses actual exact job cancellation after extraction-only outcome"
    );
    h.finish().await;
    eprintln!("Lineage negative phase: reviewed_case_cleaned");
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; controlled loopback only"]
async fn reconciliation_excerpt_lineage_preserves_independent_retention() {
    let corpus = controls();
    let h = Harness::new().await;
    let same_source = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "rc01")
        .unwrap();
    let p = fixture::setup_case(&h.state, &h.admin, same_source)
        .await
        .unwrap();
    let old_version: Uuid = p.canonical_id_map["prior_version"]["actual"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // Fixture-only natural maintenance state; never a model-history promotion.
    sqlx::query("UPDATE source_versions SET privacy_state='expired' WHERE brain_id=$1 AND id=$2")
        .bind(p.brain_id)
        .bind(old_version)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        selected(&h, &p.run).await.unwrap(),
        vec![p.prior_revision_id],
        "Independent retained excerpt survives natural parent expiry"
    );
    fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; controlled loopback only"]
async fn reconciliation_excerpt_lineage_honors_original_erasure_and_fence() {
    let corpus = controls();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "rc04")
        .unwrap();
    let h = Harness::new().await;
    let owner = h.login().await;
    let p = fixture::setup_case(&h.state, &h.admin, case).await.unwrap();
    let old_version: Uuid = p.canonical_id_map["prior_version"]["actual"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let old_source: Uuid = p.canonical_id_map["prior_source"]["actual"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let base = format!("/api/brains/{}", p.brain_id);
    let target = json!({"kind":"source","id":old_source});
    let preview = review::ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    eprintln!("Lineage negative phase: erasure_preview_ready");
    review::ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert!(
        selected(&h, &p.run).await.unwrap().is_empty(),
        "Exact parent erasure/fence cannot leave a target through its excerpt"
    );
    let fenced: bool = sqlx::query_scalar("SELECT recollect_source_model_fenced($1,$2)")
        .bind(p.brain_id)
        .bind(old_version)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(
        fenced,
        "An erased/missing original is not an authorized model source"
    );
    fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
    eprintln!("Lineage negative phase: erased_case_archived");
    h.finish().await;
    eprintln!("Lineage negative phase: erased_case_cleaned");
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; controlled loopback only"]
async fn reconciliation_excerpt_lineage_honors_inherited_transmission_fence() {
    let corpus = controls();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "rc04")
        .unwrap();
    let h = Harness::new().await;
    let owner = h.login().await;
    let p = fixture::setup_case(&h.state, &h.admin, case).await.unwrap();
    assert_eq!(
        selected(&h, &p.run).await.unwrap(),
        vec![p.prior_revision_id]
    );
    let base = format!("/api/brains/{}", p.brain_id);
    let unrelated=review::ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Owned fence receipt","media_type":"text/plain","content":"Independent synthetic privacy control.\n","retain_content":true})).await;
    let target = json!({"kind":"source","id":unrelated["id"]});
    let preview = review::ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let removal = review::ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let old_version: Uuid = p.canonical_id_map["prior_version"]["actual"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // Explicit negative fault injection in an owned fixture, matching existing
    // ancestry tests. This is a source-transmission fence, never model history
    // promotion or a claim that the unrelated erasure command created it.
    sqlx::query(
        "INSERT INTO model_input_fences(brain_id,source_version_id,request_id) VALUES($1,$2,$3)",
    )
    .bind(p.brain_id)
    .bind(old_version)
    .bind(removal["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    let retained: bool = sqlx::query_scalar(
        "SELECT privacy_state='active' FROM source_versions WHERE brain_id=$1 AND id=$2",
    )
    .bind(p.brain_id)
    .bind(old_version)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(
        retained,
        "The original bytes are still retained; a real fence explains exclusion"
    );
    assert!(selected(&h, &p.run).await.unwrap().is_empty());
    fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
    h.finish().await;
}

/// The provider receives only the actual resolved input0 and typed offered
/// memory. Its one fixed synthetic mapping is plumbing, never quality evidence.
async fn current_shadow_candidate(
    h: &Harness,
    p: &fixture::PreparedControl,
    device: Uuid,
) -> (gateway::Context, Uuid, Value) {
    use axum::{extract::State, routing::post};
    use std::sync::{Arc, Mutex};
    type Wire = (Value, Arc<Mutex<Vec<Value>>>);
    async fn wire(State((output, bodies)): State<Wire>, Json(body): Json<Value>) -> Json<Value> {
        bodies.lock().unwrap().push(body.clone());
        Json(
            json!({"model":body["model"],"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":output.to_string()}]}],
            "usage":{"input_tokens":23,"output_tokens":17,"total_tokens":40}}),
        )
    }
    let quote = p.source.text.lines().next().unwrap();
    let mapping = json!({"language":"en","mentions":[{"key":"current","kind":"concept",
        "span":{"quote":quote,"line_from":1,"line_to":1,"occurrence":null},"confidence":1.0}],"relations":[],"aliases":[]});
    let bodies = Arc::new(Mutex::new(vec![]));
    let router = Router::new()
        .route("/v1/responses", post(wire))
        .with_state((
            json!({"claims":[],"retirements":[],"mapping":mapping}),
            bodies.clone(),
        ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(address.ip().is_loopback());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = h.state.config.as_ref().clone();
    config.models.endpoint = format!("http://{address}/v1");
    config.models.key = Some("synthetic-loopback-fixture-only".into());
    config.models.openrouter_key = None;
    let state = AppState::new(h.state.pool.clone(), config).unwrap();
    let cx = gateway::Context {
        brain: p.brain_id,
        actor: p.actor_id,
        device: Some(device),
    };
    let invocation = recollect_server::autonomous::invocation(&p.run);
    let preview = gateway::Preview {
        policy_id: p.preview.policy_id,
        body: p.preview.body.clone(),
    };
    let response = tokio::spawn(async move {
        gateway::invoke_previewed_for_policy(&state, cx, invocation, &preview).await
    })
    .await
    .unwrap()
    .unwrap_or_else(|e| panic!("Current controlled gateway: {}", e.1));
    server.abort();
    assert_eq!(response.request.state, "succeeded");
    assert_eq!(bodies.lock().unwrap().as_slice(), &[p.preview.body.clone()]);
    (cx, response.request.id, mapping)
}

async fn actual_evaluator(h: &Harness, owner: &Login) -> Auth {
    let (mut parts, _) = Request::builder()
        .method("POST")
        .header("cookie", &owner.cookie)
        .header("x-csrf-token", &owner.csrf)
        .body(())
        .unwrap()
        .into_parts();
    Auth::from_request_parts(&mut parts, &h.state)
        .await
        .unwrap_or_else(|e| panic!("Actual evaluator: {}", e.1))
}

async fn other_control_device(h: &Harness, p: &fixture::PreparedControl) -> Uuid {
    // Reuse an actual still-live producer browser session only in this owned
    // fixture. Pair through the real API; no token/session is exported.
    let (session, csrf): (Uuid, Uuid) = sqlx::query_as(
        "SELECT token,csrf_token FROM sessions WHERE account_id=$1 AND expires_at>clock_timestamp() ORDER BY created_at DESC LIMIT 1",
    ).bind(p.actor_id).fetch_one(&h.admin).await.unwrap();
    let producer_browser = Login {
        cookie: format!("recollect_session={session}"),
        csrf: csrf.to_string(),
    };
    h.pair_device(&producer_browser, "Owned unrelated producer device")
        .await
        .0
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL; actual prior/support + one current loopback extraction, zero paid calls"]
async fn reconciliation_shadow_rechecks_paired_producer_current_head_and_own_support_scope() {
    let corpus = controls();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "rc01")
        .unwrap();
    let h = Harness::new().await;
    let owner = h.login().await;
    let p = fixture::setup_case(&h.state, &h.admin, case).await.unwrap();
    let base = format!("/api/brains/{}", p.brain_id);
    let auth = actual_evaluator(&h, &owner).await;
    assert_ne!(
        auth.user.id, p.actor_id,
        "Evaluator is independently authenticated"
    );
    let (producer, request, mapping) = current_shadow_candidate(&h, &p, p.device_id).await;
    let mut browser_producer = producer;
    browser_producer.device = None;
    assert!(
        shadow::admit(
            &h.state,
            &auth,
            browser_producer,
            request,
            p.run.source_version_id,
            &mapping
        )
        .await
        .is_err(),
        "A paired producer cannot be substituted by a browser identity"
    );
    let mut other_producer = producer;
    other_producer.actor = auth.user.id;
    assert!(
        shadow::admit(
            &h.state,
            &auth,
            other_producer,
            request,
            p.run.source_version_id,
            &mapping
        )
        .await
        .is_err(),
        "Evaluator authority cannot replace the actual producing actor"
    );
    let candidate = shadow::admit(
        &h.state,
        &auth,
        producer,
        request,
        p.run.source_version_id,
        &mapping,
    )
    .await
    .unwrap_or_else(|e| panic!("Actual paired producer: {}", e.1));
    let other_device = other_control_device(&h, &p).await;
    let unrelated_producer = gateway::Context {
        device: Some(other_device),
        ..producer
    };
    assert!(
        shadow::admit(
            &h.state,
            &auth,
            unrelated_producer,
            request,
            p.run.source_version_id,
            &mapping
        )
        .await
        .is_err(),
        "Another actual claimed device cannot replace original request attribution"
    );
    assert_eq!(
        selected(&h, &p.run).await.unwrap(),
        vec![p.prior_revision_id]
    );
    let (support_source, support_version): (Uuid, Uuid) = sqlx::query_as(
        "SELECT v.source_id,v.id FROM claim_supports s JOIN source_versions v ON v.brain_id=s.brain_id AND v.id=s.source_version_id WHERE s.brain_id=$1 AND s.revision_id=$2 AND v.retention_class='support_excerpt' ORDER BY v.id",
    ).bind(p.brain_id).bind(p.prior_revision_id).fetch_one(&h.admin).await.unwrap();
    assert_ne!(support_source, p.source.source_id);
    let recall = json!({"exact":{"kind":"source_version","id":support_version},"selection":p.run.selection,"mode":"investigation","context_bytes":12000,"limit":12});
    let before = review::ok(
        &h,
        "POST",
        &format!("{base}/recall"),
        &owner,
        recall.clone(),
    )
    .await;
    assert!(
        before["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == json!(support_version)),
        "Exact canonical scoped recall initially includes the retained support itself"
    );
    let dev = review::ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Owned support moved to DEV"}),
    )
    .await;
    review::ok(
        &h,
        "PUT",
        &format!("{base}/sources/{support_source}/groups"),
        &owner,
        json!({"group_ids":[dev["id"]]}),
    )
    .await;
    let after = review::ok(&h, "POST", &format!("{base}/recall"), &owner, recall).await;
    assert!(
        !after["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == json!(support_version)),
        "Actual canonical PROD recall excludes the explicitly re-scoped retained support"
    );
    let detail = review::ok(
        &h,
        "GET",
        &format!("{base}/claims/{}", p.prior_claim_id),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        detail["selected"]["revision"]["id"],
        json!(p.prior_revision_id)
    );
    assert_eq!(
        detail["selected"]["eligibility"]["investigation"], true,
        "Full-Brain canonical validity survives; exact scoped applicability explains exclusion"
    );
    assert!(
        detail["selected"]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["availability"] == "retained")
    );
    let claim_recall = review::ok(&h,"POST",&format!("{base}/recall"),&owner,
        json!({"exact":{"kind":"claim","id":p.prior_claim_id},"selection":p.run.selection,"mode":"investigation","context_bytes":12000,"limit":12})).await;
    assert!(
        claim_recall["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == json!(p.prior_claim_id)),
        "Canonical claim applicability retains its own original PROD selection despite support-source navigation group changes"
    );
    let current = selected(&h, &p.run).await.unwrap();
    let readmit = shadow::admit(
        &h.state,
        &auth,
        producer,
        request,
        p.run.source_version_id,
        &mapping,
    )
    .await;
    let requeue = shadow::queue(
        &h.state,
        &auth,
        p.brain_id,
        &[shadow::ShadowSource {
            source_version_id: p.run.source_version_id,
            candidate_id: Some(candidate),
        }],
        shadow::Mode::EntityOverlap,
        recollect_server::knowledge_mapping::source_features::HybridParameters::default(),
        "2026.08.1",
    )
    .await;
    eprintln!(
        "Own-support current-scope control: source_recall_omits=true canonical_prod_claim_retained=true targets_count={} readmit_allowed={} requeue_allowed={}",
        current.len(),
        readmit.is_ok(),
        requeue.is_ok()
    );
    assert_eq!(
        current,
        vec![p.prior_revision_id],
        "Targets must preserve actual canonical claim applicability rather than invent source-group history semantics"
    );
    assert_eq!(
        readmit.unwrap_or_else(|e| panic!("Canonical claim scope: {}", e.1)),
        candidate
    );
    assert!(
        requeue.is_ok(),
        "A currently applicable canonical claim remains an eligible original offer"
    );
    review::ok(
        &h,
        "PUT",
        &format!("{base}/sources/{support_source}/groups"),
        &owner,
        json!({"group_ids":[p.environment_id]}),
    )
    .await;
    assert_eq!(
        selected(&h, &p.run).await.unwrap(),
        vec![p.prior_revision_id]
    );
    assert_eq!(
        shadow::admit(
            &h.state,
            &auth,
            producer,
            request,
            p.run.source_version_id,
            &mapping
        )
        .await
        .unwrap_or_else(|e| panic!("Restored support scope: {}", e.1)),
        candidate
    );
    review::ok(&h,"POST",&format!("{base}/claims/{}/review",p.prior_claim_id),&owner,
        json!({"base_revision":p.prior_revision_id,"action":"revalidate","reason":"Explicit current-head negative control.","content":null,"revalidation_basis":"review_correction"})).await;
    assert!(selected(&h, &p.run).await.unwrap().is_empty());
    assert!(
        shadow::admit(
            &h.state,
            &auth,
            producer,
            request,
            p.run.source_version_id,
            &mapping
        )
        .await
        .is_err(),
        "An old offered revision cannot be re-admitted after actual human head correction"
    );
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(p.brain_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        calls, 3,
        "One real prior extraction/support pair plus one current original-device controlled extraction"
    );
    fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL; real capture lineage and controlled loopback only"]
async fn reconciliation_shadow_capture_origin_requires_its_actual_paired_device() {
    let corpus = controls();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "rc04")
        .unwrap();
    for unrelated in [false, true] {
        let h = Harness::new().await;
        let owner = h.login().await;
        let p = fixture::setup_case(&h.state, &h.admin, case).await.unwrap();
        let auth = actual_evaluator(&h, &owner).await;
        let device = if unrelated {
            other_control_device(&h, &p).await
        } else {
            p.device_id
        };
        let (producer, request, mapping) = current_shadow_candidate(&h, &p, device).await;
        let admitted = shadow::admit(
            &h.state,
            &auth,
            producer,
            request,
            p.run.source_version_id,
            &mapping,
        )
        .await;
        assert_eq!(
            admitted.is_ok(),
            !unrelated,
            "Exact capture→binding→operation must accept its own device and reject another current same-actor device"
        );
        let calls: i64 =
            sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
                .bind(p.brain_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(
            calls, 3,
            "Each fresh operation has one real prior pair and one current controlled extraction; no retry"
        );
        fixture::archive_case(&h.state, &h.admin, &p).await.unwrap();
        h.finish().await;
    }
}
