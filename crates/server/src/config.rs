use anyhow::{Context, ensure};

/// Connections held by the whole process. Six worker lanes each pin a job
/// transaction while a heartbeat connection renews its lease, three maintenance
/// tasks and the observation/privacy loops poll, and recall, graph reads and
/// HTTP requests run alongside them.
pub const DEFAULT_DB_POOL_SIZE: u32 = 32;

/// Pool size from `RECOLLECT_DB_POOL_SIZE`; an unset or invalid value falls
/// back to the default so a typo cannot stop the server from starting.
pub fn db_pool_size() -> u32 {
    std::env::var("RECOLLECT_DB_POOL_SIZE")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|size| (4..=500).contains(size))
        .unwrap_or(DEFAULT_DB_POOL_SIZE)
}

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub owner_username: String,
    pub owner_password: String,
    pub bind: String,
    pub public_origin: String,
    pub static_dir: String,
    pub neo4j_url: String,
    pub neo4j_username: String,
    pub neo4j_password: String,
    pub credential_file: String,
    pub artifact_dir: String,
    pub erasure_journal: String,
    pub erasure_mirror: Option<String>,
    pub models: ModelConfig,
    pub oidc: Option<OidcConfig>,
}

/// An operator-named model; an unset or blank value names none.
fn explicit_model(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Clone)]
pub struct ModelConfig {
    pub key: Option<String>,
    pub endpoint: String,
    pub openrouter_key: Option<String>,
    pub openrouter_endpoint: String,
    /// Optional explicit operator override from `RECOLLECT_TEXT_MODEL`. There
    /// is no built-in default: when absent the installation reports no text
    /// model and each Brain chooses its own.
    pub text_model: Option<String>,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
}
impl ModelConfig {
    fn from_env() -> anyhow::Result<Self> {
        let value = Self {
            key: std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            endpoint: "https://api.openai.com/v1".into(),
            openrouter_key: std::env::var("OPENROUTER_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            openrouter_endpoint: "https://openrouter.ai/api/v1".into(),
            text_model: explicit_model(std::env::var("RECOLLECT_TEXT_MODEL").ok()),
            embedding_model: std::env::var("RECOLLECT_EMBEDDING_MODEL")
                .unwrap_or("text-embedding-3-large".into()),
            embedding_dimensions: std::env::var("RECOLLECT_EMBEDDING_DIMENSIONS")
                .unwrap_or("3072".into())
                .parse()
                .context("Invalid embedding dimensions")?,
        };
        ensure!(
            (1..=3072).contains(&value.embedding_dimensions),
            "Embedding dimensions must be 1–3,072"
        );
        ensure!(
            value.embedding_model != "text-embedding-3-small" || value.embedding_dimensions <= 1536,
            "text-embedding-3-small supports at most 1,536 dimensions"
        );
        for name in value
            .text_model
            .iter()
            .chain(std::iter::once(&value.embedding_model))
        {
            ensure!(
                !name.is_empty()
                    && name.len() <= 120
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b)),
                "Invalid installed model name"
            );
        }
        Ok(value)
    }

    pub fn key_for(&self, provider: &str) -> Option<&str> {
        match provider {
            "openai" => self.key.as_deref(),
            "openrouter" => self.openrouter_key.as_deref(),
            _ => None,
        }
    }

    pub fn endpoint_for(&self, provider: &str) -> Option<&str> {
        match provider {
            "openai" => Some(&self.endpoint),
            "openrouter" => Some(&self.openrouter_endpoint),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let required = |name| {
            std::env::var(name).with_context(|| format!("{name} is required; use ./scripts/dev.sh"))
        };
        let mut config = Self {
            database_url: required("DATABASE_URL")?,
            owner_username: std::env::var("RECOLLECT_OWNER_USERNAME").unwrap_or("owner".into()),
            owner_password: required("RECOLLECT_OWNER_PASSWORD")?,
            bind: std::env::var("RECOLLECT_BIND").unwrap_or("127.0.0.1:8787".into()),
            public_origin: std::env::var("RECOLLECT_PUBLIC_ORIGIN")
                .unwrap_or("http://127.0.0.1:8787".into()),
            static_dir: std::env::var("RECOLLECT_STATIC_DIR").unwrap_or("web/dist".into()),
            neo4j_url: std::env::var("NEO4J_URL").unwrap_or("http://127.0.0.1:57474".into()),
            neo4j_username: std::env::var("NEO4J_USERNAME").unwrap_or("neo4j".into()),
            neo4j_password: required("NEO4J_PASSWORD")?,
            credential_file: std::env::var("RECOLLECT_CREDENTIAL_FILE")
                .unwrap_or(".data/credentials.json".into()),
            artifact_dir: std::env::var("RECOLLECT_ARTIFACT_DIR")
                .unwrap_or(".data/artifacts".into()),
            erasure_journal: std::env::var("RECOLLECT_ERASURE_JOURNAL")
                .unwrap_or(".data/erasure-journal".into()),
            erasure_mirror: std::env::var("RECOLLECT_ERASURE_MIRROR_CONFIG")
                .ok()
                .filter(|path| !path.trim().is_empty()),
            models: ModelConfig::from_env()?,
            oidc: match (
                std::env::var("RECOLLECT_OIDC_ISSUER").ok(),
                std::env::var("RECOLLECT_OIDC_CLIENT_ID").ok(),
                std::env::var("RECOLLECT_OIDC_CLIENT_SECRET").ok(),
            ) {
                (None, None, None) => None,
                (Some(issuer), Some(client_id), Some(client_secret)) => {
                    let url = reqwest::Url::parse(&issuer).context("Invalid OIDC issuer")?;
                    ensure!(
                        url.scheme() == "https"
                            || (url.scheme() == "http"
                                && matches!(
                                    url.host_str(),
                                    Some("127.0.0.1" | "localhost" | "[::1]")
                                )),
                        "OIDC requires HTTPS except loopback development"
                    );
                    ensure!(
                        !client_id.is_empty()
                            && !client_secret.is_empty()
                            && url.query().is_none()
                            && url.fragment().is_none(),
                        "OIDC issuer and client configuration is invalid"
                    );
                    let scopes = std::env::var("RECOLLECT_OIDC_SCOPES")
                        .unwrap_or("profile groups".into())
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect();
                    Some(OidcConfig {
                        issuer,
                        client_id,
                        client_secret,
                        scopes,
                    })
                }
                _ => anyhow::bail!(
                    "Set all three RECOLLECT_OIDC_ISSUER, RECOLLECT_OIDC_CLIENT_ID and RECOLLECT_OIDC_CLIENT_SECRET or none"
                ),
            },
        };
        ensure!(
            !config.owner_password.is_empty(),
            "RECOLLECT_OWNER_PASSWORD must not be empty"
        );
        ensure!(
            !config.owner_username.trim().is_empty(),
            "RECOLLECT_OWNER_USERNAME must not be empty"
        );
        let origin = reqwest::Url::parse(&config.public_origin)
            .map_err(|_| anyhow::anyhow!("RECOLLECT_PUBLIC_ORIGIN must be an HTTP(S) origin"))?;
        ensure!(
            origin.username().is_empty()
                && origin.password().is_none()
                && origin.query().is_none()
                && origin.fragment().is_none()
                && matches!(origin.path(), "" | "/")
                && (origin.scheme() == "https"
                    || (origin.scheme() == "http"
                        && matches!(origin.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")))),
            "Use an HTTPS public origin, or loopback HTTP, without credentials or a path"
        );
        config.public_origin = origin.origin().ascii_serialization();
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::explicit_model;

    #[test]
    fn the_text_model_has_no_default_and_only_an_explicit_value_names_one() {
        assert_eq!(explicit_model(None), None);
        assert_eq!(explicit_model(Some(String::new())), None);
        assert_eq!(explicit_model(Some("  \t".into())), None);
        assert_eq!(
            explicit_model(Some(" gpt-4.1-mini ".into())).as_deref(),
            Some("gpt-4.1-mini")
        );
    }
}
