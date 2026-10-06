use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpHttpInspection {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpSecretHeader {
    pub name: String,
    #[serde(default)]
    pub prefix: String,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCredentialProvision {
    pub base_revision: Uuid,
    #[serde(default)]
    pub headers: Vec<McpSecretHeader>,
    #[serde(default)]
    pub environment: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpCredentialProvisioned {
    pub configured: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpGlobalDefinition {
    pub summary: McpDefinitionSummary,
    pub manifest: McpDefinitionManifest,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct McpToolDescriptor {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub input_schema: Value,
    #[serde(default)]
    pub output_schema: Option<Value>,
    #[serde(default)]
    pub annotations: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpDefinitionManifest {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    pub placements: Vec<String>,
    pub configuration_schema: Value,
    #[serde(default)]
    pub credential_aliases: Vec<String>,
    pub tools: Vec<McpToolDescriptor>,
    /// Bounded, validated, metadata-free cached PNG; never a remote URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_png: Option<String>,
    #[serde(default)]
    pub receipt_policies: Vec<McpReceiptPolicy>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpReceiptPolicy {
    pub tool_name: String,
    pub operation_id_argument: String,
    pub receipt_tool: String,
    pub receipt_id_argument: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpDefinitionSummary {
    pub key: String,
    pub name: String,
    pub description: String,
    pub transport: String,
    pub enabled: bool,
    pub tool_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_png: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpDefinitionDetail {
    pub summary: McpDefinitionSummary,
    pub placements: Vec<String>,
    pub credential_aliases: Vec<String>,
    pub configuration_schema: Value,
    pub tools: Vec<McpToolDescriptor>,
    pub receipt_policies: Vec<McpReceiptPolicy>,
}

fn enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpConnectionInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub definition_key: String,
    pub target: String,
    pub placement: String,
    pub runner_reference: Option<String>,
    pub credential_alias: Option<String>,
    pub environment_id: Option<Uuid>,
    pub configuration: Value,
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub base_revision: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpConnectionSummary {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub name: String,
    pub description: String,
    pub definition_key: String,
    pub placement: String,
    pub environment_id: Option<Uuid>,
    pub enabled: bool,
    pub revision: Uuid,
    pub availability: String,
    pub updated_at: DateTime<Utc>,
    pub last_successful_call_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpConnectionDetail {
    pub summary: McpConnectionSummary,
    pub target: String,
    pub runner_reference: Option<String>,
    pub credential_alias: Option<String>,
    pub configuration: Value,
    pub profile_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRights {
    pub use_profile: bool,
    pub manage: bool,
    pub share: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpProfileInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub environment_id: Option<Uuid>,
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub connection_ids: Vec<Uuid>,
    pub base_revision: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpProfile {
    pub id: Uuid,
    pub brain_id: Uuid,
    pub name: String,
    pub description: String,
    pub environment_id: Option<Uuid>,
    pub enabled: bool,
    pub connection_ids: Vec<Uuid>,
    pub revision: Uuid,
    pub rights: McpRights,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpGrantInput {
    pub username: Option<String>,
    pub group_name: Option<String>,
    pub rights: McpRights,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpGrant {
    pub id: Uuid,
    pub account_id: Option<Uuid>,
    pub username: Option<String>,
    pub issuer: Option<String>,
    pub group_name: Option<String>,
    pub rights: McpRights,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpEffectiveMember {
    pub account_id: Uuid,
    pub username: String,
    pub enabled: bool,
    pub brain_role: Option<String>,
    pub rights: McpRights,
    pub direct_rights: McpRights,
    pub groups: Vec<String>,
    pub membership_until: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpProfileDetail {
    pub profile: McpProfile,
    pub grants: Option<Vec<McpGrant>>,
    pub effective_members: Vec<McpEffectiveMember>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpGrantChange {
    pub rights: McpRights,
    pub profile: Option<McpProfileDetail>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpCatalogue {
    pub definitions: Vec<McpDefinitionSummary>,
    pub connections: Vec<McpConnectionSummary>,
    pub profiles: Vec<McpProfile>,
    pub can_configure: bool,
    pub execution_state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpDiscoverRequest {
    pub profile_id: Uuid,
    pub environment_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpDiscoveredTool {
    pub connection_id: Uuid,
    pub connection_name: String,
    pub tool: McpToolDescriptor,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpUnavailableConnection {
    pub connection_id: Uuid,
    pub name: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct McpDiscovery {
    pub profile_id: Uuid,
    pub environment_id: Option<Uuid>,
    pub operation_id: Option<Uuid>,
    pub source: String,
    pub execution_state: String,
    pub tools: Vec<McpDiscoveredTool>,
    pub unavailable_connections: Vec<McpUnavailableConnection>,
    pub total: usize,
    pub offset: usize,
    pub next_offset: Option<usize>,
}
