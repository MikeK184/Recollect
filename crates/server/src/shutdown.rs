//! Both terminal and container termination use the existing durable drain path.
pub async fn requested() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut termination =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            signal = tokio::signal::ctrl_c() => signal,
            _ = termination.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
