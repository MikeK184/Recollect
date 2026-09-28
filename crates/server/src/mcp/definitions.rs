use super::*;
use serde_json::json;
use std::collections::HashSet;
use tokio::io::AsyncReadExt;

const CATALOGUE_LOCK: i64 = 73241022;

#[utoipa::path(post,path="/api/mcp/definitions/inspect-http",operation_id="inspectHttpMcpDefinition",request_body=McpHttpInspection,responses((status=200,body=McpDefinitionManifest)))]
pub async fn inspect_http(
    State(state): State<AppState>,
    auth: Auth,
    Json(input): Json<McpHttpInspection>,
) -> Result<Json<McpDefinitionManifest>> {
    auth.require_browser()?;
    if !auth.user.installation_owner {
        return Err(Error::forbidden());
    }
    let _permit = state
        .mcp_inspections
        .clone()
        .try_acquire_owned()
        .map_err(|_| capacity())?;
    text(input.name.trim(), 120, false)?;
    let tools = recollect_mcp_runtime::discovery::inspect_http(input.url.trim()).await
        .map_err(|e| Error(StatusCode::BAD_GATEWAY, e.0,
            "Could not inspect this anonymous MCP server. Check its URL and authentication requirements."))?;
    let manifest = McpDefinitionManifest {
        key: format!("http-{}", Uuid::new_v4()),
        name: input.name.trim().into(),
        description: String::new(),
        transport: "streamable_http".into(),
        command: None,
        arguments: vec![],
        placements: vec!["central".into()],
        credential_aliases: vec![],
        configuration_schema: json!({"type":"object", "properties":{}, "additionalProperties":false}),
        tools,
        receipt_policies: vec![],
    };
    validate_manifest(&manifest)?;
    crate::publication::safe_payload(
        &state,
        &serde_json::to_value(&manifest).map_err(|_| Error::invalid("Invalid manifest."))?,
    )?;
    Ok(Json(manifest))
}

#[utoipa::path(post,path="/api/mcp/definitions",operation_id="approveMcpDefinition",request_body=McpDefinitionManifest,responses((status=200,body=McpDefinitionSummary)))]
pub async fn approve(
    State(state): State<AppState>,
    auth: Auth,
    Json(manifest): Json<McpDefinitionManifest>,
) -> Result<Json<McpDefinitionSummary>> {
    auth.require_browser()?;
    if !auth.user.installation_owner {
        return Err(Error::forbidden());
    }
    validate_manifest(&manifest)?;
    let value = serde_json::to_value(&manifest)
        .map_err(|_| Error::invalid("Invalid connector manifest."))?;
    crate::publication::safe_payload(&state, &value)?;
    let mut tx = auth.tx(&state.pool).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(CATALOGUE_LOCK)
        .execute(&mut *tx)
        .await?;
    let existing: Option<(Value, bool)> =
        sqlx::query_as("SELECT manifest,enabled FROM mcp_definitions WHERE key=$1")
            .bind(&manifest.key)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some((previous, enabled)) = existing {
        if previous != value || !enabled {
            return Err(conflict(
                "This connector key already exists. Use a new key, or have the operator update it through the CLI.",
            ));
        }
    } else {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM mcp_definitions")
            .fetch_one(&mut *tx)
            .await?;
        if count >= 100 {
            return Err(capacity());
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO mcp_definitions(key,id,manifest,approved_by) VALUES($1,$2,$3,$4)")
            .bind(&manifest.key)
            .bind(id)
            .bind(&value)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO mutation_audit(id,actor_id,action,target_id,disposition) VALUES($1,$2,'mcp.definition',$3,'approved')")
            .bind(Uuid::new_v4()).bind(auth.user.id).bind(id).execute(&mut *tx).await?;
    }
    let result = load(&mut tx, &manifest.key).await?.summary();
    tx.commit().await?;
    Ok(Json(result))
}

#[derive(sqlx::FromRow)]
pub(super) struct DefinitionRow {
    pub manifest: DbJson<McpDefinitionManifest>,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}
impl DefinitionRow {
    pub fn summary(&self) -> McpDefinitionSummary {
        McpDefinitionSummary {
            key: self.manifest.key.clone(),
            name: self.manifest.name.clone(),
            description: self.manifest.description.clone(),
            transport: self.manifest.transport.clone(),
            enabled: self.enabled,
            tool_count: self.manifest.tools.len(),
            updated_at: self.updated_at,
        }
    }
    pub fn detail(self) -> McpDefinitionDetail {
        McpDefinitionDetail {
            summary: self.summary(),
            placements: self.manifest.placements.clone(),
            credential_aliases: self.manifest.credential_aliases.clone(),
            configuration_schema: self.manifest.configuration_schema.clone(),
            tools: self.manifest.tools.clone(),
            receipt_policies: self.manifest.receipt_policies.clone(),
        }
    }
}

pub(super) async fn lock(tx: &mut Tx<'_>) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
        .bind(CATALOGUE_LOCK)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(super) async fn load(tx: &mut Tx<'_>, key: &str) -> Result<DefinitionRow> {
    sqlx::query_as("SELECT manifest,enabled,updated_at FROM mcp_definitions WHERE key=$1")
        .bind(key)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)
}
pub(super) fn identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}
fn schema_tree(value: &Value, depth: usize) -> bool {
    if depth > 24 {
        return false;
    }
    match value {
        Value::Object(map) => map.iter().all(|(key, value)| {
            // No remote/file schema lookup, implicit resource rebase or model-driven
            // HTTP-header routing in this approved catalogue.
            if matches!(key.as_str(), "$id" | "x-mcp-header") {
                return false;
            }
            if matches!(key.as_str(), "$ref" | "$dynamicRef")
                && !value.as_str().is_some_and(|v| v.starts_with('#'))
            {
                return false;
            }
            if key == "$schema"
                && value.as_str() != Some("https://json-schema.org/draft/2020-12/schema")
            {
                return false;
            }
            schema_tree(value, depth + 1)
        }),
        Value::Array(values) => values.iter().all(|v| schema_tree(v, depth + 1)),
        _ => true,
    }
}
pub(super) fn validator(
    schema: &Value,
    object: bool,
    limit: usize,
) -> Result<jsonschema::Validator> {
    if serde_json::to_vec(schema).map_or(true, |v| v.len() > limit)
        || !schema_tree(schema, 0)
        || (object && schema.get("type").and_then(Value::as_str) != Some("object"))
    {
        return Err(Error::invalid(
            "Use a bounded 2020-12 schema with local references; inputs must describe objects.",
        ));
    }
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .offline()
        .build(schema)
        .map_err(|_| {
            Error::invalid(
                "The approved JSON Schema is invalid or has an unresolved local reference.",
            )
        })
}
fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace(['-', '_'], "");
    matches!(
        key.as_str(),
        "password"
            | "passwd"
            | "secret"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "apikey"
            | "authorization"
            | "privatekey"
            | "clientsecret"
            | "credentials"
    )
}
fn non_secret_configuration(value: &Value, depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    match value {
        Value::Object(values) => values
            .iter()
            .all(|(key, value)| !secret_key(key) && non_secret_configuration(value, depth + 1)),
        Value::Array(values) => values
            .iter()
            .all(|value| non_secret_configuration(value, depth + 1)),
        Value::String(value) => {
            !value.contains("-----BEGIN")
                && !value.starts_with("Bearer ")
                && !value.starts_with("sk-proj-")
        }
        _ => true,
    }
}
pub(super) fn configuration_valid(def: &McpDefinitionManifest, value: &Value) -> bool {
    value.is_object()
        && serde_json::to_vec(value).is_ok_and(|v| v.len() <= 16384)
        && non_secret_configuration(value, 0)
        && validator(&def.configuration_schema, true, 16384).is_ok_and(|v| v.is_valid(value))
}
pub fn validate_manifest(input: &McpDefinitionManifest) -> Result<()> {
    if !identifier(&input.key, 64)
        || text(&input.name, 120, false).is_err()
        || text(&input.description, 2000, true).is_err()
        || serde_json::to_vec(input).map_or(true, |v| v.len() > 524288)
    {
        return Err(Error::invalid(
            "Definition identity, description or total size is invalid.",
        ));
    }
    let stdio = input.transport == "stdio";
    if !matches!(input.transport.as_str(), "stdio" | "streamable_http")
        || (stdio
            && !input.command.as_ref().is_some_and(|c| {
                std::path::Path::new(c).is_absolute() && text(c, 2048, false).is_ok()
            }))
        || (!stdio && (input.command.is_some() || !input.arguments.is_empty()))
        || input.arguments.len() > 128
        || input.arguments.iter().any(|a| text(a, 2048, true).is_err())
    {
        return Err(Error::invalid(
            "Approve an absolute stdio executable and literal arguments, or an HTTP definition without a command.",
        ));
    }
    if input.placements.is_empty()
        || input.placements.len() > 3
        || input
            .placements
            .iter()
            .any(|p| !matches!(p.as_str(), "central" | "local" | "private"))
        || input.placements.iter().collect::<HashSet<_>>().len() != input.placements.len()
        || input.credential_aliases.len() > 50
        || input.credential_aliases.iter().any(|a| !identifier(a, 120))
        || input
            .credential_aliases
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != input.credential_aliases.len()
    {
        return Err(Error::invalid(
            "Choose distinct supported placements and bounded credential alias names.",
        ));
    }
    if input.configuration_schema.get("additionalProperties") != Some(&Value::Bool(false)) {
        return Err(Error::invalid(
            "Configuration schemas must reject undeclared properties.",
        ));
    }
    validator(&input.configuration_schema, true, 16384)?;
    if input.tools.len() > 50 {
        return Err(Error::invalid(
            "Approve at most 50 cached tools per definition.",
        ));
    }
    let mut names = HashSet::new();
    for tool in &input.tools {
        if !identifier(&tool.name, 128)
            || !names.insert(&tool.name)
            || text(&tool.description, 2000, true).is_err()
        {
            return Err(Error::invalid(
                "Tool names must be unique bounded ASCII identifiers with bounded descriptions.",
            ));
        }
        validator(&tool.input_schema, true, 32768)?;
        if let Some(schema) = &tool.output_schema {
            validator(schema, false, 32768)?;
        }
        if let Some(annotations) = &tool.annotations {
            let Some(map) = annotations.as_object() else {
                return Err(Error::invalid(
                    "Tool annotations must be an object of supported hints.",
                ));
            };
            for (key, value) in map {
                let valid = match key.as_str() {
                    "title" => value.as_str().is_some_and(|v| text(v, 120, true).is_ok()),
                    "readOnlyHint" | "destructiveHint" | "idempotentHint" | "openWorldHint" => {
                        value.is_boolean()
                    }
                    _ => false,
                };
                if !valid {
                    return Err(Error::invalid(
                        "Tool annotations contain an unsupported hint.",
                    ));
                }
            }
        }
    }
    let mut receipt_names = HashSet::new();
    for policy in &input.receipt_policies {
        if !receipt_names.insert(&policy.tool_name)
            || policy.tool_name == policy.receipt_tool
            || !identifier(&policy.operation_id_argument, 128)
            || !identifier(&policy.receipt_id_argument, 128)
        {
            return Err(Error::invalid(
                "Receipt policies require distinct approved tools and reserved argument names.",
            ));
        }
        for (name, argument) in [
            (&policy.tool_name, &policy.operation_id_argument),
            (&policy.receipt_tool, &policy.receipt_id_argument),
        ] {
            let property = input
                .tools
                .iter()
                .find(|t| &t.name == name)
                .and_then(|t| t.input_schema.get("properties"))
                .and_then(|v| v.get(argument))
                .ok_or_else(|| {
                    Error::invalid(
                        "Receipt UUID arguments must be declared in both approved tool schemas.",
                    )
                })?;
            if property.get("type").and_then(Value::as_str) != Some("string")
                || !validator(property, false, 32768)?
                    .is_valid(&Value::String(Uuid::nil().to_string()))
            {
                return Err(Error::invalid(
                    "Receipt arguments must accept UUID strings.",
                ));
            }
        }
    }
    Ok(())
}

pub async fn import_file(admin: &sqlx::PgPool, path: &str) -> anyhow::Result<()> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| anyhow::anyhow!("Cannot open the MCP approval file"))?;
    anyhow::ensure!(
        file.metadata().await?.is_file(),
        "MCP approval must be a regular JSON file"
    );
    let mut bytes = Vec::new();
    file.take(524289).read_to_end(&mut bytes).await?;
    anyhow::ensure!(bytes.len() <= 524288, "MCP approval exceeds 512 KiB");
    let manifest: McpDefinitionManifest = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("MCP approval JSON has invalid or unknown fields"))?;
    import_manifest(admin, manifest).await
}
pub async fn import_manifest(
    admin: &sqlx::PgPool,
    manifest: McpDefinitionManifest,
) -> anyhow::Result<()> {
    validate_manifest(&manifest).map_err(|e| anyhow::anyhow!(e.2))?;
    let mut tx = admin.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(CATALOGUE_LOCK)
        .execute(&mut *tx)
        .await?;
    let owner: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner AND enabled")
            .fetch_one(&mut *tx)
            .await?;
    let existing: Option<(Uuid, Value, bool)> =
        sqlx::query_as("SELECT id,manifest,enabled FROM mcp_definitions WHERE key=$1")
            .bind(&manifest.key)
            .fetch_optional(&mut *tx)
            .await?;
    let value = serde_json::to_value(&manifest)?;
    if existing
        .as_ref()
        .is_some_and(|(_, old, enabled)| old == &value && *enabled)
    {
        tx.commit().await?;
        return Ok(());
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM mcp_definitions")
        .fetch_one(&mut *tx)
        .await?;
    anyhow::ensure!(
        existing.is_some() || count < 100,
        "MCP definition capacity reached"
    );
    let id = existing.map_or_else(Uuid::new_v4, |r| r.0);
    sqlx::query("INSERT INTO mcp_definitions(key,id,manifest,approved_by) VALUES($1,$2,$3,$4) ON CONFLICT(key) DO UPDATE SET manifest=excluded.manifest,enabled=true,approved_by=excluded.approved_by,updated_at=clock_timestamp()")
        .bind(&manifest.key).bind(id).bind(&value).bind(owner).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO mutation_audit(id,actor_id,action,target_id,disposition) VALUES($1,$2,'mcp.definition',$3,'approved')")
        .bind(Uuid::new_v4()).bind(owner).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn disable(admin: &sqlx::PgPool, key: &str) -> anyhow::Result<()> {
    let mut tx = admin.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(CATALOGUE_LOCK)
        .execute(&mut *tx)
        .await?;
    let owner: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner AND enabled")
            .fetch_one(&mut *tx)
            .await?;
    let id:Option<Uuid>=sqlx::query_scalar("UPDATE mcp_definitions SET enabled=false,updated_at=clock_timestamp() WHERE key=$1 RETURNING id")
        .bind(key).fetch_optional(&mut *tx).await?;
    let id = id.ok_or_else(|| anyhow::anyhow!("Approved definition not found"))?;
    sqlx::query("INSERT INTO mutation_audit(id,actor_id,action,target_id,disposition) VALUES($1,$2,'mcp.definition',$3,'disabled')")
        .bind(Uuid::new_v4()).bind(owner).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
