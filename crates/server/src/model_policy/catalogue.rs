//! Reviewed adapter compatibility is separate from account-listed availability.
use super::*;
use crate::db::admission::{self, Admitted, Archived, BrainAccess};
use std::{
    collections::{HashMap, HashSet},
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
pub type SharedCache = std::sync::Arc<Mutex<HashMap<String, Cache>>>;

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
const OPENROUTER: &[Supported] = &[
    Supported {
        id: "openai/gpt-6-luna",
        kind: "text",
        dimensions: None,
        input: 0.10,
        output: Some(0.50),
        page: "openai/gpt-6-luna",
    },
    Supported {
        id: "meta/muse-spark-1.3-contributor",
        kind: "text",
        dimensions: None,
        input: 0.10,
        output: Some(0.20),
        page: "meta/muse-spark-1.3-contributor",
    },
    Supported {
        id: "z-ai/glm-5.3-flash",
        kind: "text",
        dimensions: None,
        input: 0.15,
        output: Some(0.50),
        page: "z-ai/glm-5.3-flash",
    },
    Supported {
        id: "z-ai/glm-4.7-flash",
        kind: "text",
        dimensions: None,
        input: 0.0605,
        output: Some(0.40),
        page: "z-ai/glm-4.7-flash",
    },
    Supported {
        id: "qwen/qwen3-embedding-8b",
        kind: "embedding",
        dimensions: Some(4096),
        input: 0.01,
        output: None,
        page: "qwen/qwen3-embedding-8b",
    },
];
fn supported(provider: &str) -> &'static [Supported] {
    match provider {
        "openai" => SUPPORTED,
        "openrouter" => OPENROUTER,
        _ => &[],
    }
}
pub(crate) fn text_supported_for(provider: &str, id: &str) -> bool {
    supported(provider)
        .iter()
        .any(|m| m.kind == "text" && m.id == id)
}
pub(crate) fn embedding_supported_for(provider: &str, id: &str, dimensions: i32) -> bool {
    supported(provider).iter().any(|m| {
        m.kind == "embedding"
            && m.id == id
            && m.dimensions
                .is_some_and(|max| (1..=max).contains(&dimensions))
            && (provider != "openrouter" || dimensions >= 32)
    })
}
pub(crate) fn returned_model_matches(
    provider: &str,
    expected: &str,
    returned: &str,
    embedding: bool,
) -> bool {
    returned == expected
        || (provider == "openai" && !embedding && returned.starts_with(&format!("{expected}-")))
        || (provider == "openrouter"
            && expected == "qwen/qwen3-embedding-8b"
            && returned == "Qwen/Qwen3-Embedding-8B")
}
pub(crate) fn text_supported(id: &str) -> bool {
    SUPPORTED.iter().any(|m| m.kind == "text" && m.id == id)
}
pub(crate) fn max_dimensions(id: &str) -> Option<i32> {
    SUPPORTED
        .iter()
        .chain(OPENROUTER.iter())
        .find(|m| m.id == id)
        .and_then(|m| m.dimensions)
}
pub(crate) fn embedding_supported(id: &str, dimensions: i32) -> bool {
    embedding_supported_for("openai", id, dimensions)
}
fn fresh(cache: &Cache) -> bool {
    cache.error.is_none() && cache.observed.is_some_and(|at| at.elapsed() < FRESH)
}
fn response(cache: &Cache, provider: &str) -> ModelCatalogue {
    let checked_on = if provider == "openrouter" {
        "2026-10-08"
    } else {
        CHECKED
    };
    let stale = !fresh(cache);
    let checked = chrono::NaiveDate::parse_from_str(checked_on, "%Y-%m-%d").unwrap();
    let pricing_stale = (Utc::now().date_naive() - checked).num_days() > 30;
    ModelCatalogue {
        models: supported(provider)
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
                        "openai/gpt-6-luna" => Some(0.01),
                        "gpt-5.6-luna" => Some(0.02),
                        "gpt-4.1-mini" => Some(0.10),
                        "gpt-4.1" => Some(0.50),
                        _ => None,
                    },
                    pricing_tier: "USD / 1M tokens · standard · short context".into(),
                    checked_on: checked_on.into(),
                    source_url: if provider == "openrouter" {
                        format!("https://openrouter.ai/{}", model.page)
                    } else {
                        format!(
                            "https://developers.openai.com/api/docs/models/{}",
                            model.page
                        )
                    },
                    pricing_stale,
                }
            })
            .collect(),
        observed_at: cache.observed_at,
        stale,
        error_code: cache.error.map(String::from),
    }
}
async fn fetch_ids(
    state: &AppState,
    provider: &str,
    suffix: &str,
) -> std::result::Result<HashSet<String>, &'static str> {
    let key = state
        .config
        .models
        .key_for(provider)
        .ok_or("model_credentials_missing")?;
    let mut reply = state
        .http
        .get(format!(
            "{}/{suffix}",
            state
                .config
                .models
                .endpoint_for(provider)
                .ok_or("model_configuration_changed")?
                .trim_end_matches('/')
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
        let id = model["id"].as_str().ok_or("model_catalogue_shape")?;
        // OpenRouter also lists dynamic ~vendor/model-latest aliases. They
        // cannot be selected by this reviewed catalogue; do not let those
        // unrelated aliases invalidate discovery of the pinned model IDs.
        if provider == "openrouter" && id.strip_prefix('~').is_some_and(|id| identifier(id, 120)) {
            continue;
        }
        if !identifier(id, 120) {
            return Err("model_catalogue_shape");
        }
        ids.insert(id.into());
    }
    Ok(ids)
}
async fn fetch(
    state: &AppState,
    provider: &str,
) -> std::result::Result<HashSet<String>, &'static str> {
    let mut ids = fetch_ids(state, provider, "models").await?;
    if provider == "openrouter" {
        ids.extend(fetch_ids(state, provider, "embeddings/models").await?);
    }
    Ok(ids)
}
pub(crate) async fn validate_selection(
    state: &AppState,
    old: &ModelPolicy,
    next: &ModelPolicy,
) -> Result<()> {
    // An unselected text model (valid only for a disabled policy) has nothing
    // to check; choosing one later is a change like any other.
    let text_selected = !next.text_model.is_empty();
    let provider_changed = old.provider != next.provider;
    let text_changed = text_selected && (provider_changed || old.text_model != next.text_model);
    let embedding_changed = provider_changed || old.embedding_model != next.embedding_model;
    if !text_changed && !embedding_changed {
        return Ok(());
    }
    // Legacy installation IDs can remain unchanged. They must never become a
    // new selection merely because configuration and account listing name them.
    if (text_changed && !text_supported_for(&next.provider, &next.text_model))
        || (embedding_changed
            && !embedding_supported_for(
                &next.provider,
                &next.embedding_model,
                next.embedding_dimensions,
            ))
    {
        return Err(failure(
            "model_configuration_changed",
            "Choose compatible supported text and embedding models and dimensions.",
        ));
    }
    let caches = state.model_catalogue.lock().await;
    let cache = caches.get(&next.provider);
    if cache.is_none_or(|c| !fresh(c)) {
        return Err(failure(
            "model_catalogue_stale",
            "Refresh available models before changing the selected model.",
        ));
    }
    for (old, next) in [
        (&old.text_model, &next.text_model),
        (&old.embedding_model, &next.embedding_model),
    ] {
        if next.is_empty() {
            continue;
        }
        if (old != next || provider_changed)
            && !cache
                .unwrap()
                .ids
                .as_ref()
                .is_some_and(|ids| ids.contains(next))
        {
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
    query: CatalogueQuery,
) -> Result<Json<ModelCatalogue>> {
    // A refresh needs an administrator under the exclusive lock; the browser
    // check and the archived refusal follow admission, in that order.
    let access = if refresh {
        BrainAccess::admin(Archived::Allow)
    } else {
        BrainAccess::READ
    };
    let Admitted {
        mut tx, archived, ..
    } = admission::brain_tx(&state.pool, &auth, brain, access).await?;
    if refresh {
        auth.require_browser()?;
        if archived {
            return Err(admission::evidence_archived());
        }
    }
    let provider = query
        .provider
        .unwrap_or(current(&state, &mut tx, brain).await?.policy.provider);
    if !matches!(provider.as_str(), "openai" | "openrouter") {
        return Err(Error::invalid("Choose a supported provider."));
    }
    tx.commit().await?;
    if refresh {
        refresh_cache(&state.model_catalogue, &provider, || {
            fetch(&state, &provider)
        })
        .await;
    }
    let mut caches = state.model_catalogue.lock().await;
    Ok(Json(response(
        caches.entry(provider.clone()).or_default(),
        &provider,
    )))
}

/// The cache mutex guards map access only and is never held across the two
/// outbound fetches: readers, and a policy update holding the exclusive Brain
/// lock, must not wait out a slow provider. Recording the attempt before the
/// fetch is the refresh guard; a concurrent refresh sees it and does not fetch.
async fn refresh_cache<F>(caches: &SharedCache, provider: &str, fetch: impl FnOnce() -> F)
where
    F: std::future::Future<Output = std::result::Result<HashSet<String>, &'static str>>,
{
    {
        let mut caches = caches.lock().await;
        let cache = caches.entry(provider.to_owned()).or_default();
        if fresh(cache) || cache.attempted.is_some_and(|at| at.elapsed() < BACKOFF) {
            return;
        }
        cache.attempted = Some(Instant::now());
    }
    let fetched = fetch().await;
    let mut caches = caches.lock().await;
    let cache = caches.entry(provider.to_owned()).or_default();
    match fetched {
        Ok(ids) => {
            cache.ids = Some(ids);
            cache.observed = Some(Instant::now());
            cache.observed_at = Some(Utc::now());
            cache.error = None;
        }
        Err(code) => cache.error = Some(code),
    }
}

#[derive(Default, Deserialize)]
pub struct CatalogueQuery {
    pub provider: Option<String>,
}
#[utoipa::path(get,path="/api/brains/{brain}/models/catalogue",operation_id="modelCatalogue",params(("brain"=Uuid,Path),("provider"=Option<String>,Query)),responses((status=200,body=ModelCatalogue)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<ModelCatalogue>> {
    catalogue(state, auth, brain, false, query).await
}
#[utoipa::path(post,path="/api/brains/{brain}/models/catalogue",operation_id="refreshModelCatalogue",params(("brain"=Uuid,Path),("provider"=Option<String>,Query)),responses((status=200,body=ModelCatalogue)))]
pub async fn refresh(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<ModelCatalogue>> {
    catalogue(state, auth, brain, true, query).await
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
        let verified = response(&cache, "openai");
        assert!(!verified.stale);
        assert!(
            verified
                .models
                .iter()
                .any(|m| m.id == "gpt-4.1-mini" && m.selectable)
        );
        assert!(!verified.models.iter().any(|m| m.id == "unverified-model"));
        cache.observed = Some(Instant::now() - FRESH);
        let expired = response(&cache, "openai");
        assert!(expired.stale);
        assert!(expired.models.iter().all(|m| !m.selectable));
        assert_eq!(expired.observed_at, verified.observed_at);
        cache.observed = Some(Instant::now());
        cache.error = Some("model_catalogue_rate_limited");
        let failed = response(&cache, "openai");
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

    #[tokio::test]
    async fn refresh_does_not_hold_the_cache_mutex_across_the_fetch() {
        let caches = SharedCache::default();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        refresh_cache(&caches, "openai", || async {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            // Readers and a concurrent refresh proceed while the fetch is in flight.
            let mut readers = caches.try_lock().expect("mutex free during fetch");
            assert!(readers.get("openai").unwrap().attempted.is_some());
            readers.entry("other".into()).or_default();
            drop(readers);
            refresh_cache(&caches, "openai", || async {
                panic!("a second refresh must not fetch inside the backoff")
            })
            .await;
            Ok(["gpt-4.1-mini".to_owned()].into())
        })
        .await;
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let held = caches.lock().await;
        let cache = held.get("openai").unwrap();
        assert!(fresh(cache) && cache.error.is_none());
        assert!(cache.ids.as_ref().unwrap().contains("gpt-4.1-mini"));
    }

    #[tokio::test]
    async fn failed_refresh_records_its_error_and_keeps_the_backoff() {
        let caches = SharedCache::default();
        refresh_cache(&caches, "openai", || async {
            Err("model_catalogue_unavailable")
        })
        .await;
        refresh_cache(&caches, "openai", || async {
            panic!("retry inside the backoff")
        })
        .await;
        let held = caches.lock().await;
        let cache = held.get("openai").unwrap();
        assert_eq!(cache.error, Some("model_catalogue_unavailable"));
        assert!(!fresh(cache));
    }
}
