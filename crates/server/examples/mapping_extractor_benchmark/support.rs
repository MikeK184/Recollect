use anyhow::{Context, Result, ensure};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use recollect_server::{
    AppState, app, config::Config, db, knowledge_mapping::production, model_gateway as gateway,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};
use tower::ServiceExt;
use uuid::Uuid;

pub const MAX_BODY: u64 = 65536;
pub const FRAMING: u64 = 2048;
pub const MAX_OUTPUT: u64 = 4096;
pub(crate) const OWNER: &str = "Recollect owned extractor benchmark; synthetic input only";

// Python is already the repository's benchmark runtime; no new crypto dependency.
pub fn sha256(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import sys,hashlib;print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child.stdin.take().context("Hash stdin")?.write_all(bytes)?;
    let out = child.wait_with_output()?;
    ensure!(out.status.success(), "SHA-256 helper failed");
    Ok(String::from_utf8(out.stdout)?.trim().into())
}
fn integer(v: &Value, name: &str) -> Result<u64> {
    v[name].as_u64().with_context(|| format!("Invalid {name}"))
}
pub fn dollars(nano: u64) -> String {
    format!("{}.{:09}", nano / 1_000_000_000, nano % 1_000_000_000)
}
fn nano_dollars(s: &str) -> Result<u64> {
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    ensure!(
        !whole.is_empty()
            && whole.bytes().all(|b| b.is_ascii_digit())
            && fraction.len() <= 9
            && fraction.bytes().all(|b| b.is_ascii_digit()),
        "Budget must be a nonnegative decimal USD string with at most nine decimals"
    );
    let whole: u64 = whole.parse()?;
    let fraction: u64 = format!("{fraction:0<9}").parse()?;
    whole
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(fraction))
        .context("Budget overflow")
}
pub fn validate_pricing(v: &Value, model: &str) -> Result<()> {
    ensure!(
        v["provider"] == "openai"
            && v["model"] == model
            && v["endpoint"] == "https://api.openai.com/v1",
        "No fresh pricing for the installed configured model; do not substitute a model"
    );
    let url = reqwest::Url::parse(v["source_url"].as_str().context("Missing pricing source")?)?;
    ensure!(
        url.scheme() == "https"
            && url.host_str() == Some("developers.openai.com")
            && url.path().ends_with(model),
        "Pricing requires an exact-model official source"
    );
    let verified =
        chrono::DateTime::parse_from_rfc3339(v["verified_at"].as_str().context("Pricing date")?)?;
    let age = chrono::Utc::now().signed_duration_since(verified);
    ensure!(
        integer(v, "valid_for_hours")? <= 24
            && age.num_seconds() >= -300
            && age.num_seconds() <= (integer(v, "valid_for_hours")? * 3600) as i64,
        "Pricing is stale or future-dated; verify official prices and re-freeze"
    );
    for name in [
        "input_nano_usd_per_token",
        "cache_write_nano_usd_per_token",
        "output_nano_usd_per_token",
    ] {
        ensure!(integer(v, name)? > 0, "Pricing must be positive");
    }
    ensure!(
        MAX_BODY + FRAMING <= integer(v, "maximum_input_tokens_before_long_context_multiplier")?,
        "Pricing needs long-context multipliers"
    );
    Ok(())
}
pub fn worst_case(v: &Value, calls: u64) -> Result<u64> {
    ensure!(calls <= 60, "Maximum campaign is 60 calls");
    let rate =
        integer(v, "input_nano_usd_per_token")?.max(integer(v, "cache_write_nano_usd_per_token")?);
    (MAX_BODY + FRAMING)
        .checked_mul(rate)
        .and_then(|i| {
            MAX_OUTPUT
                .checked_mul(integer(v, "output_nano_usd_per_token").ok()?)
                .and_then(|o| i.checked_add(o))
        })
        .and_then(|v| v.checked_mul(calls))
        .context("Cost overflow")
}
pub fn validate_approval(v: &Value, recipe_hash: &str, recipe: &Value) -> Result<()> {
    ensure!(
        v["approve_paid_calls"] == true,
        "Paid calls are not authorized"
    );
    ensure!(
        v["recipe_sha256"] == recipe_hash && v["pricing_sha256"] == recipe["pricing_sha256"],
        "Approval must bind this exact recipe and pricing"
    );
    ensure!(
        v["maximum_calls"] == 60 && v["maximum_output_tokens"] == MAX_OUTPUT,
        "Approval limits differ from the frozen run"
    );
    let reference = v["authorization_reference"]
        .as_str()
        .context("Human authorization reference required")?;
    ensure!(
        !reference.trim().is_empty() && reference != "UNAPPROVED",
        "Explicit human authorization must be recorded"
    );
    ensure!(
        nano_dollars(
            v["budget_usd"]
                .as_str()
                .context("Explicit USD budget required")?
        )? >= recipe["limits"]["worst_case_nano_usd"]
            .as_u64()
            .context("Missing frozen cost")?,
        "Approved budget cannot cover this frozen maximum; do not silently shrink or substitute the campaign"
    );
    Ok(())
}
pub fn safe_output(root: &Path, out: &Path, existing: bool) -> Result<PathBuf> {
    let base = root.join(".cache/automatic-mapping");
    fs::create_dir_all(&base)?;
    let base = base.canonicalize()?;
    ensure!(
        base.starts_with(root),
        "Benchmark cache escaped the repository"
    );
    let out = if out.is_absolute() {
        out.to_path_buf()
    } else {
        root.join(out)
    };
    ensure!(
        out.parent().context("Output parent")?.canonicalize()? == base,
        "Use one campaign directory directly under .cache/automatic-mapping"
    );
    ensure!(
        out.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("extractor-")),
        "Campaign name must start extractor-"
    );
    if existing {
        ensure!(
            out.is_dir() && out.canonicalize()?.parent() == Some(base.as_path()),
            "Missing or foreign frozen campaign"
        );
    }
    Ok(out)
}
pub fn private_json(path: &Path, value: &Value) -> Result<()> {
    let mut file = private_file(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
pub(crate) fn private_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    file.sync_all()?;
    File::open(path.parent().context("Private file parent")?)?.sync_all()?;
    Ok(file)
}
fn append(file: &mut File, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
pub(crate) fn database_url(url: &str, database: &str) -> Result<String> {
    let mut parsed = reqwest::Url::parse(url).context("Invalid database endpoint")?;
    ensure!(
        [Some("127.0.0.1"), Some("localhost"), Some("[::1]")].contains(&parsed.host_str()),
        "Benchmark is limited to the owned local PostgreSQL cluster"
    );
    parsed.set_path(database);
    Ok(parsed.into())
}
pub(crate) struct Login {
    pub(crate) cookie: String,
    pub(crate) csrf: String,
}
pub(crate) async fn http(
    state: &AppState,
    login: Option<&Login>,
    method: &str,
    uri: &str,
    body: Value,
) -> Result<(Value, String)> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("idempotency-key", Uuid::new_v4().to_string());
    if let Some(login) = login {
        req = req
            .header("cookie", &login.cookie)
            .header("x-csrf-token", &login.csrf);
    }
    let response = app(state.clone())
        .oneshot(req.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|c| c.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .to_string();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    ensure!(
        status.is_success(),
        "Owned fixture HTTP setup failed ({status}, code={})",
        body["error"]["code"].as_str().unwrap_or("redacted")
    );
    Ok((body, cookie))
}
pub(crate) async fn setup(
    out: &Path,
    mut config: Config,
) -> Result<(AppState, PgPool, PgPool, String, Login, Uuid, Uuid, Uuid)> {
    let admin_url = std::env::var("DATABASE_ADMIN_URL")
        .context("DATABASE_ADMIN_URL required for an owned empty benchmark DB")?;
    let admin_endpoint = reqwest::Url::parse(&admin_url)?;
    let app_endpoint = reqwest::Url::parse(&config.database_url)?;
    ensure!(
        admin_endpoint.host_str() == app_endpoint.host_str()
            && admin_endpoint.port_or_known_default() == app_endpoint.port_or_known_default(),
        "Admin and application endpoints differ"
    );
    let root = db::pool(&database_url(&admin_url, "postgres")?)
        .await
        .context("Owned PostgreSQL admin connection failed")?;
    let database = format!("recollect_extractor_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&root)
        .await?;
    sqlx::query(&format!("COMMENT ON DATABASE {database} IS '{OWNER}'"))
        .execute(&root)
        .await?;
    private_json(
        &out.join("owned-state.json"),
        &json!({"database":database,"owner":OWNER,"normal_data_copied":false,"worker_started":false,"state_root":out.join("state")}),
    )?;
    let admin = db::pool(&database_url(&admin_url, &database)?).await?;
    db::migrate(&admin).await?;
    config.database_url = database_url(&config.database_url, &database)?;
    config.owner_username = format!("extractor-{}", Uuid::new_v4());
    config.owner_password = Uuid::new_v4().to_string();
    config.credential_file = out.join("state/credentials.json").to_string_lossy().into();
    config.artifact_dir = out.join("state/artifacts").to_string_lossy().into();
    config.erasure_journal = out.join("state/erasure-journal").to_string_lossy().into();
    config.erasure_mirror = None;
    config.oidc = None;
    // This extractor campaign has no native graph operations or external identity provider.
    config.neo4j_url = "http://127.0.0.1:9".into();
    config.neo4j_password = Uuid::new_v4().to_string();
    fs::create_dir(out.join("state"))?;
    let actor = db::bootstrap(&admin, &config.owner_username).await?;
    recollect_server::privacy_journal::initialize(&admin, &config).await?;
    let state = AppState::new(db::pool(&config.database_url).await?, config)?;
    let (session, cookie) = http(
        &state,
        None,
        "POST",
        "/api/auth/login",
        json!({"username":state.config.owner_username,"password":state.config.owner_password}),
    )
    .await?;
    let login = Login {
        cookie,
        csrf: session["csrf_token"]
            .as_str()
            .context("Missing private CSRF")?
            .into(),
    };
    let (brain, _) = http(
        &state,
        Some(&login),
        "POST",
        "/api/brains",
        json!({"name":"Owned synthetic extractor benchmark"}),
    )
    .await?;
    let brain: Uuid = brain["id"].as_str().context("Brain ID")?.parse()?;
    let path = format!("/api/brains/{brain}/models/policy");
    let (settings, _) = http(&state, Some(&login), "GET", &path, Value::Null).await?;
    let mut policy = settings["current"]["policy"].clone();
    policy["enabled"] = json!(true);
    policy["purposes"] = json!(["extraction"]);
    policy["content_classes"] = json!(["document"]);
    policy["max_input_bytes"] = json!(16384);
    policy["max_output_tokens"] = json!(MAX_OUTPUT);
    policy["daily_token_limit"] = json!(60 * (MAX_BODY + FRAMING + MAX_OUTPUT));
    policy["max_concurrent"] = json!(1);
    policy["automatic_learning"] = json!(false);
    policy["automatic_embedding"] = json!(false);
    policy["autonomous_memory"] = json!(false);
    policy["acceptance"] = Value::Null;
    let (saved, _) = http(
        &state,
        Some(&login),
        "PUT",
        &path,
        json!({"base_change":settings["current"]["change_id"],"policy":policy}),
    )
    .await?;
    let policy = saved["change_id"]
        .as_str()
        .context("Policy version")?
        .parse()?;
    Ok((state, admin, root, database, login, actor, brain, policy))
}

async fn drop_owned_database(cluster: &PgPool, database: &str, out: &Path) -> Result<()> {
    ensure!(
        database.starts_with("recollect_extractor_")
            && database
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "Owned database identifier invalid"
    );
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=$1",
    )
    .bind(database)
    .fetch_optional(cluster)
    .await?
    .flatten();
    ensure!(
        owner.as_deref() == Some(OWNER),
        "Owned cleanup identity mismatch"
    );
    // Pool shutdown closes sockets, while the PostgreSQL backend may observe
    // their EOF shortly afterward. Wait only for this exact owned database;
    // never terminate a session or force a drop.
    let mut inactive = false;
    for _ in 0..40 {
        let active: i64 =
            sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=$1")
                .bind(database)
                .fetch_one(cluster)
                .await?;
        if active == 0 {
            inactive = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    ensure!(
        inactive,
        "Owned database still has sessions; retained for review without termination"
    );
    sqlx::query(&format!("DROP DATABASE {database}"))
        .execute(cluster)
        .await?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
            .bind(database)
            .fetch_one(cluster)
            .await?;
    ensure!(!exists, "Owned database absence not verified");
    fs::remove_dir_all(out.join("state"))?;
    Ok(())
}

fn persist_body(out: &Path, operation: Uuid, body: &Value) -> Result<(String, String, usize)> {
    let bytes = serde_json::to_vec(body)?;
    ensure!(
        bytes.len() <= MAX_BODY as usize,
        "Complete request exceeds 64KiB; no reservation or send"
    );
    let hash = sha256(&bytes)?;
    let artifact = format!("request-body-{operation}.json");
    private_json(&out.join(&artifact), body)?;
    Ok((hash, artifact, bytes.len()))
}

pub async fn execute(
    _root: &Path,
    out: &Path,
    config: Config,
    recipe: &Value,
    schedule: &[Value],
    approval: &Value,
) -> Result<()> {
    ensure!(
        config.models.key.is_some() && config.models.endpoint == "https://api.openai.com/v1",
        "Missing installed credential or changed official endpoint; no provider substitution"
    );
    execute_owned(out, config, recipe, schedule, approval).await
}

// Kept private: only the official-endpoint guard above admits normal execution.
// Tests call this with their own fixed loopback provider and owned empty state.
async fn execute_owned(
    out: &Path,
    config: Config,
    recipe: &Value,
    schedule: &[Value],
    approval: &Value,
) -> Result<()> {
    ensure!(
        schedule.len() == 60,
        "All original 60 document denominators are required"
    );
    validate_pricing(
        &recipe["pricing"],
        config
            .models
            .text_model
            .as_deref()
            .context("Set RECOLLECT_TEXT_MODEL to the frozen text model")?,
    )?;
    // Persist the campaign directory entry as well as the create-new lock.
    File::open(out)?.sync_all()?;
    File::open(out.parent().context("Campaign parent")?)?.sync_all()?;
    // Durable lock precedes setup/network. Never resume/retry an uncertain paid run.
    private_json(
        &out.join("execution-lock.json"),
        &json!({"started_at":chrono::Utc::now(),"recipe_sha256":sha256(&serde_json::to_vec(recipe)?)?,"approval_sha256":sha256(&serde_json::to_vec(approval)?)?}),
    )?;
    let mut outcomes = private_file(&out.join("outcomes.jsonl"))?;
    let mut attempts = private_file(&out.join("attempts.jsonl"))?;
    let (state, admin, root, database, login, actor, brain, policy) = setup(out, config).await?;
    let per_call = worst_case(&recipe["pricing"], 1)?;
    let budget = nano_dollars(approval["budget_usd"].as_str().context("Budget missing")?)?;
    let mut reserved = 0u64;
    let mut calls = 0u64;
    let mut stopped = false;
    for doc in schedule {
        let mut result = json!({"corpus":doc["corpus"],"id":doc["id"],"partition":doc["partition"],"text_sha256":doc["text_sha256"],"state":"not_attempted","candidates":null});
        if stopped {
            append(&mut outcomes, &result)?;
            continue;
        }
        ensure!(
            calls < 60 && reserved.checked_add(per_call).is_some_and(|v| v <= budget),
            "Frozen call/cost bound exhausted"
        );
        let (source, _) = http(&state, Some(&login), "POST", &format!("/api/brains/{brain}/sources"), json!({"title":"Synthetic extractor input","media_type":"text/plain","content":doc["text"],"retain_content":true})).await?;
        let version: Uuid = source["version"]["id"]
            .as_str()
            .context("Source version")?
            .parse()?;
        let operation = Uuid::new_v4();
        let context = gateway::Context {
            brain,
            actor,
            device: None,
        };
        let invocation = gateway::extraction(operation, version);
        let preview = match gateway::preview_for_policy(&state, context, &invocation, policy).await
        {
            Ok(preview) => preview,
            Err(error) => {
                result["state"] = json!("preflight_rejected");
                result["error_code"] = json!(error.1);
                result["preflight_reason"] = json!(error.2);
                append(&mut outcomes, &result)?;
                continue;
            }
        };
        // Resolve complete canonical provenance/instructions/schema before any
        // reservation; the gateway resolves again and rejects drift before send.
        if serde_json::to_vec(&preview.body)?.len() > MAX_BODY as usize {
            result["state"] = json!("preflight_rejected");
            result["error_code"] = json!("complete_request_body_limit");
            append(&mut outcomes, &result)?;
            continue;
        }
        let (body_hash, body_artifact, body_bytes) = persist_body(out, operation, &preview.body)?;
        if let Err(error) = validate_pricing(
            &recipe["pricing"],
            state
                .config
                .models
                .text_model
                .as_deref()
                .unwrap_or_default(),
        ) {
            result["state"] = json!("pricing_expired_before_reservation");
            result["error"] = json!(error.to_string());
            stopped = true;
            append(&mut outcomes, &result)?;
            continue;
        }
        calls += 1;
        reserved += per_call;
        append(
            &mut attempts,
            &json!({"operation":operation,"corpus":doc["corpus"],"id":doc["id"],"attempt":1,"reserved_nano_usd":per_call,"created_at":chrono::Utc::now(),
                "policy_id":preview.policy_id,"source_version_id":version,"actual_body_sha256":body_hash,"request_body_artifact":body_artifact,"complete_request_bytes":body_bytes}),
        )?;
        result["actual_body_sha256"] = json!(body_hash);
        result["request_body_artifact"] = json!(body_artifact);
        result["complete_request_bytes"] = json!(body_bytes);
        let started = Instant::now();
        let response =
            gateway::invoke_previewed_for_policy(&state, context, invocation, &preview).await;
        result["latency_ms"] = json!(started.elapsed().as_millis());
        result["operation"] = json!(operation);
        match response {
            Ok(response) => {
                result["request"] = serde_json::to_value(&response.request)?;
                let input = response.request.input_tokens.unwrap_or(0);
                let output = response.request.output_tokens.unwrap_or(0);
                if input > (MAX_BODY + FRAMING) as i64 || output > MAX_OUTPUT as i64 {
                    stopped = true;
                    result["usage_bound_violation"] = json!(true);
                }
                match response.output {
                    Some(gateway::Output::Json(value)) => {
                        // Retain every raw result locally for reproducible failure analysis; never feed it or gold into another request.
                        result["raw_output"] = value.clone();
                        if value.get("mapping").is_none_or(Value::is_null) {
                            result["state"] = json!("mapping_absent");
                        } else {
                            match production::adapt(
                                version,
                                doc["text"].as_str().context("Frozen text")?,
                                &value["mapping"],
                            ) {
                                Ok(candidates) => {
                                    result["state"] = json!("validated");
                                    result["candidates"] = serde_json::to_value(candidates)?;
                                }
                                Err(error) => {
                                    result["state"] = json!("adapter_rejected");
                                    result["adapter_error"] = json!(format!("{error:?}"));
                                }
                            }
                        }
                    }
                    _ => {
                        result["state"] = json!("output_unavailable");
                    }
                }
            }
            Err(error) => {
                result["state"] = json!("gateway_failed");
                result["error_code"] = json!(error.1);
                // Metadata only. Provider errors/messages and credentials are never printed.
                let row: Option<(String, Option<String>, Option<i64>, Option<i64>)> = sqlx::query_as("SELECT state,error_code,input_tokens,output_tokens FROM model_requests WHERE brain_id=$1 AND operation_id=$2").bind(brain).bind(operation).fetch_optional(&admin).await?;
                if let Some((state, error, input, output)) = row {
                    result["gateway_attempt"] = json!({"state":state,"error_code":error,"input_tokens":input,"output_tokens":output});
                    stopped = state == "uncertain" || state == "running";
                } else {
                    stopped = true;
                }
            }
        }
        append(&mut outcomes, &result)?;
    }
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM model_requests WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&admin)
        .await?;
    ensure!(
        requests <= 60 && requests as u64 <= calls,
        "Unexpected extra gateway calls"
    );
    private_json(
        &out.join("execution-receipt.json"),
        &json!({"status":if stopped{"stopped_review_required"}else{"completed"},"gateway_requests":requests,"calls_reserved":calls,
        "conservative_reserved_usd":dollars(reserved),"budget_usd":approval["budget_usd"],"retries":0,"model":recipe["model"],
        "corpus_documents":60,"outcome_rows":schedule.len(),"recipe_sha256":sha256(&serde_json::to_vec(recipe)?)?,"outcomes_sha256":sha256(&fs::read(out.join("outcomes.jsonl"))?)?,"compiled_runner_sha256":recipe["compiled_runner_sha256"],"canonical_identity_and_topics_measured":false,"database_state_at_receipt":if stopped{"retained_review_required"}else{"retained_cleanup_pending"}}),
    )?;
    state.pool.close().await;
    admin.close().await;
    if !stopped {
        drop_owned_database(&root, &database, out).await?;
        private_json(
            &out.join("cleanup-receipt.json"),
            &json!({"status":"cleaned","database":database,"exact_owner_verified":true,"active_sessions_before_drop":0,"drop_without_force":true,"database_absence_verified":true,"owned_state_directory_removed":true,"verified_at":chrono::Utc::now()}),
        )?;
    } else {
        private_json(
            &out.join("cleanup-receipt.json"),
            &json!({"status":"retained_review_required","database":database,"cleanup_attempted":false,"reason":"Uncertain/stopped campaign; automatic resumption and retry forbidden"}),
        )?;
    }
    root.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_cost_covers_worst_cache_write_and_all_60_calls() {
        let price = json!({"input_nano_usd_per_token":200,"cache_write_nano_usd_per_token":250,"output_nano_usd_per_token":1200});
        assert_eq!(worst_case(&price, 60).unwrap(), 1_308_672_000);
        assert_eq!(dollars(1_308_672_000), "1.308672000");
        assert!(worst_case(&price, 61).is_err());
        assert!(nano_dollars("NaN").is_err());
        assert!(nano_dollars("1.0000000001").is_err());
        assert_eq!(nano_dollars("1.31").unwrap(), 1_310_000_000);
    }
    #[test]
    fn approval_requires_explicit_exact_recipe_and_budget() {
        let recipe =
            json!({"pricing_sha256":"price","limits":{"worst_case_nano_usd":1_308_672_000u64}});
        let mut approval = json!({"approve_paid_calls":true,"recipe_sha256":"recipe","pricing_sha256":"price","maximum_calls":60,"maximum_output_tokens":4096,"budget_usd":"1.31","authorization_reference":"test-only synthetic consent"});
        assert!(validate_approval(&approval, "recipe", &recipe).is_ok());
        approval["approve_paid_calls"] = json!(false);
        assert!(validate_approval(&approval, "recipe", &recipe).is_err());
        approval["approve_paid_calls"] = json!(true);
        approval["budget_usd"] = json!("1.30");
        assert!(validate_approval(&approval, "recipe", &recipe).is_err());
        approval["budget_usd"] = json!("1.31");
        assert!(validate_approval(&approval, "changed", &recipe).is_err());
    }
    #[test]
    fn stale_or_substituted_model_pricing_blocks_before_execution() {
        let mut price = json!({"provider":"openai","model":"gpt-5.6-luna","endpoint":"https://api.openai.com/v1",
            "source_url":"https://developers.openai.com/api/docs/models/gpt-5.6-luna",
            "verified_at":chrono::Utc::now(),"valid_for_hours":24,"input_nano_usd_per_token":200,
            "cache_write_nano_usd_per_token":250,"output_nano_usd_per_token":1200,
            "maximum_input_tokens_before_long_context_multiplier":272000});
        assert!(validate_pricing(&price, "gpt-5.6-luna").is_ok());
        assert!(validate_pricing(&price, "substituted-model").is_err());
        price["verified_at"] = json!(chrono::Utc::now() - chrono::Duration::hours(25));
        assert!(validate_pricing(&price, "gpt-5.6-luna").is_err());
        price["verified_at"] = json!(chrono::Utc::now());
        price["source_url"] = json!("https://example.com/models/gpt-5.6-luna");
        assert!(validate_pricing(&price, "gpt-5.6-luna").is_err());
    }
}

#[cfg(test)]
#[path = "proof.rs"]
mod proof;
