//! Read-only normal-data performance proof through an owned PostgreSQL clone.
//! Never starts a worker, initializes/reconciles a journal or writes/cleans a
//! native projection. Existing original Brain/generation IDs are read-only.
use anyhow::{Context, Result, ensure};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use recollect_server::{AppState, app, config::Config, db};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Instant,
};
use tokio::process::Command;
use tower::ServiceExt;
use uuid::Uuid;

const OWNER: &str = "Recollect owned normal graph read-only performance clone";
const BRAIN: &str = "84b6569f-55e7-46b0-a15c-6b46a584a627";
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn url_database(url: &str, database: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(url)?;
    url.set_path(database);
    Ok(url.into())
}
async fn inventory(pool: &PgPool) -> Result<Value> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut counts = serde_json::Map::new();
    for table in tables {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM public.{}", quote(&table)))
                .fetch_one(&mut *tx)
                .await?;
        counts.insert(table, json!(count));
    }
    let schema:String=sqlx::query_scalar("SELECT md5(coalesce(string_agg(table_name||':'||column_name||':'||data_type||':'||is_nullable,',' ORDER BY table_name,ordinal_position),'')) FROM information_schema.columns WHERE table_schema='public'").fetch_one(&mut *tx).await?;
    let definitions:String=sqlx::query_scalar("SELECT md5(coalesce(string_agg(definition,E'\\n' ORDER BY definition),'')) FROM (
        SELECT pg_get_functiondef(p.oid) AS definition FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.prokind IN ('f','p')
        UNION ALL SELECT indexdef FROM pg_indexes WHERE schemaname='public'
        UNION ALL SELECT row_to_json(p)::text FROM pg_policies p WHERE schemaname='public'
        UNION ALL SELECT c.conname||':'||pg_get_constraintdef(c.oid) FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname='public'
    ) definitions").fetch_one(&mut *tx).await?;
    let migrations: Vec<String> =
        sqlx::query_scalar("SELECT name FROM recollect_migrations ORDER BY name")
            .fetch_all(&mut *tx)
            .await?;
    let epochs: Vec<(Uuid, i64)> =
        sqlx::query_as("SELECT id,analytics_epoch FROM brains ORDER BY id")
            .fetch_all(&mut *tx)
            .await?;
    let jobs: Vec<(String, i64)> =
        sqlx::query_as("SELECT state,count(*) FROM jobs GROUP BY state ORDER BY state")
            .fetch_all(&mut *tx)
            .await?;
    let requests: Vec<(String, i64)> =
        sqlx::query_as("SELECT state,count(*) FROM model_requests GROUP BY state ORDER BY state")
            .fetch_all(&mut *tx)
            .await?;
    let generation: Option<(Uuid, i32)> = sqlx::query_as("SELECT id,jsonb_array_length(descriptor->'nodes') FROM graph_generations WHERE brain_id=$1 AND kind='knowledge' AND state='ready' ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(BRAIN.parse::<Uuid>()?).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    Ok(
        json!({"schema_md5":schema,"definition_md5":definitions,"migrations":migrations,"row_counts":counts,"brain_epochs":epochs,"job_states":jobs,"model_request_states":requests,"normal_graph_generation":generation.map(|(id,nodes)|json!({"id":id,"nodes":nodes}))}),
    )
}

async fn dump_clone(root: &Path, master: &PgPool, normal: &str, target: &str) -> Result<()> {
    // The fallback streams in memory between two processes in the already owned
    // PostgreSQL container. No dump, source body or credential file is exported.
    let docker = root.join("scripts/docker.sh");
    let cluster: String =
        sqlx::query_scalar("SELECT system_identifier::text FROM pg_control_system()")
            .fetch_one(master)
            .await?;
    let identifier = Command::new(&docker)
        .args([
            "exec",
            "-i",
            "recollect-postgres-1",
            "psql",
            "-X",
            "-At",
            "-U",
            "recollect_admin",
            "-d",
            "postgres",
            "-c",
            "SELECT system_identifier FROM pg_control_system()",
        ])
        .output()
        .await
        .context("Check owned PostgreSQL container identity")?;
    ensure!(
        identifier.status.success()
            && String::from_utf8_lossy(&identifier.stdout).trim() == cluster,
        "Owned fallback container differs from configured PostgreSQL cluster"
    );
    sqlx::query(&format!("CREATE DATABASE {}", quote(target)))
        .execute(master)
        .await?;
    sqlx::query(&format!(
        "COMMENT ON DATABASE {} IS '{OWNER}'",
        quote(target)
    ))
    .execute(master)
    .await?;
    let mut dump = Command::new(&docker)
        .args([
            "exec",
            "-i",
            "recollect-postgres-1",
            "pg_dump",
            "-U",
            "recollect_admin",
            "--format=custom",
            "--no-owner",
            normal,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut restore = Command::new(&docker)
        .args([
            "exec",
            "-i",
            "recollect-postgres-1",
            "pg_restore",
            "-U",
            "recollect_admin",
            "--no-owner",
            "--dbname",
            target,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut output = dump.stdout.take().context("Missing owned dump output")?;
    let mut input = restore
        .stdin
        .take()
        .context("Missing owned restore input")?;
    tokio::io::copy(&mut output, &mut input).await?;
    drop(input);
    ensure!(
        dump.wait().await?.success() && restore.wait().await?.success(),
        "Owned clone stream failed; original was not modified"
    );
    Ok(())
}
async fn owned_drop(master: &PgPool, target: &str) -> Result<()> {
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=$1",
    )
    .bind(target)
    .fetch_optional(master)
    .await?
    .flatten();
    ensure!(
        owner.as_deref() == Some(OWNER),
        "Owned clone cleanup identity mismatch"
    );
    sqlx::query(&format!("DROP DATABASE {}", quote(target)))
        .execute(master)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut config = Config::from_env()?;
    let admin_url =
        std::env::var("DATABASE_ADMIN_URL").context("DATABASE_ADMIN_URL is required")?;
    let original_database = reqwest::Url::parse(&admin_url)?
        .path()
        .trim_start_matches('/')
        .to_string();
    ensure!(
        original_database == "recollect",
        "Proof is restricted to the configured normal Recollect database"
    );
    let admin_endpoint = reqwest::Url::parse(&admin_url)?;
    let application_endpoint = reqwest::Url::parse(&config.database_url)?;
    ensure!(
        admin_endpoint.host_str() == application_endpoint.host_str()
            && admin_endpoint.port_or_known_default()
                == application_endpoint.port_or_known_default()
            && admin_endpoint.path() == application_endpoint.path(),
        "Application and administrative URLs must name the same normal database endpoint"
    );
    let master = db::pool(&url_database(&admin_url, "postgres")?).await?;
    let original = db::pool(&admin_url).await?;
    let before = inventory(&original).await?;
    original.close().await;
    let expected_nodes = before["normal_graph_generation"]["nodes"]
        .as_u64()
        .context("Original normal Brain has no existing populated ready projection")?;
    ensure!(
        expected_nodes > 0 && expected_nodes <= 500,
        "Normal proof requires one populated bounded existing projection"
    );
    let cache = root.join(".cache/graph-implementation");
    std::fs::create_dir_all(&cache)?;
    std::fs::write(
        cache.join("normal-graph-original-before.json"),
        serde_json::to_vec_pretty(&before)?,
    )?;
    let target = format!("recollect_graph_proof_{}", Uuid::new_v4().simple());
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=$1")
        .bind(&original_database)
        .fetch_one(&master)
        .await?;
    let force_dump = std::env::var_os("RECOLLECT_GRAPH_PROOF_FORCE_DUMP").is_some();
    let require_template = std::env::var_os("RECOLLECT_GRAPH_PROOF_REQUIRE_TEMPLATE").is_some();
    ensure!(
        !require_template || (!force_dump && active == 0),
        "Template control requires zero original connections; none will be terminated"
    );
    let clone_method = if active == 0 && !force_dump {
        match sqlx::query(&format!(
            "CREATE DATABASE {} TEMPLATE {}",
            quote(&target),
            quote(&original_database)
        ))
        .execute(&master)
        .await
        {
            Ok(_) => {
                sqlx::query(&format!(
                    "COMMENT ON DATABASE {} IS '{OWNER}'",
                    quote(&target)
                ))
                .execute(&master)
                .await?;
                "template_without_source_connections"
            }
            Err(error) => {
                if require_template {
                    return Err(error).context(
                        "Original became busy; template control did not terminate any connection",
                    );
                }
                dump_clone(&root, &master, &original_database, &target).await?;
                "streamed_dump_without_terminating_source"
            }
        }
    } else {
        dump_clone(&root, &master, &original_database, &target).await?;
        "streamed_dump_without_terminating_source"
    };
    std::fs::write(
        cache.join(format!("{target}-owner.json")),
        serde_json::to_vec_pretty(
            &json!({"database":target,"ownership":OWNER,"method":clone_method,"state":"created"}),
        )?,
    )?;
    let cloned_admin = db::pool(&url_database(&admin_url, &target)?).await?;
    if std::env::var_os("RECOLLECT_GRAPH_PROOF_FREEZE_STATISTICS").is_some() {
        // Diagnose restore-time plans deterministically, only on this owned
        // clone; a later explicit ANALYZE supplies the paired statistics case.
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT tablename FROM pg_tables WHERE schemaname='public'")
                .fetch_all(&cloned_admin)
                .await?;
        for table in tables {
            sqlx::query(&format!(
                "ALTER TABLE {} SET (autovacuum_enabled=false)",
                quote(&table)
            ))
            .execute(&cloned_admin)
            .await?;
        }
    }
    // SQL-only migration in the owned clone. No bootstrap or runtime recovery.
    db::migrate(&cloned_admin)
        .await
        .context("Migrate owned normal-data clone")?;
    let restored_statistics_analyzed = clone_method == "streamed_dump_without_terminating_source"
        && std::env::var_os("RECOLLECT_GRAPH_PROOF_FREEZE_STATISTICS").is_none();
    if restored_statistics_analyzed {
        // Logical dumps omit planner statistics. Recreate them only in the
        // owned restore before measuring service readiness.
        sqlx::query("ANALYZE").execute(&cloned_admin).await?;
    }
    let diagnostic_jit_off = std::env::var_os("RECOLLECT_GRAPH_PROOF_JIT_OFF").is_some();
    if diagnostic_jit_off {
        sqlx::query(&format!("ALTER DATABASE {} SET jit=off", quote(&target)))
            .execute(&master)
            .await?;
    }
    let brain: Uuid = BRAIN.parse()?;
    let actor: Uuid = sqlx::query_scalar(
        "SELECT a.id FROM brains b JOIN accounts a ON a.id=b.owner_id WHERE b.id=$1 AND a.enabled",
    )
    .bind(brain)
    .fetch_one(&cloned_admin)
    .await
    .context("Find exact normal Brain owner only in clone")?;
    if std::env::var_os("RECOLLECT_GRAPH_PROOF_PROFILE_ONLY").is_some() {
        // Retain only this exactly commented clone for subsequent read-only
        // EXPLAIN under its real application RLS role.
        cloned_admin.close().await;
        let original = db::pool(&admin_url).await?;
        let after = inventory(&original).await?;
        original.close().await;
        ensure!(
            before == after,
            "Original changed while preparing diagnostic clone"
        );
        std::fs::write(
            cache.join(format!("{target}-owner.json")),
            serde_json::to_vec_pretty(
                &json!({"database":target,"ownership":OWNER,"method":clone_method,"state":"retained_for_read_only_explain","brain_id":brain,"actor_id":actor,"original_inventory_unchanged":true}),
            )?,
        )?;
        println!(
            "{}",
            json!({"owned_profile_clone":target,"original_unchanged":true})
        );
        return Ok(());
    }
    let session = Uuid::new_v4();
    let csrf = Uuid::new_v4();
    sqlx::query("INSERT INTO sessions(token,account_id,csrf_token,expires_at) VALUES($1,$2,$3,clock_timestamp()+interval '30 minutes')")
        .bind(session).bind(actor).bind(csrf).execute(&cloned_admin).await?;
    let model_before: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&cloned_admin)
        .await?;
    let migration: Option<String> =
        sqlx::query_scalar("SELECT max(name) FROM recollect_migrations")
            .fetch_one(&cloned_admin)
            .await?;
    config.database_url = url_database(&config.database_url, &target)?;
    config.models.key = None;
    config.models.openrouter_key = None;
    config.models.endpoint = "http://127.0.0.1:9".into();
    config.models.openrouter_endpoint = "http://127.0.0.1:9".into();
    config.owner_password = "unused-owned-clone-session".into();
    config.oidc = None;
    config.erasure_mirror = None;
    let private = cache.join(format!("{target}-private"));
    std::fs::create_dir(&private)?;
    // Exact source hydration validates retained artifacts even with chunks.
    // Graph reads open only already qualified source files and never write
    // artifacts. Retain that authorized read-only root while isolating every
    // mutable runtime path below; no source body is copied or printed.
    config.erasure_journal = private
        .join("unused-journal")
        .to_string_lossy()
        .into_owned();
    config.credential_file = private
        .join("unused-credentials.json")
        .to_string_lossy()
        .into_owned();
    config.static_dir = private.join("unused-static").to_string_lossy().into_owned();
    let pool = db::pool(&config.database_url).await?;
    let state = AppState::new(pool, config)?;
    let router = app(state.clone());
    let scope = json!({"kind":"knowledge","mode":"investigation","selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"relations":[]});
    let mut results = vec![];
    let mut center: Option<String> = None;
    let mut population_hash: Option<String> = None;
    let mut payload_consistent = true;
    for ordinal in 0..40 {
        let (endpoint, input) = if ordinal % 3 == 0 {
            ("view", json!({"scope":scope,"offset":0,"windowed":true}))
        } else {
            (
                "explore",
                json!({"scope":scope,"center":if ordinal%3==2 {center.as_ref()}else{None},"direction":"outgoing","max_hops":1,"windowed":true}),
            )
        };
        let request = Request::builder()
            .method("POST")
            .uri(format!("/api/brains/{brain}/graph/{endpoint}"))
            .header("content-type", "application/json")
            .header("cookie", format!("recollect_session={session}"))
            .header("x-csrf-token", csrf.to_string())
            .body(Body::from(input.to_string()))?;
        let started = Instant::now();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            router.clone().oneshot(request),
        )
        .await;
        let mut result = json!({"ordinal":ordinal,"endpoint":endpoint,"centered":ordinal%3==2 && center.is_some()});
        match response {
            Ok(Ok(response)) => {
                let status = response.status();
                let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024).await?;
                let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                result["http_status"] = json!(status.as_u16());
                if status == StatusCode::OK {
                    let nodes = value["nodes"].as_array().map_or(0, Vec::len);
                    let edges = value["edges"].as_array().map_or(0, Vec::len);
                    if center.is_none() {
                        center = value["nodes"][0]["key"].as_str().map(str::to_string);
                    }
                    let view = value.get("view").unwrap_or(&value);
                    result["nodes"] = json!(nodes);
                    result["edges"] = json!(edges);
                    result["coverage"] = view["coverage"].clone();
                    result["generation_id"] = view["generation"]["id"].clone();
                    if ordinal % 3 != 2 {
                        let mut canonical_nodes = value["nodes"]
                            .as_array()
                            .context("Successful graph response omitted nodes")?
                            .clone();
                        canonical_nodes.sort_by(|a, b| a["key"].as_str().cmp(&b["key"].as_str()));
                        let hash: String = sqlx::query_scalar("SELECT encode(sha256($1),'hex')")
                            .bind(serde_json::to_vec(&canonical_nodes)?)
                            .fetch_one(&cloned_admin)
                            .await?;
                        if let Some(expected) = &population_hash {
                            payload_consistent &= expected == &hash;
                        } else {
                            population_hash = Some(hash.clone());
                        }
                        result["hydrated_nodes_sha256"] = json!(hash);
                    }
                } else {
                    result["code"] = value["code"].clone();
                }
            }
            _ => {
                result["http_status"] = json!(0);
                result["code"] = json!("bounded_router_failure");
            }
        }
        result["elapsed_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
        println!(
            "{}",
            json!({"ordinal":ordinal,"endpoint":endpoint,"http_status":result["http_status"],"elapsed_ms":result["elapsed_ms"],"nodes":result["nodes"],"code":result["code"]})
        );
        results.push(result);
    }
    let model_after: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests")
        .fetch_one(&cloned_admin)
        .await?;
    // These private mutable runtime paths must remain empty. Never run
    // Harness.finish/native cleanup with original native generation IDs here.
    let private_files = std::fs::read_dir(&private)?.count();
    state.pool.close().await;
    drop(router);
    drop(state);
    cloned_admin.close().await;
    let original = db::pool(&admin_url).await?;
    let after = inventory(&original).await?;
    original.close().await;
    let original_unchanged = before == after;
    let successes = results.iter().filter(|r| r["http_status"] == 200).count();
    let populated = results.iter().any(|r| r["nodes"].as_u64().unwrap_or(0) > 0);
    let original_population_preserved = payload_consistent
        && results.iter().all(|r| {
            r["http_status"] != 200
                || (r["coverage"]["unavailable"].as_u64() == Some(0)
                    && r["coverage"]["withheld"].as_u64() == Some(0)
                    && (r["centered"] == true || r["nodes"].as_u64() == Some(expected_nodes)))
        });
    let mut times: Vec<_> = results
        .iter()
        .map(|r| r["elapsed_ms"].as_f64().unwrap())
        .collect();
    times.sort_by(f64::total_cmp);
    let p95 = times[37];
    let cleanup = owned_drop(&master, &target).await;
    let removed = cleanup.is_ok();
    if removed {
        std::fs::remove_dir(&private)?;
    }
    let report = json!({"observed_at":chrono::Utc::now(),"evidence_kind":"owned_isolated_clone_of_actual_normal_data_not_deployed_runtime","brain_id":brain,"clone_database":target,"clone_method":clone_method,"restored_statistics_analyzed":restored_statistics_analyzed,"migration":migration,"sequential_reads":40,"successes":successes,"failures":40-successes,"p95_all_outcomes_ms":p95,"max_ms":times[39],"populated_view":populated,"expected_unfiltered_nodes":expected_nodes,"diagnostic_jit_off":diagnostic_jit_off,"original_population_preserved":original_population_preserved,"original_inventory_unchanged":original_unchanged,"original_before":before,"original_after":after,"provider_calls_by_probe":model_after-model_before,"original_artifacts":"authorized_read_only_exact_qualified_source_hydration_no_copy_or_write","original_journal_or_credentials_opened":false,"private_runtime_files_created":private_files,"native_operations":"existing_generation_READ_ONLY_no_initialize_reconcile_worker_cleanup_import_rebuild","owned_clone_removed":removed,"results":results});
    std::fs::write(
        cache.join("normal-graph-clone-proof.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    std::fs::write(
        cache.join(format!("{target}-owner.json")),
        serde_json::to_vec_pretty(
            &json!({"database":target,"ownership":OWNER,"method":clone_method,"state":if removed{"removed_after_proof"}else{"retained_cleanup_failure"}}),
        )?,
    )?;
    println!(
        "{}",
        json!({"reads":40,"successes":successes,"failures":40-successes,"p95_ms":p95,"populated":populated,"original_unchanged":original_unchanged,"clone_removed":removed,"model_requests_added":model_after-model_before})
    );
    cleanup?;
    master.close().await;
    ensure!(
        original_unchanged,
        "Original inventory changed during isolated proof; no original mutation was requested by this probe"
    );
    ensure!(
        model_after == model_before && private_files == 0,
        "Read-only proof attempted a forbidden runtime side effect"
    );
    ensure!(
        successes == 40 && populated && original_population_preserved && p95 < 2000.0,
        "Actual normal-data read acceptance did not pass; inspect all-outcome receipt"
    );
    Ok(())
}
