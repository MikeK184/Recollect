//! New authored toys and pure hostile guard cases; never reads actual feeds/gold.
use super::{
    campaign, guard,
    inputs::{self, Admitted, Cohort, Family, Source},
    io,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::path::Path;
fn recipe(producer: &str) -> Value {
    json!({"schema":"recollect-offline-organization-recipe-1","candidate_freeze_sha256":inputs::FREEZE,"development_selection_sha256":inputs::SELECTION,"producer":{"prompt_label":producer,"schema_label":producer,"adapter_revision":"learning-quote-anchors-3","mapping_schema":"source-mentions-1"},"parameters":inputs::parameters(),"modes":inputs::MODES})
}
fn cohorts() -> Result<Vec<Cohort>> {
    let mut v = vec![];
    for name in ["private24", "prospective22"] {
        let mut sources = vec![];
        for (id, title, text, scope) in [
            (
                "orbit",
                "Orbit notes",
                "Orion uses Vela. Orion repeats. ééé.",
                "sky/flight/dev",
            ),
            ("pair", "Engine trace", "Orion uses Vela.", "sky/flight/dev"),
            (
                "cross",
                "Other scope",
                "Orion uses Vela.",
                "sea/flight/prod",
            ),
            ("empty", "Empty control", "", "sky/flight/dev"),
        ] {
            sources.push(Source {
                document_id: format!("{name}_{id}"),
                text_sha256: io::sha(text.as_bytes())?,
                partition: "held_out".into(),
                language: "en".into(),
                scope: scope.into(),
                title: title.into(),
                text: text.into(),
            });
        }
        for (suffix, text, scope) in [
            ("missing", "Orion uses Vela.", "sky/works/prod"),
            ("rejected", "Orion uses Vela.", "sky/works/prod"),
        ] {
            sources.push(Source {
                document_id: format!("{name}_{suffix}"),
                text_sha256: io::sha(text.as_bytes())?,
                partition: "held_out".into(),
                language: "en".into(),
                scope: scope.into(),
                title: "Independent incomplete extraction observation".into(),
                text: text.into(),
            });
        }
        if name == "private24" {
            for index in 0..53 {
                let text = "Orion uses Vela. ".repeat(320);
                sources.push(Source {
                    document_id: format!("private24_bulk_{index:03}"),
                    text_sha256: io::sha(text.as_bytes())?,
                    partition: "held_out".into(),
                    language: "en".into(),
                    scope: "sky/flight/dev".into(),
                    title: "Bounded independent trace".into(),
                    text,
                });
            }
        }
        let families = inputs::FAMILIES
            .iter()
            .map(|producer| {
                let r = recipe(producer);
                Ok(Family {
                    producer: (*producer).into(),
                    recipe_sha256: io::sha(&{
                        let mut b = serde_json::to_vec_pretty(&r)?;
                        b.push(b'\n');
                        b
                    })?,
                    recipe_bytes: {
                        let mut b = serde_json::to_vec_pretty(&r)?;
                        b.push(b'\n');
                        b
                    },
                    recipe: r,
                    source_feed_sha256: "1".repeat(64),
                    preparation_sha256: "2".repeat(64),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        v.push(Cohort{name:name.into(),corpus_bindings:json!({"manifest":"1".repeat(64),"corpus":"2".repeat(64),"guide":"3".repeat(64),"source_hashes":"4".repeat(64),"interface":"5".repeat(64)}),sources,families,readiness:"READY".into()});
    }
    Ok(v)
}
fn pricing(root: &Path) -> Result<Value> {
    let mut text = io::read_json(
        &root.join("docs/research/mapping-extractor-pricing-2026-10-18.json"),
        16384,
    )?;
    let mut embedding = io::read_json(
        &root.join("docs/research/mapping-embedding-pricing-2026-10-18.json"),
        16384,
    )?; // Conformance dates are synthetic and cannot authorize external execution.
    for p in [&mut text, &mut embedding] {
        p["verified_at"] = json!(chrono::Utc::now().to_rfc3339());
    }
    Ok(json!({"schema":"recollect-revision4-pricing-1","text":text,"embedding":embedding}))
}
pub async fn conformance(root: &Path, out: &Path) -> Result<Value> {
    let mut c = cohorts()?;
    let authored = out.with_file_name(format!(
        "{}-authored",
        out.file_name().unwrap().to_string_lossy()
    ));
    io::new_directory(&authored)?;
    let specs: std::collections::BTreeMap<_, _> = c
        .iter()
        .map(|x| (x.name.clone(), x.sources.clone()))
        .collect();
    io::json(
        &authored.join("toy-spec.json"),
        &serde_json::to_value(specs)?,
    )?;
    let authored_result = tokio::process::Command::new("python3")
        .args(["-E", "-s", "-B"])
        .arg(root.join("crates/server/examples/revision4_qualification/author_toys.py"))
        .arg(root)
        .arg(&authored)
        .arg(authored.join("toy-spec.json"))
        .kill_on_drop(true)
        .output()
        .await?;
    ensure!(
        authored_result.status.success(),
        "Independent toy admission failed"
    );
    let bindings = io::read_json(&authored.join("toy-bindings.json"), 65536)?;
    for cohort in &mut c {
        cohort.corpus_bindings = bindings[&cohort.name]["corpus_bindings"].clone();
        cohort.readiness = bindings[&cohort.name]["readiness"].as_str().unwrap().into();
    }
    let price = pricing(root)?;
    let a = Admitted {
        cohorts: c,
        pricing_sha256: io::value_hash(&price)?,
        pricing: price,
        bindings: json!({"authored_toy":true,"real_source_or_gold_opened":false,"code":inputs::code(root)?}),
        model_configuration: json!({"provider":"openai","endpoint":"https://api.openai.com/v1","text_model":"gpt-5.6-luna","embedding_model":"text-embedding-3-large","embedding_dimensions":3072}),
    };
    let prep = campaign::prepare(root, out, a, true).await?;
    let plan = io::read_json(&out.join("plan.json"), 96 * 1024 * 1024)?;
    let large = plan["cohorts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["cohort"]["name"] == "private24")
        .unwrap();
    ensure!(
        large["chunks"].as_array().is_some_and(|a| a.len() > 100)
            && large["batches"].as_array().is_some_and(|a| a.len() > 10)
            && large["policies"].as_array().is_some_and(|a| a
                .iter()
                .any(|p| p["discovery_passes"].as_u64().is_some_and(|n| n > 1))),
        "Complete vector discovery pass limits not exercised"
    );
    let execution = campaign::execute(root, out, None, true).await?;
    ensure!(execution["external_model_calls"] == 0, "No external calls");
    ensure!(
        execution["ledger"]["stopped"] == false
            && execution["ledger"]["counts"][0] == prep["maximum_calls"]["heldout_text"]
            && execution["ledger"]["counts"][1] == 6
            && execution["ledger"]["counts"][2] == prep["maximum_calls"]["embeddings"]
            && execution["ledger"]["counts"][3]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 6),
        "Healthy full native sequence incomplete; failure remains preserved"
    );
    let canonical = io::read_json(&out.join("canonical-observations.json"), 96 * 1024 * 1024)?;
    ensure!(
        canonical.as_array().is_some_and(|rows| rows.len() == 6
            && rows.iter().all(|r| r["state"] == "native_terminal")
            && rows.iter().any(|r| r["after"]["stage"]["payload"]["claims"]
                .as_array()
                .is_some_and(Vec::is_empty))),
        "All six controls and empty-stage zero-support proof required"
    );
    let canonical_proof = canonical_postconditions(&canonical)?;
    io::json(
        &out.join("canonical-postcondition-proof.json"),
        &canonical_proof,
    )?;
    let exports = campaign::export(root, out).await?;
    let mut scalar = vec![];
    for name in ["private24", "prospective22"] {
        for family in inputs::FAMILIES {
            let dir = out.join("exports").join(format!("{name}-{family}"));
            let recipe = dir.join("recipe.json");
            let outcomes = dir.join("outcomes.json");
            let result = tokio::process::Command::new("python3")
                .args(["-E", "-s", "-B"])
                .arg(root.join("scripts/score_frozen_source_corpus.py"))
                .arg("--manifest")
                .arg(root.join(bindings[name]["manifest"].as_str().unwrap()))
                .arg("--manifest-sha256")
                .arg(bindings[name]["sha256"].as_str().unwrap())
                .arg("--recipe")
                .arg(&recipe)
                .arg("--recipe-sha256")
                .arg(io::hash_file(&recipe)?)
                .arg("--outcomes")
                .arg(&outcomes)
                .arg("--outcomes-sha256")
                .arg(io::hash_file(&outcomes)?)
                .arg("--details")
                .arg(dir.join("scalar-details.json"))
                .kill_on_drop(true)
                .output()
                .await?;
            io::write(&dir.join("scalar-stdout.json"), &result.stdout)?;
            io::write(&dir.join("scalar-stderr.txt"), &result.stderr)?;
            ensure!(
                result.status.success(),
                "Pinned independent scalar evaluator rejected actual native export"
            );
            scalar.push(io::hash_file(&dir.join("scalar-stdout.json"))?);
        }
    }
    let incomplete = incomplete_vector_fixture(root, out).await?;
    let cleanup = campaign::cleanup(root, out).await?;
    let v = json!({"schema":"recollect-revision4-no-paid-conformance-1","preparation":prep,"execution":execution,"exports":exports,"cleanup":cleanup,"independent_scalar_score_sha256":scalar,"canonical_postconditions":canonical_proof,"incomplete_vector_fixture":incomplete,"external_model_calls":0,"real_quality":"NOT_MEASURED"});
    io::json(&out.join("conformance-receipt.json"), &v)?;
    Ok(v)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        args,
        guard::{Arm, Ledger},
    };
    use uuid::Uuid;
    fn dir() -> std::path::PathBuf {
        let r = crate::root()
            .unwrap()
            .join(crate::BASE)
            .join(format!("unit-{}", Uuid::new_v4()));
        io::new_directory(&r).unwrap();
        r
    }
    fn frozen() -> Value {
        let root = crate::root().unwrap();
        let p = pricing(&root).unwrap();
        json!({"pricing":p,"pricing_sha256":"a".repeat(64),"executable_sha256":"b".repeat(64),"schedule_sha256":"c".repeat(64),"maximum_calls":guard::caps(2,3,6).unwrap(),"support_template_sha256":"d".repeat(64),"budget_nano_usd":guard::cost(&guard::caps(2,3,6).unwrap()).unwrap()})
    }
    #[tokio::test]
    async fn returned_tier_fault_retains_charge_and_stops_actual_router() {
        use crate::wire::{Active, Wire};
        use std::sync::{Arc, atomic::Ordering};
        use tokio::sync::Mutex;
        for tier in [Value::Null, json!("auto"), json!("priority"), json!(42)] {
            let out = dir();
            for name in ["raw-responses", "raw-response-bindings", "wire-outcomes"] {
                io::new_directory(&out.join(name)).unwrap();
            }
            let r = frozen();
            let hash = "e".repeat(64);
            let ledger = Ledger::begin(&out, &r, &hash, &guard::template(&r, &hash), true).unwrap();
            let mut w = Wire::controlled(&out);
            w.ledger = Arc::new(Mutex::new(Some(ledger)));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let router = w.router();
            let server = tokio::spawn(async move {
                let _ = axum::serve(listener, router).await;
            });
            let body = json!({"model":"gpt-5.6-luna","input":"independent protocol fixture"});
            let op = Uuid::new_v4();
            w.register(Active {
                key: "first".into(),
                operation: op,
                body: body.clone(),
                arm: Arm::Heldout,
                payload: json!({"__controlled_response_tier":tier}),
                control: None,
                lease: None,
            })
            .await
            .unwrap();
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let url = format!("http://{address}/v1/responses");
            let first = client
                .post(&url)
                .header("content-type", "application/json")
                .body(serde_json::to_vec(&body).unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(first.status(), 400);
            assert!(
                out.join("raw-responses")
                    .join(format!("{op}.json"))
                    .exists()
            );
            assert!(w.stopped().await);
            w.clear().await;
            w.register(Active {
                key: "otherwise-valid-next".into(),
                operation: Uuid::new_v4(),
                body: body.clone(),
                arm: Arm::Heldout,
                payload: json!({}),
                control: None,
                lease: None,
            })
            .await
            .unwrap();
            let next = client
                .post(&url)
                .body(serde_json::to_vec(&body).unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(next.status(), 400);
            let summary = w.ledger.lock().await.as_ref().unwrap().summary();
            assert_eq!(summary["reservations"], 1);
            assert_eq!(summary["reserved_nano_usd"], guard::TEXT);
            assert_eq!(w.controlled.load(Ordering::SeqCst), 1);
            assert_eq!(w.external.load(Ordering::SeqCst), 0);
            io::json(&out.join("returned-tier-router-proof.json"),&json!({"fault_tier":tier,"ledger":summary,"second_valid_operation_refused":true,"controlled_responses":1,"external_calls":0})).unwrap();
            server.abort();
        }
    }
    #[test]
    fn strict_arguments_and_json() {
        for a in [
            vec![],
            vec!["--prepare"],
            vec!["--conformance", "--execute", "--out", "x"],
            vec!["--conformance", "--out", "x", "--out", "y"],
            vec!["--conformance", "--out", "x", "--wat", "z"],
        ] {
            assert!(args(a.iter().map(|s| s.to_string()).collect()).is_err());
        }
        assert!(io::strict(br#"{"x":1,"x":2}"#).is_err());
        assert!(io::strict(br#"{"x":{"y":1,"y":2}}"#).is_err());
        assert!(io::strict(b"NaN").is_err());
        assert!(
            io::strict(&format!("{}0{}", "[".repeat(33), "]".repeat(33)).into_bytes()).is_err()
        );
    }
    #[test]
    fn false_and_all_consumed_kinds_refused() {
        let r = frozen();
        let h = "e".repeat(64);
        let a = guard::template(&r, &h);
        assert!(guard::approval(&a, &r, &h).is_err());
        for kind in [
            "combined-source-facts-reconciliation-vectors-1",
            "remaining-reconciliation-vectors-1",
            "retained-control-support-replay-1",
        ] {
            let mut x = a.clone();
            x["approve_paid_calls"] = json!(true);
            x["kind"] = json!(kind);
            x["authorization_reference"] = json!("human previous consumed grant");
            assert!(guard::approval(&x, &r, &h).is_err());
        }
        let mut unknown = a;
        unknown["other"] = json!(true);
        assert!(guard::approval(&unknown, &r, &h).is_err());
    }
    #[test]
    fn current_price_and_exact_maxima() {
        let mut r = frozen();
        assert_eq!(r["maximum_calls"]["heldout_text"], 88);
        assert_eq!(r["maximum_calls"]["total"], 103);
        assert!(guard::caps(47, 1, 6).is_err());
        assert!(guard::caps(0, u64::MAX, 6).is_err());
        r["pricing"]["text"]["verified_at"] = json!("2000-01-01T00:00:00Z");
        assert!(guard::price(&r["pricing"], chrono::Utc::now()).is_err());
        let mut r = frozen();
        r["pricing"]["text"]["unknown"] = json!(true);
        assert!(guard::price(&r["pricing"], chrono::Utc::now()).is_err());
    }
    #[test]
    fn fsynced_reservation_crash_duplicate_and_uncertainty() {
        let out = dir();
        let r = frozen();
        let h = "e".repeat(64);
        let a = guard::template(&r, &h);
        let mut l = Ledger::begin(&out, &r, &h, &a, true).unwrap();
        let b = b"{}";
        let bh = io::sha(b).unwrap();
        let op = Uuid::new_v4();
        l.reserve(Arm::Heldout, "first", op, b, &bh).unwrap();
        assert!(
            l.reserve(Arm::Heldout, "second", Uuid::new_v4(), b, &bh)
                .is_err()
        );
        l.finish("first", false, json!({"delivery":"unknown"}))
            .unwrap();
        assert!(
            l.reserve(Arm::Heldout, "next", Uuid::new_v4(), b, &bh)
                .is_err()
        );
        assert!(Ledger::begin(&out, &r, &h, &a, true).is_err());
        let bytes = io::read(&out.join("reservation-ledger.jsonl"), 65536).unwrap();
        assert!(
            String::from_utf8(bytes)
                .unwrap()
                .contains("reserved_nano_usd")
        );
    }
    #[test]
    fn malformed_model_usage_schema_and_limits_refused() {
        for v in [
            json!({}),
            json!({"model":"other","usage":{"input_tokens":1,"output_tokens":1}}),
            json!({"model":"gpt-5.6-luna","status":"completed","usage":{"input_tokens":67585,"output_tokens":1},"output":[]}),
        ] {
            assert!(!guard::response_ok(&v, false));
        }
        assert!(!guard::schema(
            &json!({"unexpected":1}),
            &json!({"type":"object","properties":{},"additionalProperties":false}),
            0
        ));
        assert!(!guard::schema(
            &json!({"x":2}),
            &json!({"type":"object","properties":{"x":{"type":"integer","maximum":1}},"required":["x"],"additionalProperties":false}),
            0
        ));
    }
    #[test]
    fn descriptor_private_io_refuses_overwrite_symlink_replacement() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let out = dir();
        io::write(&out.join("a"), b"owned").unwrap();
        assert_eq!(
            std::fs::metadata(&out).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(io::write(&out.join("a"), b"change").is_err());
        symlink("a", out.join("link")).unwrap();
        assert!(io::read(&out.join("link"), 100).is_err());
        let moved = out.with_extension("retained");
        std::fs::rename(&out, &moved).unwrap();
        std::fs::create_dir(&out).unwrap();
        assert!(io::write(&out.join("foreign"), b"leak").is_err());
    }
}

fn canonical_postconditions(rows: &Value) -> Result<Value> {
    let mut proof = vec![];
    for row in rows
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Canonical rows"))?
    {
        let case = row["id"].as_str().unwrap_or("");
        let before = &row["before"];
        let after = &row["after"];
        let old = before["claims"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| anyhow::anyhow!("Prior canonical control"))?;
        let current = after["claims"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Current canonical controls"))?;
        let same = current
            .iter()
            .find(|r| r["claim_id"] == old["claim_id"])
            .ok_or_else(|| anyhow::anyhow!("Original claim identity lost"))?;
        ensure!(
            after["revisions"]
                .as_array()
                .is_some_and(|r| r.contains(&old["revision"])),
            "Original canonical revision history lost"
        );
        let new = |predicate: &str, value: &str| {
            current.iter().find(|r| {
                r["claim_id"] != old["claim_id"]
                    && r["revision"]["content"]["predicate"] == predicate
                    && r["revision"]["content"]["value"] == value
            })
        };
        match case {
            "rc01" => ensure!(
                same["current_revision"] != old["current_revision"]
                    && same["revision"]["content"]["value"] == "8443"
                    && new("uses", "PostgreSQL").is_some()
                    && after["run"]["revised"] == 1,
                "Controlled replacement/new assertion postconditions failed"
            ),
            "rc02" | "rc05" => ensure!(
                same == old && current.len() == before["claims"].as_array().unwrap().len(),
                "Empty historical/identical control changed canonical identity"
            ),
            "rc03" => ensure!(
                same["current_revision"] != old["current_revision"]
                    && same["revision"]["content"]["freshness"] == "superseded"
                    && same["revision"]["content"]["value"] == "Redis"
                    && after["run"]["retired"] == 1,
                "Controlled retirement postconditions failed"
            ),
            "rc04" => ensure!(
                same == old && new("owner", "Zoë").is_some() && new("uses", "Rust").is_some(),
                "Controlled capture owner/use postconditions failed"
            ),
            "rc06" => {
                ensure!(
                    same == old && row["offered_revision_ids"] == json!([]),
                    "Reviewed manual memory changed or offered"
                );
                let candidate = new("port", "7443")
                    .ok_or_else(|| anyhow::anyhow!("Separate current source assertion missing"))?;
                ensure!(
                    candidate["revision"]["content"]["freshness"] == "uncertain"
                        || after["run"]["conflicting"].as_i64().is_some_and(|n| n > 0)
                        || candidate["revision"]["review"] == "proposed",
                    "Protected manual conflict not retained"
                );
            }
            _ => anyhow::bail!("Unexpected control"),
        }
        let stage = &after["stage"];
        let actions = stage["payload"]["claims"].as_array().map_or(0, Vec::len)
            + stage["payload"]["retirements"]
                .as_array()
                .map_or(0, Vec::len);
        if actions > 0 {
            ensure!(
                stage["verdicts"]
                    .as_array()
                    .is_some_and(|v| v.len() == actions
                        && v.iter()
                            .all(|v| v["citation_support"] == true
                                && v["disposition"] == "supported")),
                "Positive canonical action lacks actual supported verdict"
            );
            let assessments = after["assessments"].as_array().unwrap();
            let matched = assessments
                .iter()
                .filter(|a| {
                    a["source_stage_id"] == after["run"]["id"]
                        && a["request_id"] == stage["assessment_request_id"]
                        && a["disposition"] == "supported"
                        && a["state"] == "succeeded"
                })
                .count();
            let claim_count = stage["payload"]["claims"].as_array().map_or(0, Vec::len);
            ensure!(
                matched >= claim_count,
                "Actual canonical claim assessment linkage missing"
            );
            ensure!(
                after["requests"]
                    .as_array()
                    .is_some_and(|rows| rows
                        .iter()
                        .any(|q| q["request_id"] == stage["assessment_request_id"]
                            && q["state"] == "succeeded"
                            && q["prompt_label"] == "source-support-3")),
                "Actual stage support request linkage missing"
            );
            if case == "rc03" {
                ensure!(
                    same["revision"]["content"]["supports"]
                        .as_array()
                        .is_some_and(|supports| supports
                            .iter()
                            .any(|s| s["id"] == after["run"]["source_version_id"]
                                && s["line_from"] == 1
                                && s["line_to"] == 1)),
                    "Retirement lacks source0 line1 basis"
                );
            }
        }
        proof.push(json!({"case":case,"canonical_postconditions":"PASS","original_claim_identity_preserved":true,"original_revision_history_preserved":true,"actual_supported_actions":actions,"empty_safe_not_reuse_proof":case=="rc05","real_model_quality":"NOT_MEASURED"}));
    }
    Ok(
        json!({"schema":"recollect-controlled-canonical-postconditions-1","controls":proof,"unchanged_public_fixture_sha256":inputs::CONTROLS,"real_quality":"NOT_MEASURED"}),
    )
}

async fn incomplete_vector_fixture(root: &Path, out: &Path) -> Result<Value> {
    use super::{
        native,
        owned::{Owned, api},
        pipeline,
        wire::Wire,
    };
    use std::sync::atomic::Ordering;
    let mut cohort = cohorts()?.pop().unwrap();
    cohort.name = "incomplete-vectors".into();
    let text = "Orion uses Vela.\n".repeat(700);
    cohort.sources.push(Source {
        document_id: "incomplete_vector_source".into(),
        text_sha256: io::sha(text.as_bytes())?,
        partition: "held_out".into(),
        language: "en".into(),
        scope: "shard/trace/dev".into(),
        title: "Incomplete canonical vector observation".into(),
        text,
    });
    let path = out.join("owned-incomplete-vectors");
    io::new_directory(&path)?;
    io::new_directory(&path.join("bodies"))?;
    let transport = path.join("controlled-transport");
    io::new_directory(&transport)?;
    for name in ["raw-responses", "raw-response-bindings", "wire-outcomes"] {
        io::new_directory(&transport.join(name))?;
    }
    let w = Wire::controlled(&transport);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let router = w.router();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    let base = recollect_server::config::Config::from_env()?;
    let config = pipeline::config(&base, address);
    io::json(
        &path.join("toy-spec.json"),
        &json!({"incomplete-vectors":cohort.sources}),
    )?;
    let authored = tokio::process::Command::new("python3")
        .args(["-E", "-s", "-B"])
        .arg(root.join("crates/server/examples/revision4_qualification/author_toys.py"))
        .arg(root)
        .arg(&path)
        .arg(path.join("toy-spec.json"))
        .kill_on_drop(true)
        .output()
        .await?;
    ensure!(
        authored.status.success(),
        "Incomplete fixture independent admission"
    );
    let binding = io::read_json(&path.join("toy-bindings.json"), 65536)?;
    cohort.corpus_bindings = binding["incomplete-vectors"]["corpus_bindings"].clone();
    let o = Owned::setup(&path, config).await?;
    let mut plan = pipeline::prepare_cohort(&o, cohort, &path, true).await?;
    let brain = plan.imports["incomplete_vector_source"]
        .as_ref()
        .unwrap()
        .brain;
    let auth = o.auth().await?;
    let cancelled: Vec<_> = plan
        .batches
        .iter()
        .filter(|b| b.brain == brain)
        .cloned()
        .collect();
    ensure!(
        cancelled.len() > 1,
        "Incomplete fixture needs multiple actual batches"
    );
    for batch in &cancelled {
        let _ = api(recollect_server::jobs::cancel(
            axum::extract::State(o.state.clone()),
            auth.clone(),
            axum::extract::Path((brain, batch.job)),
        )
        .await)?;
    }
    plan.batches.retain(|b| b.brain != brain);
    io::json(
        &path.join("explicit-cancelled-vector-manifest.json"),
        &json!({"declared_controlled_adversary":true,"source_version_id":plan.imports["incomplete_vector_source"].as_ref().unwrap().version,"cancelled_jobs":cancelled.iter().map(|b|b.job).collect::<Vec<_>>(),"canonical_chunks_preserved":true,"semantic_rows_or_vectors_fabricated":false,"actual_paid_scope_changed":false}),
    )?;
    let families = pipeline::execute_cohort(&o, &plan, &w, &path).await?;
    let preflight = path.join("native-preflight");
    io::new_directory(&preflight)?;
    let native = native::Native::verify(&base, root, &preflight, "2026.08.1").await?;
    let mut o = o;
    o.state = native.attach(&o.state);
    let mut proofs = vec![];
    for (family, all) in inputs::FAMILIES.iter().zip(families) {
        let dir = path.join(format!("export-{family}"));
        io::new_directory(&dir)?;
        pipeline::export_family(
            &o,
            &plan,
            &all,
            family,
            &dir,
            &json!({"controlled_incomplete_vector_fixture":true}),
            true,
        )
        .await?;
        let outcomes = io::read_json(&dir.join("outcomes.json"), 96 * 1024 * 1024)?;
        for mode in outcomes["modes"].as_array().unwrap() {
            let row = mode["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["document_id"] == "incomplete_vector_source")
                .unwrap();
            ensure!(
                row["vector_feature_available"] == false && row["ner_feature_available"] == true,
                "Incomplete canonical vectors incorrectly eligible"
            );
        }
        proofs.push(io::hash_file(&dir.join("outcomes.json"))?);
    }
    server.abort();
    ensure!(
        w.external.load(Ordering::SeqCst) == 0,
        "Incomplete fixture external call"
    );
    o.close().await?;
    let receipt = json!({"schema":"recollect-controlled-incomplete-vectors-1","actual_native_cancellation":true,"both_families_share_one_import_vector_basis":true,"full_cohort_source_mode_coverage_preserved":true,"all_five_modes_observe_incomplete_vector_false":true,"outcomes_sha256":proofs,"external_model_calls":0});
    io::json(&path.join("incomplete-vector-receipt.json"), &receipt)?;
    Ok(receipt)
}
