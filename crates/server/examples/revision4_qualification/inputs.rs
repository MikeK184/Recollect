//! Strict source-only admission; never opens corpus, gold, guide or annotations.
use super::{guard, io};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
pub const FREEZE: &str = "ffc4f4251823bde4060c2d1158482728b4913d955e508a1fef9b8cf0cb6c6431";
pub const SELECTION: &str = "084c075eb5f0a300861e2c95cd6c75ad17506d2c2b49b342070dbab45457056d";
pub const SCORER: &str = "0fc5da6216cf00acb82d4eabb8213f37021e0e00b737587dfe1a7a9917800a94";
pub const PREPARER: &str = "282b1425a96989ce64b0831f0f15c257663d25d477d67c14cf90704203e58f58";
pub const CONTROLS: &str = "d15e0171a691e1656dcd4e3955eca39859c6b73a19b8257402c692e69101cf81";
pub const FAMILIES: [&str; 2] = ["source-facts-4", "source-reconciliation-4"];
pub const MODES: [&str; 5] = [
    "native_hybrid",
    "native_weighted",
    "entity_overlap",
    "metadata",
    "eligible_vector",
];
pub fn parameters() -> Value {
    json!({"neighbors":3,"minimum_cosine":0.65,"vector_scale":0.25})
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub document_id: String,
    pub text_sha256: String,
    pub partition: String,
    pub language: String,
    pub scope: String,
    pub title: String,
    pub text: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub producer: String,
    pub recipe: Value,
    pub recipe_bytes: Vec<u8>,
    pub recipe_sha256: String,
    pub source_feed_sha256: String,
    pub preparation_sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cohort {
    pub name: String,
    pub corpus_bindings: Value,
    pub sources: Vec<Source>,
    pub families: Vec<Family>,
    pub readiness: String,
}
pub struct Admitted {
    pub cohorts: Vec<Cohort>,
    pub pricing: Value,
    pub pricing_sha256: String,
    pub bindings: Value,
    pub model_configuration: Value,
}
pub fn bound(root: &Path, pair: &Value, maximum: usize) -> Result<(PathBuf, Value)> {
    io::fields(pair, &["path", "sha256"])?;
    let p = io::contained(root, pair["path"].as_str().context("Explicit file path")?)?;
    let b = io::read(&p, maximum)?;
    ensure!(
        io::sha(&b)? == io::h(&pair["sha256"])?,
        "Pinned input changed"
    );
    Ok((p, io::strict(&b)?))
}
pub fn code(root: &Path) -> Result<Value> {
    let mut files = BTreeMap::new();
    for dir in [
        "crates/server/examples/revision4_qualification",
        "crates/server/examples/frozen_organization_export",
    ] {
        for f in std::fs::read_dir(root.join(dir))? {
            let p = f?.path();
            ensure!(
                p.is_file() && p.extension().is_some_and(|x| x == "rs" || x == "py"),
                "Unexpected transitive source"
            );
            files.insert(
                p.strip_prefix(root)?.to_string_lossy().to_string(),
                io::hash_file(&p)?,
            );
        }
    }
    for p in [
        "crates/server/examples/revision4_qualification.rs",
        "crates/server/examples/frozen_organization_export.rs",
        "Cargo.toml",
        "Cargo.lock",
        "crates/server/Cargo.toml",
        "crates/protocol/Cargo.toml",
        "crates/agent/Cargo.toml",
        "crates/mcp-runtime/Cargo.toml",
        "scripts/frozen_source_corpus.py",
        "scripts/score_frozen_source_corpus.py",
        "scripts/prepare_frozen_organization.py",
        "scripts/docker.sh",
        "compose.yaml",
        "infra/product/compose.yaml",
    ] {
        files.insert(p.into(), io::hash_file(&root.join(p))?);
    }
    Ok(json!(files))
}
pub fn pins(root: &Path, freeze: &Value, selection: &Value) -> Result<()> {
    ensure!(
        freeze["grouping"]["parameters"] == parameters()
            && selection["parameters"] == parameters()
            && selection["selected"] == true
            && freeze["grouping"]["arms"] == json!(MODES),
        "Frozen candidate/selection tuple differs"
    );
    for (k, n) in [("source_sha256", 198), ("migration_sha256", 64)] {
        let t = freeze[k].as_object().context("Product pins")?;
        ensure!(t.len() == n, "Product pin denominator differs");
        for (p, h) in t {
            ensure!(
                io::hash_file(&io::contained(root, p)?)? == io::h(h)?,
                "Product source drift"
            );
        }
    }
    ensure!(
        io::hash_file(&root.join("scripts/score_frozen_source_corpus.py"))? == SCORER
            && io::hash_file(&root.join("scripts/prepare_frozen_organization.py"))? == PREPARER,
        "Reviewed scripts drift"
    );
    ensure!(
        io::hash_file(
            &root.join("docs/research/automatic-reconciliation-native-controls-v3-2026-10-18.json")
        )? == CONTROLS,
        "Original six controls drift"
    );
    Ok(())
}
pub fn admit(root: &Path, path: &Path, hash: &str) -> Result<Admitted> {
    admit_validated(root, path, hash, true)
}
pub fn admit_frozen(root: &Path, path: &Path, hash: &str) -> Result<Admitted> {
    admit_validated(root, path, hash, false)
}
fn admit_validated(root: &Path, path: &Path, hash: &str, fresh_prices: bool) -> Result<Admitted> {
    let raw = io::read(path, 65536)?;
    ensure!(io::sha(&raw)? == hash, "Input manifest hash differs");
    let m = io::strict(&raw)?;
    io::fields(
        &m,
        &[
            "schema",
            "candidate_freeze",
            "development_selection",
            "pricing",
            "cohorts",
        ],
    )?;
    ensure!(
        m["schema"] == "recollect-revision4-campaign-inputs-1"
            && m["candidate_freeze"]["sha256"] == FREEZE
            && m["development_selection"]["sha256"] == SELECTION,
        "Campaign input identity differs"
    );
    let (_, freeze) = bound(root, &m["candidate_freeze"], 256 * 1024)?;
    let (_, selection) = bound(root, &m["development_selection"], 256 * 1024)?;
    pins(root, &freeze, &selection)?;
    let (_, pricing) = bound(root, &m["pricing"], 65536)?;
    let time = if fresh_prices {
        chrono::Utc::now()
    } else {
        let text = chrono::DateTime::parse_from_rfc3339(
            pricing["text"]["verified_at"]
                .as_str()
                .context("Price time")?,
        )?
        .with_timezone(&chrono::Utc);
        let embedding = chrono::DateTime::parse_from_rfc3339(
            pricing["embedding"]["verified_at"]
                .as_str()
                .context("Price time")?,
        )?
        .with_timezone(&chrono::Utc);
        text.max(embedding)
    };
    guard::price(&pricing, time)?;
    let receipt_path = root.join(
        ".cache/automatic-mapping/revision4-source-feeds-2026-10-18/preparation-receipt.json",
    );
    ensure!(
        io::hash_file(&receipt_path)?
            == "904409f14ab49b6a491d643dfbc2fd4a01bfad90c1d4b8e84a0461b81c55c0bf",
        "Reviewed actual preparation receipt drift"
    );
    let receipt = io::read_json(&receipt_path, 65536)?;
    let cs = m["cohorts"].as_array().context("Two explicit cohorts")?;
    ensure!(cs.len() == 2, "Cohort count differs");
    let mut cohorts = vec![];
    let mut names = BTreeSet::new();
    let mut input_bindings = vec![];
    for c in cs {
        io::fields(c, &["name", "manifest", "families"])?;
        let name = c["name"].as_str().context("Cohort name")?;
        ensure!(
            ["private24", "prospective22"].contains(&name) && names.insert(name),
            "Cohort names differ"
        );
        io::fields(&c["manifest"], &["path", "sha256"])?;
        let manifest_path = io::contained(
            root,
            c["manifest"]["path"].as_str().context("Manifest path")?,
        )?;
        let manifest_hash = io::h(&c["manifest"]["sha256"])?;
        ensure!(
            io::hash_file(&manifest_path)? == manifest_hash,
            "Content-free manifest binding drift"
        ); // Manifest/gold content stays unopened; reviewed preparation binds it.
        let fs = c["families"].as_array().context("Both current producers")?;
        ensure!(fs.len() == 2, "Family count differs");
        let mut families = vec![];
        let mut expected_sources = None;
        let mut corpus = None;
        let mut readiness = None;
        let mut producers = BTreeSet::new();
        for f in fs {
            io::fields(f, &["producer", "recipe", "source_feed", "preparation"])?;
            let producer = f["producer"].as_str().context("Producer")?;
            ensure!(
                FAMILIES.contains(&producer) && producers.insert(producer),
                "Producer set differs"
            );
            let (recipe_path, recipe) = bound(root, &f["recipe"], 16384)?;
            let (_, feed) = bound(root, &f["source_feed"], 96 * 1024 * 1024)?;
            let (_, prep) = bound(root, &f["preparation"], 256 * 1024)?;
            let reviewed = receipt["outputs"]
                .as_array()
                .context("Reviewed feeds")?
                .iter()
                .find(|x| x["cohort"] == name && x["producer"] == producer)
                .context("Reviewed feed missing")?;
            for (key, rk) in [
                ("recipe", "recipe_sha256"),
                ("source_feed", "source_feed_sha256"),
                ("preparation", "preparation_sha256"),
            ] {
                ensure!(
                    f[key]["sha256"] == reviewed[rk],
                    "Unreviewed preparation artifact"
                );
            }
            ensure!(
                reviewed["manifest_sha256"] == manifest_hash,
                "Reviewed cohort manifest differs"
            );
            validate_recipe(&recipe, producer)?;
            io::fields(
                &feed,
                &["schema", "corpus_bindings", "recipe_sha256", "sources"],
            )?;
            ensure!(
                feed["schema"] == "recollect-native-organization-source-feed-1"
                    && feed["recipe_sha256"] == f["recipe"]["sha256"],
                "Source feed recipe differs"
            );
            let sources: Vec<Source> = serde_json::from_value(feed["sources"].clone())?;
            let expected = if name == "private24" { 24 } else { 22 };
            ensure!(
                sources.len() == expected,
                "Full source denominator required"
            );
            let mut ids = BTreeSet::new();
            for s in &sources {
                ensure!(
                    !s.document_id.is_empty()
                        && s.document_id.len() <= 256
                        && ids.insert(&s.document_id)
                        && s.text.len() <= 2 * 1024 * 1024
                        && s.title.len() <= 16384
                        && io::sha(s.text.as_bytes())? == s.text_sha256
                        && s.partition == "held_out",
                    "Source identity/hash/count differs"
                );
                let parts: Vec<_> = s.scope.split('/').collect();
                ensure!(
                    parts.len() == 3
                        && parts.iter().all(|p| !p.is_empty()
                            && p.chars()
                                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))),
                    "Scope declaration invalid"
                );
            }
            io::fields(
                &prep,
                &[
                    "schema",
                    "input_sha256",
                    "output_sha256",
                    "counts",
                    "complete_topic_gold_readiness",
                    "not_measured",
                ],
            )?;
            ensure!(
                prep["schema"] == "recollect-native-organization-preparation-1"
                    && prep["input_sha256"]["candidate_freeze"] == FREEZE
                    && prep["input_sha256"]["development_selection"] == SELECTION,
                "Preparation candidate differs"
            );
            if let Some(e) = &expected_sources {
                ensure!(
                    &feed["sources"] == e,
                    "Both families must share exact source-only content"
                );
            } else {
                expected_sources = Some(feed["sources"].clone());
            }
            if let Some(e) = &corpus {
                ensure!(&feed["corpus_bindings"] == e, "Family corpus differs");
            } else {
                corpus = Some(feed["corpus_bindings"].clone());
            }
            readiness = Some(
                prep["complete_topic_gold_readiness"]
                    .as_str()
                    .context("Readiness")?
                    .to_owned(),
            );
            families.push(Family {
                producer: producer.into(),
                recipe_bytes: io::read(&recipe_path, 16384)?,
                recipe,
                recipe_sha256: io::h(&f["recipe"]["sha256"])?.into(),
                source_feed_sha256: io::h(&f["source_feed"]["sha256"])?.into(),
                preparation_sha256: io::h(&f["preparation"]["sha256"])?.into(),
            });
            input_bindings.push(f.clone());
        }
        let sources = serde_json::from_value(expected_sources.context("Sources missing")?)?;
        cohorts.push(Cohort {
            name: name.into(),
            corpus_bindings: corpus.context("Corpus hashes")?,
            sources,
            families,
            readiness: readiness.context("Readiness missing")?,
        });
    }
    Ok(Admitted {
        cohorts,
        pricing,
        pricing_sha256: io::h(&m["pricing"]["sha256"])?.into(),
        bindings: json!({"reviewed_cohort_counts":receipt["outputs"].as_array().unwrap().iter().map(|r|(r["cohort"].as_str().unwrap().to_owned(),r["counts"].clone())).collect::<BTreeMap<_,_>>(),"manifest":hash,"manifest_file":{"path":path.strip_prefix(root)?.to_string_lossy(),"sha256":hash},"candidate_freeze_file":m["candidate_freeze"],"development_selection_file":m["development_selection"],"candidate_freeze":FREEZE,"development_selection":SELECTION,"families":input_bindings,"code":code(root)?}),
        model_configuration: freeze["model_configuration"].clone(),
    })
}
pub fn validate_recipe(r: &Value, producer: &str) -> Result<()> {
    io::fields(
        r,
        &[
            "schema",
            "candidate_freeze_sha256",
            "development_selection_sha256",
            "producer",
            "parameters",
            "modes",
        ],
    )?;
    io::fields(
        &r["producer"],
        &[
            "prompt_label",
            "schema_label",
            "adapter_revision",
            "mapping_schema",
        ],
    )?;
    ensure!(
        r["schema"] == "recollect-offline-organization-recipe-1"
            && r["candidate_freeze_sha256"] == FREEZE
            && r["development_selection_sha256"] == SELECTION
            && r["producer"]["prompt_label"] == producer
            && r["producer"]["schema_label"] == producer
            && r["producer"]["adapter_revision"] == "learning-quote-anchors-3"
            && r["producer"]["mapping_schema"] == "source-mentions-1"
            && r["parameters"] == parameters()
            && r["modes"] == json!(MODES),
        "Recipe differs"
    );
    Ok(())
}
