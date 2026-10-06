pub mod agent;
mod connector_icons;
pub mod credentials;
pub mod definitions;
mod grants;
pub mod private;
mod profiles;
pub mod runtime;

use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    workspace,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde_json::Value;
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

pub use grants::{__path_grant, __path_remove_grant, grant, remove_grant};
pub use profiles::{
    __path_create_profile, __path_discover, __path_get_profile, __path_update_profile,
    create_profile, discover, get_profile, update_profile,
};
type Tx<'a> = Transaction<'a, Postgres>;

fn text(value: &str, limit: usize, empty: bool) -> Result<()> {
    if (!empty && value.trim().is_empty())
        || value.chars().count() > limit
        || value.chars().any(char::is_control)
    {
        return Err(Error::invalid(
            "Use bounded text without control characters.",
        ));
    }
    Ok(())
}
fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "mcp_configuration_conflict", message)
}
fn denied() -> Error {
    Error(
        StatusCode::FORBIDDEN,
        "mcp_permission_required",
        "This profile requires a separate permission for the requested action.",
    )
}
fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "mcp_catalogue_capacity",
        "The MCP catalogue capacity has been reached.",
    )
}
fn mutation_error(error: sqlx::Error) -> Error {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        conflict("That name or grant already exists. Refresh the catalogue.")
    } else {
        error.into()
    }
}
async fn transaction<'a>(
    state: &'a AppState,
    auth: &Auth,
    brain: Uuid,
    write: bool,
) -> Result<Tx<'a>> {
    if write {
        auth.require_browser()?;
    }
    let mut tx = auth.tx(&state.pool).await?;
    db::lock_brain(&mut tx, brain, write).await?;
    db::require_role(&mut tx, brain, false).await?;
    if write {
        require_open(&mut tx, brain).await?;
    }
    definitions::lock(&mut tx).await?;
    Ok(tx)
}
async fn require_open(tx: &mut Tx<'_>, brain: Uuid) -> Result<()> {
    let archived: bool = sqlx::query_scalar("SELECT archived FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&mut **tx)
        .await?;
    if archived {
        return Err(conflict(
            "Reopen this Brain before configuring or using execution profiles.",
        ));
    }
    Ok(())
}
async fn require_admin(tx: &mut Tx<'_>, brain: Uuid) -> Result<()> {
    if db::require_role(tx, brain, false).await? != "admin" {
        return Err(Error::forbidden());
    }
    Ok(())
}
async fn environment(tx: &mut Tx<'_>, brain: Uuid, id: Option<Uuid>) -> Result<()> {
    if !workspace::selection_valid(
        tx,
        brain,
        &ScopeSelection {
            repository_ids: vec![],
            area_ids: vec![],
            environment_id: id,
        },
    )
    .await?
    {
        return Err(Error::missing());
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct ConnectionRow {
    id: Uuid,
    brain_id: Uuid,
    name: String,
    description: String,
    definition_key: String,
    environment_id: Option<Uuid>,
    target: String,
    placement: String,
    runner_reference: Option<String>,
    credential_alias: Option<String>,
    configuration: Value,
    enabled: bool,
    revision: Uuid,
    updated_at: DateTime<Utc>,
}
impl ConnectionRow {
    fn input(&self) -> McpConnectionInput {
        McpConnectionInput {
            name: self.name.clone(),
            description: self.description.clone(),
            definition_key: self.definition_key.clone(),
            target: self.target.clone(),
            placement: self.placement.clone(),
            runner_reference: self.runner_reference.clone(),
            credential_alias: self.credential_alias.clone(),
            environment_id: self.environment_id,
            configuration: self.configuration.clone(),
            enabled: self.enabled,
            base_revision: Some(self.revision),
        }
    }
    fn summary(&self, definition: &definitions::DefinitionRow) -> McpConnectionSummary {
        let availability = if !self.enabled {
            "disabled"
        } else if !definition.enabled {
            "definition_disabled"
        } else if valid_connection(&self.input(), &definition.manifest).is_err() {
            "configuration_invalid"
        } else {
            "configured"
        };
        McpConnectionSummary {
            id: self.id,
            brain_id: self.brain_id,
            name: self.name.clone(),
            description: self.description.clone(),
            definition_key: self.definition_key.clone(),
            placement: self.placement.clone(),
            private_runner_id: private_binding_id(
                &self.placement,
                self.runner_reference.as_deref(),
            ),
            environment_id: self.environment_id,
            enabled: self.enabled,
            revision: self.revision,
            availability: availability.into(),
            updated_at: self.updated_at,
            last_successful_call_at: None,
        }
    }
}
fn private_binding_id(placement: &str, reference: Option<&str>) -> Option<Uuid> {
    if placement != "private" {
        return None;
    }
    let reference = reference?;
    let id = Uuid::parse_str(reference.strip_prefix("private:")?).ok()?;
    (reference == format!("private:{id}")).then_some(id)
}

#[cfg(test)]
mod private_binding_tests {
    use super::*;

    #[test]
    fn summary_reveals_only_private_uuid_binding() {
        let id = Uuid::parse_str("abcdefab-cdef-4abc-8def-abcdefabcdef").unwrap();
        let reference = format!("private:{id}");
        assert_eq!(private_binding_id("private", Some(&reference)), Some(id));
        assert_eq!(private_binding_id("central", Some(&reference)), None);
        assert_eq!(private_binding_id("local", Some(&reference)), None);
        assert_eq!(private_binding_id("private", Some("device:host")), None);
        assert_eq!(private_binding_id("private", Some("private:invalid")), None);
        assert_eq!(
            private_binding_id("private", Some(&format!("private:{}", id.simple()))),
            None
        );
        assert_eq!(
            private_binding_id("private", Some(&reference.to_uppercase())),
            None
        );
        assert_eq!(
            private_binding_id(
                "private",
                Some(&format!("private:{}", id.to_string().to_uppercase()))
            ),
            None
        );
        assert_eq!(private_binding_id("private", None), None);
    }

    #[test]
    fn legacy_summary_without_binding_remains_readable() {
        let id = Uuid::new_v4();
        let legacy = serde_json::json!({
            "id": id,
            "brain_id": id,
            "name": "Existing connection",
            "description": "",
            "definition_key": "existing",
            "placement": "private",
            "environment_id": null,
            "enabled": true,
            "revision": id,
            "availability": "configured",
            "updated_at": "2026-10-06T12:00:00Z",
            "last_successful_call_at": null
        });
        let summary: McpConnectionSummary = serde_json::from_value(legacy).unwrap();
        assert_eq!(summary.private_runner_id, None);
    }
}
async fn connection(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ConnectionRow> {
    sqlx::query_as("SELECT * FROM mcp_connections WHERE brain_id=$1 AND id=$2")
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)
}
fn valid_connection(input: &McpConnectionInput, def: &McpDefinitionManifest) -> Result<()> {
    text(&input.name, 120, false)?;
    text(&input.description, 2000, true)?;
    text(&input.target, 2048, false)?;
    if !def.placements.contains(&input.placement)
        || input
            .credential_alias
            .as_ref()
            .is_some_and(|alias| !def.credential_aliases.contains(alias))
    {
        return Err(Error::invalid(
            "Select an approved placement and credential alias for this definition.",
        ));
    }
    if (input.placement == "central" && input.runner_reference.is_some())
        || (input.placement != "central"
            && !input.runner_reference.as_ref().is_some_and(|r| {
                !r.is_empty()
                    && r.len() <= 200
                    && r.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
            }))
    {
        return Err(Error::invalid(
            "Central placement has no runner reference; local/private placement needs a bounded reference.",
        ));
    }
    if def.transport == "streamable_http" {
        let target = reqwest::Url::parse(&input.target)
            .map_err(|_| Error::invalid("Supply a valid MCP HTTP target."))?;
        if !(target.scheme() == "https"
            || (target.scheme() == "http"
                && matches!(target.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
            || target.host_str().is_none()
            || !target.username().is_empty()
            || target.password().is_some()
            || target.query().is_some()
            || target.fragment().is_some()
            || input.target.chars().any(char::is_whitespace)
        {
            return Err(Error::invalid(
                "MCP targets require HTTPS except loopback HTTP, without userinfo, query or fragment.",
            ));
        }
    }
    if !definitions::configuration_valid(def, &input.configuration) {
        return Err(Error::invalid(
            "Configuration must match the approved schema and contain no inline credentials; choose a credential alias.",
        ));
    }
    Ok(())
}
async fn connection_detail(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<McpConnectionDetail> {
    let row = connection(tx, brain, id).await?;
    let def = definitions::load(tx, &row.definition_key).await?;
    let profile_ids=sqlx::query_scalar("SELECT profile_id FROM mcp_profile_connections WHERE brain_id=$1 AND connection_id=$2 ORDER BY profile_id")
        .bind(brain).bind(id).fetch_all(&mut **tx).await?;
    Ok(McpConnectionDetail {
        summary: row.summary(&def),
        target: row.target,
        runner_reference: row.runner_reference,
        credential_alias: row.credential_alias,
        configuration: row.configuration,
        profile_ids,
    })
}

#[utoipa::path(get,path="/api/brains/{brain}/mcp",operation_id="mcpCatalogue",params(("brain"=Uuid,Path)),responses((status=200,body=McpCatalogue)))]
pub async fn catalogue(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<McpCatalogue>> {
    let mut tx = transaction(&state, &auth, brain, false).await?;
    let role = db::require_role(&mut tx, brain, false).await?;
    let profiles = profiles::list(&mut tx, brain).await?;
    let manager = role == "admin" || profiles.iter().any(|p| p.rights.manage);
    let definitions: Vec<definitions::DefinitionRow> =
        sqlx::query_as("SELECT manifest,enabled,updated_at FROM mcp_definitions ORDER BY key")
            .fetch_all(&mut *tx)
            .await?;
    let connections: Vec<ConnectionRow> =
        sqlx::query_as("SELECT * FROM mcp_connections WHERE brain_id=$1 ORDER BY lower(name),id")
            .bind(brain)
            .fetch_all(&mut *tx)
            .await?;
    // Only successful calls the current actor may inspect, with the current
    // connection configuration. This timestamp never asserts live connectivity.
    let successful: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT c.connection_id,max(c.completed_at) FROM mcp_calls c JOIN mcp_connections n ON n.id=c.connection_id AND n.revision=c.connection_revision JOIN mcp_definitions d ON d.key=n.definition_key AND d.updated_at=c.definition_revision WHERE c.brain_id=$1 AND c.state='succeeded' AND c.completed_at IS NOT NULL AND recollect_mcp_can(c.profile_id,'use') GROUP BY c.connection_id",
    )
    .bind(brain)
    .fetch_all(&mut *tx)
    .await?;
    let connections = connections
        .into_iter()
        .filter(|c| manager || profiles.iter().any(|p| p.connection_ids.contains(&c.id)))
        .filter_map(|c| {
            definitions
                .iter()
                .find(|d| d.manifest.key == c.definition_key)
                .map(|d| {
                    let mut summary = c.summary(d);
                    summary.last_successful_call_at = successful
                        .iter()
                        .find(|(id, _)| *id == c.id)
                        .map(|(_, at)| *at);
                    summary
                })
        })
        .collect();
    let definitions = if role == "admin" {
        definitions.iter().map(|d| d.summary()).collect()
    } else {
        vec![]
    };
    tx.commit().await?;
    Ok(Json(McpCatalogue {
        definitions,
        connections,
        profiles,
        can_configure: role == "admin",
        execution_state: "not_connected".into(),
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/mcp/definitions/{key}",operation_id="mcpDefinition",params(("brain"=Uuid,Path),("key"=String,Path)),responses((status=200,body=McpDefinitionDetail)))]
pub async fn get_definition(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, key)): Path<(Uuid, String)>,
) -> Result<Json<McpDefinitionDetail>> {
    let mut tx = transaction(&state, &auth, brain, false).await?;
    require_admin(&mut tx, brain).await?;
    let result = definitions::load(&mut tx, &key).await?.detail();
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/brains/{brain}/mcp/connections/{id}",operation_id="mcpConnection",params(("brain"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=McpConnectionDetail)))]
pub async fn get_connection(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<McpConnectionDetail>> {
    let mut tx = transaction(&state, &auth, brain, false).await?;
    require_admin(&mut tx, brain).await?;
    let result = connection_detail(&mut tx, brain, id).await?;
    tx.commit().await?;
    Ok(Json(result))
}
async fn write_connection(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    id: Option<Uuid>,
    headers: HeaderMap,
    mut input: McpConnectionInput,
) -> Result<Json<McpConnectionDetail>> {
    input.name = input.name.trim().into();
    let key = commands::key(&headers)?;
    let mut tx = transaction(state, auth, brain, true).await?;
    require_admin(&mut tx, brain).await?;
    let mut disabling_unchanged = false;
    if let Some(id) = id {
        let old = connection(&mut tx, brain, id).await?;
        if input.base_revision != Some(old.revision) {
            return Err(conflict(
                "This connection changed. Refresh it before saving.",
            ));
        }
        disabling_unchanged = !input.enabled
            && input.definition_key == old.definition_key
            && input.target == old.target
            && input.placement == old.placement
            && input.runner_reference == old.runner_reference
            && input.credential_alias == old.credential_alias
            && input.environment_id == old.environment_id
            && input.configuration == old.configuration;
        let incompatible:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM mcp_profile_connections m JOIN mcp_profiles p ON p.id=m.profile_id WHERE m.connection_id=$1 AND m.brain_id=$2 AND $3::uuid IS NOT NULL AND p.environment_id IS DISTINCT FROM $3)")
            .bind(old.id).bind(brain).bind(input.environment_id).fetch_one(&mut *tx).await?;
        if incompatible {
            return Err(conflict(
                "This environment would conflict with profiles using the connection. Reassign those memberships first.",
            ));
        }
    } else {
        if input.base_revision.is_some() {
            return Err(Error::invalid("A new connection has no base revision."));
        }
        if let Some(saved) = commands::reserve::<McpConnectionDetail>(
            &mut tx,
            key.as_deref(),
            &format!("mcp.connection.create:{brain}"),
            serde_json::to_value(&input).expect("connection serialization"),
        )
        .await?
        {
            tx.commit().await?;
            return Ok(Json(saved));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM mcp_connections WHERE brain_id=$1")
                .bind(brain)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 100 {
            return Err(capacity());
        }
    }
    environment(&mut tx, brain, input.environment_id).await?;
    let def = definitions::load(&mut tx, &input.definition_key).await?;
    if !def.enabled && input.enabled {
        return Err(conflict(
            "This definition is disabled. Choose an approved definition or disable the connection.",
        ));
    }
    text(&input.name, 120, false)?;
    text(&input.description, 2000, true)?;
    if !disabling_unchanged {
        valid_connection(&input, &def.manifest)?;
        if input.placement == "private" && input.enabled {
            private::require(
                &mut tx,
                brain,
                input.runner_reference.as_deref().unwrap_or_default(),
            )
            .await?;
        }
    }
    let existing = id.is_some();
    let id = id.unwrap_or_else(Uuid::new_v4);
    let revision = Uuid::new_v4();
    sqlx::query("INSERT INTO mcp_connections(id,brain_id,name,description,definition_key,environment_id,target,placement,runner_reference,credential_alias,configuration,enabled,revision,created_by) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,definition_key=excluded.definition_key,environment_id=excluded.environment_id,target=excluded.target,placement=excluded.placement,runner_reference=excluded.runner_reference,credential_alias=excluded.credential_alias,configuration=excluded.configuration,enabled=excluded.enabled,revision=excluded.revision,updated_at=clock_timestamp()")
        .bind(id).bind(brain).bind(&input.name).bind(&input.description).bind(&input.definition_key).bind(input.environment_id).bind(&input.target).bind(&input.placement).bind(&input.runner_reference).bind(&input.credential_alias).bind(&input.configuration).bind(input.enabled).bind(revision).bind(auth.user.id).execute(&mut *tx).await.map_err(mutation_error)?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "mcp.connection",
        id,
        if existing { "updated" } else { "created" },
    )
    .await?;
    let result = connection_detail(&mut tx, brain, id).await?;
    if !existing {
        commands::finish(&mut tx, key.as_deref(), brain, &result).await?;
    }
    tx.commit().await?;
    Ok(Json(result))
}
#[utoipa::path(post,path="/api/brains/{brain}/mcp/connections",operation_id="createMcpConnection",params(("brain"=Uuid,Path)),request_body=McpConnectionInput,responses((status=200,body=McpConnectionDetail)))]
pub async fn create_connection(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<McpConnectionInput>,
) -> Result<Json<McpConnectionDetail>> {
    write_connection(&state, &auth, brain, None, headers, input).await
}
#[utoipa::path(put,path="/api/brains/{brain}/mcp/connections/{id}",operation_id="updateMcpConnection",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpConnectionInput,responses((status=200,body=McpConnectionDetail)))]
pub async fn update_connection(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<McpConnectionInput>,
) -> Result<Json<McpConnectionDetail>> {
    write_connection(&state, &auth, brain, Some(id), headers, input).await
}
