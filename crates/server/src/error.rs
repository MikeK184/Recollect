use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use recollect_protocol::ApiError;

pub struct Error(pub StatusCode, pub &'static str, pub &'static str);
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn unauthorized() -> Self {
        Self(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Sign in to continue.",
        )
    }
    pub fn forbidden() -> Self {
        Self(
            StatusCode::FORBIDDEN,
            "forbidden",
            "This action requires Brain administrator access.",
        )
    }
    pub fn missing() -> Self {
        Self(
            StatusCode::NOT_FOUND,
            "not_found",
            "This resource is unavailable or you do not have access.",
        )
    }
    pub fn invalid(message: &'static str) -> Self {
        Self(StatusCode::BAD_REQUEST, "invalid_input", message)
    }
}

impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        // Driver payloads can contain user input; never forward or log them here.
        tracing::warn!(kind = "database_error", "Database operation failed");
        Self(
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "Database operation failed. Retry shortly.",
        )
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ApiError {
                code: self.1.into(),
                message: self.2.into(),
            }),
        )
            .into_response()
    }
}
