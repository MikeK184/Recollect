//! A pipe-owned child supervisor. The SDK launches this mode in its own process
//! group. Parent death closes stdin; the supervisor then terminates only that
//! live group. No persisted PID or command-name scan is used for cleanup.
use crate::{Result, RuntimeError, WIRE_LIMIT};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub const COMMAND: &str = "RECOLLECT_MCP_SUPERVISOR_COMMAND";
pub const ARGUMENTS: &str = "RECOLLECT_MCP_SUPERVISOR_ARGUMENTS";
pub const PARENT: &str = "RECOLLECT_MCP_SUPERVISOR_PARENT";

pub async fn forward_lines<R, W>(read: R, mut write: W, limit: usize) -> std::io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut read = BufReader::new(read);
    let mut line = Vec::new();
    loop {
        line.clear();
        // take() bounds accumulation even when the backend never sends a newline.
        let n = (&mut read)
            .take((limit + 1) as u64)
            .read_until(b'\n', &mut line)
            .await?;
        if n == 0 {
            write.shutdown().await?;
            return Ok(());
        }
        if line.len() > limit {
            return Err(std::io::Error::other("provider_line_too_large"));
        }
        write.write_all(&line).await?;
        write.flush().await?;
    }
}

#[cfg(unix)]
pub async fn run() -> Result<()> {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::{getpgrp, getpid, getppid},
    };
    use tokio::signal::unix::{SignalKind, signal};
    // This mode can never signal the invoking shell's process group.
    let group = getpid();
    if group != getpgrp() {
        return Err(RuntimeError("supervisor_requires_owned_group"));
    }
    let parent = std::env::var(PARENT)
        .ok()
        .and_then(|p| p.parse::<i32>().ok())
        .filter(|p| *p > 1)
        .ok_or(RuntimeError("supervisor_parent_invalid"))?;
    if getppid().as_raw() != parent {
        return Err(RuntimeError("supervisor_parent_lost"));
    }
    let mut term =
        signal(SignalKind::terminate()).map_err(|_| RuntimeError("supervisor_signal_failed"))?;
    let mut interrupt =
        signal(SignalKind::interrupt()).map_err(|_| RuntimeError("supervisor_signal_failed"))?;
    let command = std::env::var(COMMAND).map_err(|_| RuntimeError("supervisor_command_missing"))?;
    let arguments: Vec<String> = serde_json::from_str(
        &std::env::var(ARGUMENTS).map_err(|_| RuntimeError("supervisor_arguments_missing"))?,
    )
    .map_err(|_| RuntimeError("supervisor_arguments_invalid"))?;
    if !std::path::Path::new(&command).is_absolute() {
        return Err(RuntimeError("supervisor_command_invalid"));
    }
    let mut child = tokio::process::Command::new(command)
        .args(arguments)
        .env_remove(COMMAND)
        .env_remove(ARGUMENTS)
        .env_remove(PARENT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| RuntimeError("provider_spawn_failed"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or(RuntimeError("provider_pipe_failed"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(RuntimeError("provider_pipe_failed"))?;
    // Keep every ordinary child in this inherited group, including subprocesses.
    // EOF, broken output, an oversized frame or a signal all end ownership.
    tokio::select! {
        _ = forward_lines(tokio::io::stdin(), stdin, WIRE_LIMIT) => {},
        _ = forward_lines(stdout, tokio::io::stdout(), WIRE_LIMIT) => {},
        _ = child.wait() => {},
        _ = term.recv() => {},
        _ = interrupt.recv() => {},
        // Protocol forwarding can block behind a backend that stops reading.
        // Reparenting remains observable independently; no recorded PID is killed.
        _ = async {
            loop {
                if getppid().as_raw() != parent { break; }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        } => {},
    }
    // We are still the live group leader, so PID reuse cannot change ownership.
    let _ = killpg(group, Signal::SIGTERM);
    let _ = tokio::time::timeout(std::time::Duration::from_millis(750), child.wait()).await;
    // End any remaining non-daemonizing descendants as well as this supervisor.
    let _ = killpg(group, Signal::SIGKILL);
    Err(RuntimeError("supervisor_shutdown_failed"))
}

#[cfg(not(unix))]
pub async fn run() -> Result<()> {
    Err(RuntimeError("managed_stdio_platform_unsupported"))
}
