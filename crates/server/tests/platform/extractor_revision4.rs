//! Independently authored toys and controlled loopback responses. Synthetic
//! qualification rows exist only in owned test databases and prove no quality.
use super::*;
use recollect_server::{
    autonomous, db,
    knowledge_mapping::{producer, production},
};

fn brain(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}
fn id(v: &Value) -> Uuid {
    v["id"].as_str().unwrap().parse().unwrap()
}
fn version(v: &Value) -> Uuid {
    v["version"]["id"].as_str().unwrap().parse().unwrap()
}
fn mapping(occurrence: usize) -> Value {
    json!({"language":"en","mentions":[{"key":"piece","kind":"concept","confidence":1.0,
        "span":{"quote":"aba","line_from":1,"line_to":1,"occurrence":occurrence}}],"aliases":[],"relations":[]})
}
async fn catalogue(h: &Harness, b: Uuid, prompt: &str, adapter: &str) {
    sqlx::query("INSERT INTO knowledge_mapping_release_producers(id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_receipt_sha256,fixture_sha256,recipe_sha256,held_out_documents,entity_precision,entity_recall,endpoint_precision,endpoint_recall) SELECT $1,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model',$3,$3,$4,'source-mentions-1',repeat('a',64),repeat('b',64),repeat('c',64),18,0.96,0.85,0.96,0.85 FROM model_policy_heads h JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id WHERE h.brain_id=$2")
        .bind(Uuid::new_v4().to_string()).bind(b).bind(prompt).bind(adapter).execute(&h.admin).await.unwrap();
}
async fn selected(h: &Harness, owner: &Login, base: &str, claim: &Value) -> Value {
    ok(
        h,
        "GET",
        &format!("{base}/claims/{}", claim.as_str().unwrap()),
        owner,
        Value::Null,
    )
    .await["selected"]["revision"]
        .clone()
}
async fn change(h: &Harness, owner: &Login, base: &str, old: &Value, text: &str) -> Value {
    ok(h,"POST",&format!("{base}/sources/{}/versions",old["id"].as_str().unwrap()),owner,
        json!({"base_version":old["version"]["id"],"title":"Independent lifecycle toy","media_type":"text/plain","retain_content":true,"content":text})).await
}
fn assertion(value: &str, replaces: Value) -> Value {
    json!({"subject":"LanternUnit","predicate":"mode","value":value,"rationale":"The source directly states this mode.","line_from":1,"line_to":1,"context_role":null,"kind":"claim","procedure":null,"replaces_revision":replaces})
}
async fn execute(
    h: &Harness,
    owner: &Login,
    base: &str,
    p: &Provider,
    doc: &Value,
    output: Value,
) -> Value {
    *p.candidates.lock().unwrap() = output;
    let run = learn(h, owner, base, doc).await;
    model_job(h).await;
    sqlx::query_scalar::<_, sqlx::types::Json<Value>>(
        "SELECT to_jsonb(r) FROM learning_runs r WHERE id=$1",
    )
    .bind(id(&run))
    .fetch_one(&h.admin)
    .await
    .unwrap()
    .0
}

#[test]
fn revision4_current_and_retry_builder_schemas_preserve_exact_action_enums() {
    let mut run: recollect_protocol::LearningRun=serde_json::from_value(json!({
        "id":Uuid::new_v4(),"brain_id":Uuid::new_v4(),"source_version_id":Uuid::new_v4(),"policy_id":Uuid::new_v4(),"actor_id":Uuid::new_v4(),"device_id":null,"operation_id":null,
        "selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,"automatic":true,"job_id":Uuid::new_v4(),"state":"queued","error_code":null,"request_id":null,"claim_ids":[],"proposed":0,"accepted":0,"blocked":0,"conflicting":0,"reused":0,"created_at":"2026-10-18T00:00:00Z","finished_at":null})).unwrap();
    let revision = Uuid::new_v4();
    run.reconciliation_inputs = vec![revision];
    for attempt in [0, 1, 2] {
        run.automatic_attempt = attempt;
        let inv = autonomous::invocation(&run);
        assert_eq!(
            (&*inv.prompt_label, &*inv.schema_label),
            ("source-reconciliation-4", "source-reconciliation-4")
        );
        let gateway::Format::Json { schema, .. } = inv.format else {
            panic!()
        };
        let fields = &schema["properties"]["claims"]["items"]["properties"];
        assert_eq!(fields["replaces_revision"]["enum"], json!([null, revision]));
        assert_eq!(
            schema["properties"]["retirements"]["items"]["properties"]["revision_id"]["enum"],
            json!([revision])
        );
        for field in [
            "subject",
            "predicate",
            "value",
            "rationale",
            "line_from",
            "line_to",
            "context_role",
            "kind",
            "replaces_revision",
        ] {
            assert!(
                fields[field]["description"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())
            );
        }
        assert!(inv.instructions.contains("including overlapping matches"));
        assert!(
            inv.instructions
                .contains("input0 independently asserts the same value")
        );
        assert!(inv.instructions.contains("Omission, lack of mention"));
        assert_eq!(
            inv.instructions
                .contains("preceding completed response was rejected"),
            attempt > 0
        );
    }
    run.reconciliation_inputs.clear();
    let gateway::Format::Json { schema, .. } = autonomous::invocation(&run).format else {
        panic!()
    };
    assert_eq!(schema["properties"]["retirements"]["maxItems"], 0);
    let inv = gateway::extraction(Uuid::new_v4(), Uuid::new_v4());
    assert_eq!(
        (&*inv.prompt_label, &*inv.schema_label),
        ("source-facts-4", "source-facts-4")
    );
    assert!(inv.instructions.contains("sentence-valued assertion"));
}

#[tokio::test]
#[ignore = "Owned PostgreSQL; controlled loopback provider, zero paid calls"]
async fn revision4_native_current_retention_requires_exact_catalogue_tuple() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    let doc = source(&h, &owner, &base, "ababa\n").await;
    production_mapping::process(&h).await;
    let run = execute(
        &h,
        &owner,
        &base,
        &p,
        &doc,
        json!({"claims":[],"mapping":mapping(2)}),
    )
    .await;
    assert_eq!(run["state"], "succeeded");
    let actual:(String,String,String,i32,String)=sqlx::query_as("SELECT q.prompt_label,q.schema_label,s.payload->>'adapter_revision',(s.payload#>>'{mentions,0,span,byte_start}')::integer,c.state FROM learning_runs r JOIN model_requests q ON q.id=r.request_id JOIN knowledge_mapping_stages s ON s.run_id=r.id JOIN knowledge_mapping_coverage c ON c.run_id=r.id WHERE r.id=$1").bind(id(&run)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        actual,
        (
            "source-facts-4".into(),
            "source-facts-4".into(),
            production::ADAPTER.into(),
            2,
            "blocked_profile".into()
        )
    );
    for (prompt, adapter) in [
        ("source-facts-3", "learning-quote-anchors-2"),
        ("source-facts-4", "learning-quote-anchors-2"),
    ] {
        catalogue(&h, brain(&base), prompt, adapter).await;
    }
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("producer:{}", e.1)),
        0
    );
    catalogue(&h, brain(&base), "source-facts-4", production::ADAPTER).await;
    // The earlier routing check intentionally backed this stage off for30s.
    // Advance only this owned fixture schedule, avoiding a timing-dependent test.
    sqlx::query(
        "UPDATE knowledge_mapping_stages SET next_check_at=clock_timestamp() WHERE run_id=$1",
    )
    .bind(id(&run))
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("producer:{}", e.1)),
        1
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let stored:(String,i32)=sqlx::query_as("SELECT i.adapter_revision,m.byte_start FROM knowledge_mapping_inputs i JOIN knowledge_mentions m ON m.input_id=i.id WHERE i.brain_id=$1").bind(brain(&base)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(stored, (production::ADAPTER.into(), 2));
    assert_eq!(p.calls.load(Ordering::SeqCst), 1);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Owned PostgreSQL upgrade fixture; controlled loopback, zero paid calls"]
async fn revision4_upgrade_preserves_adapter2_head_stage_and_rejects_wrong_adapter() {
    let mut h = Harness::new_through(63).await;
    let (p, server) = configure_provider(&mut h).await;
    let owner = h.login().await;
    let b = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent upgrade toy"}),
    )
    .await;
    let base = format!("/api/brains/{}", b["id"].as_str().unwrap());
    allow(&h, &owner, &base, |_| {}).await;
    let doc = source(&h, &owner, &base, "ababa\n").await;
    production_mapping::process(&h).await;
    let run = execute(
        &h,
        &owner,
        &base,
        &p,
        &doc,
        json!({"claims":[],"mapping":mapping(1)}),
    )
    .await;
    let rid = id(&run);
    // Trusted-storage fixture models exact historical bytes, never production
    // relabeling. The old coordinate selected the first disjoint occurrence.
    sqlx::query("UPDATE model_requests SET prompt_label='source-facts-3',schema_label='source-facts-3' WHERE id=(SELECT request_id FROM learning_runs WHERE id=$1)").bind(rid).execute(&h.admin).await.unwrap();
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("CREATE TEMP TABLE old_stage ON COMMIT DROP AS SELECT * FROM knowledge_mapping_stages WHERE run_id=$1").bind(rid).execute(&mut *tx).await.unwrap();
    sqlx::query("DELETE FROM knowledge_mapping_stages WHERE run_id=$1")
        .bind(rid)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE old_stage SET payload=jsonb_set(payload,'{adapter_revision}','\"learning-quote-anchors-2\"'::jsonb)").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_stages SELECT * FROM old_stage")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let head = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256) SELECT $1,$2,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model','source-facts-3','source-facts-3','learning-quote-anchors-2','source-mentions-1','qualified',repeat('d',64) FROM model_policy_heads h JOIN model_policies p ON p.id=h.policy_id WHERE h.brain_id=$2").bind(head).bind(brain(&base)).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES($1,$2)")
        .bind(brain(&base))
        .bind(head)
        .execute(&h.admin)
        .await
        .unwrap();
    let before: Value =
        sqlx::query_scalar("SELECT to_jsonb(s) FROM knowledge_mapping_stages s WHERE run_id=$1")
            .bind(rid)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    db::migrate(&h.admin).await.unwrap();
    let after: Value =
        sqlx::query_scalar("SELECT to_jsonb(s) FROM knowledge_mapping_stages s WHERE run_id=$1")
            .bind(rid)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(before, after);
    let current: Uuid = sqlx::query_scalar(
        "SELECT change_id FROM knowledge_mapping_producer_heads WHERE brain_id=$1",
    )
    .bind(brain(&base))
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(current, head);
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("producer:{}", e.1)),
        1
    );
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let stored:String=sqlx::query_scalar("SELECT adapter_revision FROM knowledge_mapping_inputs WHERE brain_id=$1 AND state='published'").bind(brain(&base)).fetch_one(&h.admin).await.unwrap();
    assert_eq!(stored, "learning-quote-anchors-2");
    let wrong = execute(
        &h,
        &owner,
        &base,
        &p,
        &doc,
        json!({"claims":[],"mapping":mapping(2)}),
    )
    .await;
    // New request has adapter3 bytes but old head remains adapter2: no route.
    assert_eq!(
        producer::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("producer:{}", e.1)),
        0
    );
    let status: String =
        sqlx::query_scalar("SELECT state FROM knowledge_mapping_coverage WHERE run_id=$1")
            .bind(id(&wrong))
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(status, "blocked_profile");
    assert_eq!(p.calls.load(Ordering::SeqCst), 2);
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Owned PostgreSQL; actual learning/support with controlled loopback only"]
async fn revision4_native_reuse_replacement_omission_retirement_and_conflict_protection() {
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis"]);
    })
    .await;
    let first = source(&h, &owner, &base, "LanternUnit.mode = steady\n").await;
    let run = execute(
        &h,
        &owner,
        &base,
        &p,
        &first,
        json!({"claims":[assertion("steady",Value::Null)],"retirements":[],"mapping":null}),
    )
    .await;
    assert_eq!(run["state"], "succeeded", "{run}");
    assert_eq!(run["accepted"], 1);
    let claim = run["claim_ids"][0].clone();
    let prior = selected(&h, &owner, &base, &claim).await;
    let same = change(&h, &owner, &base, &first, "LanternUnit.mode = steady\n").await;
    let reuse = execute(
        &h,
        &owner,
        &base,
        &p,
        &same,
        json!({"claims":[assertion("steady",Value::Null)],"retirements":[],"mapping":null}),
    )
    .await;
    assert_eq!(reuse["state"], "succeeded", "{reuse}");
    assert_eq!(reuse["reused"], 1);
    assert_eq!(selected(&h, &owner, &base, &claim).await["id"], prior["id"]);
    let updated = change(&h, &owner, &base, &same, "LanternUnit.mode = pulsed\n").await;
    let revised = execute(
        &h,
        &owner,
        &base,
        &p,
        &updated,
        json!({"claims":[assertion("pulsed",prior["id"].clone())],"retirements":[],"mapping":null}),
    )
    .await;
    assert_eq!(revised["state"], "succeeded", "{revised}");
    assert_eq!(revised["revised"], 1);
    let current = selected(&h, &owner, &base, &claim).await;
    assert_eq!(current["content"]["value"], "pulsed");
    let notes = change(
        &h,
        &owner,
        &base,
        &updated,
        "A routine maintenance note contains no mode declaration.\n",
    )
    .await;
    let omitted = execute(
        &h,
        &owner,
        &base,
        &p,
        &notes,
        json!({"claims":[],"retirements":[],"mapping":null}),
    )
    .await;
    assert_eq!(omitted["state"], "succeeded");
    assert_eq!(omitted["retired"], 0);
    assert_eq!(
        selected(&h, &owner, &base, &claim).await["id"],
        current["id"]
    );
    let removed = change(
        &h,
        &owner,
        &base,
        &notes,
        "LanternUnit was removed; its pulsed mode declaration is explicitly obsolete.\n",
    )
    .await;
    let retired=execute(&h,&owner,&base,&p,&removed,json!({"claims":[],"retirements":[{"revision_id":current["id"],"reason":"The unit was removed and its mode is explicitly obsolete.","line_from":1,"line_to":1}],"mapping":null})).await;
    assert_eq!(retired["state"], "succeeded", "{retired}");
    assert_eq!(retired["retired"], 1);
    let conflict = source(
        &h,
        &owner,
        &base,
        "LanternUnit.mode = slow\nLanternUnit.mode = fast\n",
    )
    .await;
    let mut alternative = assertion("fast", Value::Null);
    alternative["line_from"] = json!(2);
    alternative["line_to"] = json!(2);
    let result=execute(&h,&owner,&base,&p,&conflict,json!({"claims":[assertion("slow",Value::Null),alternative],"retirements":[],"mapping":null})).await;
    assert_eq!(result["state"], "succeeded", "{result}");
    assert_eq!(result["conflicting"], 2);
    assert_eq!(result["accepted"], 0);
    for claim in result["claim_ids"].as_array().unwrap() {
        assert_eq!(
            selected(&h, &owner, &base, claim).await["admission"],
            "uncertain_evidence"
        );
    }
    for body in p
        .bodies
        .lock()
        .unwrap()
        .iter()
        .filter(|body| body["text"]["format"]["name"] == "source_reconciliation")
    {
        assert!(
            body["instructions"]
                .as_str()
                .unwrap()
                .contains("input0 independently asserts the same value")
        );
        assert!(body["text"]["format"]["schema"]["properties"]["claims"]["items"]["properties"]["value"]["description"].is_string());
    }
    server.abort();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Owned PostgreSQL; actual shadow gateway admission with controlled loopback only"]
async fn revision4_shadow_revalidates_retained_request_payload_and_column_identity() {
    use recollect_server::{
        graph::analytics::shadow::{self, Mode, ShadowSource},
        knowledge_mapping::source_features::HybridParameters,
    };
    let (h, owner, base, p, server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    let doc = source(&h, &owner, &base, "ababa\n").await;
    production_mapping::process(&h).await;
    let auth = organization_shadow_privacy::evaluator(&h, &owner).await;
    let cx = gateway::Context {
        brain: brain(&base),
        actor: auth.user.id,
        device: auth.device_id,
    };
    // Exercise actual known request tuples, then prove historical bytes cannot
    // be reinterpreted through a newer stored column or payload adapter.
    for (prompt, adapter, ordinal) in [
        ("source-facts-2", "learning-quote-anchors-1", 1),
        ("source-reconciliation-2", "learning-quote-anchors-1", 1),
        ("source-facts-3", "learning-quote-anchors-2", 1),
        ("source-reconciliation-3", "learning-quote-anchors-2", 1),
        ("source-facts-4", production::ADAPTER, 2),
        ("source-reconciliation-4", production::ADAPTER, 2),
    ] {
        let offered = mapping(ordinal);
        *p.candidates.lock().unwrap() = json!({"claims":[],"mapping":offered});
        let mut inv = gateway::extraction(Uuid::new_v4(), version(&doc));
        inv.prompt_label = prompt.into();
        inv.schema_label = prompt.into();
        let response = gateway::invoke(&h.state, cx, inv)
            .await
            .unwrap_or_else(|e| panic!("gateway:{}", e.1));
        let candidate = shadow::admit(
            &h.state,
            &auth,
            cx,
            response.request.id,
            version(&doc),
            &offered,
        )
        .await
        .unwrap_or_else(|e| panic!("admit:{}", e.1));
        let stored:(String,String,i32)=sqlx::query_as("SELECT adapter_revision,payload->>'adapter_revision',(payload#>>'{mentions,0,span,byte_start}')::integer FROM knowledge_shadow_candidates WHERE id=$1").bind(candidate).fetch_one(&h.admin).await.unwrap();
        assert_eq!(
            stored,
            (
                adapter.into(),
                adapter.into(),
                if ordinal == 2 { 2 } else { 0 }
            )
        );
        let mut tx = h.admin.begin().await.unwrap();
        sqlx::query("CREATE TEMP TABLE preserved_candidate ON COMMIT DROP AS SELECT * FROM knowledge_shadow_candidates WHERE id=$1").bind(candidate).execute(&mut *tx).await.unwrap();
        sqlx::query("DELETE FROM knowledge_shadow_candidates WHERE id=$1")
            .bind(candidate)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE preserved_candidate SET payload=jsonb_set(payload,'{adapter_revision}','\"unrecognized-adapter\"'::jsonb)").execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO knowledge_shadow_candidates SELECT * FROM preserved_candidate")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let input = [ShadowSource {
            source_version_id: version(&doc),
            candidate_id: Some(candidate),
        }];
        let rejected = shadow::queue(
            &h.state,
            &auth,
            brain(&base),
            &input,
            Mode::EntityOverlap,
            HybridParameters::default(),
            "toy-unused-engine",
        )
        .await;
        assert_eq!(rejected.err().unwrap().1, "shadow_input_invalid");
        // Restore via re-admission of the exact original response.
        sqlx::query("DELETE FROM knowledge_shadow_candidates WHERE id=$1")
            .bind(candidate)
            .execute(&h.admin)
            .await
            .unwrap();
        let candidate = shadow::admit(
            &h.state,
            &auth,
            cx,
            response.request.id,
            version(&doc),
            &offered,
        )
        .await
        .unwrap_or_else(|e| panic!("readmit:{}", e.1));
        let mut tx = h.admin.begin().await.unwrap();
        sqlx::query("CREATE TEMP TABLE crossed_candidate ON COMMIT DROP AS SELECT * FROM knowledge_shadow_candidates WHERE id=$1").bind(candidate).execute(&mut *tx).await.unwrap();
        sqlx::query("DELETE FROM knowledge_shadow_candidates WHERE id=$1")
            .bind(candidate)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE crossed_candidate SET adapter_revision=CASE WHEN adapter_revision='learning-quote-anchors-3' THEN 'learning-quote-anchors-2' ELSE 'learning-quote-anchors-3' END").execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO knowledge_shadow_candidates SELECT * FROM crossed_candidate")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let rejected = shadow::queue(
            &h.state,
            &auth,
            brain(&base),
            &[ShadowSource {
                source_version_id: version(&doc),
                candidate_id: Some(candidate),
            }],
            Mode::EntityOverlap,
            HybridParameters::default(),
            "toy-unused-engine",
        )
        .await;
        assert_eq!(rejected.err().unwrap().1, "shadow_input_invalid");
        sqlx::query("DELETE FROM knowledge_shadow_candidates WHERE id=$1")
            .bind(candidate)
            .execute(&h.admin)
            .await
            .unwrap();
        let candidate = shadow::admit(
            &h.state,
            &auth,
            cx,
            response.request.id,
            version(&doc),
            &offered,
        )
        .await
        .unwrap_or_else(|e| panic!("readmit:{}", e.1));
        shadow::queue(
            &h.state,
            &auth,
            brain(&base),
            &[ShadowSource {
                source_version_id: version(&doc),
                candidate_id: Some(candidate),
            }],
            Mode::EntityOverlap,
            HybridParameters::default(),
            "toy-unused-engine",
        )
        .await
        .unwrap_or_else(|e| panic!("queue:{}", e.1));
    }
    assert_eq!(p.calls.load(Ordering::SeqCst), 6);
    server.abort();
    h.finish().await;
}
