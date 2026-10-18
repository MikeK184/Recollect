//! Sole disclosed provider-body transformation; canonical worker bytes stay intact.
use super::io;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub fn rule() -> Value {
    json!({"schema":"recollect-revision4-provider-transform-1","responses":"require absent service_tier; add only service_tier=default","embeddings":"byte_identity","maximum_provider_bytes":65536})
}
pub fn provider(canonical: &[u8], embedding: bool) -> Result<Vec<u8>> {
    ensure!(canonical.len() <= 65536, "Canonical request bound");
    let mut value = io::strict(canonical)?;
    let object = value.as_object_mut().context("Canonical request object")?;
    ensure!(
        !object.contains_key("service_tier"),
        "Preexisting service tier refused"
    );
    if embedding {
        return Ok(canonical.to_vec());
    }
    object.insert("service_tier".into(), json!("default"));
    let bytes = serde_json::to_vec(&value)?;
    ensure!(bytes.len() <= 65536, "Transformed request bound");
    let mut recovered = io::strict(&bytes)?;
    ensure!(
        recovered
            .as_object_mut()
            .context("Provider object")?
            .remove("service_tier")
            == Some(json!("default"))
            && recovered == io::strict(canonical)?,
        "Add-only transport transform"
    );
    Ok(bytes)
}
fn sibling(path: &Path, prefix: &str) -> Result<PathBuf> {
    Ok(path.with_file_name(format!(
        "{prefix}{}",
        path.file_name()
            .context("Body filename")?
            .to_str()
            .context("Body filename UTF8")?
    )))
}
pub fn binding(canonical_path: &Path, canonical: &[u8], embedding: bool) -> Result<Value> {
    let provider = provider(canonical, embedding)?;
    Ok(
        json!({"rule":rule(),"rule_sha256":io::value_hash(&rule())?,"canonical_path":canonical_path,"canonical_sha256":io::sha(canonical)?,"canonical_bytes":canonical.len(),"provider_path":sibling(canonical_path,"provider-")?,"provider_sha256":io::sha(&provider)?,"provider_bytes":provider.len()}),
    )
}
pub fn freeze(path: &Path, canonical: &[u8], embedding: bool) -> Result<()> {
    let provider = provider(canonical, embedding)?;
    io::write(path, canonical)?;
    io::write(&sibling(path, "provider-")?, &provider)?;
    io::json(
        &sibling(path, "binding-")?,
        &binding(
            Path::new(path.file_name().context("Body filename")?),
            canonical,
            embedding,
        )?,
    )
}
pub fn verify(path: &Path, canonical: &[u8], embedding: bool) -> Result<()> {
    ensure!(
        io::read(path, 65536)? == canonical
            && io::read(&sibling(path, "provider-")?, 65536)? == provider(canonical, embedding)?
            && io::read_json(&sibling(path, "binding-")?, 65536)?
                == binding(
                    Path::new(path.file_name().context("Body filename")?),
                    canonical,
                    embedding
                )?,
        "Dual frozen body drift"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_add_only_and_embedding_identity() {
        let canonical = br#"{ "model":"gpt-5.6-luna", "input":[{"role":"user","content":"toy"}]}"#;
        let transmitted = provider(canonical, false).unwrap();
        let mut recovered = io::strict(&transmitted).unwrap();
        assert_eq!(
            recovered.as_object_mut().unwrap().remove("service_tier"),
            Some(json!("default"))
        );
        assert_eq!(recovered, io::strict(canonical).unwrap());
        assert_eq!(provider(canonical, true).unwrap(), canonical);
        for tier in ["default", "auto", "priority", "flex", "scale", "fast"] {
            let b = serde_json::to_vec(&json!({"service_tier":tier})).unwrap();
            assert!(provider(&b, false).is_err());
            assert!(provider(&b, true).is_err());
        }
        let b = serde_json::to_vec(&json!({"x":"x".repeat(65520)})).unwrap();
        assert!(b.len() < 65536 && provider(&b, false).is_err());
    }
    #[test]
    fn returned_tier_is_required_after_full_charge() {
        let healthy = json!({"service_tier":"default","model":"gpt-5.6-luna","status":"completed","output":[{"type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"not candidate JSON"}]}],"usage":{"input_tokens":10,"output_tokens":10,"total_tokens":20}});
        assert!(super::super::guard::response_ok(&healthy, false));
        let mut absent = healthy.clone();
        absent.as_object_mut().unwrap().remove("service_tier");
        assert!(!super::super::guard::response_ok(&absent, false));
        for tier in ["auto", "priority", "fast", "flex", "scale"] {
            let mut wrong = healthy.clone();
            wrong["service_tier"] = json!(tier);
            assert!(!super::super::guard::response_ok(&wrong, false));
        }
    }
}
