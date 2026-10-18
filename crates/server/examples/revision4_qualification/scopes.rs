//! Declared synthetic corpus scopes become canonical IDs through native APIs.
//! Scope strings are fixture setup, never inferred from document/model text.
use super::owned as legacy;
use anyhow::{Context, Result, ensure};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use recollect_protocol::ScopeSelection;
use recollect_server::{AppState, app};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Imported {
    pub brain: Uuid,
    pub source: Uuid,
    pub version: Uuid,
    pub device: Uuid,
    pub operation: Uuid,
    pub policy: Uuid,
    pub selection: ScopeSelection,
}
struct Brain {
    id: Uuid,
    repositories: BTreeMap<String, Uuid>,
    environments: BTreeMap<String, Uuid>,
}
pub struct Scopes {
    token: String,
    device: Uuid,
    brains: BTreeMap<String, Brain>,
}
fn id(value: &Value) -> Result<Uuid> {
    value
        .as_str()
        .context("Native fixture ID missing")?
        .parse()
        .context("Invalid native ID")
}
async fn device(
    state: &AppState,
    token: &str,
    method: &str,
    path: &str,
    body: Value,
) -> Result<Value> {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("idempotency-key", Uuid::new_v4().to_string())
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body.to_string()))?;
    let response = app(state.clone()).oneshot(req).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    ensure!(
        status.is_success(),
        "Owned scoped fixture {method} failed ({status},code={})",
        value["error"]["code"].as_str().unwrap_or("redacted")
    );
    Ok(value)
}
impl Scopes {
    pub async fn new(state: &AppState, login: &legacy::Login) -> Result<Self> {
        let (start, _) = legacy::http(
            state,
            None,
            "POST",
            "/api/devices/pairings",
            json!({"name":"Native conformance corpus importer"}),
        )
        .await?;
        legacy::http(
            state,
            Some(login),
            "POST",
            &format!(
                "/api/devices/pairings/{}/approve",
                start["user_code"].as_str().context("Pairing user code")?
            ),
            json!({"approve":true}),
        )
        .await?;
        let (poll, _) = legacy::http(
            state,
            None,
            "POST",
            "/api/devices/pairings/poll",
            json!({"device_code":start["device_code"]}),
        )
        .await?;
        let token = poll["token"]
            .as_str()
            .context("Owned claimed token")?
            .to_owned();
        let claimed = id(&poll["device"]["id"])?;
        legacy::http(
            state,
            None,
            "POST",
            "/api/devices/pairings/finish",
            json!({"device_code":start["device_code"]}),
        )
        .await?;
        Ok(Self {
            token,
            device: claimed,
            brains: BTreeMap::new(),
        })
    }
    pub async fn import(
        &mut self,
        state: &AppState,
        login: &legacy::Login,
        admin: &sqlx::PgPool,
        scope: &str,
        title: &str,
        text: &str,
    ) -> Result<Imported> {
        let parts: Vec<_> = scope.split('/').collect();
        ensure!(
            parts.len() == 3
                && parts.iter().all(|p| !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))),
            "Corpus scope must declare Brain/repository/environment; never infer it"
        );
        let key = parts[0].to_owned();
        if !self.brains.contains_key(&key) {
            let (brain, _) = legacy::http(
                state,
                Some(login),
                "POST",
                "/api/brains",
                json!({"name":format!("Native conformance {key}")}),
            )
            .await?;
            let brain = id(&brain["id"])?;
            let path = format!("/api/brains/{brain}/models/policy");
            let (current, _) = legacy::http(state, Some(login), "GET", &path, Value::Null).await?;
            let mut policy = current["current"]["policy"].clone();
            policy["enabled"] = json!(true);
            policy["purposes"] = json!(["extraction", "embedding"]);
            policy["content_classes"] =
                json!(["document", "claim", "raw_session", "support_excerpt"]);
            policy["max_input_bytes"] = json!(16384);
            policy["max_output_tokens"] = json!(4096);
            policy["max_concurrent"] = json!(1);
            policy["daily_token_limit"] = json!(10_000_000);
            policy["automatic_learning"] = json!(false);
            policy["autonomous_memory"] = json!(false);
            policy["automatic_embedding"] = json!(false);
            policy["acceptance"] = Value::Null;
            let (_policy, _) = legacy::http(
                state,
                Some(login),
                "PUT",
                &path,
                json!({"base_change":current["current"]["change_id"],"policy":policy}),
            )
            .await?;
            self.brains.insert(
                key.clone(),
                Brain {
                    id: brain,
                    repositories: BTreeMap::new(),
                    environments: BTreeMap::new(),
                },
            );
        }
        let brain = self.brains.get_mut(&key).unwrap();
        let base = format!("/api/brains/{}", brain.id);
        let repository = if parts[1] == "unknown" {
            None
        } else {
            if !brain.repositories.contains_key(parts[1]) {
                let origin = format!("conformance.test/{}/{}", key, parts[1]);
                device(state,&self.token,"POST",&format!("{base}/workspace/checkouts"),json!({"workspace_root":format!("/native-conformance/{key}"),"complete":false,"notes":[],"checkouts":[{"local_path":format!("/native-conformance/{key}/{}",parts[1]),"origin":format!("https://{origin}.git"),"branch":null,"head":null,"dirty":null,"status":"available"}]})).await?;
                let catalogue = device(
                    state,
                    &self.token,
                    "GET",
                    &format!("{base}/workspace"),
                    Value::Null,
                )
                .await?;
                let repository = catalogue["repositories"]
                    .as_array()
                    .context("Canonical repository catalogue")?
                    .iter()
                    .find(|r| r["canonical_origin"] == origin)
                    .context("Native canonical repository missing")?;
                brain
                    .repositories
                    .insert(parts[1].into(), id(&repository["id"])?);
            }
            brain.repositories.get(parts[1]).copied()
        };
        let environment = if parts[2] == "unknown" {
            None
        } else {
            if !brain.environments.contains_key(parts[2]) {
                let (group, _) = legacy::http(
                    state,
                    Some(login),
                    "POST",
                    &format!("{base}/evidence/groups"),
                    json!({"kind":"environment","name":parts[2]}),
                )
                .await?;
                brain
                    .environments
                    .insert(parts[2].into(), id(&group["id"])?);
            }
            brain.environments.get(parts[2]).copied()
        };
        let selection: ScopeSelection = serde_json::from_value(
            json!({"repository_ids":repository.into_iter().collect::<Vec<_>>(),"area_ids":[],"environment_id":environment}),
        )?;
        let task = device(
            state,
            &self.token,
            "POST",
            &format!("{base}/workspace/tasks"),
            json!({"label":"Owned canonical corpus import","selection":selection}),
        )
        .await?;
        let operation = device(
            state,
            &self.token,
            "POST",
            &format!(
                "{base}/workspace/tasks/{}/operations",
                task["task"]["id"].as_str().context("Scoped task ID")?
            ),
            json!({"kind":"write"}),
        )
        .await?;
        let operation = id(&operation["id"])?;
        let source=device(state,&self.token,"POST",&format!("{base}/sources"),json!({"title":title,"operation_id":operation,"media_type":"text/plain","content":text,"retain_content":true})).await?;
        let version = id(&source["version"]["id"])?;
        let expected: Uuid = sqlx::query_scalar("SELECT id FROM jobs WHERE brain_id=$1 AND target_id=$2 AND kind='source.process' AND lane='capture' AND state='queued'").bind(brain.id).bind(version).fetch_one(admin).await?;
        let pending: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM jobs WHERE lane='capture' AND state IN('queued','running') ORDER BY id",
        )
        .fetch_all(admin)
        .await?;
        ensure!(
            pending == vec![expected],
            "Foreign capture work prevents exact source materialization"
        );
        let job = recollect_server::worker::claim(&state.pool, "capture")
            .await?
            .context("Canonical import process missing")?;
        ensure!(
            job.id == expected
                && job.brain_id == brain.id
                && job.target_id == version
                && job.kind == "source.process",
            "Source materialization claim mismatch"
        );
        Box::pin(recollect_server::worker::execute(state, &job))
            .await
            .map_err(|_| anyhow::anyhow!("Canonical materialization failed"))?;
        let current_policy: Uuid =
            sqlx::query_scalar("SELECT policy_id FROM model_policy_heads WHERE brain_id=$1")
                .bind(brain.id)
                .fetch_one(admin)
                .await?;
        Ok(Imported {
            brain: brain.id,
            source: id(&source["id"])?,
            version: id(&source["version"]["id"])?,
            device: self.device,
            operation,
            policy: current_policy,
            selection,
        })
    }
}
impl Scopes {
    pub async fn learning(
        &self,
        state: &AppState,
        s: &Imported,
    ) -> Result<recollect_protocol::LearningRun> {
        let v=device(state,&self.token,"POST",&format!("/api/brains/{}/learning",s.brain),json!({"source_version_id":s.version,"selection":s.selection,"manifest_revision_id":null,"operation_id":s.operation})).await?;
        Ok(serde_json::from_value(v)?)
    }
}
