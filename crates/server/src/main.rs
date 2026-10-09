use recollect_server::{
    ApiDoc, AppState, app, config::Config, db, privacy_journal, recovery, worker,
};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = std::env::args().nth(1).unwrap_or("serve".into());
    if command == "mcp-supervise" {
        return Ok(recollect_mcp_runtime::supervisor::run().await?);
    }
    if command == "openapi" {
        println!("{}", ApiDoc::openapi().to_pretty_json()?);
        return Ok(());
    }
    if command == "build-info" {
        println!("{}", recollect_server::health::build());
        return Ok(());
    }
    if command == "recovery-schema" {
        println!(
            "{}",
            serde_json::json!({"postgres_major":17,"migrations":db::MIGRATIONS.iter().map(|(name,_)|name).collect::<Vec<_>>()})
        );
        return Ok(());
    }
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or("recollect_server=info,tower_http=info".into())
                .add_directive("recollect_mcp_runtime=warn".parse()?),
        )
        .with(tracing_subscriber::fmt::layer().json().with_filter(
            tracing_subscriber::filter::filter_fn(|metadata| {
                // An explicit RUST_LOG directive must not re-enable raw SDK,
                // transport or credential-provider diagnostics containing data.
                metadata.target().starts_with("recollect_")
                    || metadata.target().starts_with("tower_http")
            }),
        ))
        .init();
    let config = Config::from_env()?;
    if !matches!(command.as_str(), "recovery-prepare" | "recovery-release") {
        recovery::guard(&config).await?;
    }
    match command.as_str() {
        "migrate"
        | "recover-owner"
        | "privacy-reconcile"
        | "backup-prepare"
        | "recovery-prepare"
        | "recovery-release"
        | "mcp-definition-import"
        | "mcp-definition-disable" => {
            let url = std::env::var("DATABASE_ADMIN_URL").map_err(|_| {
                anyhow::anyhow!("DATABASE_ADMIN_URL required for migrations/recovery")
            })?;
            let admin = db::pool(&url).await?;
            if command == "recovery-release" {
                recovery::release(&admin, &config).await?;
                admin.close().await;
                return Ok(());
            }
            if command == "recovery-prepare" {
                recovery::held(&config, &admin).await?;
            }
            db::migrate(&admin).await?;
            db::bootstrap(&admin, &config.owner_username).await?;
            privacy_journal::initialize(&admin, &config).await?;
            privacy_journal::reconcile(&admin, &config).await?;
            if command == "backup-prepare" {
                recovery::checkpoint(&admin, &config).await?;
            }
            if command == "recovery-prepare" {
                recovery::prepare(&admin, &config).await?;
            }
            if command == "recover-owner" {
                db::recover_owner(&admin).await?;
            }
            if command == "mcp-definition-import" || command == "mcp-definition-disable" {
                let argument = std::env::args().nth(2).ok_or_else(|| {
                    anyhow::anyhow!("Supply an approval JSON path or existing definition key")
                })?;
                if command == "mcp-definition-import" {
                    recollect_server::mcp::definitions::import_file(&admin, &argument).await?;
                } else {
                    recollect_server::mcp::definitions::disable(&admin, &argument).await?;
                }
            }
            tracing::info!(command, "Operator command completed");
            admin.close().await;
        }
        "serve" | "worker" | "worker-once" | "privacy-once" => {
            let pool = db::pool(&config.database_url).await?;
            db::ready(&pool).await?;
            let privileged: bool = sqlx::query_scalar("SELECT rolsuper OR rolbypassrls OR pg_has_role(current_user, 'recollect_admin', 'MEMBER') FROM pg_roles WHERE rolname=current_user")
                .fetch_one(&pool).await?;
            anyhow::ensure!(
                !privileged,
                "Server must use the non-owner application database role"
            );
            if let Err(error) = privacy_journal::barrier(&pool, &config).await {
                if let Ok(admin_url) = std::env::var("DATABASE_ADMIN_URL") {
                    let admin = db::pool(&admin_url).await?;
                    privacy_journal::reconcile(&admin, &config).await?;
                    admin.close().await;
                    privacy_journal::barrier(&pool, &config).await?;
                } else {
                    return Err(error);
                }
            }
            if command == "privacy-once" {
                let state = AppState::new(pool, config)?;
                let processed = privacy_journal::run_once(&state).await?;
                tracing::info!(processed, "Privacy maintenance pass completed");
                return Ok(());
            }
            if command == "worker" {
                return worker::run(AppState::new(pool, config)?).await;
            }
            if command == "worker-once" {
                let lane = std::env::args().nth(2).unwrap_or("interactive".into());
                anyhow::ensure!(
                    matches!(lane.as_str(), "interactive" | "capture" | "model" | "heavy"),
                    "Choose an existing worker lane: interactive, capture, model or heavy"
                );
                let worked = worker::run_once(&AppState::new(pool, config)?, &lane).await?;
                tracing::info!(worked, "Worker pass completed");
                return Ok(());
            }
            let listener = tokio::net::TcpListener::bind(&config.bind).await?;
            tracing::info!(address=%config.bind,"Recollect listening");
            let state = AppState::new(pool, config)?;
            let stop = recollect_mcp_runtime::CancellationToken::new();
            let executor = tokio::spawn(recollect_server::mcp::runtime::central::run(
                state.clone(),
                stop.clone(),
            ));
            let shutdown = stop.clone();
            let served = axum::serve(listener, app(state))
                .with_graceful_shutdown(async move {
                    let _ = recollect_server::shutdown::requested().await;
                    shutdown.cancel();
                    tracing::info!("Draining active MCP calls before shutdown");
                })
                .await;
            stop.cancel();
            executor.await?;
            served?;
        }
        _ => anyhow::bail!(
            "Usage: recollect-server [serve|worker|worker-once|privacy-once|privacy-reconcile|migrate|recover-owner|backup-prepare|recovery-schema|recovery-prepare|recovery-release|mcp-definition-import PATH|mcp-definition-disable KEY|openapi]"
        ),
    }
    Ok(())
}
