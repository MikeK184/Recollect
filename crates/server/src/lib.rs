pub mod access;
pub mod answers;
pub mod artifacts;
pub mod auth;
mod automation;
pub mod autonomous;
pub mod brains;
pub mod capture;
pub mod commands;
pub mod config;
pub mod credentials;
pub mod db;
pub mod devices;
pub mod error;
pub mod evidence;
pub mod graph;
pub mod handovers;
pub mod health;
pub mod jobs;
pub mod learning;
pub mod manifests;
pub mod mcp;
pub mod memory;
pub mod memory_conflicts;
pub mod memory_evidence;
pub mod memory_policy;
pub mod memory_review;
pub mod memory_rules;
pub mod model_gateway;
pub mod model_policy;
pub mod oidc;
pub mod operations;
pub mod privacy;
pub mod privacy_journal;
pub mod procedures;
pub mod publication;
pub mod recovery;
pub mod retention;
pub mod retrieval;
pub mod semantic;
pub mod shutdown;
pub mod team;
pub mod worker;
pub mod workspace;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use utoipa::OpenApi;

type LoginAttempts = Arc<Mutex<HashMap<String, (Instant, u32)>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: Arc<config::Config>,
    pub http: reqwest::Client,
    pub login_attempts: LoginAttempts,
    pub agent_requests: Arc<tokio::sync::Semaphore>,
    pub mcp_inspections: Arc<tokio::sync::Semaphore>,
    pub answer_capacity: Arc<tokio::sync::Semaphore>,
    pub metrics: Arc<operations::Metrics>,
}
impl AppState {
    pub fn new(pool: sqlx::PgPool, config: config::Config) -> anyhow::Result<Self> {
        Ok(Self {
            pool,
            config: Arc::new(config),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
            agent_requests: Arc::new(tokio::sync::Semaphore::new(16)),
            mcp_inspections: Arc::new(tokio::sync::Semaphore::new(4)),
            answer_capacity: Arc::new(tokio::sync::Semaphore::new(4)),
            metrics: Arc::new(operations::Metrics::default()),
        })
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login,
        auth::me,
        auth::logout,
        mcp::catalogue,
        mcp::get_definition,
        mcp::get_connection,
        mcp::create_connection,
        mcp::update_connection,
        mcp::get_profile,
        mcp::create_profile,
        mcp::update_profile,
        mcp::grant,
        mcp::remove_grant,
        mcp::discover,
        mcp::runtime::submit,
        mcp::runtime::get,
        mcp::runtime::observations::list,
        mcp::runtime::observations::status,
        mcp::runtime::list,
        mcp::runtime::cancel,
        mcp::runtime::release,
        mcp::runtime::status,
        mcp::runtime::reconcile,
        mcp::runtime::resolve,
        mcp::runtime::runner::register,
        mcp::runtime::runner::heartbeat,
        mcp::runtime::runner::claim,
        mcp::runtime::runner::start,
        mcp::runtime::runner::complete,
        mcp::runtime::runner::receipt_removals,
        mcp::runtime::runner::defer,
        mcp::runtime::runner::instance,
        mcp::runtime::runner::credentials,
        mcp::private::list,
        mcp::private::create,
        mcp::private::update,
        brains::list,
        brains::create,
        brains::get,
        brains::update,
        brains::audit,
        health::status,
        operations::status,
        capture::get_policy,
        capture::update_policy,
        capture::bind,
        capture::bindings,
        capture::publish,
        capture::events,
        capture::devices,
        capture::report_device,
        retrieval::recall,
        answers::create,
        answers::get,
        answers::cancel,
        semantic::get,
        semantic::reindex,
        semantic::retry,
        graph::get,
        graph::rebuild,
        graph::view,
        graph::path,
        graph::explore,
        graph::analytics::get,
        graph::analytics::queue,
        graph::analytics::view,
        automation::get,
        automation::update,
        mcp::definitions::approve,
        mcp::definitions::inspect_http,
        model_policy::get,
        model_policy::update,
        model_policy::history,
        model_policy::usage,
        model_gateway::check,
        learning::create,
        learning::list,
        learning::retry,
        handovers::create,
        handovers::list,
        handovers::retry,
        procedures::get_input,
        jobs::list,
        jobs::processing,
        jobs::retry,
        jobs::cancel,
        team::list,
        team::invite,
        team::enroll,
        team::revoke_invite,
        team::status,
        team::reset,
        team::oidc_account,
        team::history,
        access::get,
        access::set,
        access::remove,
        access::by_name,
        access::set_group,
        access::remove_group,
        access::transfer,
        oidc::options,
        oidc::start,
        oidc::callback,
        devices::start,
        devices::view,
        devices::approve,
        devices::poll,
        devices::finish,
        devices::cancel,
        devices::list,
        devices::revoke,
        devices::revoke_self,
        evidence::catalogue,
        evidence::policy,
        evidence::create_group,
        evidence::update_group,
        evidence::remove_group,
        evidence::import,
        evidence::append,
        evidence::organize,
        evidence::history,
        evidence::content,
        evidence::process,
        workspace::catalogue,
        workspace::refresh,
        workspace::alias,
        workspace::create_task,
        workspace::change_scope,
        workspace::begin,
        workspace::close,
        workspace::detail,
        workspace::operation,
        publication::policy,
        publication::set_policy,
        publication::publish,
        publication::list,
        publication::detail,
        publication::files,
        publication::file,
        publication::facts,
        publication::artifact,
        publication::process,
        manifests::list,
        manifests::detail,
        manifests::revision,
        manifests::create,
        manifests::update,
        memory::create,
        memory::update,
        memory::list,
        memory::detail,
        memory_review::review,
        memory_review::history,
        memory_conflicts::resolve,
        retention::get,
        retention::update,
        evidence::excerpt,
        privacy::preview,
        privacy::erase,
        privacy::list,
        privacy::retry,
        privacy::device_sync,
        privacy::device_ack,
        memory_evidence::list,
        memory_evidence::detail
    ),
    info(
        title = "Recollect API",
        description = "Local Recollect product API. Cookie sessions and CSRF protect mutations."
    )
)]
pub struct ApiDoc;

pub(crate) fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/enroll", post(team::enroll))
        .route("/auth/options", get(oidc::options))
        .route("/brains/{brain}/mcp", get(mcp::catalogue))
        .route("/brains/{brain}/mcp/runtime", get(mcp::runtime::status))
        .route(
            "/brains/{brain}/mcp/calls/{id}/reconcile",
            post(mcp::runtime::reconcile),
        )
        .route(
            "/brains/{brain}/mcp/calls/{id}/resolve",
            post(mcp::runtime::resolve),
        )
        .route("/mcp/runner/register", post(mcp::runtime::runner::register))
        .route(
            "/mcp/runner/heartbeat",
            post(mcp::runtime::runner::heartbeat),
        )
        .route("/mcp/runner/claim", post(mcp::runtime::runner::claim))
        .route("/mcp/runner/start", post(mcp::runtime::runner::start))
        .route("/mcp/runner/complete", post(mcp::runtime::runner::complete))
        .route(
            "/mcp/runner/receipt-removals",
            post(mcp::runtime::runner::receipt_removals),
        )
        .route("/mcp/runner/defer", post(mcp::runtime::runner::defer))
        .route("/mcp/runner/instance", post(mcp::runtime::runner::instance))
        .route(
            "/mcp/runner/credentials",
            post(mcp::runtime::runner::credentials),
        )
        .route(
            "/brains/{brain}/mcp/private-runners",
            get(mcp::private::list).post(mcp::private::create),
        )
        .route(
            "/brains/{brain}/mcp/private-runners/{id}",
            axum::routing::put(mcp::private::update),
        )
        .route(
            "/brains/{brain}/mcp/definitions/{key}",
            get(mcp::get_definition),
        )
        .route(
            "/brains/{brain}/mcp/connections",
            post(mcp::create_connection),
        )
        .route(
            "/brains/{brain}/mcp/connections/{id}",
            get(mcp::get_connection).put(mcp::update_connection),
        )
        .route("/brains/{brain}/mcp/profiles", post(mcp::create_profile))
        .route(
            "/brains/{brain}/mcp/profiles/{id}",
            get(mcp::get_profile).put(mcp::update_profile),
        )
        .route(
            "/brains/{brain}/mcp/profiles/{id}/grants",
            axum::routing::put(mcp::grant),
        )
        .route(
            "/brains/{brain}/mcp/profiles/{id}/grants/{grant}",
            axum::routing::delete(mcp::remove_grant),
        )
        .route("/brains/{brain}/mcp/discover", post(mcp::discover))
        .route(
            "/brains/{brain}/mcp/calls",
            get(mcp::runtime::list).post(mcp::runtime::submit),
        )
        .route("/brains/{brain}/mcp/calls/{id}", get(mcp::runtime::get))
        .route(
            "/brains/{brain}/mcp/calls/{id}/observations",
            get(mcp::runtime::observations::list),
        )
        .route(
            "/brains/{brain}/capture/managed",
            get(mcp::runtime::observations::status),
        )
        .route(
            "/brains/{brain}/mcp/calls/{id}/cancel",
            post(mcp::runtime::cancel),
        )
        .route(
            "/brains/{brain}/mcp/session/release",
            post(mcp::runtime::release),
        )
        .route("/auth/oidc/start", get(oidc::start))
        .route("/auth/oidc/callback", get(oidc::callback))
        .route("/team", get(team::list))
        .route("/team/invitations", post(team::invite))
        .route(
            "/team/invitations/{id}",
            axum::routing::delete(team::revoke_invite),
        )
        .route("/team/accounts/{id}", axum::routing::patch(team::status))
        .route("/team/accounts/{id}/reset", post(team::reset))
        .route("/team/oidc-accounts", post(team::oidc_account))
        .route("/team/audit", get(team::history))
        .route("/devices/pairings", post(devices::start))
        .route("/devices/pairings/{code}", get(devices::view))
        .route("/devices/pairings/{code}/approve", post(devices::approve))
        .route("/devices/pairings/poll", post(devices::poll))
        .route("/devices/pairings/finish", post(devices::finish))
        .route("/devices/pairings/cancel", post(devices::cancel))
        .route("/devices", get(devices::list))
        .route("/devices/{id}", axum::routing::delete(devices::revoke))
        .route("/devices/revoke-self", post(devices::revoke_self))
        .route("/brains", get(brains::list).post(brains::create))
        .route("/brains/{id}", get(brains::get).patch(brains::update))
        .route("/brains/{id}/audit", get(brains::audit))
        .route("/brains/{id}/access", get(access::get))
        .route("/brains/{id}/grants", post(access::by_name))
        .route(
            "/brains/{id}/grants/{account}",
            axum::routing::put(access::set).delete(access::remove),
        )
        .route(
            "/brains/{id}/group-grants",
            axum::routing::put(access::set_group),
        )
        .route(
            "/brains/{id}/group-grants/{mapping}",
            axum::routing::delete(access::remove_group),
        )
        .route("/brains/{id}/owner", post(access::transfer))
        .route("/brains/{id}/jobs", get(jobs::list))
        .route("/brains/{id}/processing", get(jobs::processing))
        .route("/brains/{id}/jobs/{job}/retry", post(jobs::retry))
        .route("/brains/{id}/jobs/{job}/cancel", post(jobs::cancel))
        .route("/status", get(health::status))
        .route("/operations", get(operations::status))
        .route(
            "/brains/{brain}/recall",
            post(retrieval::recall).layer(DefaultBodyLimit::max(32768)),
        )
        .route(
            "/brains/{brain}/answer-requests",
            post(answers::create).layer(DefaultBodyLimit::max(32768)),
        )
        .route("/brains/{brain}/answer-requests/{id}", get(answers::get))
        .route(
            "/brains/{brain}/answer-requests/{id}/cancel",
            post(answers::cancel),
        )
        .route("/brains/{brain}/semantic", get(semantic::get))
        .route("/brains/{brain}/graph", get(graph::get))
        .route("/brains/{brain}/graph/rebuild", post(graph::rebuild))
        .route("/brains/{brain}/graph/view", post(graph::view))
        .route("/brains/{brain}/graph/path", post(graph::path))
        .route("/brains/{brain}/graph/explore", post(graph::explore))
        .route(
            "/brains/{brain}/graph/analytics",
            get(graph::analytics::get).post(graph::analytics::queue),
        )
        .route(
            "/brains/{brain}/graph/analytics/{id}/view",
            post(graph::analytics::view),
        )
        .route("/brains/{brain}/semantic/reindex", post(semantic::reindex))
        .route(
            "/brains/{brain}/semantic/batches/{batch}/retry",
            post(semantic::retry),
        )
        .route(
            "/brains/{brain}/capture/policy",
            get(capture::get_policy).put(capture::update_policy),
        )
        .route(
            "/brains/{brain}/capture/bindings",
            get(capture::bindings).post(capture::bind),
        )
        .route(
            "/brains/{brain}/capture/events",
            get(capture::events).post(capture::publish),
        )
        .route(
            "/brains/{brain}/capture/devices",
            get(capture::devices).post(capture::report_device),
        )
        .route(
            "/brains/{brain}/models/policy",
            get(model_policy::get).put(model_policy::update),
        )
        .route(
            "/brains/{brain}/models/policy/history",
            get(model_policy::history),
        )
        .route("/brains/{brain}/models/usage", get(model_policy::usage))
        .route(
            "/brains/{brain}/automation",
            get(automation::get).put(automation::update),
        )
        .route("/mcp/definitions", post(mcp::definitions::approve))
        .route(
            "/mcp/definitions/inspect-http",
            post(mcp::definitions::inspect_http),
        )
        .route("/brains/{brain}/models/check", post(model_gateway::check))
        .route(
            "/brains/{brain}/learning",
            get(learning::list).post(learning::create),
        )
        .route(
            "/brains/{brain}/learning/{run}/retry",
            post(learning::retry),
        )
        .route(
            "/brains/{brain}/handover-inputs/{revision}",
            get(procedures::get_input),
        )
        .route("/brains/{brain}/evidence", get(evidence::catalogue))
        .route(
            "/brains/{brain}/handovers",
            get(handovers::list).post(handovers::create),
        )
        .route(
            "/brains/{brain}/handovers/{run}/retry",
            post(handovers::retry),
        )
        .route(
            "/brains/{brain}/evidence/policy",
            axum::routing::put(evidence::policy),
        )
        .route(
            "/brains/{brain}/evidence/groups",
            post(evidence::create_group),
        )
        .route(
            "/brains/{brain}/evidence/groups/{group}",
            axum::routing::patch(evidence::update_group).delete(evidence::remove_group),
        )
        .route(
            "/brains/{brain}/sources",
            post(evidence::import).layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        .route(
            "/brains/{brain}/sources/{source}/versions",
            get(evidence::history)
                .post(evidence::append)
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        .route(
            "/brains/{brain}/sources/{source}/versions/{version}",
            get(evidence::content),
        )
        .route(
            "/brains/{brain}/sources/{source}/groups",
            axum::routing::put(evidence::organize),
        )
        .route(
            "/brains/{brain}/sources/{source}/process",
            post(evidence::process),
        )
        .route("/brains/{brain}/workspace", get(workspace::catalogue))
        .route(
            "/brains/{brain}/workspace/checkouts",
            post(workspace::refresh).layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        .route(
            "/brains/{brain}/workspace/repositories/{repository}/origins",
            post(workspace::alias),
        )
        .route(
            "/brains/{brain}/workspace/tasks",
            post(workspace::create_task),
        )
        .route(
            "/brains/{brain}/workspace/tasks/{task}",
            get(workspace::detail),
        )
        .route(
            "/brains/{brain}/workspace/tasks/{task}/scope",
            axum::routing::put(workspace::change_scope),
        )
        .route(
            "/brains/{brain}/workspace/tasks/{task}/operations",
            post(workspace::begin),
        )
        .route(
            "/brains/{brain}/workspace/tasks/{task}/close",
            post(workspace::close),
        )
        .route(
            "/brains/{brain}/workspace/operations/{operation}",
            get(workspace::operation),
        )
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .route(
            "/brains/{brain}/repositories/policy",
            get(publication::policy).put(publication::set_policy),
        )
        .route(
            "/brains/{brain}/repositories/{repository}/snapshots",
            get(publication::list)
                .post(publication::publish)
                .layer(DefaultBodyLimit::max(
                    recollect_protocol::PUBLICATION_MAX_BYTES,
                )),
        )
        .route(
            "/brains/{brain}/repository-snapshots/{snapshot}",
            get(publication::detail),
        )
        .route(
            "/brains/{brain}/repository-snapshots/{snapshot}/files",
            get(publication::files),
        )
        .route(
            "/brains/{brain}/repository-snapshots/{snapshot}/files/{file}",
            get(publication::file),
        )
        .route(
            "/brains/{brain}/repository-snapshots/{snapshot}/facts",
            get(publication::facts),
        )
        .route(
            "/brains/{brain}/repository-snapshots/{snapshot}/artifacts/{kind}",
            get(publication::artifact),
        )
        .route(
            "/brains/{brain}/repository-snapshots/{snapshot}/process",
            post(publication::process),
        )
        .route(
            "/brains/{brain}/revision-manifests",
            get(manifests::list)
                .post(manifests::create)
                .layer(DefaultBodyLimit::max(24 * 1024 * 1024)),
        )
        .route(
            "/brains/{brain}/revision-manifests/{manifest}",
            get(manifests::detail)
                .put(manifests::update)
                .layer(DefaultBodyLimit::max(24 * 1024 * 1024)),
        )
        .route(
            "/brains/{brain}/revision-manifests/{manifest}/revisions/{revision}",
            get(manifests::revision),
        )
        .route(
            "/brains/{brain}/claims",
            get(memory::list)
                .post(memory::create)
                .layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route(
            "/brains/{brain}/claims/{claim}",
            get(memory::detail)
                .put(memory::update)
                .layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route(
            "/brains/{brain}/claims/{claim}/review",
            get(memory_review::history)
                .post(memory_review::review)
                .layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route(
            "/brains/{brain}/claim-conflicts/resolve",
            post(memory_conflicts::resolve).layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        .route(
            "/brains/{brain}/retention",
            get(retention::get).put(retention::update),
        )
        .route("/brains/{brain}/excerpts", post(evidence::excerpt))
        .route("/brains/{brain}/erasures/preview", post(privacy::preview))
        .route(
            "/brains/{brain}/erasures",
            get(privacy::list).post(privacy::erase),
        )
        .route(
            "/brains/{brain}/erasures/{request}/retry",
            post(privacy::retry),
        )
        .route(
            "/brains/{brain}/privacy-sync",
            get(privacy::device_sync).post(privacy::device_ack),
        )
        .route("/brains/{brain}/claim-evidence", get(memory_evidence::list))
        .route(
            "/brains/{brain}/claim-evidence/{kind}/{evidence}",
            get(memory_evidence::detail),
        )
        .fallback(|| async { error::Error::missing() })
}

pub fn app(state: AppState) -> Router {
    let static_dir = state.config.static_dir.clone();
    let api = api_routes().route(
        "/brains/{brain}/mcp/agent",
        axum::routing::any(mcp::agent::serve).layer(DefaultBodyLimit::max(256 * 1024)),
    );
    Router::new()
        .nest("/api", api)
        .route(
            "/health/live",
            get(|| async { Json(serde_json::json!({"live":true})) }),
        )
        .route("/health/ready", get(health::ready))
        .fallback_service(
            ServeDir::new(&static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html"))),
        )
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), origin_guard))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request| {
                let method = match *request.method() {
                    Method::GET => "GET",
                    Method::POST => "POST",
                    Method::PUT => "PUT",
                    Method::PATCH => "PATCH",
                    Method::DELETE => "DELETE",
                    Method::HEAD => "HEAD",
                    Method::OPTIONS => "OPTIONS",
                    _ => "OTHER",
                };
                tracing::info_span!("http", method)
            }),
        )
        .layer(middleware::from_fn_with_state(
            state.metrics.clone(),
            operations::record,
        ))
        .with_state(state)
}

async fn origin_guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) && let Some(origin) = request.headers().get(header::ORIGIN)
        && origin.to_str().ok() != Some(state.config.public_origin.as_str())
    {
        return error::Error(
            StatusCode::FORBIDDEN,
            "origin_denied",
            "This browser origin is not allowed.",
        )
        .into_response();
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}
