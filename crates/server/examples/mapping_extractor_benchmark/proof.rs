//! Controlled end-to-end proof for the standalone approved scope. No external provider.
use super::*;
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
struct Wire {
    calls: AtomicUsize,
    uncertain: bool,
    out: PathBuf,
}
async fn wire(State(wire): State<Arc<Wire>>, Json(body): Json<Value>) -> Response {
    let call = wire.calls.fetch_add(1, Ordering::SeqCst) + 1;
    let attempts: Vec<Value> = fs::read_to_string(wire.out.join("attempts.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(
        attempts.len(),
        call,
        "Durable reservation must precede every HTTP send"
    );
    let attempt = attempts.last().unwrap();
    assert_eq!(
        attempt["actual_body_sha256"],
        sha256(&serde_json::to_vec(&body).unwrap()).unwrap()
    );
    let persisted: Value = serde_json::from_slice(
        &fs::read(
            wire.out
                .join(attempt["request_body_artifact"].as_str().unwrap()),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        persisted, body,
        "The sent body is the exact durably frozen canonical preview"
    );
    assert!(serde_json::to_vec(&body).unwrap().len() <= MAX_BODY as usize);
    if wire.uncertain {
        // A successful HTTP status with an over-limit body leaves billability
        // uncertain. This takes the actual gateway uncertain path immediately.
        return vec![b'x'; 1024 * 1024 + 1].into_response();
    }
    Json(json!({"model":body["model"],"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"claims\":[],\"mapping\":null}"}]}],"usage":{"input_tokens":10,"output_tokens":4,"total_tokens":14}})).into_response()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Owned PostgreSQL required; three isolated canonical60 campaigns, controlled loopback only"]
async fn standalone60_exact_preflight_reservation_drift_and_uncertain_stop() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let config = Config::from_env().unwrap();
    let (mut recipe, schedule, _) = super::super::recipe(&root, &config).unwrap();
    recipe["compiled_runner_sha256"] =
        json!(sha256(&fs::read(std::env::current_exe().unwrap()).unwrap()).unwrap());
    for (uncertain, overflow) in [(false, false), (false, true), (true, false)] {
        let mut schedule = schedule.clone();
        if overflow {
            for doc in &mut schedule {
                let text = format!(
                    "{}{}",
                    "\u{0001}".repeat(12000),
                    doc["id"].as_str().unwrap()
                );
                doc["text_sha256"] = json!(sha256(text.as_bytes()).unwrap());
                doc["text"] = json!(text);
            }
        }
        let out = root.join(format!(
            ".cache/automatic-mapping/extractor-standalone60-proof-{}",
            Uuid::new_v4()
        ));
        fs::create_dir(&out).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut fake = config.clone();
        fake.models.endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        fake.models.key = Some("owned-loopback-only".into());
        fake.models.openrouter_key = None;
        let wire_state = Arc::new(Wire {
            calls: AtomicUsize::new(0),
            uncertain,
            out: out.clone(),
        });
        let app = Router::new()
            .route("/v1/responses", post(wire))
            .with_state(wire_state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let approval = json!({"budget_usd":"2.00","authorization_reference":"CONTROLLED TEST ONLY; no paid authorization"});
        execute_owned(&out, fake.clone(), &recipe, &schedule, &approval)
            .await
            .unwrap();
        let receipt: Value =
            serde_json::from_slice(&fs::read(out.join("execution-receipt.json")).unwrap()).unwrap();
        let cleanup: Value =
            serde_json::from_slice(&fs::read(out.join("cleanup-receipt.json")).unwrap()).unwrap();
        assert_eq!(
            cleanup["status"],
            if uncertain {
                "retained_review_required"
            } else {
                "cleaned"
            }
        );
        let rows: Vec<Value> = fs::read_to_string(out.join("outcomes.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows.len(), 60);
        assert_eq!(
            wire_state.calls.load(Ordering::SeqCst),
            if uncertain {
                1
            } else if overflow {
                0
            } else {
                60
            }
        );
        assert_eq!(
            receipt["calls_reserved"],
            if uncertain {
                1
            } else if overflow {
                0
            } else {
                60
            }
        );
        assert_eq!(
            receipt["gateway_requests"],
            if uncertain {
                1
            } else if overflow {
                0
            } else {
                60
            }
        );
        assert_eq!(receipt["retries"], 0);
        assert!(
            execute_owned(&out, fake.clone(), &recipe, &schedule, &approval)
                .await
                .is_err(),
            "Durable campaign lock prevents replay before setup or send"
        );
        if uncertain {
            assert_eq!(receipt["status"], "stopped_review_required");
            assert_eq!(rows[0]["gateway_attempt"]["state"], "uncertain");
            assert!(rows[1..].iter().all(|r| r["state"] == "not_attempted"));
            let owned: Value =
                serde_json::from_slice(&fs::read(out.join("owned-state.json")).unwrap()).unwrap();
            let database = owned["database"].as_str().unwrap();
            fake.database_url = database_url(&config.database_url, database).unwrap();
            fake.artifact_dir = out.join("state/artifacts").to_string_lossy().into();
            fake.erasure_journal = out.join("state/erasure-journal").to_string_lossy().into();
            fake.erasure_mirror = None;
            fake.neo4j_url = "http://127.0.0.1:9".into();
            let admin_url = std::env::var("DATABASE_ADMIN_URL").unwrap();
            let admin = db::pool(&database_url(&admin_url, database).unwrap())
                .await
                .unwrap();
            let state = AppState::new(db::pool(&fake.database_url).await.unwrap(), fake).unwrap();
            let (brain,actor,policy,version):(Uuid,Uuid,Uuid,Uuid)=sqlx::query_as("SELECT b.id,b.owner_id,h.policy_id,v.id FROM brains b JOIN model_policy_heads h ON h.brain_id=b.id JOIN source_versions v ON v.brain_id=b.id LIMIT 1").fetch_one(&admin).await.unwrap();
            let context = gateway::Context {
                brain,
                actor,
                device: None,
            };
            let operation = Uuid::new_v4();
            let mut inv = gateway::extraction(operation, version);
            inv.instructions
                .push_str(&"oversize untrusted test instruction ".repeat(3000));
            assert!(
                gateway::preview_for_policy(&state, context, &inv, policy)
                    .await
                    .is_err(),
                "Complete-body overflow is rejected without reservation/admission"
            );
            assert!(
                persist_body(
                    &out,
                    operation,
                    &json!({"oversize":"x".repeat(MAX_BODY as usize)})
                )
                .is_err()
            );
            assert!(!out.join(format!("request-body-{operation}.json")).exists());
            let inv = gateway::extraction(operation, version);
            let preview = gateway::preview_for_policy(&state, context, &inv, policy)
                .await
                .unwrap_or_else(|e| panic!("{}", e.1));
            let mut changed = gateway::extraction(operation, version);
            changed.instructions.push_str(" body drift");
            let err = gateway::invoke_previewed_for_policy(&state, context, changed, &preview)
                .await
                .err()
                .unwrap();
            assert_eq!(err.1, "model_request_changed");
            let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
                .fetch_one(&admin)
                .await
                .unwrap();
            assert_eq!(requests, 1);
            assert_eq!(wire_state.calls.load(Ordering::SeqCst), 1);
            assert_eq!(
                fs::read_to_string(out.join("attempts.jsonl"))
                    .unwrap()
                    .lines()
                    .count(),
                1
            );
            state.pool.close().await;
            admin.close().await;
            drop(state);
            drop(admin);
            // The production uncertain branch must retain state for review.
            // Independent exact-owner cleanup happens only after this test
            // process releases all of its auxiliary validation connections.
            assert!(out.join("state").is_dir());
        } else {
            assert_eq!(receipt["status"], "completed");
            assert!(rows.iter().all(|r| r["state"]
                == if overflow {
                    "preflight_rejected"
                } else {
                    "mapping_absent"
                }));
            if overflow {
                assert!(
                    rows.iter().all(|r| r["preflight_reason"]
                        == "The complete model request exceeds the application limit."),
                    "The actual overflow path, not another qualification rejection, must preserve all60 rows"
                );
            }
            assert!(!out.join("state").exists());
        }
        private_json(&out.join("controlled-proof.json"),&json!({"passed":true,"uncertain_case":uncertain,"all60_overflow_case":overflow,"outcome_rows":60,"external_provider_calls":0,"paid_calls":0,"actual_gateway_calls":wire_state.calls.load(Ordering::SeqCst),"exact_body_durable_before_reservation_send":true,"owned_database_removed":!uncertain,"retention_expected":uncertain,"quality":"NOT_MEASURED"})).unwrap();
        server.abort();
    }
}
