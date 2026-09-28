//! Protocol/process integration only. Callers must hold current server authority
//! before connecting/resolving secrets and again immediately before sending.
pub mod agent_tls;
pub mod agent_transport;
pub mod credentials;
pub mod discovery;
pub mod executor;
mod http;
pub mod manager;
pub mod outbox;
mod session;
pub mod supervisor;

pub use session::{Connected, DispatchOutcome};
pub use tokio_util::sync::CancellationToken;

pub const WIRE_LIMIT: usize = 1024 * 1024;
pub const RESULT_LIMIT: usize = 256 * 1024;
pub const USEFUL_IDLE: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// Stable errors intentionally carry no provider payload or credential values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeError(pub &'static str);
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for RuntimeError {}
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Applies the same structured redaction as capture, including exact credentials.
pub fn sanitize(value: &serde_json::Value, secrets: &[String]) -> serde_json::Value {
    recollect_protocol::sanitize_capture_value(value, secrets)
}
