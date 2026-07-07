use super::{review::ok, *};
use recollect_server::privacy_journal;
use std::{path::Path, process::Command};

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn recovery_migrations_forward_only_and_reject_unknown_or_gapped_history() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Journal ordering probe"}),
    )
    .await;
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let before: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM privacy_requests_sequence_seq")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let mut ordering = h.admin.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(73241028)")
        .execute(&mut *ordering)
        .await
        .unwrap();
    let pool = h.admin.clone();
    let request = Uuid::new_v4();
    let blocked = tokio::spawn(async move {
        sqlx::query_scalar::<_,i64>("INSERT INTO privacy_requests(id,brain_id,target,cause,manifest) VALUES($1,$2,'{}','erase','{}') RETURNING sequence /* recovery_order_probe */")
            .bind(request).bind(brain).fetch_one(&pool).await.unwrap()
    });
    let mut waiting = false;
    for _ in 0..200 {
        waiting=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'INSERT%recovery_order_probe%')")
            .fetch_one(&h.admin).await.unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(waiting, "Concurrent insert reached the ownership lock");
    let during: (i64, bool) =
        sqlx::query_as("SELECT last_value,is_called FROM privacy_requests_sequence_seq")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        during, before,
        "Statement trigger must run before identity allocation"
    );
    let abandoned:i64=sqlx::query_scalar("INSERT INTO privacy_requests(id,brain_id,target,cause,manifest) VALUES($1,$2,'{}','erase','{}') RETURNING sequence")
        .bind(Uuid::new_v4()).bind(brain).fetch_one(&mut *ordering).await.unwrap();
    ordering.rollback().await.unwrap();
    let committed = blocked.await.unwrap();
    assert!(
        committed > abandoned,
        "An aborted transaction can leave a legitimate numeric gap"
    );
    sqlx::query("DELETE FROM privacy_requests WHERE id=$1")
        .bind(request)
        .execute(&h.admin)
        .await
        .unwrap();
    let database = format!("recollect_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&h.root)
        .await
        .unwrap();
    sqlx::query(&format!("COMMENT ON DATABASE {database} IS 'Recollect disposable recovery migration integration test'")).execute(&h.root).await.unwrap();
    let mut url = reqwest::Url::parse(&std::env::var("DATABASE_ADMIN_URL").unwrap()).unwrap();
    url.set_path(&database);
    let admin = db::pool(url.as_str()).await.unwrap();
    let mut tx = admin.begin().await.unwrap();
    sqlx::query("CREATE TABLE recollect_migrations(name text PRIMARY KEY,applied_at timestamptz NOT NULL DEFAULT now())").execute(&mut *tx).await.unwrap();
    for (name, sql) in &db::MIGRATIONS[..db::MIGRATIONS.len() - 1] {
        sqlx::raw_sql(sql).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO recollect_migrations(name) VALUES($1)")
            .bind(name)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
    assert!(db::ready(&admin).await.is_err());
    db::migrate(&admin).await.unwrap();
    db::ready(&admin).await.unwrap();
    sqlx::query("INSERT INTO recollect_migrations(name) VALUES('999_unrecognized')")
        .execute(&admin)
        .await
        .unwrap();
    assert!(db::migrate(&admin).await.is_err());
    assert!(db::ready(&admin).await.is_err());
    sqlx::query(
        "DELETE FROM recollect_migrations WHERE name='999_unrecognized' OR name='002_durable_work'",
    )
    .execute(&admin)
    .await
    .unwrap();
    assert!(db::migrate(&admin).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM recollect_migrations")
        .fetch_one(&admin)
        .await
        .unwrap();
    assert_eq!(count, db::MIGRATIONS.len() as i64 - 1);
    admin.close().await;
    sqlx::query(&format!("DROP DATABASE {database} WITH (FORCE)"))
        .execute(&h.root)
        .await
        .unwrap();
    h.finish().await;
}

pub(crate) fn fixture_script(root: &Path, arguments: &[&str], script: &str) -> Value {
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .args(arguments)
        .current_dir(root)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Controlled recovery fixture operation failed"
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

pub(crate) fn mirror_fixture(installation: Uuid) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let fixture = std::env::var("RECOLLECT_TEST_RECOVERY_FIXTURE").unwrap();
    fixture_script(
        root,
        &[&fixture, &installation.to_string()],
        r#"
import sys,json,platform
from pathlib import Path
sys.path.insert(0, 'scripts')
from recovery_transport import Sftp,execute,private_json
f=json.loads(Path(sys.argv[1]).read_text()); root=Path(f['root']) / sys.argv[2]
root.mkdir(mode=0o700)
arch={'arm64':'arm64','aarch64':'arm64','x86_64':'amd64'}[platform.machine()]
age=(Path('.cache/recovery-tools') / f'age-v1.3.2-{platform.system().lower()}-{arch}' / 'age').resolve()
keygen=age.with_name('age-keygen'); identity=root/'age-identity'
execute([keygen,'-o',identity]); recipient=execute([keygen,'-y',identity]).decode().strip()
Sftp(f['remote']).initialize(sys.argv[2])
config={**f['remote'],'recipients':[recipient],'age_binary':str(age),'sftp_binary':'/usr/bin/sftp'}
config_path=root/'mirror.json'; private_json(config_path,config)
print(json.dumps({'config':str(config_path),'identity':str(identity),'root':str(root)}))
"#,
    )
}

#[tokio::test]
#[ignore = "Requires owned PostgreSQL/Neo4j and RECOLLECT_TEST_RECOVERY_FIXTURE"]
async fn remote_erasure_failure_denies_locally_and_retries_one_encrypted_entry() {
    let mut h = Harness::new().await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let fixture = std::env::var("RECOLLECT_TEST_RECOVERY_FIXTURE").unwrap();
    let installation: Uuid = sqlx::query_scalar("SELECT id FROM privacy_installation")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let initialized = mirror_fixture(installation);
    let directory = initialized["root"].as_str().unwrap();
    let mirror = initialized["config"].as_str().unwrap();
    let original: Value = serde_json::from_slice(&std::fs::read(mirror).unwrap()).unwrap();
    let mut bad = original.clone();
    let unknown_hosts = Path::new(directory).join("unknown_hosts");
    std::fs::write(&unknown_hosts, "").unwrap();
    bad["known_hosts_file"] = json!(unknown_hosts);
    std::fs::write(mirror, serde_json::to_vec(&bad).unwrap()).unwrap();
    let mut config = h.state.config.as_ref().clone();
    config.erasure_mirror = Some(mirror.into());
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.router = app(h.state.clone());
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Remote privacy recovery drill"}),
    )
    .await;
    let base = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let input = |title: &str| json!({"title":title,"media_type":"text/plain","content":title,"retain_content":true});
    let erased = ok(
        &h,
        "POST",
        &format!("{base}/sources"),
        &owner,
        input("Private fixture erased after checkpoint"),
    )
    .await;
    let control = ok(
        &h,
        "POST",
        &format!("{base}/sources"),
        &owner,
        input("Independent retained fixture"),
    )
    .await;
    let target = json!({"kind":"source","id":erased["id"]});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    let request = ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    privacy_journal::maintain(&h.state).await.unwrap();
    let status = ok(&h, "GET", &format!("{base}/erasures"), &owner, Value::Null).await;
    assert_eq!(status["items"][0]["error_code"], "journal_unavailable");
    assert_eq!(status["items"][0]["journaled"], false);
    let version_url = |s: &Value| {
        format!(
            "{base}/sources/{}/versions/{}",
            s["id"].as_str().unwrap(),
            s["version"]["id"].as_str().unwrap()
        )
    };
    let denied = ok(&h, "GET", &version_url(&erased), &owner, Value::Null).await;
    assert_eq!(denied["version"]["privacy_state"], "erased");
    assert!(denied["content"].is_null());
    let retained = ok(&h, "GET", &version_url(&control), &owner, Value::Null).await;
    assert_eq!(retained["content"], "Independent retained fixture");
    std::fs::write(mirror, serde_json::to_vec(&original).unwrap()).unwrap();
    privacy_journal::maintain(&h.state).await.unwrap();
    // Startup re-synchronization exercises retry/rename over the same remote
    // identity. No second request or additional canonical sequence is created.
    privacy_journal::barrier(&h.state.pool, &h.state.config)
        .await
        .unwrap();
    let status = ok(&h, "GET", &format!("{base}/erasures"), &owner, Value::Null).await;
    assert_eq!(status["items"].as_array().unwrap().len(), 1);
    assert_eq!(status["items"][0]["state"], "complete");
    let remote = fixture_script(
        root,
        &[&fixture, &installation.to_string(), directory],
        r#"
import sys,json
from pathlib import Path
sys.path.insert(0,'scripts')
from recovery_transport import Sftp,decrypt
from recovery import fetch_journals
f=json.loads(Path(sys.argv[1]).read_text()); root=Path(sys.argv[3]); c=Sftp(f['remote'])
names=c.list(sys.argv[2],'journal'); assert len(names)==1
encrypted=root/'received.age'; c.get(sys.argv[2],'journal/'+names[0],encrypted)
cfg=json.loads((root/'mirror.json').read_text()); plain=root/'received.json'
decrypt(cfg['age_binary'],root/'age-identity',encrypted,plain)
wrapped=json.loads(plain.read_text()); assert wrapped['previous'] is None
entry=wrapped['entry']; entry['encrypted']=not b'Private fixture' in encrypted.read_bytes()
recovered=fetch_journals({'installation_id':sys.argv[2],'remote':f['remote'],'backup_dir':str(root),
    'age_binary':cfg['age_binary'],'recipients':cfg['recipients']},root/'age-identity')
assert recovered['erasure_entries']==1 and recovered['analytical_entries']==0
plain.unlink(); print(json.dumps(entry))
"#,
    );
    assert_eq!(remote["id"], request["id"]);
    assert_eq!(remote["installation_id"], json!(installation));
    assert_eq!(remote["encrypted"], true);
    assert!(!remote.to_string().contains("Private fixture"));
    assert!(
        std::fs::read_dir(Path::new(&h.state.config.erasure_journal).join(".mirror-uploads"))
            .unwrap()
            .next()
            .is_none()
    );
    h.finish().await;
    std::fs::remove_dir_all(directory).unwrap();
}
