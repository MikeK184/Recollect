use crate::{
    error::{Error, Result},
    memory_evidence::Tx,
    memory_policy::assertion_key,
};
use axum::http::StatusCode;
use recollect_protocol::{AssertionRule, ClaimContent, ClaimRevision, FactValidity};
use sqlx::types::Json;
use uuid::Uuid;

pub fn family(a: &ClaimContent, b: &ClaimContent) -> bool {
    assertion_key(&a.subject, true) == assertion_key(&b.subject, true)
        && assertion_key(&a.predicate, true) == assertion_key(&b.predicate, true)
}
pub fn same_value(a: &ClaimContent, b: &ClaimContent) -> bool {
    assertion_key(&a.value, false) == assertion_key(&b.value, false)
}
fn bounds(v: &FactValidity) -> (Option<i64>, Option<i64>) {
    match v.kind.as_str() {
        "interval" => (
            v.from.map(|t| t.timestamp_micros()),
            v.to.map(|t| t.timestamp_micros()),
        ),
        "point" => {
            let bucket = match v.precision.as_str() {
                "second" => 1,
                "minute" => 60,
                "hour" => 3600,
                "day" => 86400,
                _ => return (None, None),
            };
            v.from.map_or((None, None), |t| {
                let start = t.timestamp().div_euclid(bucket) * bucket * 1_000_000;
                (Some(start), Some(start + bucket * 1_000_000))
            })
        }
        _ => (None, None),
    }
}
pub fn overlaps(a: &ClaimContent, b: &ClaimContent) -> bool {
    let (a_scope, b_scope) = (&a.selection, &b.selection);
    if a_scope
        .environment_id
        .zip(b_scope.environment_id)
        .is_some_and(|(a, b)| a != b)
        || (!a_scope.repository_ids.is_empty()
            && !b_scope.repository_ids.is_empty()
            && !a_scope
                .repository_ids
                .iter()
                .any(|id| b_scope.repository_ids.contains(id)))
    {
        return false;
    }
    let (a_start, a_end) = bounds(&a.validity);
    let (b_start, b_end) = bounds(&b.validity);
    a_end.zip(b_start).is_none_or(|(end, start)| end > start)
        && b_end.zip(a_start).is_none_or(|(end, start)| end > start)
}
pub fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "review_capacity",
        "Claim family, revision or review-rule capacity reached.",
    )
}
pub async fn check_family(
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
    existing: Option<Uuid>,
) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims c JOIN claim_revisions r ON r.id=c.current_revision WHERE c.brain_id=$1 AND r.subject_key=$2 AND r.predicate_key=$3 AND ($4::uuid IS NULL OR c.id<>$4)")
        .bind(brain).bind(assertion_key(&content.subject,true)).bind(assertion_key(&content.predicate,true)).bind(existing).fetch_one(&mut **tx).await?;
    if count >= 500 {
        return Err(capacity());
    }
    Ok(())
}
pub async fn advance_epoch(tx: &mut Tx<'_>, brain: Uuid) -> Result<()> {
    sqlx::query("INSERT INTO memory_epochs(brain_id,epoch) VALUES($1,1) ON CONFLICT(brain_id) DO UPDATE SET epoch=memory_epochs.epoch+1")
        .bind(brain).execute(&mut **tx).await?;
    Ok(())
}
pub async fn matching(
    tx: &mut Tx<'_>,
    r: &ClaimRevision,
    include_exempted: bool,
) -> Result<Vec<AssertionRule>> {
    let rules: Vec<Json<AssertionRule>> = sqlx::query_scalar("SELECT a.rule FROM assertion_rules a WHERE a.brain_id=$1 AND NOT a.payload_erased AND a.subject_key=$2 AND a.predicate_key=$3 AND a.value_key=$4 AND ($6 OR NOT EXISTS(SELECT 1 FROM memory_rule_exceptions e WHERE e.rule_id=a.id AND e.revision_id=$5 AND e.brain_id=a.brain_id)) ORDER BY a.created_at,a.id")
        .bind(r.brain_id).bind(assertion_key(&r.content.subject,true)).bind(assertion_key(&r.content.predicate,true)).bind(assertion_key(&r.content.value,false)).bind(r.id).bind(include_exempted).fetch_all(&mut **tx).await?;
    Ok(rules
        .into_iter()
        .map(|r| r.0)
        .filter(|rule| overlaps(&rule.content, &r.content))
        .collect())
}
pub async fn conflicts(tx: &mut Tx<'_>, r: &ClaimRevision) -> Result<Vec<ClaimRevision>> {
    if r.lifecycle != "active"
        || r.review == "rejected"
        || r.content.freshness == "superseded"
        || !matching(tx, r, false).await?.is_empty()
    {
        return Ok(Vec::new());
    }
    let candidates: Vec<Json<ClaimRevision>> = sqlx::query_scalar("SELECT r.revision FROM claims c JOIN claim_revisions r ON r.id=c.current_revision WHERE c.brain_id=$1 AND c.id<>$2 AND r.subject_key=$3 AND r.predicate_key=$4 AND r.value_key<>$5 AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active' AND recollect_memory_supported(r.brain_id,r.id) ORDER BY c.id LIMIT 500")
        .bind(r.brain_id).bind(r.claim_id).bind(assertion_key(&r.content.subject,true)).bind(assertion_key(&r.content.predicate,true)).bind(assertion_key(&r.content.value,false)).fetch_all(&mut **tx).await?;
    let mut result = Vec::new();
    for Json(candidate) in candidates {
        if candidate.lifecycle == "active"
            && candidate.review != "rejected"
            && candidate.content.freshness != "superseded"
            && overlaps(&candidate.content, &r.content)
            && matching(tx, &candidate, false).await?.is_empty()
        {
            result.push(candidate);
        }
    }
    Ok(result)
}
pub async fn add(
    tx: &mut Tx<'_>,
    old: &ClaimRevision,
    decision: Uuid,
    kind: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM assertion_rules WHERE brain_id=$1")
        .bind(old.brain_id)
        .fetch_one(&mut **tx)
        .await?;
    if count >= 10_000 {
        return Err(capacity());
    }
    let rule = AssertionRule {
        id: Uuid::new_v4(),
        brain_id: old.brain_id,
        claim_id: old.claim_id,
        revision_id: old.id,
        decision_id: decision,
        kind: kind.into(),
        content: old.content.clone(),
        created_at: at,
    };
    sqlx::query("INSERT INTO assertion_rules(id,brain_id,decision_id,subject_key,predicate_key,value_key,rule,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(rule.id).bind(rule.brain_id).bind(decision).bind(assertion_key(&old.content.subject,true)).bind(assertion_key(&old.content.predicate,true)).bind(assertion_key(&old.content.value,false)).bind(Json(rule)).bind(at).execute(&mut **tx).await?;
    Ok(())
}
pub async fn exempt(
    tx: &mut Tx<'_>,
    r: &ClaimRevision,
    decision: Uuid,
    withdrawal_only: bool,
) -> Result<()> {
    for rule in matching(tx, r, true).await? {
        if !withdrawal_only || rule.kind == "withdrawal" {
            sqlx::query("INSERT INTO memory_rule_exceptions(rule_id,revision_id,decision_id,brain_id) VALUES($1,$2,$3,$4)")
                .bind(rule.id).bind(r.id).bind(decision).bind(r.brain_id).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
