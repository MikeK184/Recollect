use super::*;
use recollect_server::{graph::analytics, privacy_journal};

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j/GDS and RECOLLECT_TEST_RECOVERY_FIXTURE"]
async fn recovery_analytics_ownership_is_mirrored_before_native_work_and_retained_for_restore() {
    let mut h = Harness::new().await;
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let fixture = crate::recovery::mirror_fixture(installation);
    let mut config = (*h.state.config).clone();
    config.erasure_mirror = Some(fixture["config"].as_str().unwrap().into());
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    let owner = h.login().await;
    let (_, base, scope) = super::recovery_tests::setup(&h, &owner).await;
    let report = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    completed(&h, &owner, &base, &report).await;
    let attempt: Uuid = sqlx::query_scalar("SELECT id FROM analytics_attempts WHERE report_id=$1")
        .bind(report["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let remote = crate::recovery::fixture_script(
        root,
        &[
            &std::env::var("RECOLLECT_TEST_RECOVERY_FIXTURE").unwrap(),
            &installation.to_string(),
            fixture["root"].as_str().unwrap(),
            &attempt.to_string(),
        ],
        r#"
import sys,json
from pathlib import Path
sys.path.insert(0,'scripts')
from recovery_transport import Sftp,decrypt
from recovery import fetch_journals
f=json.loads(Path(sys.argv[1]).read_text()); root=Path(sys.argv[3]); c=Sftp(f['remote'])
names=c.list(sys.argv[2],'analytics'); assert names==[sys.argv[4]+'.json.age']
c.get(sys.argv[2],'analytics/'+names[0],root/'entry.age')
cfg=json.loads((root/'mirror.json').read_text())
decrypt(cfg['age_binary'],root/'age-identity',root/'entry.age',root/'entry.json')
entry=json.loads((root/'entry.json').read_text()); assert entry['closed'] is True
recovered=fetch_journals({'installation_id':sys.argv[2],'remote':f['remote'],'backup_dir':str(root),
    'age_binary':cfg['age_binary'],'recipients':cfg['recipients']},root/'age-identity')
assert recovered['analytical_entries']==1
(root/'entry.json').unlink(); print(json.dumps({'id':entry['id'],'closed':entry['closed']}))
"#,
    );
    assert_eq!(remote["id"], json!(attempt));
    assert_eq!(remote["closed"], true);
    h.finish().await;
    std::fs::remove_dir_all(fixture["root"].as_str().unwrap()).unwrap();
}

async fn scratch(h: &Harness, name: &str) {
    assert_eq!(cypher(h,"UNWIND range(0,1) AS source WITH gds.graph.project($graph,source,null) AS graph RETURN graph.graphName",json!({"graph":name})).await,json!([[name]]));
}
async fn exists(h: &Harness, name: &str) -> bool {
    cypher(
        h,
        "CALL gds.graph.list() YIELD graphName WHERE graphName=$graph RETURN graphName",
        json!({"graph":name}),
    )
    .await
        == json!([[name]])
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_journal_ownership_blocks_erasure_and_replays_older_database() {
    let mut h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = super::recovery_tests::setup(&h, &owner).await;
    // An actual older PostgreSQL database, taken before any analytical attempt.
    // The retained installation journal remains outside both database copies.
    let backup = format!("recollect_test_{}", Uuid::new_v4().simple());
    eprintln!("Disposable analytical restore fixture: {backup}");
    h.state.pool.close().await;
    h.admin.close().await;
    sqlx::query(&format!("CREATE DATABASE {backup} TEMPLATE {}", h.database))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {backup} IS 'Recollect disposable analytics restore test created by crates/server/tests/platform/graph_analytics_journal.rs'"))
        .execute(&h.root).await.unwrap();
    let mut url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    url.set_path(&h.database);
    h.admin = db::pool(url.as_str()).await.unwrap();
    h.state = AppState::new(
        db::pool(&h.state.config.database_url).await.unwrap(),
        (*h.state.config).clone(),
    )
    .unwrap();
    h.router = app(h.state.clone());
    url.set_path(&backup);
    let backup_admin = db::pool(url.as_str()).await.unwrap();
    let mut restored = (*h.state.config).clone();
    let mut url = reqwest::Url::parse(&restored.database_url).unwrap();
    url.set_path(&backup);
    restored.database_url = url.to_string();
    let backup_pool = db::pool(&restored.database_url).await.unwrap();

    let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    completed(&h, &owner, &base, &r).await;
    let id: Uuid = r["id"].as_str().unwrap().parse().unwrap();
    let (attempt, installation): (Uuid,Uuid) = sqlx::query_as("SELECT a.id,i.id FROM analytics_attempts a CROSS JOIN privacy_installation i WHERE a.report_id=$1")
        .bind(id).fetch_one(&h.admin).await.unwrap();
    let directory = std::path::Path::new(&h.state.config.erasure_journal)
        .join(format!("analytics-{installation}"));
    let entry = directory.join(format!("{attempt}.json"));
    let original = tokio::fs::read(&entry).await.unwrap();
    let owned = format!(
        "recollect_analytics_{}_{}",
        installation.simple(),
        attempt.simple()
    );
    scratch(&h, &owned).await;
    // Missing ownership is a recovery error, never permission to forget native
    // state. Even a cleaned canonical row retains ownership until its deadline.
    for path in [directory.join("installation.json"), entry.clone()] {
        let held = path.with_extension("tmp");
        tokio::fs::rename(&path, &held).await.unwrap();
        assert!(
            analytics::reconcile(&h.state.config, &h.state.pool)
                .await
                .is_err()
        );
        assert!(exists(&h, &owned).await);
        tokio::fs::rename(&held, &path).await.unwrap();
    }
    let mut mismatch: Value = serde_json::from_slice(&original).unwrap();
    mismatch["privacy_sequence"] = json!(mismatch["privacy_sequence"].as_i64().unwrap() + 100);
    tokio::fs::write(&entry, serde_json::to_vec(&mismatch).unwrap())
        .await
        .unwrap();
    let target = json!({"kind":"snapshot","id":scope["snapshot_id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let erased = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    let request: Uuid = erased["id"].as_str().unwrap().parse().unwrap();
    privacy_journal::maintain(&h.state).await.unwrap();
    let pending: (String, bool) =
        sqlx::query_as("SELECT state,graph_pending FROM privacy_requests WHERE id=$1")
            .bind(request)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(pending, ("error".into(), true));
    assert!(
        exists(&h, &owned).await,
        "A forged newer privacy sequence cannot permit erasure completion while scratch survives"
    );
    tokio::fs::write(&entry, &original).await.unwrap();
    privacy_journal::maintain(&h.state).await.unwrap();
    let done: (String, bool) =
        sqlx::query_as("SELECT state,graph_pending FROM privacy_requests WHERE id=$1")
            .bind(request)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(done, ("complete".into(), false));
    assert!(!exists(&h, &owned).await);

    let reports: i64 = sqlx::query_scalar("SELECT count(*) FROM analytics_attempts")
        .fetch_one(&backup_admin)
        .await
        .unwrap();
    assert_eq!(
        reports, 0,
        "The older database has no canonical attempt ownership to consult"
    );
    scratch(&h, &owned).await;
    let unrelated = format!(
        "recollect_analytics_{}_{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    scratch(&h, &unrelated).await;
    assert!(
        privacy_journal::barrier(&backup_pool, &restored)
            .await
            .is_err()
    );
    privacy_journal::reconcile(&backup_admin, &restored)
        .await
        .unwrap();
    privacy_journal::barrier(&backup_pool, &restored)
        .await
        .unwrap();
    assert!(!exists(&h, &owned).await);
    assert!(
        exists(&h, &unrelated).await,
        "Recovery must not evict another installation's catalog entry"
    );
    assert_eq!(
        cypher(
            &h,
            "CALL gds.graph.drop($graph) YIELD graphName RETURN graphName",
            json!({"graph":unrelated})
        )
        .await,
        json!([[unrelated]])
    );
    let replayed: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM privacy_requests WHERE id=$1 AND state='complete' AND NOT graph_pending)")
            .bind(request)
            .fetch_one(&backup_admin)
            .await
            .unwrap();
    assert!(replayed);
    backup_pool.close().await;
    backup_admin.close().await;
    sqlx::query(&format!("DROP DATABASE {backup} WITH (FORCE)"))
        .execute(&h.root)
        .await
        .unwrap();
    assert_eq!(
        read(&h, &owner, &base, &r["id"]).await["report"]["state"],
        "removed"
    );
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM analytics_attempts WHERE brain_id=$1 AND cleaned_at IS NULL",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
    h.finish().await;
}
