//! One-shot bounded authorization and fsynced no-refund reservation ledger.
use super::io;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs::File, io::Write, path::Path};
use uuid::Uuid;
pub const SCHEMA: &str = "recollect-revision4-qualification-approval-1";
pub const KIND: &str = "revision4-heldout-both-families-with-conditional-controls-1";
pub const INPUT: u64 = 67584;
pub const OUTPUT: u64 = 4096;
pub const TEXT: u64 = 21_811_200;
pub const EMBED: u64 = 8_785_920;
pub fn price(
    p: &Value,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<chrono::DateTime<chrono::Utc>> {
    io::fields(p, &["schema", "text", "embedding"])?;
    ensure!(
        p["schema"] == "recollect-revision4-pricing-1",
        "Pricing schema"
    );
    let mut deadline = now + chrono::Duration::hours(24);
    for (field, model, rate, out) in [
        ("text", "gpt-5.6-luna", 200, 1200),
        ("embedding", "text-embedding-3-large", 130, 0),
    ] {
        let v = &p[field];
        if field == "text" {
            io::fields(
                v,
                &[
                    "version",
                    "provider",
                    "model",
                    "endpoint",
                    "tier",
                    "verified_at",
                    "valid_for_hours",
                    "source_url",
                    "input_nano_usd_per_token",
                    "cached_input_nano_usd_per_token",
                    "cache_write_nano_usd_per_token",
                    "output_nano_usd_per_token",
                    "maximum_input_tokens_before_long_context_multiplier",
                    "note",
                ],
            )?;
        } else {
            io::fields(
                v,
                &[
                    "version",
                    "provider",
                    "model",
                    "endpoint",
                    "tier",
                    "verified_at",
                    "valid_for_hours",
                    "source_url",
                    "input_nano_usd_per_token",
                    "output_nano_usd_per_token",
                    "note",
                ],
            )?;
        }
        ensure!(
            v["version"] == 1
                && v["tier"]
                    == if field == "text" {
                        "standard text Responses API; no tools, regional routing or fast mode"
                    } else {
                        "standard embeddings API; no Batch discount or regional multiplier"
                    },
            "Exact standard service tier required"
        );
        let at = chrono::DateTime::parse_from_rfc3339(
            v["verified_at"].as_str().context("Verification time")?,
        )?
        .with_timezone(&chrono::Utc);
        ensure!(
            v["provider"] == "openai"
                && v["model"] == model
                && v["endpoint"] == "https://api.openai.com/v1"
                && v["valid_for_hours"] == 24
                && at <= now
                && now < at + chrono::Duration::hours(24)
                && v["input_nano_usd_per_token"] == rate
                && v["output_nano_usd_per_token"] == out,
            "Exact recent standard public pricing required"
        );
        ensure!(
            v["source_url"].as_str()
                == Some(&format!(
                    "https://developers.openai.com/api/docs/models/{model}"
                )),
            "Official exact model price source required"
        );
        if field == "text" {
            ensure!(
                v["cache_write_nano_usd_per_token"] == 250
                    && v["cached_input_nano_usd_per_token"] == 20
                    && v["maximum_input_tokens_before_long_context_multiplier"]
                        .as_u64()
                        .is_some_and(|n| n > INPUT),
                "Cache-write/long-context pricing differs"
            );
        }
        deadline = deadline.min(at + chrono::Duration::hours(24));
    }
    Ok(deadline)
}
pub fn caps(empty: u64, batches: u64, controls: u64) -> Result<Value> {
    ensure!(
        empty <= 46 && controls <= 6 && batches <= 2000,
        "Count bounds"
    );
    let held = 2 * (46 - empty);
    Ok(
        json!({"heldout_text":held,"control_text":controls,"embeddings":batches,"support":6,"total":held+controls+batches+6}),
    )
}
pub fn cost(c: &Value) -> Result<u64> {
    io::fields(
        c,
        &[
            "heldout_text",
            "control_text",
            "embeddings",
            "support",
            "total",
        ],
    )?;
    let sum = ["heldout_text", "control_text", "embeddings", "support"]
        .iter()
        .try_fold(0u64, |n, k| n.checked_add(c[k].as_u64()?))
        .context("Count overflow")?;
    ensure!(c["total"] == sum, "Total count differs");
    let n = |k| c[k].as_u64().context("Integer count");
    n("heldout_text")?
        .checked_add(n("control_text")?)
        .and_then(|v| v.checked_add(n("support").ok()?))
        .and_then(|v| v.checked_mul(TEXT))
        .and_then(|v| v.checked_add(n("embeddings").ok()?.checked_mul(EMBED)?))
        .context("Cost overflow")
}
pub fn template(recipe: &Value, recipe_sha: &str) -> Value {
    json!({"schema":SCHEMA,"kind":KIND,"approve_paid_calls":false,"recipe_sha256":recipe_sha,"pricing_sha256":recipe["pricing_sha256"],"executable_sha256":recipe["executable_sha256"],"schedule_sha256":recipe["schedule_sha256"],"maximum_calls":recipe["maximum_calls"],"maximum_input_tokens":INPUT,"maximum_output_tokens":OUTPUT,"budget_nano_usd":recipe["budget_nano_usd"],"authorization_reference":"UNAPPROVED","conditional_support":{"maximum_calls":6,"template_sha256":recipe["support_template_sha256"],"canonical_dependencies_only":true,"actual_body_freeze":"before_each_transmission"}})
}
pub fn approval(a: &Value, r: &Value, hash: &str) -> Result<u64> {
    io::fields(
        a,
        &[
            "schema",
            "kind",
            "approve_paid_calls",
            "recipe_sha256",
            "pricing_sha256",
            "executable_sha256",
            "schedule_sha256",
            "maximum_calls",
            "maximum_input_tokens",
            "maximum_output_tokens",
            "budget_nano_usd",
            "authorization_reference",
            "conditional_support",
        ],
    )?;
    io::fields(
        &a["maximum_calls"],
        &[
            "heldout_text",
            "control_text",
            "embeddings",
            "support",
            "total",
        ],
    )?;
    io::fields(
        &a["conditional_support"],
        &[
            "maximum_calls",
            "template_sha256",
            "canonical_dependencies_only",
            "actual_body_freeze",
        ],
    )?;
    ensure!(
        a["schema"] == SCHEMA && a["kind"] == KIND && a["approve_paid_calls"] == true,
        "New explicit paid grant required"
    );
    let expected = template(r, hash);
    for k in [
        "recipe_sha256",
        "pricing_sha256",
        "executable_sha256",
        "schedule_sha256",
        "maximum_calls",
        "maximum_input_tokens",
        "maximum_output_tokens",
        "conditional_support",
    ] {
        ensure!(a[k] == expected[k], "Approval scope differs");
    }
    let reference = a["authorization_reference"]
        .as_str()
        .context("Human authorization reference")?;
    ensure!(
        reference.len() >= 16
            && reference.len() <= 4096
            && reference != "UNAPPROVED"
            && !reference.to_lowercase().contains("approved60")
            && !reference.to_lowercase().contains("remaining111")
            && !reference.to_lowercase().contains("support6"),
        "New human authorization reference required"
    );
    let budget = a["budget_nano_usd"]
        .as_u64()
        .context("Positive integer nano USD")?;
    ensure!(
        budget > 0
            && budget >= cost(&a["maximum_calls"])?
            && budget >= r["budget_nano_usd"].as_u64().context("Frozen budget")?,
        "Budget below full worst case"
    );
    Ok(budget)
}
#[derive(Clone, Copy, Debug)]
pub enum Arm {
    Heldout,
    Control,
    Embedding,
    Support,
}
impl Arm {
    pub fn index(self) -> usize {
        match self {
            Self::Heldout => 0,
            Self::Control => 1,
            Self::Embedding => 2,
            Self::Support => 3,
        }
    }
    pub fn key(self) -> &'static str {
        ["heldout_text", "control_text", "embeddings", "support"][self.index()]
    }
}
pub struct Ledger {
    file: File,
    keys: BTreeSet<String>,
    ops: BTreeSet<Uuid>,
    counts: [u64; 4],
    caps: [u64; 4],
    reserved: u64,
    budget: u64,
    pending: Option<String>,
    stopped: bool,
    deadline: chrono::DateTime<chrono::Utc>,
}
impl Ledger {
    pub fn begin(out: &Path, r: &Value, hash: &str, a: &Value, simulated: bool) -> Result<Self> {
        let budget = if simulated {
            cost(&r["maximum_calls"])?
        } else {
            approval(a, r, hash)?
        };
        let deadline = price(&r["pricing"], chrono::Utc::now())?;
        let mut lock = io::exclusive(&out.join("execution-lock.json"))?;
        serde_json::to_writer(
            &mut lock,
            &json!({"recipe_sha256":hash,"approval_sha256":io::value_hash(a)?,"simulation_only":simulated,"resumption":"forbidden"}),
        )?;
        lock.sync_all()?;
        let file = io::exclusive(&out.join("reservation-ledger.jsonl"))?;
        let mut l = Self {
            file,
            keys: BTreeSet::new(),
            ops: BTreeSet::new(),
            counts: [0; 4],
            caps: [0; 4],
            reserved: 0,
            budget,
            pending: None,
            stopped: false,
            deadline,
        };
        for (i, k) in ["heldout_text", "control_text", "embeddings", "support"]
            .iter()
            .enumerate()
        {
            l.caps[i] = r["maximum_calls"][k].as_u64().context("Frozen count")?;
        }
        l.append(json!({"event":"campaign_started","simulation_only":simulated,"maximum_calls":r["maximum_calls"],"budget_nano_usd":budget}))?;
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
        arm: Arm,
        key: &str,
        op: Uuid,
        bytes: &[u8],
        expected: &str,
    ) -> Result<()> {
        ensure!(
            !self.stopped
                && self.pending.is_none()
                && !self.keys.contains(key)
                && !self.ops.contains(&op)
                && self.counts[arm.index()] < self.caps[arm.index()]
                && chrono::Utc::now() < self.deadline,
            "Reservation refused"
        );
        ensure!(
            bytes.len() <= 65536 && io::sha(bytes)? == expected,
            "Body hash/limit differs"
        );
        let charge = if matches!(arm, Arm::Embedding) {
            EMBED
        } else {
            TEXT
        };
        let next = self
            .reserved
            .checked_add(charge)
            .context("Budget overflow")?;
        ensure!(next <= self.budget, "Budget exhausted");
        self.append(json!({"event":"reserved","key":key,"operation_id":op,"body_sha256":expected,"bytes":bytes.len(),"arm":arm.key(),"charged_nano_usd":charge,"campaign_reserved_nano_usd":next,"at":chrono::Utc::now()}))?;
        self.keys.insert(key.into());
        self.ops.insert(op);
        self.counts[arm.index()] += 1;
        self.reserved = next;
        self.pending = Some(key.into());
        Ok(())
    }
    pub fn finish(&mut self, key: &str, ok: bool, receipt: Value) -> Result<()> {
        ensure!(self.pending.as_deref() == Some(key), "Unknown reservation");
        self.append(json!({"event":"finished","key":key,"acceptable_response":ok,"receipt":receipt,"at":chrono::Utc::now()}))?;
        self.pending = None;
        if !ok {
            self.stopped = true;
        }
        Ok(())
    }
    pub fn stop(&mut self) -> Result<()> {
        self.stopped = true;
        self.append(json!({"event":"campaign_stopped","pending":self.pending}))
    }
    pub fn stopped(&self) -> bool {
        self.stopped || self.pending.is_some()
    }
    pub fn summary(&self) -> Value {
        json!({"reservations":self.keys.len(),"reserved_nano_usd":self.reserved,"counts":self.counts,"pending":self.pending,"stopped":self.stopped})
    }
}
pub fn response_ok(v: &Value, embedding: bool) -> bool {
    let expected = if embedding {
        "text-embedding-3-large"
    } else {
        "gpt-5.6-luna"
    };
    let model = v["model"].as_str().is_some_and(|m| {
        m == expected
            || (!embedding
                && m.strip_prefix(&format!("{expected}-"))
                    .is_some_and(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()))
    });
    if embedding {
        model
            && v["object"] == "list"
            && v["usage"]["prompt_tokens"] == v["usage"]["total_tokens"]
            && v["usage"]["prompt_tokens"]
                .as_u64()
                .is_some_and(|n| n <= INPUT)
            && v["usage"]["total_tokens"]
                .as_u64()
                .is_some_and(|n| n <= INPUT)
            && v["data"].as_array().is_some_and(|a| {
                !a.is_empty()
                    && a.len() <= 20
                    && a.iter().enumerate().all(|(i, r)| {
                        r["object"] == "embedding"
                            && r["index"] == i
                            && r["embedding"].as_array().is_some_and(|v| {
                                v.len() == 3072
                                    && v.iter().all(|n| n.as_f64().is_some_and(f64::is_finite))
                            })
                    })
            })
    } else {
        model
            && v["service_tier"] == "default"
            && v["usage"]["input_tokens"]
                .as_u64()
                .zip(v["usage"]["output_tokens"].as_u64())
                .and_then(|(a, b)| a.checked_add(b))
                == v["usage"]["total_tokens"].as_u64()
            && v["status"] == "completed"
            && v["usage"]["input_tokens"]
                .as_u64()
                .is_some_and(|n| n <= INPUT)
            && v["usage"]["output_tokens"]
                .as_u64()
                .is_some_and(|n| n <= OUTPUT)
            && v["usage"]["total_tokens"]
                .as_u64()
                .is_some_and(|n| n <= INPUT + OUTPUT)
            && v["output"].as_array().is_some_and(|a| {
                a.len() == 1
                    && a[0]["type"] == "message"
                    && a[0]["role"] == "assistant"
                    && a[0]["status"] == "completed"
                    && a[0]["content"].as_array().is_some_and(|c| {
                        c.len() == 1 && c[0]["type"] == "output_text" && c[0]["text"].is_string()
                    })
            })
    }
}

/// Validate the frozen strict JSON schema without repairing a provider payload.
pub fn schema(value: &Value, s: &Value, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    let types = match &s["type"] {
        Value::String(t) => vec![t.as_str()],
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
        _ => return false,
    };
    if !types.iter().any(|t| match *t {
        "null" => value.is_null(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.as_f64().is_some_and(f64::is_finite),
        "boolean" => value.is_boolean(),
        _ => false,
    }) {
        return false;
    }
    if s["enum"].as_array().is_some_and(|a| !a.contains(value)) {
        return false;
    }
    if value.is_null() {
        return true;
    }
    if let Some(text) = value.as_str() {
        let n = text.chars().count() as u64;
        if s["minLength"].as_u64().is_some_and(|x| n < x)
            || s["maxLength"].as_u64().is_some_and(|x| n > x)
        {
            return false;
        }
    }
    if let Some(n) = value.as_f64() {
        if s["minimum"].as_f64().is_some_and(|x| n < x)
            || s["maximum"].as_f64().is_some_and(|x| n > x)
        {
            return false;
        }
    }
    if let Some(a) = value.as_array() {
        if s["minItems"].as_u64().is_some_and(|x| (a.len() as u64) < x)
            || s["maxItems"].as_u64().is_some_and(|x| (a.len() as u64) > x)
            || !a.iter().all(|v| schema(v, &s["items"], depth + 1))
        {
            return false;
        }
    }
    if let Some(o) = value.as_object() {
        let Some(props) = s["properties"].as_object() else {
            return false;
        };
        if s["required"].as_array().is_some_and(|a| {
            a.iter()
                .any(|k| !k.as_str().is_some_and(|k| o.contains_key(k)))
        }) {
            return false;
        }
        for (k, v) in o {
            if let Some(p) = props.get(k) {
                if !schema(v, p, depth + 1) {
                    return false;
                }
            } else if s["additionalProperties"] == false {
                return false;
            }
        }
    }
    true
}
pub fn structured_ok(body: &Value, response: &Value, embedding: bool) -> bool {
    if !response_ok(response, embedding) {
        return false;
    }
    if embedding {
        return response["data"].as_array().map(Vec::len) == body["input"].as_array().map(Vec::len);
    }
    let texts: Vec<_> = response["output"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|m| m["content"].as_array().into_iter().flatten())
        .filter(|v| v["type"] == "output_text")
        .collect();
    if texts.len() != 1 {
        return false;
    }
    let Some(text) = texts[0]["text"].as_str() else {
        return false;
    };
    let Ok(value) = io::strict(text.as_bytes()) else {
        return false;
    };
    schema(&value, &body["text"]["format"]["schema"], 0)
}
