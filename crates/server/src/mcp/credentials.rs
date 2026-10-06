//! Explicit owner provisioning. Only references and metadata enter the database.
use super::*;
use std::{collections::BTreeMap, path::PathBuf, sync::LazyLock};
use tokio::io::AsyncReadExt;

static PROVISION: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));

pub fn binding_path(config: &crate::config::Config) -> PathBuf {
    std::env::var_os("RECOLLECT_MCP_CREDENTIALS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| sibling(config, "mcp-bindings"))
}
fn sibling(config: &crate::config::Config, kind: &str) -> PathBuf {
    let account = PathBuf::from(&config.credential_file);
    let stem = account.file_stem().unwrap_or_default().to_string_lossy();
    account.with_file_name(format!("{stem}.{kind}.json"))
}
fn unavailable() -> Error {
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        "credential_provider_unavailable",
        "The installation credential provider is unavailable. No tool has been run.",
    )
}
fn valid_value(value: &str) -> bool {
    (4..=16384).contains(&value.len()) && !value.contains('\0')
}
async fn bindings(path: &PathBuf) -> Result<Value> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(serde_json::json!({"bindings":[]}));
        }
        Err(_) => return Err(unavailable()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(unavailable());
    }
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| unavailable())?;
    let mut bytes = Vec::new();
    file.take(131073)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| unavailable())?;
    if bytes.len() > 131072 {
        return Err(unavailable());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    if !value.get("bindings").is_some_and(Value::is_array) {
        return Err(unavailable());
    }
    Ok(value)
}
async fn write_bindings(path: PathBuf, value: Value) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let bytes = serde_json::to_vec(&value).map_err(|_| unavailable())?;
        if bytes.len() > 131072 {
            return Err(unavailable());
        }
        let parent = path.parent().ok_or_else(unavailable)?;
        std::fs::create_dir_all(parent).map_err(|_| unavailable())?;
        let temporary = parent.join(format!(".mcp-bindings-{}.tmp", Uuid::new_v4()));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(|_| unavailable())?;
            file.write_all(&bytes).map_err(|_| unavailable())?;
            file.sync_all().map_err(|_| unavailable())?;
            std::fs::rename(&temporary, &path).map_err(|_| unavailable())?;
            std::fs::File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|_| unavailable())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    })
    .await
    .map_err(|_| unavailable())?
}

#[utoipa::path(post,path="/api/brains/{brain}/mcp/connections/{id}/credentials",operation_id="provisionMcpCredentials",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=McpCredentialProvision,responses((status=200,body=McpCredentialProvisioned)))]
pub async fn provision(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<McpCredentialProvision>,
) -> Result<Json<McpCredentialProvisioned>> {
    auth.require_browser()?;
    if !auth.user.installation_owner {
        return Err(Error::forbidden());
    }
    let mut tx = transaction(&state, &auth, brain, true).await?;
    require_admin(&mut tx, brain).await?;
    let row = connection(&mut tx, brain, id).await?;
    if row.revision != input.base_revision {
        return Err(conflict(
            "This connection changed. Reopen it before saving credentials.",
        ));
    }
    let definition = definitions::load(&mut tx, &row.definition_key).await?;
    let alias = row
        .credential_alias
        .as_ref()
        .ok_or_else(|| Error::invalid("Choose an approved credential alias first."))?;
    if row.placement != "central"
        || !definition.enabled
        || !definition.manifest.credential_aliases.contains(alias)
        || !definitions::configuration_valid(&definition.manifest, &row.configuration)
    {
        return Err(Error::invalid(
            "Credentials can be provisioned only for a compatible central connection.",
        ));
    }
    if input.headers.len() + input.environment.len() == 0
        || input.headers.len() + input.environment.len() > 32
        || (definition.manifest.transport == "stdio" && !input.headers.is_empty())
        || (definition.manifest.transport == "streamable_http" && !input.environment.is_empty())
    {
        return Err(Error::invalid(
            "Choose at most 32 credential destinations matching the connector transport.",
        ));
    }
    let mut names = std::collections::HashSet::new();
    for header in &input.headers {
        let name = header.name.to_ascii_lowercase();
        if !recollect_mcp_runtime::credentials::header_destination(&name)
            || !matches!(header.prefix.as_str(), "" | "Bearer ")
            || !valid_value(&header.value)
            || !names.insert(name)
            || reqwest::header::HeaderValue::from_str(&format!("{}{}", header.prefix, header.value))
                .is_err()
        {
            return Err(Error::invalid(
                "Use distinct permitted headers and bounded credential values.",
            ));
        }
    }
    for (name, value) in &input.environment {
        if !recollect_mcp_runtime::credentials::environment_destination(name) || !valid_value(value)
        {
            return Err(Error::invalid(
                "Use permitted environment variables and bounded credential values.",
            ));
        }
    }
    let _guard = PROVISION.lock().await;
    let path = binding_path(&state.config);
    let mut file = bindings(&path).await?;
    let rows = file["bindings"].as_array_mut().ok_or_else(unavailable)?;
    let indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, binding)| {
            (binding["connection_id"].as_str() == Some(id.to_string().as_str())
                && binding["alias"].as_str() == Some(alias)
                && binding["runner_reference"].as_str() == Some("central"))
            .then_some(index)
        })
        .collect();
    if indices.len() > 1 || (indices.is_empty() && rows.len() >= 1000) {
        return Err(unavailable());
    }
    let previous = indices
        .first()
        .map(|index| rows[*index].clone())
        .unwrap_or(Value::Null);
    let secret_path = sibling(&state.config, "mcp-secrets");
    if let Ok(metadata) = tokio::fs::symlink_metadata(&secret_path).await
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err(unavailable());
    }
    let mut additions = Vec::new();
    let mut headers = BTreeMap::new();
    let mut environment = BTreeMap::new();
    let mut reference = |old: &Value, value: String| {
        let key = if old["provider"] == "local_file" && old["path"].as_str() == secret_path.to_str()
        {
            old["key"]
                .as_str()
                .and_then(|key| Uuid::parse_str(key).ok())
                .unwrap_or_else(Uuid::new_v4)
        } else {
            Uuid::new_v4()
        };
        additions.push((key, value));
        serde_json::json!({"provider":"local_file","path":secret_path,"key":key})
    };
    for header in input.headers {
        let name = header.name.to_ascii_lowercase();
        headers.insert(
            name.clone(),
            serde_json::json!({
                "source":reference(&previous["headers"][&name]["source"], header.value),
                "prefix":header.prefix
            }),
        );
    }
    for (name, value) in input.environment {
        environment.insert(
            name.clone(),
            reference(&previous["environment"][&name], value),
        );
    }
    let binding = serde_json::json!({
        "connection_id":id,"alias":alias,"runner_reference":"central",
        "headers":headers,"environment":environment
    });
    if let Some(index) = indices.first() {
        rows[*index] = binding;
    } else {
        rows.push(binding);
    }
    // Audit is metadata only. File effects do not grant independent tool access.
    sqlx::query("INSERT INTO mutation_audit(id,actor_id,brain_id,action,target_id,disposition) VALUES($1,$2,$3,'mcp.credentials',$4,'configured')")
        .bind(Uuid::new_v4()).bind(auth.user.id).bind(brain).bind(id).execute(&mut *tx).await?;
    crate::credentials::store_private_many(
        secret_path.to_str().ok_or_else(unavailable)?,
        additions,
    )
    .await?;
    write_bindings(path, file).await?;
    tx.commit().await?;
    Ok(Json(McpCredentialProvisioned { configured: true }))
}
