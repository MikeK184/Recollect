#[path = "../support/mod.rs"]
pub mod support;
const WIRE_BOUND_FOR_FIXTURE: usize = recollect_mcp_runtime::WIRE_LIMIT + 20;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rmcp::ServiceExt;
    if std::env::args().nth(1).as_deref() == Some("mcp-supervise") {
        return Ok(recollect_mcp_runtime::supervisor::run().await?);
    }
    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("executor" | "blocked-executor")
    ) {
        use recollect_mcp_runtime::{
            CancellationToken, Connected, credentials::ResolvedCredentials,
        };
        let executable = std::env::current_exe()?;
        let definition = support::definition(Some(executable.to_str().unwrap().into()));
        let connected = std::sync::Arc::new(
            Connected::open(
                &definition,
                "owned-executor-fixture",
                &serde_json::json!({}),
                ResolvedCredentials::anonymous(),
                &executable,
            )
            .await?,
        );
        let reply = connected
            .call(
                "inspect",
                serde_json::json!({}),
                std::time::Duration::from_secs(3),
                CancellationToken::new(),
            )
            .await;
        anyhow::ensure!(reply.state == "succeeded", "fixture call failed");
        let backend = reply.result.as_ref().unwrap()["structuredContent"]["pid"]
            .as_u64()
            .unwrap();
        if std::env::args().nth(1).as_deref() == Some("blocked-executor") {
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(backend as i32),
                nix::sys::signal::Signal::SIGSTOP,
            )?;
            let mut blocked = Vec::new();
            for _ in 0..64 {
                let connected = connected.clone();
                blocked.push(tokio::spawn(async move {
                    connected
                        .call(
                            "inspect",
                            serde_json::json!({"text":"x".repeat(32000)}),
                            std::time::Duration::from_secs(300),
                            CancellationToken::new(),
                        )
                        .await
                }));
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            anyhow::ensure!(
                blocked.iter().all(|t| !t.is_finished()),
                "expected blocked fixture calls"
            );
        }
        println!(
            "{}",
            serde_json::json!({"supervisor":connected.owned_pid,"backend":backend})
        );
        std::future::pending::<()>().await;
        return Ok(());
    }
    let configuration: serde_json::Value =
        serde_json::from_str(&std::env::var("RECOLLECT_MCP_CONFIGURATION").unwrap_or("{}".into()))?;
    let fixture = support::Fixture {
        legacy: configuration["legacy"].as_bool().unwrap_or(false),
        marker: configuration["marker"]
            .as_str()
            .map(std::path::PathBuf::from),
        ..Default::default()
    };
    if std::env::args().nth(1).as_deref() == Some("serve-http") {
        use rmcp::transport::streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
        };
        use std::sync::Arc;
        let service = StreamableHttpService::new(
            move || Ok(fixture.clone()),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default(),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        println!(
            "{}",
            serde_json::json!({"target":format!("http://{}/mcp",listener.local_addr()?)})
        );
        axum::serve(listener, axum::Router::new().route_service("/mcp", service)).await?;
        return Ok(());
    }
    if let Some(marker) = &fixture.marker {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(marker)
            .await?;
        file.write_all(format!("start {}\n", std::process::id()).as_bytes())
            .await?;
    }
    if fixture.legacy {
        // Model a pre-discover server before handing the real initialize/session
        // exchange to rmcp. A modern server's discover error does not turn its
        // already selected connection lifecycle into a legacy implementation.
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let mut input = tokio::io::BufReader::new(tokio::io::stdin());
        let mut first = String::new();
        input.read_line(&mut first).await?;
        let request: serde_json::Value = serde_json::from_str(&first)?;
        anyhow::ensure!(
            request["method"] == "server/discover",
            "expected initial probe"
        );
        let mut output = tokio::io::stdout();
        let reply = serde_json::json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-32601,"message":"method unavailable"}});
        output.write_all(format!("{reply}\n").as_bytes()).await?;
        output.flush().await?;
        fixture.serve((input, output)).await?.waiting().await?;
    } else {
        fixture
            .serve(rmcp::transport::stdio())
            .await?
            .waiting()
            .await?;
    }
    Ok(())
}
