//! Isolated native ready-commit -> gateway-admission counterfactual. No core hooks.
use super::*;
use axum::body::Bytes;

use std::{path::PathBuf, time::Duration};
use tokio::sync::oneshot;

#[derive(Default)]
struct LocalProvider {
    bodies: Mutex<Vec<Vec<u8>>>,
}
async fn embedding(State(p): State<Arc<LocalProvider>>, bytes: Bytes) -> Json<Value> {
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    p.bodies.lock().unwrap().push(bytes.to_vec());
    let mut vector = vec![0.0; body["dimensions"].as_u64().unwrap() as usize];
    vector[0] = 1.0;
    Json(
        json!({"object":"list","model":body["model"],"data":body["input"].as_array().unwrap().iter().enumerate().map(|(i,_)|json!({"object":"embedding","index":i,"embedding":vector})).collect::<Vec<_>>(),"usage":{"prompt_tokens":6,"total_tokens":6}}),
    )
}
fn state_root() -> PathBuf {
    Harness::semantic_admission_state_dir()
        .canonicalize()
        .expect("Scoped semantic fixture state must already exist")
}
fn base() -> PathBuf {
    state_root()
        .parent()
        .expect("Scoped semantic fixture parent")
        .to_owned()
}
fn persist(name: &str, value: &Value) {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let path = base().join(name);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    serde_json::to_writer_pretty(&mut f, value).unwrap();
    f.write_all(b"\n").unwrap();
    f.sync_all().unwrap();
}
async fn wait_blocked(admin: &PgPool, blocker: i32, query_part: &str) -> i32 {
    tokio::time::timeout(Duration::from_secs(15),async {
        loop {
            let pid:Option<i32>=sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) AND position($2 in query)>0 ORDER BY pid LIMIT 1")
                .bind(blocker).bind(query_part).fetch_optional(admin).await.unwrap();
            if let Some(pid)=pid {return pid;}
            tokio::task::yield_now().await;
        }
    }).await.expect("Native PostgreSQL blocking relationship did not occur")
}
const SNAPSHOT_SQL: &str = "SELECT jsonb_build_object('job_state',j.state,'lease_present',j.lease_token IS NOT NULL,'lease_future',coalesce(j.lease_until>clock_timestamp(),false),'batch_state',b.state,'entry_count',(SELECT count(*) FROM semantic_entries WHERE batch_id=b.id),'running_entries',(SELECT count(*) FROM semantic_entries WHERE batch_id=b.id AND state='running'),'ready_entries',(SELECT count(*) FROM semantic_entries WHERE batch_id=b.id AND state='ready'),'embeddings',(SELECT count(*) FROM semantic_entries WHERE batch_id=b.id AND embedding IS NOT NULL),'requests',(SELECT count(*) FROM model_requests WHERE operation_id=b.id),'publication_audits',(SELECT count(*) FROM mutation_audit WHERE action='semantic.publish' AND target_id=b.id)) FROM jobs j JOIN semantic_batches b ON b.job_id=j.id WHERE j.id=$1";
async fn snapshot_at<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
    executor: E,
    job: &worker::ClaimedJob,
) -> Value {
    sqlx::query_scalar(SNAPSHOT_SQL)
        .bind(job.id)
        .fetch_one(executor)
        .await
        .unwrap()
}
async fn snapshot(h: &Harness, job: &worker::ClaimedJob) -> Value {
    snapshot_at(&h.admin, job).await
}
async fn cleanup(h: Harness, receipt: &str) {
    let Harness {
        router,
        state,
        admin,
        root,
        database,
    } = h;
    let marker: Option<String> = sqlx::query_scalar(
        "SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=$1",
    )
    .bind(&database)
    .fetch_one(&root)
    .await
    .unwrap();
    assert_eq!(
        marker,
        Some(format!("Recollect semantic lease rehearsal {database}"))
    );
    assert!(database.starts_with("recollect_lease_rehearsal_") && database.len() == 58);
    let paths = [
        state.config.credential_file.clone(),
        state.config.artifact_dir.clone(),
        state.config.erasure_journal.clone(),
    ];
    state.pool.close().await;
    admin.close().await;
    drop(router);
    drop(state);
    drop(admin);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=$1")
                    .bind(&database)
                    .fetch_one(&root)
                    .await
                    .unwrap();
            if count == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("Exact owned DB still has sessions; no FORCE or termination permitted");
    sqlx::query(&format!("DROP DATABASE {database}"))
        .execute(&root)
        .await
        .unwrap();
    let absent: bool =
        sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
            .bind(&database)
            .fetch_one(&root)
            .await
            .unwrap();
    assert!(absent);
    root.close().await;
    // Held parent descriptors, exact nonce-derived entries, no symlink traversal.
    let python = r#"import os,stat,sys,shutil
parent=os.path.abspath(sys.argv[1]); database=sys.argv[2]
fd=os.open(parent,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
assert stat.S_IMODE(os.fstat(fd).st_mode)==0o700 and os.fstat(fd).st_uid==os.getuid()
for path in sys.argv[3:]:
 assert os.path.dirname(os.path.abspath(path))==parent
 name=os.path.basename(path); assert name.startswith(database+'-')
 try: st=os.stat(name,dir_fd=fd,follow_symlinks=False)
 except FileNotFoundError: continue
 assert st.st_uid==os.getuid() and not stat.S_ISLNK(st.st_mode)
 if stat.S_ISDIR(st.st_mode):
  owned=os.open(name,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd)
  assert os.fstat(owned).st_ino==st.st_ino
  assert shutil.rmtree.avoids_symlink_attacks
  for child in os.listdir(owned):
   mode=os.stat(child,dir_fd=owned,follow_symlinks=False).st_mode
   if stat.S_ISDIR(mode): shutil.rmtree(child,dir_fd=owned)
   else: os.unlink(child,dir_fd=owned)
  assert os.listdir(owned)==[];os.close(owned)
  assert os.stat(name,dir_fd=fd,follow_symlinks=False).st_ino==st.st_ino
  os.rmdir(name,dir_fd=fd)
 else:
  assert stat.S_ISREG(st.st_mode);os.unlink(name,dir_fd=fd)
os.close(fd)
"#;
    let state_root = state_root();
    let status = tokio::process::Command::new("python3")
        .args(["-E", "-s", "-B", "-c", python])
        .arg(state_root)
        .arg(&database)
        .args(paths)
        .status()
        .await
        .unwrap();
    assert!(status.success());
    persist(
        receipt,
        &json!({"database":database,"marker_verified":true,"sessions_before_drop":0,"database_absent":true,"force":false,"terminated_sessions":0,"exact_owned_state_removed":true}),
    );
}
async fn fixture() -> (
    Harness,
    Login,
    String,
    Arc<LocalProvider>,
    tokio::task::JoinHandle<()>,
    worker::ClaimedJob,
    gateway::Preview,
) {
    let mut h = Harness::new_semantic_admission().await;
    let provider = Arc::new(LocalProvider::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new()
        .route("/v1/embeddings", post(embedding))
        .with_state(provider.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*h.state.config).clone();
    config.models.endpoint = format!("http://{address}/v1");
    config.models.key = Some("synthetic-lease-rehearsal-only".into());
    config.models.openrouter_key = None;
    assert!(
        reqwest::Url::parse(&config.models.endpoint)
            .unwrap()
            .host_str()
            == Some("127.0.0.1")
    );
    assert_eq!(
        config.models.key.as_deref(),
        Some("synthetic-lease-rehearsal-only")
    );
    assert!(config.models.openrouter_key.is_none());
    h.state = AppState::new(h.state.pool.clone(), config).unwrap();
    h.state.http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    h.router = app(h.state.clone());
    let owner = h.login().await;
    let brain = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Independent semantic lease boundary"}),
    )
    .await;
    let url = format!("/api/brains/{}", brain["id"].as_str().unwrap());
    let cx = context(&h, &url).await;
    source(&h, &owner, &url, "Aster uses the declared local endpoint.").await;
    while worker::run_once(&h.state, "capture").await.unwrap() {}
    enable(&h, &owner, &url).await;
    ok(
        &h,
        "POST",
        &format!("{url}/semantic/reindex"),
        &owner,
        json!({"base_profile":null}),
    )
    .await;
    semantic::maintain_brain(&h.state, cx.brain, cx.actor)
        .await
        .unwrap_or_else(|e| panic!("Semantic fixture discovery rejected: {}", e.1));
    let job = worker::claim(&h.state.pool, "model")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.kind, "semantic.generate");
    let policy: Uuid = sqlx::query_scalar("SELECT policy_id FROM semantic_batches WHERE id=$1")
        .bind(job.target_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let entries: Vec<Uuid> = sqlx::query_scalar(
        "SELECT entry_id FROM semantic_batch_inputs WHERE batch_id=$1 ORDER BY ordinal",
    )
    .bind(job.target_id)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    let invocation = gateway::Invocation {
        operation: job.target_id,
        purpose: "embedding".into(),
        inputs: entries
            .into_iter()
            .map(|id| gateway::InputRef {
                kind: "semantic_entry".into(),
                id,
            })
            .collect(),
        query: None,
        instructions: String::new(),
        prompt_label: semantic::REPRESENTATION.into(),
        schema_label: "embedding-float-1".into(),
        format: gateway::Format::Embedding,
        work_lease: None,
        metadata_replay: false,
        expected_json: None,
    };
    let preview = gateway::preview_for_policy(&h.state, cx, &invocation, policy)
        .await
        .unwrap_or_else(|e| panic!("Canonical fixture preview rejected: {}", e.1));
    assert_eq!(provider.bodies.lock().unwrap().len(), 0);
    persist(
        &format!("{}-ownership.json", h.database),
        &json!({"database":h.database,"marker":format!("Recollect semantic lease rehearsal {}",h.database),"brain":cx.brain,"job":job.id,"batch":job.target_id,"source":"independent authored toy","provider_loopback":true,"configured_provider_synthetic_only":true,"inherited_provider_environment_ignored":true}),
    );
    (h, owner, url, provider, server, job, preview)
}
async fn run(previewed: bool, mutation: Option<&str>) {
    let (h, _owner, _url, p, server, job, preview) = fixture().await;
    let id = Uuid::new_v4();
    let stem = format!(
        "semantic_execution_admission-{}-{}-{id}",
        mutation.unwrap_or("healthy"),
        if previewed { "previewed" } else { "normal" }
    );
    let before = snapshot(&h, &job).await;
    assert_eq!(before["batch_state"], "queued");
    assert!(before["entry_count"].as_i64().is_some_and(|n| n > 0));
    assert_eq!(before["requests"], 0);
    assert_eq!(before["embeddings"], 0);
    assert_eq!(before["publication_audits"], 0);
    let mut replacement_token = None;
    let mut barrier = Value::Null;
    let mut held = None;
    let mut gate = None;
    if mutation.is_some() {
        let key = (id.as_u128() as i64) & i64::MAX;
        let mut connection = h.admin.acquire().await.unwrap();
        let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(key)
            .execute(&mut *connection)
            .await
            .unwrap();
        sqlx::raw_sql(&format!("CREATE FUNCTION fixture_semantic_ready_gate() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN PERFORM pg_advisory_xact_lock({key});RETURN NEW;END;$$;CREATE TRIGGER fixture_semantic_ready_gate BEFORE UPDATE ON semantic_batches FOR EACH ROW WHEN(NEW.state='running') EXECUTE FUNCTION fixture_semantic_ready_gate()"))
            .execute(&h.admin).await.unwrap();
        gate = Some((connection, gate_pid, key));
    }
    let state = h.state.clone();
    let work_job = job.clone();
    let work_preview = gateway::Preview {
        policy_id: preview.policy_id,
        body: preview.body.clone(),
    };
    let task = tokio::spawn(async move {
        if previewed {
            Box::pin(semantic::execute_previewed(
                &state,
                &work_job,
                &work_preview,
            ))
            .await
        } else {
            Box::pin(semantic::execute(&state, &work_job)).await
        }
    });
    if let Some((mut connection, gate_pid, key)) = gate {
        let ready_pid = wait_blocked(&h.admin, gate_pid, "UPDATE semantic_batches").await;
        let pool = h.admin.clone();
        let brain = job.brain_id;
        let (pid_send, pid_receive) = oneshot::channel();
        let (tx_send, tx_receive) = oneshot::channel();
        tokio::spawn(async move {
            let mut control = pool.begin().await.unwrap();
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *control)
                .await
                .unwrap();
            pid_send.send(pid).unwrap();
            // NO KEY UPDATE blocks gateway FOR UPDATE but permits cancellation audit FK KEY SHARE.
            sqlx::query("SELECT id FROM brains WHERE id=$1 FOR NO KEY UPDATE")
                .bind(brain)
                .fetch_one(&mut *control)
                .await
                .unwrap();
            let _ = tx_send.send(control);
        });
        let control_pid = pid_receive.await.unwrap();
        let queued_control = wait_blocked(&h.admin, ready_pid, "FOR NO KEY UPDATE").await;
        assert_eq!(queued_control, control_pid);
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(key)
            .execute(&mut *connection)
            .await
            .unwrap();
        drop(connection);
        let mut control = tokio::time::timeout(Duration::from_secs(15), tx_receive)
            .await
            .unwrap()
            .unwrap();
        let committed = snapshot(&h, &job).await;
        assert_eq!(committed["batch_state"], "running");
        assert_eq!(committed["running_entries"], committed["entry_count"]);
        assert_eq!(committed["requests"], 0);
        let gateway_pid = wait_blocked(&h.admin, control_pid, "recollect_lock_brain").await;
        assert!(!task.is_finished());
        let changed = if mutation == Some("cancel") {
            // Exact owned fixture state transition, not an API authorization test.
            // The native cancel API needs the advisory lock already held by the
            // blocked gateway, so using it here would manufacture a deadlock.
            let changed=sqlx::query("UPDATE jobs SET state='cancelled',error_code='cancelled_by_admin',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND brain_id=$2 AND lease_token=$3 AND state='running'")
                .bind(job.id).bind(job.brain_id).bind(job.lease_token).execute(&mut *control).await.unwrap();
            assert_eq!(changed.rows_affected(), 1);
            true
        } else {
            sqlx::query("UPDATE jobs SET lease_until=clock_timestamp()-interval '1 second' WHERE id=$1 AND lease_token=$2 AND state='running'").bind(job.id).bind(job.lease_token).execute(&h.admin).await.unwrap();
            let replacement = worker::claim(&h.state.pool, "model")
                .await
                .unwrap()
                .unwrap();
            assert_eq!(replacement.id, job.id);
            assert_eq!(replacement.target_id, job.target_id);
            assert_eq!(replacement.actor_id, job.actor_id);
            assert_eq!(replacement.device_id, job.device_id);
            assert_ne!(replacement.lease_token, job.lease_token);
            replacement_token = Some(replacement.lease_token);
            true
        };
        let after_mutation = if mutation == Some("cancel") {
            snapshot_at(&mut *control, &job).await
        } else {
            snapshot(&h, &job).await
        };
        assert_eq!(p.bodies.lock().unwrap().len(), 0);
        assert_eq!(after_mutation["requests"], 0);
        barrier = json!({"gate_pid":gate_pid,"ready_pid":ready_pid,"queued_control_pid":queued_control,"control_pid":control_pid,"gateway_pid":gateway_pid,"native_blocking_relationships_verified":true,"ready_commit_visible":committed,"mutated_while_gateway_blocked":after_mutation,"mutation_snapshot_visibility":if mutation==Some("cancel"){"control_transaction_uncommitted"}else{"independent_connection_committed"},"barrier_release":"control_commit_makes_cancellation_visible_before_gateway_row_acquisition","exact_fixture_cancel_or_native_reclaim_staged":changed,"cancellation_method":"exact_owned_administrative_state_transition_no_API_claim","lease_tokens_persisted":false});
        held = Some(control);
    }
    let control_committed = if let Some(control) = held {
        control.commit().await.unwrap();
        true
    } else {
        false
    };
    let completion = tokio::time::timeout(Duration::from_secs(15), task)
        .await
        .unwrap()
        .unwrap();
    let mut after = snapshot(&h, &job).await;
    if let Some(token) = replacement_token {
        let preserved:bool=sqlx::query_scalar("SELECT state='running' AND lease_token=$2 AND lease_until>clock_timestamp() FROM jobs WHERE id=$1").bind(job.id).bind(token).fetch_one(&h.admin).await.unwrap();
        after["replacement_lease_preserved"] = json!(preserved);
    }
    let bodies = p.bodies.lock().unwrap().clone();
    let expected = serde_json::to_vec(&preview.body).unwrap();
    let parity = bodies.iter().all(|b| b == &expected);
    let proof = json!({"database":h.database,"job":job.id,"batch":job.target_id,"mutation":mutation,"previewed":previewed,"barrier":barrier,"control_committed_before_worker_completion":control_committed,"before":before,"after":after,"provider_transmissions":bodies.len(),"preview_provider_bytes_equal":parity,"worker_completed_without_heartbeat_loss":completion.is_ok(),"external_calls":0});
    persist(&format!("{stem}.json"), &proof);
    server.abort();
    cleanup(h, &format!("{stem}-cleanup.json")).await;
    assert!(
        completion.is_ok(),
        "Heartbeat loss cannot count as gateway lease-boundary proof"
    );
    assert!(parity, "Canonical preview/provider bytes differ");
    if mutation.is_some() {
        if mutation == Some("cancel") {
            assert_eq!(after["job_state"], "cancelled");
            assert_eq!(after["lease_present"], false);
        } else {
            assert_eq!(after["replacement_lease_preserved"], true);
        }
        assert_eq!(after["ready_entries"], 0);
        assert_eq!(after["embeddings"], 0);
        assert_eq!(after["publication_audits"], 0);
        // Baseline must record exactly one request/transmission and zero publication,
        // then fail these same strict assertions; the fixed source must satisfy them.
        assert!(
            after["requests"] == 0 || (after["requests"] == 1 && bodies.len() == 1),
            "Counterfactual must be exact one request/transmission"
        );
        assert_eq!(
            after["requests"], 0,
            "Stale semantic worker reached model reservation"
        );
        assert_eq!(bodies.len(), 0, "Stale semantic worker reached provider");
    } else {
        assert_eq!(after["requests"], 1);
        assert_eq!(bodies.len(), 1);
        assert_eq!(after["batch_state"], "succeeded");
        assert_eq!(after["ready_entries"], after["entry_count"]);
        assert_eq!(after["embeddings"], after["entry_count"]);
        assert_eq!(after["publication_audits"], 1);
    }
}
#[tokio::test]
#[ignore = "Requires isolated repository-owned PostgreSQL"]
async fn semantic_execution_admission_cancel_normal() {
    Box::pin(run(false, Some("cancel"))).await
}
#[tokio::test]
#[ignore = "Requires isolated repository-owned PostgreSQL"]
async fn semantic_execution_admission_cancel_previewed() {
    Box::pin(run(true, Some("cancel"))).await
}
#[tokio::test]
#[ignore = "Requires isolated repository-owned PostgreSQL"]
async fn semantic_execution_admission_reclaim_normal() {
    Box::pin(run(false, Some("reclaim"))).await
}
#[tokio::test]
#[ignore = "Requires isolated repository-owned PostgreSQL"]
async fn semantic_execution_admission_reclaim_previewed() {
    Box::pin(run(true, Some("reclaim"))).await
}
#[tokio::test]
#[ignore = "Requires isolated repository-owned PostgreSQL"]
async fn semantic_execution_admission_healthy_normal() {
    Box::pin(run(false, None)).await
}
#[tokio::test]
#[ignore = "Requires isolated repository-owned PostgreSQL"]
async fn semantic_execution_admission_healthy_previewed() {
    Box::pin(run(true, None)).await
}
