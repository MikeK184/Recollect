//! Synthetic trusted-profile plumbing proof; never an extractor-quality receipt.
use super::*;
use axum::extract::FromRequestParts;
use recollect_server::{auth::Auth, knowledge_mapping::storage};
use std::collections::BTreeSet;

async fn auth(h: &Harness, login: &Login) -> Auth {
    let (mut parts, _) = Request::builder()
        .method("POST")
        .header("cookie", &login.cookie)
        .header("x-csrf-token", &login.csrf)
        .body(())
        .unwrap()
        .into_parts();
    Auth::from_request_parts(&mut parts, &h.state)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1))
}
fn bid(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}
pub(super) fn mapping(source: &Value, text: &str, terms: &[String]) -> Value {
    let mentions:Vec<_>=terms.iter().enumerate().map(|(n,term)|{
        let start=text.find(term).unwrap();json!({"key":format!("m{n}"),"kind":"technology","span":{"byte_start":start,"byte_end":start+term.len(),"quote":term},"confidence":1.0})
    }).collect();
    let relation:Vec<_>=(0..terms.len().saturating_sub(1).min(8)).map(|n|
        json!({"from":format!("m{n}"),"to":format!("m{}",n+1),"relation":"related_to","span":{"byte_start":0,"byte_end":text.len(),"quote":text}})).collect();
    json!({"schema":"source-mentions-1","adapter_revision":"synthetic-organization-plumbing-1","source_version_id":source["version"]["id"],"language":"en","mentions":mentions,"relations":relation,"aliases":[]})
}
pub(super) async fn staged(
    h: &Harness,
    owner: &Login,
    base: &str,
    source: &Value,
    payload: &Value,
) -> Uuid {
    let auth = auth(h, owner).await;
    let prepared = storage::prepare_source(
        &h.state,
        &auth,
        bid(base),
        source["version"]["id"].as_str().unwrap().parse().unwrap(),
        &serde_json::to_vec(payload).unwrap(),
    )
    .await
    .unwrap_or_else(|e| panic!("{}", e.1));
    storage::stage_source(&h.state, &auth, prepared)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1))
}
async fn capture(h: &Harness) {
    while worker::run_once(&h.state, "capture").await.unwrap() {}
}

/// Install an explicitly synthetic qualified producer only inside this owned
/// disposable DB. All request/run/input bindings come from the real fake-HTTP
/// gateway path. This tests authorization plumbing, not model extraction quality.
pub(super) async fn synthetic_publish(
    h: &Harness,
    owner: &Login,
    base: &str,
    source: &Value,
    input: Uuid,
    profile: &mut Option<Uuid>,
) {
    let run = learn(h, owner, base, source).await;
    let state = h.state.clone();
    // Match the independent production worker lane without retaining a large
    // test caller's debug poll frame inside gateway preparation.
    assert!(
        tokio::spawn(async move { worker::run_once(&state, "model").await })
            .await
            .unwrap()
            .unwrap()
    );
    let brain = bid(base);
    let run_id: Uuid = run["id"].as_str().unwrap().parse().unwrap();
    let (request, policy): (Uuid, Uuid) = sqlx::query_as(
        "SELECT request_id,policy_id FROM learning_runs WHERE id=$1 AND state='succeeded'",
    )
    .bind(run_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let change = if let Some(change) = *profile {
        change
    } else {
        let change = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256)
         SELECT $1,$2,provider,model,returned_model,prompt_label,schema_label,'synthetic-organization-plumbing-1','source-mentions-1','qualified',repeat('a',64) FROM model_requests WHERE id=$3")
            .bind(change).bind(brain).bind(request).execute(&h.admin).await.unwrap();
        sqlx::query(
            "INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES($1,$2)",
        )
        .bind(brain)
        .bind(change)
        .execute(&h.admin)
        .await
        .unwrap();
        *profile = Some(change);
        change
    };
    sqlx::query("UPDATE knowledge_mapping_inputs SET producer_request_id=$2,producer_policy_id=$3,producer_change_id=$4 WHERE id=$1")
        .bind(input).bind(request).bind(policy).bind(change).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_origins(input_id,brain_id,run_id,request_id,policy_id,producer_change_id,offered_inputs)
        SELECT $1,$2,$3,$4,$5,$6,(SELECT coalesce(jsonb_agg(jsonb_build_object('kind',kind,'id',input_id) ORDER BY kind,input_id),'[]'::jsonb) FROM model_request_inputs WHERE request_id=$4)")
        .bind(input).bind(brain).bind(run_id).bind(request).bind(policy).bind(change).execute(&h.admin).await.unwrap();
    let mut publication = h.admin.begin().await.unwrap();
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(owner_id(h, owner).await.to_string())
        .execute(&mut *publication)
        .await
        .unwrap();
    let current: bool = sqlx::query_scalar("SELECT recollect_mapping_origin_current($1,$2)")
        .bind(brain)
        .bind(input)
        .fetch_one(&mut *publication)
        .await
        .unwrap();
    assert!(
        current,
        "Synthetic publication still requires all actual request/source/context gates"
    );
    sqlx::query("UPDATE knowledge_mapping_inputs SET state='published' WHERE id=$1")
        .bind(input)
        .execute(&mut *publication)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE knowledge_mapping_origins SET published_at=clock_timestamp() WHERE input_id=$1",
    )
    .bind(input)
    .execute(&mut *publication)
    .await
    .unwrap();
    publication.commit().await.unwrap();
    sqlx::query("WITH added AS (INSERT INTO knowledge_topics(id,brain_id,kind,anchor_entity_id,change_id,recipe)
        SELECT gen_random_uuid(),$1,'facet',entity_id,gen_random_uuid(),'entity-facet-1' FROM knowledge_mentions WHERE input_id=$2
        ON CONFLICT(brain_id,anchor_entity_id) WHERE kind='facet' DO NOTHING RETURNING id,change_id)
        INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action) SELECT change_id,$1,id,$3,'create' FROM added")
        .bind(brain).bind(input).bind(owner_id(h,owner).await).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight)
        SELECT $1,t.id,m.entity_id,$2,'entity_facet',1.0 FROM knowledge_mentions m JOIN knowledge_topics t ON t.brain_id=m.brain_id AND t.anchor_entity_id=m.entity_id AND t.kind='facet'
        WHERE m.input_id=$2 ON CONFLICT DO NOTHING").bind(brain).bind(input).execute(&h.admin).await.unwrap();
}
async fn owner_id(h: &Harness, owner: &Login) -> Uuid {
    auth(h, owner).await.user.id
}
async fn view(
    h: &Harness,
    owner: &Login,
    base: &str,
    scope: &Value,
    center: Value,
    cursor: Value,
    limit: usize,
) -> Value {
    let requested_cursor = cursor.clone();
    let started = std::time::Instant::now();
    let (status, value, _) = h
        .call(
            "POST",
            &format!("{base}/organization/view"),
            Some(owner),
            json!({"scope":scope,"center":center,"cursor":cursor,"limit":limit}),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{value}; cursor={requested_cursor}; elapsed_ms={}",
        started.elapsed().as_millis()
    );
    value
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and fake loopback provider"]
async fn organization_staging_is_invisible_and_all_large_detail_tails_are_reachable() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("recollect_server=warn,tower_http=error")
        .try_init();
    let (h, owner, base, p, provider_server) = setup().await;
    // First capture must work before PostgreSQL's background analyzer has
    // observed this newly populated cohort. Keep that real ingestion boundary
    // deterministic instead of allowing timing-dependent automatic statistics.
    for table in [
        "knowledge_mapping_inputs",
        "knowledge_mentions",
        "knowledge_entities",
        "knowledge_topics",
        "knowledge_topic_memberships",
        "knowledge_relation_candidates",
    ] {
        sqlx::query(&format!(
            "ALTER TABLE {table} SET (autovacuum_enabled=false)"
        ))
        .execute(&h.admin)
        .await
        .unwrap();
    }
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let terms: Vec<_> = (0..128).map(|n| format!("Technology{n:03}")).collect();
    let text = terms.join(" ");
    let scope = json!({"kind":"knowledge"});
    let mut profile = None;
    for n in 0..18 {
        let source = source(&h, &owner, &base, &text).await;
        capture(&h).await;
        let input = staged(&h, &owner, &base, &source, &mapping(&source, &text, &terms)).await;
        if n == 0 {
            let empty = view(&h, &owner, &base, &scope, Value::Null, Value::Null, 50).await;
            assert!(empty["entities"].as_array().unwrap().is_empty());
            assert!(empty["topics"].as_array().unwrap().is_empty());
            assert_eq!(empty["coverage"]["mapped_sources"], 0);
        }
        synthetic_publish(&h, &owner, &base, &source, input, &mut profile).await;
    }
    let untrusted = source(&h, &owner, &base, "UntrustedPoison").await;
    capture(&h).await;
    staged(
        &h,
        &owner,
        &base,
        &untrusted,
        &mapping(&untrusted, "UntrustedPoison", &["UntrustedPoison".into()]),
    )
    .await;
    let mut cursor = Value::Null;
    let mut seen_mentions = BTreeSet::new();
    let mut seen_memberships = BTreeSet::new();
    let mut seen_connections = BTreeSet::new();
    let mut seen_entities = BTreeSet::new();
    let mut seen_topics = BTreeSet::new();
    let mut pages = 0;
    let mut latency_ms = vec![];
    loop {
        let started = std::time::Instant::now();
        let page = view(&h, &owner, &base, &scope, Value::Null, cursor, 50).await;
        latency_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        pages += 1;
        assert!(pages < 100, "Every bounded phase must make progress");
        assert!(page["entities"].as_array().unwrap().len() <= 50);
        assert!(page["topics"].as_array().unwrap().len() <= 50);
        assert!(page["representative_mentions"].as_array().unwrap().len() <= 100);
        let rows = page["mentions"].as_array().unwrap().len()
            + page["memberships"].as_array().unwrap().len()
            + page["connections"].as_array().unwrap().len();
        assert!(rows <= 90);
        let cards: Vec<_> = page["entities"]
            .as_array()
            .unwrap()
            .iter()
            .chain(page["topics"].as_array().unwrap())
            .chain(page["context_cards"].as_array().unwrap())
            .collect();
        assert!(cards.len() + page["evidence"].as_array().unwrap().len() <= 500);
        let card_ids: BTreeSet<_> = cards.iter().map(|c| c["id"].as_str().unwrap()).collect();
        let evidence: BTreeSet<_> = page["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["key"].as_str().unwrap())
            .collect();
        assert_eq!(page["coverage"]["qualified_sources"], 19);
        assert_eq!(page["coverage"]["mapped_sources"], 18);
        assert_eq!(page["coverage"]["unassigned_sources"], 1);
        for c in page["entities"].as_array().unwrap() {
            assert_ne!(c["label"], "UntrustedPoison");
            seen_entities.insert(c["id"].as_str().unwrap().to_string());
            assert_eq!(c["source_count"], 18);
        }
        for c in page["topics"].as_array().unwrap() {
            seen_topics.insert(c["id"].as_str().unwrap().to_string());
            assert_eq!(c["source_count"], 18);
        }
        for m in page["mentions"].as_array().unwrap() {
            assert!(seen_mentions.insert(m["id"].as_str().unwrap().to_string()));
            assert!(card_ids.contains(m["entity_id"].as_str().unwrap()));
            assert!(evidence.contains(m["source_key"].as_str().unwrap()));
        }
        for m in page["memberships"].as_array().unwrap() {
            assert!(seen_memberships.insert(format!(
                "{}:{}:{}",
                m["topic_id"], m["entity_id"], m["input_id"]
            )));
            assert!(card_ids.contains(m["topic_id"].as_str().unwrap()));
            assert!(card_ids.contains(m["entity_id"].as_str().unwrap()));
            assert!(evidence.contains(m["source_key"].as_str().unwrap()));
        }
        for r in page["connections"].as_array().unwrap() {
            assert!(seen_connections.insert(r["id"].as_str().unwrap().to_string()));
            assert!(card_ids.contains(r["from"].as_str().unwrap()));
            assert!(card_ids.contains(r["to"].as_str().unwrap()));
            assert!(evidence.contains(r["source_key"].as_str().unwrap()));
        }
        cursor = page["next_cursor"].clone();
        if cursor.is_null() {
            break;
        }
    }
    let mut ordered = latency_ms.clone();
    ordered.sort_by(f64::total_cmp);
    let p95 = ordered[(ordered.len() * 95).div_ceil(100).saturating_sub(1)];
    let receipt = json!({"fixture":"synthetic-trusted-organization-plumbing","planner_statistics":"automatic_analyze_disabled_for_fresh_mapping_tables","source_count":19,"published_inputs":18,"untrusted_inputs":1,"requested_pages":pages,"successful_pages":pages,"failed_pages":0,"mention_rows":seen_mentions.len(),"membership_rows":seen_memberships.len(),"connection_rows":seen_connections.len(),"page_latency_ms":latency_ms,"p95_ms":p95,"paid_model_calls":0});
    let receipt_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/graph-implementation");
    std::fs::create_dir_all(&receipt_dir).unwrap();
    std::fs::write(
        receipt_dir.join("organization-paging-receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert!(
        p95 < 2000.0,
        "Every requested page is counted; p95_ms={p95}"
    );
    assert_eq!(seen_mentions.len(), 2304);
    assert_eq!(seen_memberships.len(), 2304);
    assert_eq!(seen_connections.len(), 144);
    assert_eq!(seen_entities.len(), 128);
    assert_eq!(seen_topics.len(), 128);
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        18,
        "Only real loopback learning requests; organization reads make none"
    );
    h.finish().await;
    provider_server.abort();
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and fake loopback provider"]
async fn organization_scope_names_aliases_and_versioned_overrides_follow_exact_contributors() {
    let (h, owner, base, p, provider_server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let a_group = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Public fixture area"}),
    )
    .await;
    let b_group = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"area","name":"Private fixture area"}),
    )
    .await;
    let manual = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"collection","name":"Manual membership stays"}),
    )
    .await;
    let mut profile = None;
    let mut inputs = vec![];
    let mut sources = vec![];
    for (text, terms, area) in [
        ("Vault runs safely.", vec!["Vault".to_string()], &a_group),
        (
            "Vault is also known as HiddenAlias.",
            vec!["Vault".to_string(), "HiddenAlias".to_string()],
            &b_group,
        ),
    ] {
        let source = source(&h, &owner, &base, text).await;
        capture(&h).await;
        ok(
            &h,
            "PUT",
            &format!("{base}/sources/{}/groups", source["id"].as_str().unwrap()),
            &owner,
            json!({"group_ids":[area["id"],manual["id"]]}),
        )
        .await;
        let mut payload = mapping(&source, text, &terms);
        if terms.len() > 1 {
            payload["aliases"] = json!([{"left":"m0","right":"m1","span":{"byte_start":0,"byte_end":text.len(),"quote":text}}]);
        }
        let input = staged(&h, &owner, &base, &source, &payload).await;
        synthetic_publish(&h, &owner, &base, &source, input, &mut profile).await;
        inputs.push(input);
        sources.push(source);
    }
    // Supported equivalence is explicit in this synthetic fixture. The scoped
    // reader must still require its exact source; shared entity IDs grant none.
    sqlx::query("UPDATE knowledge_relation_candidates SET disposition='accepted' WHERE brain_id=$1 AND relation='alias'")
        .bind(bid(&base)).execute(&h.admin).await.unwrap();
    let full = json!({"kind":"knowledge"});
    let narrow = json!({"kind":"knowledge","selection":{"area_ids":[a_group["id"]]}});
    let all = view(&h, &owner, &base, &full, Value::Null, Value::Null, 50).await;
    let vault = all["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["label"] == "Vault")
        .unwrap()
        .clone();
    let path = format!(
        "{base}/organization/topics/{}",
        vault["id"].as_str().unwrap()
    );
    let rename = json!({"base_change_id":vault["change_id"],"change":{"action":"rename","label":"Private derived name","basis_input":inputs[1]}});
    let renamed = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            rename.clone(),
            Some("organization-rename"),
        )
        .await;
    assert_eq!(renamed.0, StatusCode::OK, "{}", renamed.1);
    let replay = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            rename.clone(),
            Some("organization-rename"),
        )
        .await;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1, renamed.1);
    let mut conflicting = rename.clone();
    conflicting["change"]["label"] = json!("Different");
    assert_eq!(
        h.keyed(
            "PUT",
            &path,
            Some(&owner),
            conflicting,
            Some("organization-rename")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let scoped = view(&h, &owner, &base, &narrow, Value::Null, Value::Null, 50).await;
    let encoded = scoped.to_string();
    assert!(!encoded.contains("HiddenAlias"));
    assert!(!encoded.contains("Private derived name"));
    assert_eq!(scoped["entities"].as_array().unwrap().len(), 1);
    assert_eq!(scoped["entities"][0]["source_count"], 1);
    assert!(scoped["connections"].as_array().unwrap().is_empty());
    assert_eq!(scoped["topics"][0]["label"], "Vault");
    assert_eq!(scoped["topics"][0]["source_count"], 1);
    let centered = view(
        &h,
        &owner,
        &base,
        &narrow,
        json!({"kind":"topic","id":vault["id"]}),
        Value::Null,
        1,
    )
    .await;
    assert_eq!(centered["topics"][0]["label"], "Vault");
    let mut cursor = all["next_cursor"].clone();
    if cursor.is_null() {
        cursor = json!({"after_source":null,"card_offset":0,"row_cursor":{"phase":"done"},"epoch":all["epoch"],"scope":full,"center":null,"expires_at":all["expires_at"]});
    }
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/organization/view"),
            Some(&owner),
            json!({"scope":narrow,"cursor":cursor})
        )
        .await
        .0,
        StatusCode::CONFLICT,
        "A cursor never crosses normalized scopes or epochs"
    );
    let stale =
        json!({"base_change_id":vault["change_id"],"change":{"action":"pin","pinned":true}});
    assert_eq!(
        h.keyed(
            "PUT",
            &path,
            Some(&owner),
            stale,
            Some("organization-stale-pin")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let pin =
        json!({"base_change_id":renamed.1["change_id"],"change":{"action":"pin","pinned":true}});
    let pinned = h
        .keyed("PUT", &path, Some(&owner), pin, Some("organization-pin"))
        .await;
    assert_eq!(pinned.0, StatusCode::OK, "{}", pinned.1);
    let (reader_id, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let forbidden =
        json!({"base_change_id":pinned.1["change_id"],"change":{"action":"pin","pinned":false}});
    assert_eq!(
        h.keyed(
            "PUT",
            &path,
            Some(&reader),
            forbidden,
            Some("organization-reader-pin")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let split = json!({"base_change_id":pinned.1["change_id"],"change":{"action":"split","input_ids":[inputs[1]]}});
    let split = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            split,
            Some("organization-split"),
        )
        .await;
    assert_eq!(split.0, StatusCode::OK, "{}", split.1);
    assert!(split.1["split_topic_id"].is_string());
    let after_split = view(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"topic","id":vault["id"]}),
        Value::Null,
        50,
    )
    .await;
    assert_eq!(after_split["topics"][0]["id"], vault["id"]);
    assert_eq!(after_split["topics"][0]["label"], "Vault");
    assert_eq!(after_split["topics"][0]["source_count"], 1);
    assert_eq!(after_split["topics"][0]["pinned"], true);
    let exclude = json!({"base_change_id":split.1["change_id"],"change":{"action":"exclude","input_ids":[inputs[0]],"excluded":true}});
    let excluded = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            exclude,
            Some("organization-exclude"),
        )
        .await;
    assert_eq!(excluded.0, StatusCode::OK, "{}", excluded.1);
    assert!(
        view(
            &h,
            &owner,
            &base,
            &full,
            json!({"kind":"topic","id":vault["id"]}),
            Value::Null,
            50
        )
        .await["topics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let include = json!({"base_change_id":excluded.1["change_id"],"change":{"action":"exclude","input_ids":[inputs[0]],"excluded":false}});
    let included = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            include,
            Some("organization-include"),
        )
        .await;
    assert_eq!(included.0, StatusCode::OK, "{}", included.1);
    // Actual source unavailability removes all semantic derivatives from reads
    // immediately, including a human rename/pin; it cannot preserve plaintext.
    sqlx::query("UPDATE source_versions SET privacy_state='expired' WHERE id=$1")
        .bind(
            sources[0]["version"]["id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap(),
        )
        .execute(&h.admin)
        .await
        .unwrap();
    let withdrawn = view(
        &h,
        &owner,
        &base,
        &full,
        json!({"kind":"topic","id":vault["id"]}),
        Value::Null,
        50,
    )
    .await;
    assert!(withdrawn["topics"].as_array().unwrap().is_empty());
    let pin_missing =
        json!({"base_change_id":included.1["change_id"],"change":{"action":"pin","pinned":false}});
    assert_eq!(
        h.keyed(
            "PUT",
            &path,
            Some(&owner),
            pin_missing,
            Some("organization-withdrawn-pin")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let manual_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM evidence_memberships WHERE group_id=$1")
            .bind(manual["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        manual_count, 2,
        "Automatic topic commands preserve every manual membership"
    );
    assert!(
        h.call("GET", &format!("{base}/audit"), Some(&owner), Value::Null)
            .await
            .1
            .to_string()
            .contains("organization.topic")
    );
    h.finish().await;
    provider_server.abort();
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and fake loopback provider"]
async fn organization_alias_human_decisions_are_positive_scoped_versioned_and_reversible() {
    let (h, owner, base, p, provider_server) = setup().await;
    allow(&h, &owner, &base, |_| {}).await;
    *p.candidates.lock().unwrap() = json!({"claims":[]});
    let positive = "app is also known as service.";
    let mut profile = None;
    let mut sources = vec![];
    let mut inputs = vec![];
    for text in [
        positive,
        "It is no longer true that app is also known as service.",
        "app is also known as service. This statement was retracted.",
    ] {
        let source = source(&h, &owner, &base, text).await;
        capture(&h).await;
        let mut payload = mapping(&source, text, &["app".into(), "service".into()]);
        let start = text.find("app is also known as service").unwrap();
        let end = start + "app is also known as service".len();
        // This deliberately valid clipped span is adversarial input: literal
        // byte validation alone must not turn its omitted context positive.
        payload["aliases"] = json!([{"left":"m0","right":"m1","span":{"byte_start":start,"byte_end":end+usize::from(text.as_bytes().get(end)==Some(&b'.')),"quote":&text[start..end+usize::from(text.as_bytes().get(end)==Some(&b'.'))]}}]);
        let input = staged(&h, &owner, &base, &source, &payload).await;
        synthetic_publish(&h, &owner, &base, &source, input, &mut profile).await;
        sources.push(source);
        inputs.push(input);
    }
    let scope = json!({"kind":"knowledge"});
    let list_path = format!("{base}/organization/aliases/view");
    let all = ok(&h, "POST", &list_path, &owner, json!({"scope":scope})).await;
    assert_eq!(all["candidates"].as_array().unwrap().len(), 3);
    let candidates = all["candidates"].as_array().unwrap();
    let selected = candidates
        .iter()
        .find(|a| a["input_id"] == inputs[0].to_string())
        .unwrap();
    assert_eq!(selected["acceptance_state"], "explicit_positive_statement");
    assert_eq!(selected["disposition"], "pending");
    assert!(selected["change_id"].is_null());
    for other in candidates
        .iter()
        .filter(|a| a["input_id"] != inputs[0].to_string())
    {
        assert_eq!(other["acceptance_state"], "unsupported_context");
    }
    let entities_before: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id,entity_id FROM knowledge_mentions WHERE input_id=$1 ORDER BY id")
            .bind(inputs[0])
            .fetch_all(&h.admin)
            .await
            .unwrap();
    let id = selected["id"].as_str().unwrap();
    let path = format!("{base}/organization/aliases/{id}");
    let accepted_input = json!({"base_change_id":null,"action":"accept"});
    let accepted = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            accepted_input.clone(),
            Some("alias-accept"),
        )
        .await;
    assert_eq!(accepted.0, StatusCode::OK, "{}", accepted.1);
    assert_eq!(accepted.1["disposition"], "accepted");
    let replay = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            accepted_input.clone(),
            Some("alias-accept"),
        )
        .await;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1, accepted.1);
    assert_eq!(
        h.keyed(
            "PUT",
            &path,
            Some(&owner),
            accepted_input,
            Some("alias-stale")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let organized = view(&h, &owner, &base, &scope, Value::Null, Value::Null, 50).await;
    assert_eq!(
        organized["connections"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["interpretation"] == "supported_alias")
            .count(),
        1
    );
    for (n, other) in candidates
        .iter()
        .filter(|a| a["input_id"] != inputs[0].to_string())
        .enumerate()
    {
        let other_path = format!(
            "{base}/organization/aliases/{}",
            other["id"].as_str().unwrap()
        );
        let failed = h
            .keyed(
                "PUT",
                &other_path,
                Some(&owner),
                json!({"base_change_id":null,"action":"accept"}),
                Some(&format!("alias-context-{n}")),
            )
            .await;
        assert_eq!(failed.0, StatusCode::CONFLICT);
        assert_eq!(failed.1["code"], "alias_evidence_unsupported");
    }
    let (reader_id, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let revoke = json!({"base_change_id":accepted.1["change_id"],"action":"revoke"});
    assert_eq!(
        h.keyed(
            "PUT",
            &path,
            Some(&reader),
            revoke.clone(),
            Some("alias-reader-revoke")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let revoked = h
        .keyed("PUT", &path, Some(&owner), revoke, Some("alias-revoke"))
        .await;
    assert_eq!(revoked.0, StatusCode::OK, "{}", revoked.1);
    assert_eq!(revoked.1["disposition"], "pending");
    let entities_after: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id,entity_id FROM knowledge_mentions WHERE input_id=$1 ORDER BY id")
            .bind(inputs[0])
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        entities_before, entities_after,
        "Equivalence is a reversible link, never destructive identity merging"
    );
    let organized = view(&h, &owner, &base, &scope, Value::Null, Value::Null, 50).await;
    assert!(
        organized["connections"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["interpretation"] != "supported_alias")
    );
    let rejected = h
        .keyed(
            "PUT",
            &path,
            Some(&owner),
            json!({"base_change_id":revoked.1["change_id"],"action":"reject"}),
            Some("alias-reject"),
        )
        .await;
    assert_eq!(rejected.0, StatusCode::OK);
    assert_eq!(rejected.1["disposition"], "rejected");
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM knowledge_alias_changes WHERE candidate_id=$1")
            .bind(id.parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(count, 3);
    let target = json!({"kind":"source","id":sources[0]["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    assert_eq!(preview["knowledge_alias_candidates"], 1);
    assert_eq!(preview["knowledge_alias_changes"], 3);
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM knowledge_alias_changes WHERE candidate_id=$1),(SELECT count(*) FROM knowledge_alias_heads WHERE candidate_id=$1)").bind(id.parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
    assert_eq!(
        counts,
        (0, 0),
        "Physical alias decisions depend on their exact source candidate"
    );
    let listed = ok(&h, "POST", &list_path, &owner, json!({"scope":scope})).await;
    assert_eq!(listed["candidates"].as_array().unwrap().len(), 2);
    assert!(
        h.keyed(
            "PUT",
            &path,
            Some(&owner),
            json!({"base_change_id":rejected.1["change_id"],"action":"accept"}),
            Some("alias-erased-reaccept")
        )
        .await
        .0
        .is_client_error()
    );
    assert!(
        ok(&h, "GET", &format!("{base}/audit"), &owner, Value::Null)
            .await
            .to_string()
            .contains("organization.alias")
    );
    h.finish().await;
    provider_server.abort();
}
