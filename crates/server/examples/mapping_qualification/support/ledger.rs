//! One fsynced common campaign budget. An uncertain attempt cannot be replayed.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    SourceFacts,
    Reconciliation,
    Embedding,
}
impl Arm {
    pub fn name(self) -> &'static str {
        match self {
            Self::SourceFacts => "source_facts",
            Self::Reconciliation => "reconciliation",
            Self::Embedding => "embedding",
        }
    }
}
pub fn nano_usd(value: &str) -> Result<u64> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    ensure!(
        !whole.is_empty()
            && whole.bytes().all(|b| b.is_ascii_digit())
            && fraction.len() <= 9
            && fraction.bytes().all(|b| b.is_ascii_digit()),
        "Budget must be a nonnegative decimal USD string with at most9decimal places"
    );
    let w: u64 = whole.parse()?;
    let f: u64 = format!("{fraction:0<9}").parse()?;
    w.checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(f))
        .context("Budget overflow")
}
pub fn validate_approval(approval: &Value, recipe: &Value, hash: &str) -> Result<u64> {
    let expected_kind = recipe["campaign_kind"]
        .as_str()
        .unwrap_or("combined-source-facts-reconciliation-vectors-1");
    ensure!(
        [
            "combined-source-facts-reconciliation-vectors-1",
            "remaining-reconciliation-vectors-1"
        ]
        .contains(&expected_kind)
            && approval["approve_paid_calls"] == true
            && approval["campaign_kind"] == expected_kind,
        "New explicit approval for the exact campaign kind required"
    );
    if expected_kind == "remaining-reconciliation-vectors-1" {
        ensure!(
            recipe["limits"]["source_facts_requests"] == 0
                && approval["maximum_source_facts_requests"] == 0
                && approval["reused_source_facts_sha256"]
                    == recipe["reused_source_facts"]["artifact_sha256"]
                && recipe["reused_source_facts"]["artifact_sha256"]["recipe.json"]
                    .as_str()
                    .is_some_and(|s| s.len() == 64)
                && recipe["reused_source_facts"]["artifact_sha256"]["outcomes.jsonl"]
                    .as_str()
                    .is_some_and(|s| s.len() == 64)
                && recipe["reused_source_facts"]["artifact_sha256"]["execution-receipt.json"]
                    .as_str()
                    .is_some_and(|s| s.len() == 64),
            "Remaining approval must bind immutable completed60 evidence and zero repeated source-facts calls"
        );
    }
    ensure!(
        approval["recipe_sha256"] == hash
            && approval["text_pricing_sha256"] == recipe["source_facts"]["pricing_sha256"]
            && approval["embedding_pricing_sha256"] == recipe["optional_vectors"]["pricing_sha256"]
            && approval["control_sha256"] == recipe["reconciliation"]["controls_sha256"]
            && approval["chunk_manifest_sha256"]
                == recipe["optional_vectors"]["source_chunk_manifest_sha256"],
        "Approval must bind exact current recipe, both prices, the reviewed controls and every complete chunk"
    );
    ensure!(
        approval["maximum_requests"] == recipe["limits"]["requests"]
            && approval["maximum_text_requests"] == recipe["limits"]["text_requests"]
            && approval["maximum_embedding_requests"] == recipe["limits"]["embedding_requests"]
            && approval["maximum_output_tokens"] == 4096,
        "Combined campaign request/token bounds differ"
    );
    let reference = approval["authorization_reference"]
        .as_str()
        .context("Human authorization reference missing")?;
    ensure!(
        !reference.trim().is_empty() && reference != "UNAPPROVED",
        "Explicit human authorization missing"
    );
    let budget = nano_usd(
        approval["budget_usd"]
            .as_str()
            .context("Explicit budget missing")?,
    )?;
    ensure!(
        budget
            >= recipe["limits"]["total_nano_usd"]
                .as_u64()
                .context("Frozen combined maximum missing")?,
        "Approved budget is below frozen worst case"
    );
    Ok(budget)
}
fn create(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}
fn record(file: &mut File, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
pub struct Ledger {
    file: File,
    budget: u64,
    reserved: u64,
    counts: [u64; 3],
    prices: [u64; 3],
    caps: [u64; 3],
    keys: BTreeSet<String>,
    inflight: Option<(String, Uuid)>,
    stopped: bool,
    price_deadline: chrono::DateTime<chrono::Utc>,
}
impl Ledger {
    pub fn begin(
        out: &Path,
        recipe: &Value,
        recipe_hash: &str,
        approval: &Value,
        budget: u64,
    ) -> Result<Self> {
        let mut deadlines = vec![];
        for price in [
            &recipe["source_facts"]["pricing"],
            &recipe["optional_vectors"]["pricing"],
        ] {
            let verified = chrono::DateTime::parse_from_rfc3339(
                price["verified_at"]
                    .as_str()
                    .context("Frozen price verification time")?,
            )?
            .with_timezone(&chrono::Utc);
            let hours = price["valid_for_hours"]
                .as_i64()
                .context("Frozen price validity")?;
            ensure!(
                (1..=24).contains(&hours),
                "Price validity exceeds reviewed boundary"
            );
            deadlines.push(verified + chrono::Duration::hours(hours));
        }
        let price_deadline = *deadlines.iter().min().unwrap();
        ensure!(
            chrono::Utc::now() < price_deadline,
            "Pricing expired before campaign setup"
        );
        // Create-new lock is deliberately first, before DB ownership/setup/network.
        let mut lock = create(&out.join("execution-lock.json"))?;
        record(
            &mut lock,
            &json!({"recipe_sha256":recipe_hash,"approval":approval,"started_at":chrono::Utc::now(),"resumption":"forbidden"}),
        )?;
        File::open(out)?.sync_all()?;
        let mut file = create(&out.join("cost-ledger.jsonl"))?;
        record(
            &mut file,
            &json!({"event":"campaign_started","recipe_sha256":recipe_hash,"budget_nano_usd":budget,"maximum_requests":recipe["limits"]["requests"]}),
        )?;
        File::open(out)?.sync_all()?;
        let caps = [
            recipe["limits"]["source_facts_requests"]
                .as_u64()
                .context("Missing source facts request count")?,
            recipe["limits"]["reconciliation_requests"]
                .as_u64()
                .context("Missing reconciliation request count")?,
            recipe["limits"]["embedding_requests"]
                .as_u64()
                .context("Missing embedding request count")?,
        ];
        ensure!(
            caps[1] > 0
                && caps[2] > 0
                && (caps[0] > 0 || recipe["campaign_kind"] == "remaining-reconciliation-vectors-1"),
            "Only the declared remaining campaign may have zero source-facts calls"
        );
        let text = recipe["limits"]["reconciliation_nano_usd"]
            .as_u64()
            .context("Missing text cost")?
            / caps[1];
        let embed = recipe["limits"]["optional_vectors_nano_usd"]
            .as_u64()
            .context("Missing embedding cost")?
            / caps[2];
        ensure!(
            text * caps[0] == recipe["limits"]["source_facts_nano_usd"]
                && embed * caps[2] == recipe["limits"]["optional_vectors_nano_usd"]
                && text * caps[1] == recipe["limits"]["reconciliation_nano_usd"]
                && caps[0] + caps[1] == recipe["limits"]["text_requests"]
                && caps.iter().sum::<u64>() == recipe["limits"]["requests"]
                && text * (caps[0] + caps[1]) + embed * caps[2]
                    == recipe["limits"]["total_nano_usd"],
            "Per-call cost division must be exact"
        );
        Ok(Self {
            file,
            budget,
            reserved: 0,
            counts: [0; 3],
            prices: [text, text, embed],
            caps,
            keys: BTreeSet::new(),
            inflight: None,
            stopped: false,
            price_deadline,
        })
    }
    pub fn reserve(
        &mut self,
        arm: Arm,
        key: &str,
        operation: Uuid,
        preflight: Value,
    ) -> Result<()> {
        ensure!(
            !self.stopped && self.inflight.is_none(),
            "Campaign stopped or an attempt remains uncertain; no automatic replay"
        );
        if chrono::Utc::now() >= self.price_deadline {
            self.stopped = true;
            anyhow::bail!(
                "Frozen official pricing expired before request; no further call permitted"
            );
        }
        let slot = match arm {
            Arm::SourceFacts => 0,
            Arm::Reconciliation => 1,
            Arm::Embedding => 2,
        };
        let cap = self.caps[slot];
        ensure!(
            self.counts[slot] < cap && !self.keys.contains(key),
            "Frozen request bound or one-shot document identity exceeded"
        );
        let next = self
            .reserved
            .checked_add(self.prices[slot])
            .context("Cost overflow")?;
        ensure!(
            next <= self.budget,
            "Common campaign budget exhausted before request"
        );
        record(
            &mut self.file,
            &json!({"event":"attempt_reserved","arm":arm.name(),"key":key,"operation":operation,"attempt":1,"reserved_nano_usd":self.prices[slot],"campaign_reserved_nano_usd":next,"preflight":preflight,"at":chrono::Utc::now()}),
        )?;
        self.reserved = next;
        self.counts[slot] += 1;
        self.keys.insert(key.into());
        self.inflight = Some((key.into(), operation));
        Ok(())
    }
    pub fn finish(&mut self, key: &str, outcome: Value, uncertain: bool) -> Result<()> {
        let Some((active, operation)) = &self.inflight else {
            anyhow::bail!("No reserved attempt to complete");
        };
        ensure!(
            active == key,
            "Completion identity differs from durable reservation"
        );
        record(
            &mut self.file,
            &json!({"event":"attempt_finished","key":key,"operation":operation,"outcome":outcome,"uncertain":uncertain,"at":chrono::Utc::now()}),
        )?;
        self.stopped |= uncertain;
        self.inflight = None;
        Ok(())
    }
    pub fn stopped(&self) -> bool {
        self.stopped || self.inflight.is_some()
    }
    pub fn summary(&self) -> Value {
        json!({"source_facts_attempts":self.counts[0],"reconciliation_attempts":self.counts[1],"embedding_attempts":self.counts[2],"reserved_nano_usd":self.reserved,"budget_nano_usd":self.budget,"stopped":self.stopped()})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn recipe() -> Value {
        json!({"source_facts":{"pricing_sha256":"text","pricing":{"verified_at":chrono::Utc::now(),"valid_for_hours":24}},"reconciliation":{"controls_sha256":"controls"},"optional_vectors":{"pricing_sha256":"embedding","source_chunk_manifest_sha256":"chunks","pricing":{"verified_at":chrono::Utc::now(),"valid_for_hours":24}},"limits":{"requests":147,"text_requests":126,"source_facts_requests":60,"reconciliation_requests":66,"embedding_requests":21,"source_facts_nano_usd":1308672000u64,"reconciliation_nano_usd":1439539200u64,"optional_vectors_nano_usd":184504320u64,"total_nano_usd":2932715520u64}})
    }
    fn approval() -> Value {
        json!({"approve_paid_calls":true,"campaign_kind":"combined-source-facts-reconciliation-vectors-1","recipe_sha256":"recipe","text_pricing_sha256":"text","embedding_pricing_sha256":"embedding","control_sha256":"controls","chunk_manifest_sha256":"chunks","maximum_requests":147,"maximum_text_requests":126,"maximum_embedding_requests":21,"maximum_output_tokens":4096,"authorization_reference":"unit-test synthetic consent only","budget_usd":"3"})
    }
    #[test]
    fn original_approval_and_low_budget_never_authorize_combined_scope() {
        let mut a = approval();
        assert_eq!(
            validate_approval(&a, &recipe(), "recipe").unwrap(),
            3_000_000_000
        );
        a["budget_usd"] = json!("2");
        assert!(validate_approval(&a, &recipe(), "recipe").is_err());
        a["budget_usd"] = json!("3");
        a["maximum_requests"] = json!(60);
        assert!(validate_approval(&a, &recipe(), "recipe").is_err());
        assert!(nano_usd("NaN").is_err());
    }
    #[test]
    fn expanded_scope_requires_its_actual_counts_and_larger_budget() {
        let mut r = recipe();
        r["limits"]["requests"] = json!(171);
        r["limits"]["text_requests"] = json!(138);
        r["limits"]["reconciliation_requests"] = json!(78);
        r["limits"]["reconciliation_nano_usd"] = json!(1701273600u64);
        r["limits"]["embedding_requests"] = json!(33);
        r["limits"]["optional_vectors_nano_usd"] = json!(289935360u64);
        r["limits"]["total_nano_usd"] = json!(3299880960u64);
        let mut a = approval();
        assert!(validate_approval(&a, &r, "recipe").is_err());
        a["maximum_requests"] = json!(171);
        a["maximum_text_requests"] = json!(138);
        a["maximum_embedding_requests"] = json!(33);
        assert!(validate_approval(&a, &r, "recipe").is_err());
        a["budget_usd"] = json!("3.30");
        assert_eq!(validate_approval(&a, &r, "recipe").unwrap(), 3_300_000_000);
    }
    #[test]
    fn remaining_scope_zero_facts_cap_and_original_approval_collision() {
        let mut r = recipe();
        r["campaign_kind"] = json!("remaining-reconciliation-vectors-1");
        r["limits"] = json!({"requests":111,"text_requests":78,"source_facts_requests":0,"reconciliation_requests":78,"embedding_requests":33,"source_facts_nano_usd":0,"reconciliation_nano_usd":1701273600u64,"optional_vectors_nano_usd":289935360u64,"total_nano_usd":1991208960u64});
        r["reused_source_facts"] = json!({"artifact_sha256":{"recipe.json":"a".repeat(64),"outcomes.jsonl":"b".repeat(64),"execution-receipt.json":"c".repeat(64)}});
        let mut a = approval();
        a["budget_usd"] = json!("2");
        a["maximum_requests"] = json!(111);
        a["maximum_source_facts_requests"] = json!(0);
        a["maximum_text_requests"] = json!(78);
        a["maximum_embedding_requests"] = json!(33);
        assert!(
            validate_approval(&a, &r, "recipe").is_err(),
            "Same$2 budget and changed bounds do not authorize a different campaign kind"
        );
        a["campaign_kind"] = r["campaign_kind"].clone();
        assert!(
            validate_approval(&a, &r, "recipe").is_err(),
            "Reused60 immutable hashes must be explicitly bound"
        );
        a["reused_source_facts_sha256"] = r["reused_source_facts"]["artifact_sha256"].clone();
        assert_eq!(validate_approval(&a, &r, "recipe").unwrap(), 2_000_000_000);
        a["maximum_source_facts_requests"] = json!(1);
        assert!(validate_approval(&a, &r, "recipe").is_err());
        a["maximum_source_facts_requests"] = json!(0);
        a["approve_paid_calls"] = json!(false);
        assert!(validate_approval(&a, &r, "recipe").is_err());
        a["approve_paid_calls"] = json!(true);
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../.cache/automatic-mapping/ledger-test-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir(&out).unwrap();
        let mut ledger = Ledger::begin(&out, &r, "recipe", &a, 2_000_000_000).unwrap();
        assert!(
            ledger
                .reserve(
                    Arm::SourceFacts,
                    "facts/forbidden",
                    Uuid::new_v4(),
                    json!({})
                )
                .is_err(),
            "The remaining scope cannot reserve even one repeated60call"
        );
        assert_eq!(ledger.summary()["reserved_nano_usd"], 0);
        ledger
            .reserve(
                Arm::Reconciliation,
                "recon/allowed",
                Uuid::new_v4(),
                json!({}),
            )
            .unwrap();
        ledger
            .finish("recon/allowed", json!({"state":"failed"}), false)
            .unwrap();
        assert_eq!(ledger.summary()["reserved_nano_usd"], 21_811_200);
        assert_eq!(ledger.summary()["source_facts_attempts"], 0);
        drop(ledger);
        std::fs::remove_dir_all(out).unwrap();
    }
    #[test]
    fn pricing_expiry_stops_before_any_additional_reservation() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../.cache/automatic-mapping/ledger-test-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut ledger =
            Ledger::begin(&root, &recipe(), "recipe", &approval(), 3_000_000_000).unwrap();
        ledger.price_deadline = chrono::Utc::now() - chrono::Duration::seconds(1);
        assert!(
            ledger
                .reserve(Arm::SourceFacts, "facts/expired", Uuid::new_v4(), json!({}))
                .is_err()
        );
        assert!(ledger.stopped());
        assert_eq!(ledger.summary()["source_facts_attempts"], 0);
        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn one_common_ledger_reserves_every_failure_and_refuses_uncertain_replay() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../.cache/automatic-mapping/ledger-test-{}",
            Uuid::new_v4()
        ));
        std::fs::create_dir(&root).unwrap();
        let r = recipe();
        let a = approval();
        let mut l = Ledger::begin(&root, &r, "recipe", &a, 3_000_000_000).unwrap();
        l.reserve(Arm::SourceFacts, "facts/d01", Uuid::new_v4(), json!({}))
            .unwrap();
        assert!(
            l.reserve(Arm::Reconciliation, "recon/d01", Uuid::new_v4(), json!({}))
                .is_err()
        );
        l.finish("facts/d01", json!({"state":"failed"}), false)
            .unwrap();
        l.reserve(Arm::Embedding, "embed/h06/1", Uuid::new_v4(), json!({}))
            .unwrap();
        l.finish("embed/h06/1", json!({"state":"uncertain"}), true)
            .unwrap();
        assert!(
            l.reserve(Arm::Reconciliation, "recon/d01", Uuid::new_v4(), json!({}))
                .is_err()
        );
        assert_eq!(l.summary()["reserved_nano_usd"], 30_597_120);
        assert!(Ledger::begin(&root, &r, "recipe", &a, 3_000_000_000).is_err());
        drop(l);
        std::fs::remove_dir_all(root).unwrap();
    }
}
