//! Reviewed adapter compatibility is separate from account-listed availability.
use super::*;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const CHECKED: &str = "2026-10-05";
const FRESH: Duration = Duration::from_secs(300);
const BACKOFF: Duration = Duration::from_secs(60);
const MAX_BODY: usize = 1_048_576;

#[derive(Default)]
pub struct Cache {
    ids: Option<HashSet<String>>,
    observed_at: Option<DateTime<Utc>>,
    observed: Option<Instant>,
    attempted: Option<Instant>,
    error: Option<&'static str>,
}
pub type SharedCache = std::sync::Arc<Mutex<Cache>>;

struct Supported {
    id: &'static str,
    kind: &'static str,
    dimensions: Option<i32>,
    input: f64,
    output: Option<f64>,
    page: &'static str,
}
const SUPPORTED: &[Supported] = &[
    Supported {
        id: "gpt-5.6-luna",
        kind: "text",
        dimensions: None,
        input: 0.20,
        output: Some(1.20),
        page: "gpt-5.6-luna",
    },
    Supported {
        id: "gpt-4.1-mini",
        kind: "text",
        dimensions: None,
        input: 0.40,
        output: Some(1.60),
        page: "gpt-4.1-mini",
    },
    Supported {
        id: "gpt-4.1-mini-2025-04-14",
        kind: "text",
        dimensions: None,
        input: 0.40,
        output: Some(1.60),
        page: "gpt-4.1-mini",
    },
    Supported {
        id: "gpt-4.1",
        kind: "text",
        dimensions: None,
        input: 2.0,
        output: Some(8.0),
        page: "gpt-4.1",
    },
    Supported {
        id: "gpt-4.1-2025-04-14",
        kind: "text",
        dimensions: None,
        input: 2.0,
        output: Some(8.0),
        page: "gpt-4.1",
    },
    Supported {
        id: "text-embedding-3-large",
        kind: "embedding",
        dimensions: Some(3072),
        input: 0.13,
        output: None,
        page: "text-embedding-3-large",
    },
    Supported {
        id: "text-embedding-3-small",
        kind: "embedding",
        dimensions: Some(1536),
        input: 0.02,
        output: None,
        page: "text-embedding-3-small",
    },
];
pub(crate) fn text_supported(id: &str) -> bool {
    SUPPORTED.iter().any(|m| m.kind == "text" && m.id == id)
}
pub(crate) fn max_dimensions(id: &str) -> Option<i32> {
    SUPPORTED
        .iter()
        .find(|m| m.id == id)
        .and_then(|m| m.dimensions)
}
pub(crate) fn embedding_supported(id: &str, dimensions: i32) -> bool {
    max_dimensions(id).is_some_and(|max| (1..=max).contains(&dimensions))
}
fn fresh(cache: &Cache) -> bool {
    cache.error.is_none() && cache.observed.is_some_and(|at| at.elapsed() < FRESH)
}
fn response(cache: &Cache) -> ModelCatalogue {
    let stale = !fresh(cache);
    let checked = chrono::NaiveDate::parse_from_str(CHECKED, "%Y-%m-%d").unwrap();
    let pricing_stale = (Utc::now().date_naive() - checked).num_days() > 30;
    ModelCatalogue {
        models: SUPPORTED
            .iter()
            .map(|model| {
                let available = cache.ids.as_ref().map(|ids| ids.contains(model.id));
                CatalogueModel {
                    id: model.id.into(),
                    kind: model.kind.into(),
                    available,
                    selectable: !stale && available == Some(true),
                    max_dimensions: model.dimensions,
                    input_usd_per_million: Some(model.input),
                    output_usd_per_million: model.output,
                    cached_input_usd_per_million: match model.page {
                        "gpt-5.6-luna" => Some(0.02),
                        "gpt-4.1-mini" => Some(0.10),
                        "gpt-4.1" => Some(0.50),
                        _ => None,
                    },
                    pricing_tier: "USD / 1M tokens · standard · short context".into(),
                    checked_on: CHECKED.into(),
                    source_url: format!(
                        "https://developers.openai.com/api/docs/models/{}",
                        model.page
                    ),
                    pricing_stale,
                }
            })
            .collect(),
        observed_at: cache.observed_at,
        stale,
        error_code: cache.error.map(String::from),
    }
}
async fn fetch(state: &AppState) -> std::result::Result<HashSet<String>, &'static str> {
    let key = state
        .config
        .models
        .key
        .as_deref()
        .ok_or("model_credentials_missing")?;
    let mut reply = state
        .http
        .get(format!(
            "{}/models",
            state.config.models.endpoint.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|_| "model_catalogue_transport")?;
    if !reply.status().is_success() {
        return Err(if reply.status().as_u16() == 429 {
            "model_catalogue_rate_limited"
        } else {
            "model_catalogue_http"
        });
    }
    if reply
        .content_length()
        .is_some_and(|length| length > MAX_BODY as u64)
    {
        return Err("model_catalogue_body_limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = reply
        .chunk()
        .await
        .map_err(|_| "model_catalogue_transport")?
    {
        if bytes.len() + chunk.len() > MAX_BODY {
            return Err("model_catalogue_body_limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "model_catalogue_shape")?;
    let models = value["data"]
        .as_array()
        .filter(|rows| rows.len() <= 10000)
        .ok_or("model_catalogue_shape")?;
    let mut ids = HashSet::new();
    for model in models {
        let id = model["id"]
            .as_str()
            .filter(|id| identifier(id, 120))
            .ok_or("model_catalogue_shape")?;
        ids.insert(id.into());
    }
    Ok(ids)
}
pub(crate) async fn validate_selection(
    state: &AppState,
    old: &ModelPolicy,
    next: &ModelPolicy,
) -> Result<()> {
    if old.text_model == next.text_model && old.embedding_model == next.embedding_model {
        return Ok(());
    }
    // Legacy installation IDs can remain unchanged. They must never become a
    // new selection merely because configuration and account listing name them.
    if (old.text_model != next.text_model && !text_supported(&next.text_model))
        || (old.embedding_model != next.embedding_model
            && !embedding_supported(&next.embedding_model, next.embedding_dimensions))
    {
        return Err(failure(
            "model_configuration_changed",
            "Choose compatible supported text and embedding models and dimensions.",
        ));
    }
    let cache = state.model_catalogue.lock().await;
    if !fresh(&cache) {
        return Err(failure(
            "model_catalogue_stale",
            "Refresh available models before changing the selected model.",
        ));
    }
    for (old, next) in [
        (&old.text_model, &next.text_model),
        (&old.embedding_model, &next.embedding_model),
    ] {
        if old != next && !cache.ids.as_ref().is_some_and(|ids| ids.contains(next)) {
            return Err(failure(
                "model_unavailable",
                "The selected model was not available in the latest account model list.",
            ));
        }
    }
    Ok(())
}
async fn catalogue(
    state: AppState,
    auth: Auth,
    brain: Uuid,
    refresh: bool,
) -> Result<Json<ModelCatalogue>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, brain, refresh).await?;
    if refresh {
        auth.require_browser()?;
        db::require_writer(&mut tx, brain).await?;
    }
    tx.commit().await?;
    let mut cache = state.model_catalogue.lock().await;
    if refresh && !fresh(&cache) && cache.attempted.is_none_or(|at| at.elapsed() >= BACKOFF) {
        cache.attempted = Some(Instant::now());
        match fetch(&state).await {
            Ok(ids) => {
                cache.ids = Some(ids);
                cache.observed = Some(Instant::now());
                cache.observed_at = Some(Utc::now());
                cache.error = None;
            }
            Err(code) => cache.error = Some(code),
        }
    }
    Ok(Json(response(&cache)))
}

#[utoipa::path(get,path="/api/brains/{brain}/models/catalogue",operation_id="modelCatalogue",params(("brain"=Uuid,Path)),responses((status=200,body=ModelCatalogue)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<ModelCatalogue>> {
    catalogue(state, auth, brain, false).await
}
#[utoipa::path(post,path="/api/brains/{brain}/models/catalogue",operation_id="refreshModelCatalogue",params(("brain"=Uuid,Path)),responses((status=200,body=ModelCatalogue)))]
pub async fn refresh(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
) -> Result<Json<ModelCatalogue>> {
    catalogue(state, auth, brain, true).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_expiry_and_failed_refresh_retain_provenance_without_selectability() {
        let mut cache = Cache {
            ids: Some(["gpt-4.1-mini".to_owned(), "unverified-model".to_owned()].into()),
            observed_at: Some(Utc::now()),
            observed: Some(Instant::now()),
            ..Default::default()
        };
        let verified = response(&cache);
        assert!(!verified.stale);
        assert!(
            verified
                .models
                .iter()
                .any(|m| m.id == "gpt-4.1-mini" && m.selectable)
        );
        assert!(!verified.models.iter().any(|m| m.id == "unverified-model"));
        cache.observed = Some(Instant::now() - FRESH);
        let expired = response(&cache);
        assert!(expired.stale);
        assert!(expired.models.iter().all(|m| !m.selectable));
        assert_eq!(expired.observed_at, verified.observed_at);
        cache.observed = Some(Instant::now());
        cache.error = Some("model_catalogue_rate_limited");
        let failed = response(&cache);
        assert!(failed.stale);
        let retained = failed
            .models
            .iter()
            .find(|m| m.id == "gpt-4.1-mini")
            .unwrap();
        assert_eq!(retained.available, Some(true));
        assert!(!retained.selectable);
        assert_eq!(retained.checked_on, CHECKED);
    }

    #[test]
    fn adapter_compatibility_does_not_infer_capabilities_from_account_ids() {
        assert!(text_supported("gpt-4.1-mini-2025-04-14"));
        assert!(!text_supported("gpt-4.1-nano"));
        assert!(!text_supported("ft:gpt-4.1-mini"));
        assert!(embedding_supported("text-embedding-3-small", 1536));
        assert!(embedding_supported("text-embedding-3-large", 256));
        assert!(!embedding_supported("text-embedding-3-small", 3072));
        assert!(!embedding_supported("text-embedding-3-large", 0));
    }
}
