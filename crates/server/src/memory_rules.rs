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
    Ok(matching_many(tx, &[r], include_exempted)
        .await?
        .pop()
        .unwrap_or_default())
}
/// Rules matching each revision, index-aligned, from one statement. The textual
/// key match is SQL; validity/scope overlap stays in `overlaps`, the one
/// implementation of that predicate.
pub async fn matching_many(
    tx: &mut Tx<'_>,
    revisions: &[&ClaimRevision],
    include_exempted: bool,
) -> Result<Vec<Vec<AssertionRule>>> {
    let Some(first) = revisions.first() else {
        return Ok(Vec::new());
    };
    let mut distinct: Vec<&ClaimRevision> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for r in revisions {
        if seen.insert(r.id) {
            distinct.push(r);
        }
    }
    let rows: Vec<(Uuid, Json<AssertionRule>)> = sqlx::query_as("SELECT k.rev,a.rule FROM unnest($2::uuid[],$3::text[],$4::text[],$5::text[]) AS k(rev,s,p,v) JOIN assertion_rules a ON a.brain_id=$1 AND NOT a.payload_erased AND a.subject_key=k.s AND a.predicate_key=k.p AND a.value_key=k.v WHERE ($6 OR NOT EXISTS(SELECT 1 FROM memory_rule_exceptions e WHERE e.rule_id=a.id AND e.revision_id=k.rev AND e.brain_id=a.brain_id)) ORDER BY k.rev,a.created_at,a.id")
        .bind(first.brain_id)
        .bind(distinct.iter().map(|r| r.id).collect::<Vec<_>>())
        .bind(distinct.iter().map(|r| assertion_key(&r.content.subject, true)).collect::<Vec<_>>())
        .bind(distinct.iter().map(|r| assertion_key(&r.content.predicate, true)).collect::<Vec<_>>())
        .bind(distinct.iter().map(|r| assertion_key(&r.content.value, false)).collect::<Vec<_>>())
        .bind(include_exempted)
        .fetch_all(&mut **tx)
        .await?;
    Ok(revisions
        .iter()
        .map(|r| {
            rows.iter()
                .filter(|(rev, rule)| *rev == r.id && overlaps(&rule.0.content, &r.content))
                .map(|(_, rule)| rule.0.clone())
                .collect()
        })
        .collect())
}
/// `recollect_memory_supported` for many revisions in one statement.
pub async fn supported(
    tx: &mut Tx<'_>,
    brain: Uuid,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, bool>> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    Ok(sqlx::query_as::<_, (Uuid, bool)>(
        "SELECT t.id,recollect_memory_supported($1,t.id) FROM unnest($2::uuid[]) AS t(id)",
    )
    .bind(brain)
    .bind(ids)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .collect())
}
pub async fn conflicts(tx: &mut Tx<'_>, r: &ClaimRevision) -> Result<Vec<ClaimRevision>> {
    let blocked = !matching(tx, r, false).await?.is_empty();
    Ok(conflicts_many(tx, &[r], &[blocked])
        .await?
        .pop()
        .unwrap_or_default())
}
/// Unresolved conflicting current revisions for each revision, index-aligned.
/// `blocked[i]` says revision `i` is itself matched by a review rule, which
/// suppresses its conflicts. One statement gathers the current, supported,
/// active revisions of every involved subject/predicate family and one more
/// resolves which of those are rule-blocked; the per-candidate predicates are
/// the same as the single-revision form (family cap keeps each family under the
/// original 500-row bound).
pub async fn conflicts_many(
    tx: &mut Tx<'_>,
    revisions: &[&ClaimRevision],
    blocked: &[bool],
) -> Result<Vec<Vec<ClaimRevision>>> {
    let mut result = vec![Vec::new(); revisions.len()];
    let live: Vec<usize> = (0..revisions.len())
        .filter(|i| {
            let r = revisions[*i];
            !blocked[*i]
                && r.lifecycle == "active"
                && r.review != "rejected"
                && r.content.freshness != "superseded"
        })
        .collect();
    if live.is_empty() {
        return Ok(result);
    }
    let family = |r: &ClaimRevision| {
        (
            assertion_key(&r.content.subject, true),
            assertion_key(&r.content.predicate, true),
        )
    };
    let families: std::collections::BTreeSet<_> =
        live.iter().map(|i| family(revisions[*i])).collect();
    // Current, active revisions of every involved family. The support audit is
    // expensive, so it runs only for revisions that could actually conflict.
    let rows: Vec<(String, String, String, Uuid, Json<ClaimRevision>)> = sqlx::query_as("SELECT r.subject_key,r.predicate_key,r.value_key,c.id,r.revision FROM unnest($2::text[],$3::text[]) AS f(s,p) JOIN claim_revisions r ON r.brain_id=$1 AND r.subject_key=f.s AND r.predicate_key=f.p JOIN claims c ON c.brain_id=$1 AND c.current_revision=r.id WHERE recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active' ORDER BY c.id")
        .bind(revisions[0].brain_id)
        .bind(families.iter().map(|f| f.0.clone()).collect::<Vec<_>>())
        .bind(families.iter().map(|f| f.1.clone()).collect::<Vec<_>>())
        .fetch_all(&mut **tx)
        .await?;
    let possible = |row: &(String, String, String, Uuid, Json<ClaimRevision>),
                    r: &ClaimRevision| {
        let (subject, predicate) = family(r);
        row.0 == subject
            && row.1 == predicate
            && row.3 != r.claim_id
            && row.2 != assertion_key(&r.content.value, false)
    };
    let involved: std::collections::BTreeSet<Uuid> = rows
        .iter()
        .filter(|row| live.iter().any(|i| possible(row, revisions[*i])))
        .map(|row| row.4.0.id)
        .collect();
    let supported = supported(
        tx,
        revisions[0].brain_id,
        &involved.into_iter().collect::<Vec<_>>(),
    )
    .await?;
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|row| supported.get(&row.4.0.id) == Some(&true))
        .collect();
    let candidates: Vec<&ClaimRevision> = rows
        .iter()
        .filter(|row| {
            row.4.0.lifecycle == "active"
                && row.4.0.review != "rejected"
                && row.4.0.content.freshness != "superseded"
        })
        .map(|row| &row.4.0)
        .collect();
    let candidate_rules = matching_many(tx, &candidates, false).await?;
    let ruled: std::collections::HashSet<Uuid> = candidates
        .iter()
        .zip(&candidate_rules)
        .filter(|(_, rules)| !rules.is_empty())
        .map(|(c, _)| c.id)
        .collect();
    for i in live {
        let r = revisions[i];
        result[i] = rows
            .iter()
            .filter(|row| possible(row, r))
            .take(500)
            .map(|row| &row.4.0)
            .filter(|c| {
                c.lifecycle == "active"
                    && c.review != "rejected"
                    && c.content.freshness != "superseded"
                    && overlaps(&c.content, &r.content)
                    && !ruled.contains(&c.id)
            })
            .cloned()
            .collect();
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
