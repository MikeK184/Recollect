//! Gold-free native support-audit adapter. This measures assertion support;
//! controlled producer-action fixtures separately test mutation enforcement.
use super::*;

const ADAPTER: &str = "native-revision-assertion-v1";
const BUDGET: i64 = 250_000;

async fn selection(
    h: &Harness,
    owner: &Login,
    base: &str,
    name: &Value,
    ids: &mut std::collections::BTreeMap<String, Value>,
) -> Value {
    let environment = if let Some(name) = name.as_str() {
        if !ids.contains_key(name) {
            let group = ok(
                h,
                "POST",
                &format!("{base}/evidence/groups"),
                owner,
                json!({"kind":"environment","name":name}),
            )
            .await;
            ids.insert(name.to_owned(), group["id"].clone());
        }
        ids[name].clone()
    } else {
        Value::Null
    };
    json!({"repository_ids":[],"area_ids":[],"environment_id":environment})
}

#[tokio::test]
#[ignore = "Opt-in synthetic paid native support baseline; RECOLLECT_REAL_SUPPORT_EVAL=1"]
async fn memory_support_installed_provider_baseline_opt_in() {
    if std::env::var("RECOLLECT_REAL_SUPPORT_EVAL").as_deref() != Ok("1") {
        eprintln!("Real support evaluation not requested; no external call made.");
        return;
    }
    let provider =
        std::env::var("RECOLLECT_REAL_SUPPORT_PROVIDER").unwrap_or_else(|_| "openai".into());
    assert!(matches!(provider.as_str(), "openai" | "openrouter"));
    let openrouter_text = std::env::var("RECOLLECT_REAL_SUPPORT_TEXT_MODEL")
        .unwrap_or_else(|_| "z-ai/glm-4.7-flash".into());
    assert!(matches!(
        openrouter_text.as_str(),
        "z-ai/glm-4.7-flash" | "z-ai/glm-5.3-flash" | "openai/gpt-6-luna"
    ));
    let partition =
        std::env::var("RECOLLECT_REAL_SUPPORT_PARTITION").unwrap_or_else(|_| "calibration".into());
    assert!(matches!(
        partition.as_str(),
        "calibration" | "held_out" | "all"
    ));
    let corpus_bytes = include_bytes!("../fixtures/memory-support/corpus.json");
    let corpus: Value = serde_json::from_slice(corpus_bytes).unwrap();
    let corpus_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/memory-support/corpus.json");
    assert_eq!(std::fs::read(&corpus_path).unwrap(), corpus_bytes);
    let hashed = std::process::Command::new("shasum")
        .args(["-a", "256"])
        .arg(corpus_path)
        .output()
        .unwrap();
    assert!(hashed.status.success());
    let hash = String::from_utf8(hashed.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    let output = std::path::PathBuf::from(
        std::env::var("RECOLLECT_REAL_SUPPORT_RESULTS")
            .expect("Choose new repo-local ignored results path"),
    );
    assert!(
        output.starts_with(".cache")
            && output
                .components()
                .all(|c| !matches!(c, std::path::Component::ParentDir)),
        "Keep results inside ignored repo proof storage"
    );
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let _owned = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .expect("Never overwrite another evaluation");
    let _receipt_owned = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.with_extension("receipts.json"))
        .expect("Never overwrite another receipt ledger");
    let mut results = json!({"version":1,"corpus":"synthetic-memory-support-v1","mode":"real-model","versions":{"requested_model":"pending","returned_model":"pending","prompt":"source-support-3","schema":"source-support-3","verifier":"source-support-3"},"observations":[]});
    let mut receipts = json!({"adapter":ADAPTER,"corpus_sha256":hash,"partition":partition,"allowance":BUDGET,"cases":[]});
    let mut spent = 0i64;
    for case in corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| partition == "all" || c["partition"] == partition)
    {
        // This adapter reads no expected verdict, category, critical flag or
        // producer identity while constructing any provider-visible content.
        let evidence = &case["evidence"];
        let candidate = &case["candidate"];
        let h = Harness::new().await;
        assert!(
            h.state.config.models.key_for(&provider).is_some(),
            "Installed provider credential required"
        );
        assert_eq!(
            h.state.config.models.endpoint_for(&provider),
            Some(if provider == "openrouter" {
                "https://openrouter.ai/api/v1"
            } else {
                "https://api.openai.com/v1"
            })
        );
        if provider == "openrouter" && spent == 0 {
            let account: Value = h
                .state
                .http
                .get("https://openrouter.ai/api/v1/key")
                .bearer_auth(h.state.config.models.key_for(&provider).unwrap())
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            assert!(
                account["data"]["usage"].as_f64().unwrap() + 1.25 <= 8.0,
                "Reserve the entire 250k-token synthetic audit below the campaign ceiling"
            );
            assert!(account["data"]["limit_remaining"].as_f64().unwrap() >= 3.25);
        }
        let owner = h.login().await;
        let b = ok(
            &h,
            "POST",
            "/api/brains",
            &owner,
            json!({"name":"Isolated synthetic support baseline","managed_memory":false}),
        )
        .await;
        let base = format!("/api/brains/{}", b["id"].as_str().unwrap());
        let brain = b["id"].as_str().unwrap().parse::<Uuid>().unwrap();
        if provider == "openrouter" {
            let listing = ok(
                &h,
                "POST",
                &format!("{base}/models/catalogue?provider=openrouter"),
                &owner,
                Value::Null,
            )
            .await;
            assert!(
                listing["error_code"].is_null(),
                "Refresh must succeed before paid dispatch"
            );
            let current = ok(
                &h,
                "GET",
                &format!("{base}/models/policy"),
                &owner,
                Value::Null,
            )
            .await;
            let mut policy = current["current"]["policy"].clone();
            policy["provider"] = json!("openrouter");
            policy["text_model"] = json!(openrouter_text);
            policy["embedding_model"] = json!("qwen/qwen3-embedding-8b");
            policy["embedding_dimensions"] = json!(1024);
            ok(&h,"PUT",&format!("{base}/models/policy"),&owner,json!({"base_change":current["current"]["change_id"],"policy":policy,"rebuild_embeddings":true})).await;
        }
        let mut ids = std::collections::BTreeMap::new();
        let source_scope = selection(
            &h,
            &owner,
            &base,
            &evidence["selection"]["environment"],
            &mut ids,
        )
        .await;
        let claim_scope = selection(
            &h,
            &owner,
            &base,
            &candidate["selection"]["environment"],
            &mut ids,
        )
        .await;
        let version = if evidence["role"] == "source_document" {
            let task = ok(
                &h,
                "POST",
                &format!("{base}/workspace/tasks"),
                &owner,
                json!({"label":"Synthetic import","selection":source_scope}),
            )
            .await;
            let op = ok(
                &h,
                "POST",
                &format!(
                    "{base}/workspace/tasks/{}/operations",
                    task["task"]["id"].as_str().unwrap()
                ),
                &owner,
                json!({"kind":"write"}),
            )
            .await;
            ok(&h,"POST",&format!("{base}/sources"),&owner,json!({"title":"Synthetic evidence","media_type":"text/plain","retain_content":true,"content":evidence["text"],"operation_id":op["id"]})).await["version"]["id"].clone()
        } else {
            capture_policy(&h, &owner, &base, true).await;
            let (_, token) = h.pair_device(&owner, "Synthetic support evidence").await;
            let bound = binding(&h, &base, &token, source_scope).await;
            let mut captured = event(
                &bound,
                "synthetic-proof",
                evidence["text"].as_str().unwrap(),
            );
            match evidence["role"].as_str().unwrap() {
                "user_assertion" => {}
                "assistant_statement" => {
                    captured["event"]["kind"] = json!("reply");
                    captured["event"]["host_event"] = json!("Stop");
                }
                "reported_tool_observation" => {
                    captured["event"]["kind"] = json!("tool_result");
                    captured["event"]["host_event"] = json!("PostToolUse");
                    captured["event"]["tool_name"] = json!("synthetic_observation");
                    captured["event"]["tool_use_id"] = json!("synthetic-call");
                }
                _ => panic!("Unrecognized native role"),
            }
            device_ok(
                &h,
                "POST",
                &format!("{base}/capture/events"),
                &token,
                captured,
            )
            .await["source_version_id"]
                .clone()
        };
        process(&h).await;
        let mut input = review::proposal(
            &version,
            "Recorded assertion",
            candidate["assertion"].as_str().unwrap(),
        );
        input["content"]["rationale"] = json!("");
        input["content"]["predicate"] = json!("assertion");
        input["content"]["validity"] =
            json!({"kind":"unknown","from":null,"to":null,"precision":"unknown"});
        input["content"]["selection"] = claim_scope.clone();
        input["content"]["supports"][0]["line_from"] = candidate["line_from"].clone();
        input["content"]["supports"][0]["line_to"] = candidate["line_to"].clone();
        let (status, revision, _) = h
            .call("POST", &format!("{base}/claims"), Some(&owner), input)
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Adapter must enter canonical claim path; case {}: {}",
            case["id"],
            revision["code"]
        );
        allow(&h, &owner, &base, |policy| {
            policy["autonomous_memory"] = json!(true);
            policy["automatic_learning"] = json!(false);
            policy["automatic_embedding"] = json!(false);
            policy["purposes"] = json!(["extraction", "synthesis"]);
            policy["content_classes"] = json!(["document", "raw_session", "tool_output", "claim"]);
            policy["max_input_bytes"] = json!(32768);
            policy["max_output_tokens"] = json!(if provider == "openrouter"
                && openrouter_text == "z-ai/glm-5.3-flash"
            {
                4096
            } else {
                1024
            });
            policy["daily_token_limit"] = json!(BUDGET - spent);
            policy["max_concurrent"] = json!(1);
        })
        .await;
        autonomous::run_once(&h.state)
            .await
            .map_err(|e| e.1)
            .unwrap();
        // Isolate the audit lane without changing assertion, support, verdict
        // or eligibility. Autonomous policy also queues source learning; this
        // experiment must never accidentally pay for that separate producer.
        sqlx::query("UPDATE jobs SET not_before=clock_timestamp()+interval '1 day' WHERE brain_id=$1 AND lane='model' AND kind<>'claim.support' AND state='queued'")
            .bind(brain).execute(&h.admin).await.unwrap();
        let job = worker::claim(&h.state.pool, "model")
            .await
            .unwrap()
            .expect("Automatic audit queued");
        assert_eq!(
            job.kind, "claim.support",
            "Execute only native support assessment"
        );
        let target: Uuid =
            sqlx::query_scalar("SELECT revision_id FROM memory_support_assessments WHERE id=$1")
                .bind(job.target_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        assert_eq!(
            target,
            revision["id"].as_str().unwrap().parse::<Uuid>().unwrap()
        );
        // Journal immutable identities before the gateway can admit HTTP.
        // An interrupted evaluation can inspect this exact owned database;
        // rerunning never overwrites the artifact or blindly resends a case.
        let ledger_index = receipts["cases"].as_array().unwrap().len();
        receipts["cases"].as_array_mut().unwrap().push(json!({"case_id":case["id"],"state":"pending","database":h.database,"brain_id":brain,"source_version":version,"revision_id":revision["id"],"assessment_id":job.target_id,"job_id":job.id}));
        std::fs::write(
            output.with_extension("receipts.json"),
            serde_json::to_vec_pretty(&receipts).unwrap(),
        )
        .unwrap();
        let started = std::time::Instant::now();
        if let Err(failure) = worker::execute(&h.state, &job).await {
            worker::fail(&h.state.pool, &job, failure).await.unwrap();
        }
        let latency = started.elapsed().as_millis() as u64;
        let assessed: Value =
            sqlx::query_scalar("SELECT to_jsonb(a) FROM memory_support_assessments a WHERE id=$1")
                .bind(job.target_id)
                .fetch_one(&h.admin)
                .await
                .unwrap();
        let requests: Vec<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(m)-ARRAY['call_token'] FROM model_requests m WHERE brain_id=$1 ORDER BY created_at,id",
        )
        .bind(brain)
        .fetch_all(&h.admin)
        .await
        .unwrap();
        let charge = requests
            .iter()
            .map(|r| r["charged_tokens"].as_i64().unwrap())
            .sum::<i64>();
        spent += charge;
        assert!(spent <= BUDGET, "Whole-run allowance exceeded");
        let claim = ok(
            &h,
            "GET",
            &format!("{base}/claims/{}", revision["claim_id"].as_str().unwrap()),
            &owner,
            Value::Null,
        )
        .await;
        let recall=ok(&h,"POST",&format!("{base}/recall"),&owner,json!({"query":"","exact":{"kind":"claim","id":revision["claim_id"]},"selection":claim_scope,"channels":["exact"],"context_bytes":32768,"limit":20})).await;
        let delivered = recall["context"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["revision_id"] == revision["id"]);
        let usable = claim["selected"]["eligibility"]["investigation"] == true && delivered;
        let complete = assessed["state"] == "succeeded";
        let uncertain = requests
            .iter()
            .any(|r| matches!(r["state"].as_str(), Some("running" | "uncertain")));
        let state = if complete {
            "completed"
        } else if uncertain {
            "uncertain"
        } else {
            "failed"
        };
        results["observations"].as_array_mut().unwrap().push(json!({"case_id":case["id"],"state":state,"verdict":if complete {assessed["disposition"].clone()} else {Value::Null},"usable":usable,"request_count":requests.len(),"charged_tokens":charge,"latency_ms":latency}));
        let requested = requests
            .first()
            .map(|r| r["model"].clone())
            .unwrap_or_else(|| json!(h.state.config.models.text_model));
        assert!(
            results["versions"]["requested_model"] == "pending"
                || results["versions"]["requested_model"] == requested,
            "Mixed requested models require separate evaluations"
        );
        results["versions"]["requested_model"] = requested;
        if let Some(returned) = requests.iter().find_map(|r| r["returned_model"].as_str()) {
            assert!(
                results["versions"]["returned_model"] == "pending"
                    || results["versions"]["returned_model"] == returned,
                "Mixed returned models require separate evaluations"
            );
            results["versions"]["returned_model"] = json!(returned);
        }
        receipts["cases"][ledger_index] = json!({"case_id":case["id"],"state":"recorded","database":h.database,"brain_id":brain,"source_version":version,"revision_id":revision["id"],"assessment_id":job.target_id,"job_id":job.id,"policy_id":assessed["policy_id"],"request_ids":requests.iter().map(|r|r["id"].clone()).collect::<Vec<_>>(),"gateway_receipts":requests,"assessment_state":assessed["state"],"disposition":assessed["disposition"],"reason":assessed["reason"],"eligibility":claim["selected"]["eligibility"],"delivered_exact_revision":delivered});
        receipts["provider"] = json!(provider);
        receipts["actual_cost_usd"] = json!(
            receipts["cases"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|c| c["gateway_receipts"].as_array().into_iter().flatten())
                .filter_map(|r| r["cost_usd"].as_f64())
                .sum::<f64>()
        );
        std::fs::write(&output, serde_json::to_vec_pretty(&results).unwrap()).unwrap();
        std::fs::write(
            output.with_extension("receipts.json"),
            serde_json::to_vec_pretty(&receipts).unwrap(),
        )
        .unwrap();
        eprintln!(
            "Synthetic case {}: {state}; usable={usable}; calls={}; charged={charge}",
            case["id"],
            requests.len()
        );
        assert!(
            !uncertain,
            "Uncertain paid attempt retained in owned database {}; reconcile before any retry",
            h.database
        );
        h.finish().await;
        if BUDGET - spent < 70000 {
            eprintln!(
                "Remaining allowance cannot guarantee one bounded admission; stopping without replay"
            );
            break;
        }
    }
    eprintln!("Native synthetic support evidence: {}", output.display());
}
