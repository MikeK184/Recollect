//! Set-based recall read path: statement count and lexical index eligibility.
use super::*;
use std::sync::{
    Once,
    atomic::{AtomicUsize, Ordering},
};
use tracing_subscriber::{Layer, layer::SubscriberExt};

static STATEMENTS: AtomicUsize = AtomicUsize::new(0);
static INSTALL: Once = Once::new();

/// Counts every statement sqlx reports on its query target.
struct Counter;
impl<S: tracing::Subscriber> Layer<S> for Counter {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() == "sqlx::query" {
            STATEMENTS.fetch_add(1, Ordering::SeqCst);
        }
    }
}
fn counting() {
    INSTALL.call_once(|| {
        let _ =
            tracing::subscriber::set_global_default(tracing_subscriber::registry().with(Counter));
    });
}
async fn counted(h: &Harness, base: &str, login: &Login, input: Value) -> (Value, usize) {
    let before = STATEMENTS.load(Ordering::SeqCst);
    let result = recall(h, base, login, input).await;
    (result, STATEMENTS.load(Ordering::SeqCst) - before)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn recall_statement_count_does_not_grow_with_candidates() {
    counting();
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Recall statement corpus"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let mut versions = Vec::new();
    for i in 0..85 {
        let text = format!(
            "Gauge reading {i} is recorded in the statement fixture.\nAnother line about {i}.\n"
        );
        versions.push(
            source(&h, &base, &owner, &format!("Gauge note {i}"), &text).await["version"]["id"]
                .clone(),
        );
    }
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    for i in 0..15 {
        let mut input = super::super::review::proposal(
            &versions[i % versions.len()],
            &format!("Gauge subject {i}"),
            &format!("value {i}"),
        );
        input["content"]["supports"][0]["line_from"] = json!(1);
        input["content"]["supports"][0]["line_to"] = json!(1);
        let proposed = ok(&h, "POST", &format!("{base}/claims"), &owner, input).await;
        // Unreviewed proposals are not usable support; review each as accepted.
        ok(
            &h,
            "POST",
            &format!("{base}/claims/{}/review", proposed["claim_id"].as_str().unwrap()),
            &owner,
            json!({"base_revision":proposed["id"],"action":"accept","reason":"Synthetic evidence reviewed by the signed-in account.","content":null,"revalidation_basis":null}),
        )
        .await;
    }
    // Warm statistics and caches so the measured request is steady-state.
    recall(&h, &base, &owner, json!({"query":"Gauge"})).await;
    let (lexical, lexical_count) = counted(&h, &base, &owner, json!({"query":"Gauge"})).await;
    assert_eq!(lexical["coverage"]["examined"], 100, "{lexical}");
    let (automatic, automatic_count) = counted(
        &h,
        &base,
        &owner,
        json!({"strategy":"auto","semantic_request_id":Uuid::new_v4(),"query":"What is the Gauge?"}),
    )
    .await;
    assert_eq!(automatic["coverage"]["examined"], 100, "{automatic}");
    eprintln!("RECALL_STATEMENTS lexical={lexical_count} auto={automatic_count}");
}

/// Seed many rows directly (foreign keys and triggers are bypassed on the
/// disposable database only), analyze, and read the lexical pass's plan.
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn lexical_window_reaches_rows_through_the_recall_vector_gin_indexes() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Recall plan corpus"}),
    )
    .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain_id}");
    let version =
        source(&h, &base, &owner, "Template", "Template text.\n").await["version"]["id"].clone();
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &owner,
        super::super::review::proposal(&version, "Template", "value"),
    )
    .await;
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::raw_sql(
        "SET LOCAL session_replication_role=replica;
        WITH t AS (SELECT * FROM claim_revisions LIMIT 1), n AS (SELECT gen_random_uuid() id,gen_random_uuid() cid,i FROM generate_series(1,20000) i)
        INSERT INTO claim_revisions(id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key,privacy_state)
        SELECT n.id,n.cid,t.brain_id,t.recorded_at,
          jsonb_set(jsonb_set(jsonb_set(t.revision,'{id}',to_jsonb(n.id::text)),'{claim_id}',to_jsonb(n.cid::text)),
            '{content,value}',to_jsonb('filler value '||n.i||CASE WHEN n.i%10000=0 THEN ' needleword' ELSE '' END||' '||repeat('padding ',400))),
          'subject '||n.i,'configuration','filler '||n.i,t.privacy_state FROM t,n;
        WITH t AS (SELECT * FROM source_versions LIMIT 1), n AS (SELECT gen_random_uuid() id,gen_random_uuid() sid,i FROM generate_series(1,20000) i)
        INSERT INTO source_versions(id,source_id,brain_id,title,media_type,source_uri,observed_at,artifact_id,byte_length,created_by,device_id,created_at,processing,retention_class,privacy_state)
        SELECT n.id,n.sid,t.brain_id,'filler title '||n.i,t.media_type,NULL,NULL,NULL,20,t.created_by,NULL,t.created_at,'ready',t.retention_class,'active' FROM t,n;
        INSERT INTO source_chunks(id,brain_id,version_id,ordinal,byte_start,byte_end,line_start,line_end,content)
        SELECT gen_random_uuid(),v.brain_id,v.id,0,0,20,1,1,'filler text '||right(v.title,4)||CASE WHEN right(v.title,4)::text ~ '0000$' THEN ' needleword' ELSE '' END||' '||repeat('padding ',400)
        FROM source_versions v WHERE v.title LIKE 'filler title %';
        INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record)
        SELECT gen_random_uuid(),(SELECT id FROM brains LIMIT 1),gen_random_uuid(),i,jsonb_build_object('kind','function','name','filler'||i||CASE WHEN i%10000=0 THEN ' needleword' ELSE '' END,'file','src/f'||i||'.rs','line',1,'pad',repeat('padding ',400))
        FROM generate_series(1,20000) i;
        INSERT INTO manifest_revisions(id,manifest_id,brain_id,revision)
        SELECT gen_random_uuid(),gen_random_uuid(),(SELECT id FROM brains LIMIT 1),jsonb_build_object('name','filler manifest '||i||CASE WHEN i%10000=0 THEN ' needleword' ELSE '' END,'entries','[]'::jsonb,'pad',repeat('padding ',400))
        FROM generate_series(1,20000) i;",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    sqlx::raw_sql("ANALYZE claim_revisions;ANALYZE source_versions;ANALYZE source_chunks;ANALYZE repository_facts;ANALYZE manifest_revisions;")
        .execute(&h.admin)
        .await
        .unwrap();
    // Short, high-cardinality title vectors are legitimately cheaper to scan on
    // a narrow table; the title index must still be eligible when selective.
    const GIN: [&str; 4] = [
        "claim_recall_lexical",
        "source_recall_lexical",
        "fact_recall_lexical",
        "manifest_recall_lexical",
    ];
    // The windows answer each table as the migration role (row-level security
    // keeps the app role off these indexes); prove each window's query plan.
    for (table, column, index) in [
        ("claim_revisions", "claim_id", "claim_recall_lexical"),
        ("source_chunks", "version_id", "source_recall_lexical"),
        ("repository_facts", "id", "fact_recall_lexical"),
        ("manifest_revisions", "id", "manifest_recall_lexical"),
    ] {
        let plan: Vec<String> = sqlx::query_scalar(&format!(
            "EXPLAIN SELECT {column} FROM {table} WHERE brain_id=$1 AND recall_vector @@ websearch_to_tsquery('simple','needleword')"
        ))
        .bind(brain_id)
        .fetch_all(&h.admin)
        .await
        .unwrap();
        let plan = plan.join("\n");
        assert!(
            plan.contains(index),
            "{table} does not use {index}:\n{plan}"
        );
        eprintln!("WINDOW_PLAN {table}\n{plan}");
    }
    let mut eligible = h.admin.begin().await.unwrap();
    sqlx::query("SET LOCAL enable_seqscan=off")
        .execute(&mut *eligible)
        .await
        .unwrap();
    let plan: Vec<String> = sqlx::query_scalar("EXPLAIN SELECT source_id FROM source_versions WHERE brain_id=$1 AND recall_vector @@ websearch_to_tsquery('simple','needleword')")
        .bind(brain_id)
        .fetch_all(&mut *eligible)
        .await
        .unwrap();
    assert!(plan.join("\n").contains("source_recall_title"));
    eligible.rollback().await.unwrap();
    async fn scans(h: &Harness) -> std::collections::BTreeMap<String, i64> {
        sqlx::query_as::<_, (String, i64)>(
            "SELECT indexrelname::text,idx_scan FROM pg_stat_user_indexes WHERE indexrelname=ANY($1)",
        )
        .bind(GIN.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .fetch_all(&h.admin)
        .await
        .unwrap()
        .into_iter()
        .collect()
    }
    let before = scans(&h).await;
    let result = recall(&h, &base, &owner, json!({"query":"needleword"})).await;
    assert!(
        result["coverage"]["examined"].as_u64().unwrap() >= 1,
        "{result}"
    );
    // The whole recall (as the RLS-bound app role) must have used every GIN index.
    let mut waited = 0;
    loop {
        let after = scans(&h).await;
        let unused: Vec<_> = GIN.iter().filter(|i| after[**i] <= before[**i]).collect();
        if unused.is_empty() {
            break;
        }
        assert!(waited < 40, "recall did not scan {unused:?}");
        waited += 1;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    // And the candidate pass itself reads the Brain through the windows.
    let actor: Uuid = sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let mut tx = recollect_server::db::actor_tx(&h.state.pool, actor)
        .await
        .unwrap_or_else(|_| panic!("actor transaction"));
    let plan = recollect_server::retrieval::explain_lexical_window(&mut tx, brain_id, "needleword")
        .await
        .unwrap_or_else(|_| panic!("explain"))
        .join("\n");
    for window in [
        "recollect_window_claims",
        "recollect_window_sources",
        "recollect_window_facts",
        "recollect_window_manifests",
    ] {
        assert!(plan.contains(window), "{window} is not used:\n{plan}");
    }
    tx.rollback().await.unwrap();
    h.finish().await;
}
