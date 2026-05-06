use crate::{AppState, auth::Auth};
use axum::{Json, extract::State, http::StatusCode};
use recollect_protocol::{DependencyStatus, ServiceStatus};
use serde_json::json;

pub async fn check(state: &AppState) -> ServiceStatus {
    let postgres = async {
        let value: std::result::Result<(String, String, String), _> = sqlx::query_as(
            "SELECT current_setting('server_version'), extversion, ('[1,0]'::vector <-> '[1,1]'::vector)::text FROM pg_extension WHERE extname='vector'")
            .fetch_one(&state.pool).await;
        match value {
            Ok((version, vector, distance)) if distance == "1" => DependencyStatus {
                name: "PostgreSQL / pgvector".into(),
                connected: true,
                detail: format!("PostgreSQL {version} · pgvector {vector}"),
            },
            _ => DependencyStatus {
                name: "PostgreSQL / pgvector".into(),
                connected: false,
                detail: "Database or vector query failed".into(),
            },
        }
    };
    let graph = async {
        let query = state.http.post(format!("{}/db/neo4j/query/v2",state.config.neo4j_url.trim_end_matches('/')))
            .basic_auth(&state.config.neo4j_username,Some(&state.config.neo4j_password))
            .json(&json!({"statement":"CALL dbms.components() YIELD versions RETURN versions[0] AS neo4j, gds.version() AS gds"}))
            .send().await;
        if let Ok(response) = query
            && response.status().is_success()
            && let Ok(body) = response.json::<serde_json::Value>().await
        {
            let errors = body
                .get("errors")
                .and_then(|v| v.as_array())
                .is_some_and(|v| !v.is_empty());
            let versions = body.pointer("/data/values/0").and_then(|v| v.as_array());
            if !errors
                && let Some(v) = versions
                && let (Some(neo), Some(gds)) = (
                    v.first().and_then(|x| x.as_str()),
                    v.get(1).and_then(|x| x.as_str()),
                )
            {
                return DependencyStatus {
                    name: "Neo4j / Graph Data Science".into(),
                    connected: true,
                    detail: format!("Neo4j {neo} · GDS {gds}"),
                };
            }
        }
        DependencyStatus {
            name: "Neo4j / Graph Data Science".into(),
            connected: false,
            detail: "Graph or GDS query failed".into(),
        }
    };
    let (postgres, graph) = tokio::join!(postgres, graph);
    ServiceStatus {
        ready: postgres.connected && graph.connected,
        dependencies: vec![postgres, graph],
    }
}

pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let ready = check(&state).await.ready;
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(json!({"ready":ready})),
    )
}

#[utoipa::path(get,path="/api/status",responses((status=200,body=ServiceStatus)))]
pub async fn status(State(state): State<AppState>, _auth: Auth) -> Json<ServiceStatus> {
    Json(check(&state).await)
}
