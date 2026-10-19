//! New campaign kind and durable one-attempt support-only budget boundary.
use crate::{KIND, common, hash};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs::File, io::Write, path::Path};
use uuid::Uuid;

pub fn validate_approval(a: &Value, r: &Value, h: &str) -> Result<()> {
    ensure!(
        a["approve_paid_calls"] == true && a["campaign_kind"] == KIND && r["campaign_kind"] == KIND,
        "New exact support6 approval required; false or old grants cannot authorize support"
    );
    for (left, right) in [
        ("pricing_sha256", &r["pricing_sha256"]),
        (
            "control_outcomes_sha256",
            &r["original"]["control_outcomes_sha256"],
        ),
        ("controls_sha256", &r["original"]["controls_sha256"]),
        (
            "semantic_manifest_sha256",
            &r["conformance"]["semantic_manifest_sha256"],
        ),
    ] {
        ensure!(
            &a[left] == right && right.as_str().is_some_and(|s| s.len() == 64),
            "Approval binding differs"
        );
    }
    ensure!(
        a["recipe_sha256"] == h
            && a["maximum_support_requests"] == 6
            && a["maximum_external_extraction_requests"] == 0
            && a["maximum_embedding_requests"] == 0
            && a["maximum_output_tokens"] == 4096,
        "Support-only recipe/count/token bounds differ"
    );
    let reference = a["authorization_reference"]
        .as_str()
        .context("Human authorization reference required")?;
    ensure!(
        !reference.trim().is_empty() && reference != "UNAPPROVED",
        "Explicit new human authorization required"
    );
    let budget = decimal(a["budget_usd"].as_str().context("New budget required")?)?;
    ensure!(
        budget
            >= r["limits"]["total_nano_usd"]
                .as_u64()
                .context("Frozen cost required")?
            && budget <= 150_000_000,
        "Budget outside reviewed support6 maximum and15cent cap"
    );
    Ok(())
}
fn decimal(s: &str) -> Result<u64> {
    let (a, b) = s.split_once('.').unwrap_or((s, ""));
    ensure!(
        !a.is_empty()
            && a.bytes().all(|c| c.is_ascii_digit())
            && b.len() <= 9
            && b.bytes().all(|c| c.is_ascii_digit()),
        "Budget decimal invalid"
    );
    a.parse::<u64>()?
        .checked_mul(1_000_000_000)
        .and_then(|n| n.checked_add(format!("{b:0<9}").parse().ok()?))
        .context("Budget overflow")
}
pub fn support_body(
    bytes: &[u8],
    expected_hash: &str,
    model: &str,
    instructions: &str,
    schema: &Value,
) -> Result<Value> {
    ensure!(bytes.len() <= 65536, "Complete body exceeds64KiB; no send");
    ensure!(
        common::sha256(bytes)? == expected_hash,
        "Frozen request bytes changed; no send"
    );
    let body: Value = serde_json::from_slice(bytes)?;
    ensure!(
        body["model"] == model
            && body["max_output_tokens"] == 4096
            && body["store"] == false
            && body["instructions"] == instructions
            && body["text"]["format"]["name"] == "source_support"
            && body["text"]["format"]["schema"] == *schema,
        "Unexpected model/prompt/schema/output or storage; no send"
    );
    ensure!(
        body.as_object().context("Body object")?.keys().all(|s| [
            "input",
            "instructions",
            "max_output_tokens",
            "model",
            "reasoning",
            "store",
            "text"
        ]
        .contains(&s.as_str())),
        "Unexpected body/tools; no send"
    );
    ensure!(
        body["input"].as_str().is_some_and(|s| !s.is_empty()),
        "Missing canonical support inputs"
    );
    Ok(body)
}
pub fn canonical_body(bytes: &[u8], canonical: &Value) -> Result<()> {
    ensure!(
        bytes == serde_json::to_vec(canonical)?,
        "Actual wire input differs from fresh complete canonical gateway resolution; no send"
    );
    Ok(())
}
pub fn response_bounds(value: &Value, model: &str) -> bool {
    let returned = value["model"].as_str();
    let model_ok = returned.is_some_and(|m| {
        m == model
            || m.strip_prefix(&format!("{model}-"))
                .is_some_and(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
    });
    model_ok
        && value["usage"]["input_tokens"]
            .as_u64()
            .is_some_and(|n| n <= 67584)
        && value["usage"]["output_tokens"]
            .as_u64()
            .is_some_and(|n| n <= 4096)
}
pub struct Ledger {
    file: File,
    keys: BTreeSet<String>,
    operations: BTreeSet<Uuid>,
    per: u64,
    total: u64,
    reserved: u64,
    pending: Option<String>,
    stopped: bool,
    pub completed: usize,
    pub uncertain: usize,
    simulated: bool,
}
impl Ledger {
    pub fn new(path: &Path, per: u64, total: u64, simulated: bool) -> Result<Self> {
        ensure!(
            per.checked_mul(6) == Some(total),
            "Exact shared six-call budget required"
        );
        let mut l = Self {
            file: common::private_file(path)?,
            keys: BTreeSet::new(),
            operations: BTreeSet::new(),
            per,
            total,
            reserved: 0,
            pending: None,
            stopped: false,
            completed: 0,
            uncertain: 0,
            simulated,
        };
        l.append(json!({"event":"ledger_started","campaign_kind":KIND,"maximum_support_requests":6,"per_request_nano_usd":per,"total_nano_usd":total,"simulation_only":simulated}))?;
        Ok(l)
    }
    fn append(&mut self, v: Value) -> Result<()> {
        serde_json::to_writer(&mut self.file, &v)?;
        self.file.write_all(b"\n")?;
        self.file.sync_all()?;
        Ok(())
    }
    pub fn reserve(
        &mut self,
        key: &str,
        op: Uuid,
        bytes: &[u8],
        expected_hash: &str,
        metadata: Value,
    ) -> Result<()> {
        ensure!(
            !self.stopped
                && self.pending.is_none()
                && self.keys.len() < 6
                && !self.keys.contains(key)
                && !self.operations.contains(&op),
            "Attempt duplicated, uncertain, concurrent or exhausted; no send"
        );
        ensure!(
            bytes.len() <= 65536 && common::sha256(bytes)? == expected_hash,
            "Complete request bytes changed or oversized; no send"
        );
        let next = self
            .reserved
            .checked_add(self.per)
            .context("Budget overflow")?;
        ensure!(next <= self.total, "Budget exhausted; no send");
        self.append(json!({"event":"attempt_reserved","key":key,"operation":op,"attempt":1,"body_sha256":expected_hash,"complete_body_bytes":bytes.len(),"reserved_nano_usd":self.per,"campaign_reserved_nano_usd":next,"metadata":metadata,"at":chrono::Utc::now(),"simulation_only":self.simulated}))?;
        self.keys.insert(key.into());
        self.operations.insert(op);
        self.pending = Some(key.into());
        self.reserved = next;
        Ok(())
    }
    pub fn finish(&mut self, key: &str, uncertain: bool, receipt: Value) -> Result<()> {
        ensure!(
            self.pending.as_deref() == Some(key),
            "Unknown attempt finish"
        );
        self.append(json!({"event":"attempt_finished","key":key,"uncertain":uncertain,"receipt":receipt,"at":chrono::Utc::now(),"simulation_only":self.simulated}))?;
        self.pending = None;
        self.completed += 1;
        if uncertain {
            self.stopped = true;
            self.uncertain += 1;
        }
        Ok(())
    }
    pub fn summary(&self) -> Value {
        json!({"reserved_attempts":self.keys.len(),"finished_attempts":self.completed,"uncertain":self.uncertain,"reserved_nano_usd":self.reserved,"pending":self.pending,"stopped":self.stopped,"simulation_only":self.simulated})
    }
}

pub async fn proof(out: &Path) -> Result<()> {
    // Synthetic local-only transport exercises exact bytes after actual durable
    // admission. No provider key/official endpoint is used or constructed.
    use axum::{Router, body::Bytes, extract::State, http::StatusCode, routing::post};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let sent = Arc::new(AtomicUsize::new(0));
    async fn receive(State(n): State<Arc<AtomicUsize>>, body: Bytes) -> StatusCode {
        assert!(!body.is_empty());
        n.fetch_add(1, Ordering::SeqCst);
        StatusCode::OK
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(
        axum::serve(
            listener,
            Router::new()
                .route("/v1/responses", post(receive))
                .with_state(sent.clone()),
        )
        .into_future(),
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()?;
    let body = json!({"model":"gpt-5.6-luna","max_output_tokens":4096,"store":false,"instructions":"controlled support instructions","input":"canonical controlled data","text":{"format":{"name":"source_support","schema":{"type":"object"}}}});
    let bytes = serde_json::to_vec(&body)?;
    let h = common::sha256(&bytes)?;
    let check = |b: &[u8], expected: &str| {
        support_body(
            b,
            expected,
            "gpt-5.6-luna",
            "controlled support instructions",
            &json!({"type":"object"}),
        )
    };
    let mut rejected = 0;
    for bad in [
        vec![b' '; 65537],
        serde_json::to_vec(&json!({"model":"different"}))?,
        {
            let mut b = bytes.clone();
            b.push(b' ');
            b
        },
    ] {
        ensure!(check(&bad, &h).is_err(), "Bad body admitted");
        rejected += 1;
    }
    for (key, value) in [
        ("model", json!("different")),
        ("max_output_tokens", json!(4097)),
        ("instructions", json!("changed")),
        ("store", json!(true)),
        ("tools", json!([])),
    ] {
        let mut bad = body.clone();
        bad[key] = value;
        let b = serde_json::to_vec(&bad)?;
        ensure!(
            check(&b, &common::sha256(&b)?).is_err(),
            "Unexpected body admitted"
        );
        rejected += 1;
    }
    ensure!(sent.load(Ordering::SeqCst) == 0, "Rejected body sent");
    for invalid in [
        json!({"model":"gpt-5.6-luna"}),
        json!({"model":"other","usage":{"input_tokens":1,"output_tokens":1}}),
        json!({"model":"gpt-5.6-luna","usage":{"input_tokens":67585,"output_tokens":1}}),
        json!({"model":"gpt-5.6-luna","usage":{"input_tokens":1,"output_tokens":4097}}),
    ] {
        ensure!(
            !response_bounds(&invalid, "gpt-5.6-luna"),
            "Missing/changed model/usage accepted"
        );
    }
    let mut l = Ledger::new(
        &out.join("six-call-ledger.jsonl"),
        21_811_200,
        130_867_200,
        true,
    )?;
    let first = Uuid::new_v4();
    for n in 0..6 {
        let key = format!("rc{:02}", n + 1);
        let op = if n == 0 { first } else { Uuid::new_v4() };
        check(&bytes, &h)?;
        l.reserve(&key, op, &bytes, &h, json!({"loopback":true}))?;
        ensure!(
            l.reserve("concurrent", Uuid::new_v4(), &bytes, &h, json!({}))
                .is_err(),
            "Concurrent attempt admitted"
        );
        client
            .post(format!("http://{address}/v1/responses"))
            .body(bytes.clone())
            .send()
            .await?
            .error_for_status()?;
        l.finish(&key, false, json!({"state":"loopback_completed"}))?;
    }
    ensure!(
        l.reserve("seventh", Uuid::new_v4(), &bytes, &h, json!({}))
            .is_err()
            && l.reserve("rc01", first, &bytes, &h, json!({})).is_err(),
        "Cap/replay bypass"
    );
    ensure!(
        Ledger::new(
            &out.join("six-call-ledger.jsonl"),
            21_811_200,
            130_867_200,
            true
        )
        .is_err(),
        "Durable ledger resumed"
    );
    let mut uncertain = Ledger::new(
        &out.join("uncertain-ledger.jsonl"),
        21_811_200,
        130_867_200,
        true,
    )?;
    uncertain.reserve("rc01", Uuid::new_v4(), &bytes, &h, json!({"loopback":true}))?;
    uncertain.finish("rc01", true, json!({"state":"simulated_unknown_outcome"}))?;
    ensure!(
        uncertain
            .reserve("rc02", Uuid::new_v4(), &bytes, &h, json!({}))
            .is_err(),
        "Uncertain attempt replayed or continued"
    );
    let fake = json!({"campaign_kind":KIND,"pricing_sha256":"a".repeat(64),"original":{"control_outcomes_sha256":"b".repeat(64),"controls_sha256":"c".repeat(64)},"conformance":{"semantic_manifest_sha256":"d".repeat(64)},"limits":{"total_nano_usd":130867200}});
    let fake_hash = hash(&fake)?;
    let false_grant = crate::approval_template(&fake, &fake_hash);
    ensure!(
        validate_approval(&false_grant, &fake, &fake_hash).is_err(),
        "False grant admitted"
    );
    for (dir, name) in [
        ("extractor-approved60-2026-10-18", "approval-granted.json"),
        (
            "extractor-remaining111-current-v4-2026-10-18",
            "granted-approval.json",
        ),
    ] {
        let old = crate::read(
            &crate::root()?
                .join(".cache/automatic-mapping")
                .join(dir)
                .join(name),
        )?;
        ensure!(
            validate_approval(&old, &fake, &fake_hash).is_err(),
            "Completed old grant admitted"
        );
    }
    let mut stale = crate::read(&crate::root()?.join(crate::PRICE))?;
    stale["verified_at"] = json!("2000-01-01T00:00:00Z");
    ensure!(
        common::validate_pricing(&stale, "gpt-5.6-luna").is_err(),
        "Stale pricing admitted"
    );
    server.abort();
    common::private_json(
        &out.join("guard-proof-receipt.json"),
        &json!({"status":"passed","compiled_runner_sha256":common::sha256(&std::fs::read(std::env::current_exe()?)?)?,"code_manifest_sha256":hash(&crate::code_bindings(&crate::root()?)?)?,"external_requests":0,"loopback_forwarded_requests":sent.load(Ordering::SeqCst),"body_rejections_before_transmission":rejected,"false_and_both_old_grants_rejected":true,"stale_price_rejected":true,"missing_usage_or_changed_model_rejected":true,"concurrency_and_six_cap_and_duplicate_rejected":true,"uncertain_stops_without_replay":true,"durable_ledger_resumption_rejected":true,"ledger":l.summary()}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimals_do_not_accept_nonfinite_or_negative_budgets() {
        assert!(decimal("NaN").is_err());
        assert!(decimal("-1").is_err());
        assert!(decimal("0.0000000001").is_err());
        assert_eq!(decimal("0.130867200").unwrap(), 130867200);
    }
}
