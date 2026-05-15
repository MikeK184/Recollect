use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs, memory,
    memory_evidence::{self, Tx},
    memory_policy, memory_rules as rules, publication,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

pub(crate) fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "review_conflict", message)
}
pub(crate) fn reason(reason: &str) -> Result<()> {
    if reason.trim().is_empty()
        || reason.chars().count() > 2000
        || reason
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(Error::invalid(
            "Provide a review reason of 1–2,000 characters.",
        ));
    }
    Ok(())
}
pub(crate) async fn check_revision(tx: &mut Tx<'_>, old: &ClaimRevision, base: Uuid) -> Result<()> {
    if old.id != base {
        return Err(conflict("This claim changed. Reload before reviewing it."));
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM claim_revisions WHERE brain_id=$1 AND claim_id=$2",
    )
    .bind(old.brain_id)
    .bind(old.claim_id)
    .fetch_one(&mut **tx)
    .await?;
    if count >= 1000 {
        return Err(rules::capacity());
    }
    Ok(())
}
pub(crate) async fn next_time(
    tx: &mut Tx<'_>,
    previous: Option<DateTime<Utc>>,
) -> Result<DateTime<Utc>> {
    Ok(sqlx::query_scalar("SELECT greatest(clock_timestamp(),coalesce($1::timestamptz + interval '1 microsecond',clock_timestamp()))")
        .bind(previous).fetch_one(&mut **tx).await?)
}
pub(crate) async fn revised(
    tx: &mut Tx<'_>,
    auth: &Auth,
    old: &ClaimRevision,
    content: Option<ClaimContent>,
    decision: Uuid,
) -> Result<ClaimRevision> {
    let mut r = old.clone();
    r.id = Uuid::new_v4();
    r.recorded_at = next_time(tx, Some(old.recorded_at)).await?;
    r.reviewer_id = Some(auth.user.id);
    r.review_decision_id = Some(decision);
    r.admission = "reviewed".into();
    if let Some(content) = content {
        if content != old.content {
            r.actor_id = auth.user.id;
            r.actor_name = auth.user.username.clone();
            r.origin = "browser_authored".into();
            r.device_id = None;
            r.operation_id = None;
        }
        r.content = content;
    }
    Ok(r)
}
pub(crate) fn accept_revision(r: &mut ClaimRevision) {
    r.review = "accepted".into();
    r.lifecycle = "active".into();
    r.acceptance_policy = None;
}
pub(crate) async fn require_unblocked(tx: &mut Tx<'_>, r: &ClaimRevision) -> Result<()> {
    if !rules::matching(tx, r, true).await?.is_empty() {
        return Err(conflict(
            "An existing review rule blocks this value. Use explicit revalidation.",
        ));
    }
    Ok(())
}
pub(crate) async fn outcome(
    state: &AppState,
    tx: &mut Tx<'_>,
    decision: ReviewDecision,
) -> Result<ReviewOutcome> {
    let at = next_time(tx, None).await?;
    let mut claims = Vec::new();
    for transition in &decision.transitions {
        let SqlJson(r): SqlJson<ClaimRevision> =
            sqlx::query_scalar("SELECT revision FROM claim_revisions WHERE brain_id=$1 AND id=$2")
                .bind(decision.brain_id)
                .bind(transition.after_revision)
                .fetch_one(&mut **tx)
                .await?;
        claims.push(memory::view(state, tx, r, at, None).await?);
    }
    Ok(ReviewOutcome { decision, claims })
}
pub(crate) async fn finish(
    state: &AppState,
    auth: &Auth,
    tx: &mut Tx<'_>,
    key: Option<&str>,
    decision: ReviewDecision,
) -> Result<ReviewOutcome> {
    sqlx::query(
        "INSERT INTO memory_decisions(id,brain_id,decision,created_at) VALUES($1,$2,$3,$4)",
    )
    .bind(decision.id)
    .bind(decision.brain_id)
    .bind(SqlJson(&decision))
    .bind(decision.recorded_at)
    .execute(&mut **tx)
    .await?;
    for t in &decision.transitions {
        sqlx::query(
            "INSERT INTO memory_decision_claims(decision_id,claim_id,brain_id) VALUES($1,$2,$3)",
        )
        .bind(decision.id)
        .bind(t.claim_id)
        .bind(decision.brain_id)
        .execute(&mut **tx)
        .await?;
    }
    let audit = db::audit(
        tx,
        auth.user.id,
        decision.brain_id,
        "claim.review",
        decision.id,
        &decision.action,
    )
    .await?;
    jobs::enqueue(tx, auth.user.id, decision.brain_id, audit).await?;
    // Replay keeps the immutable decision but recalculates eligibility against current rules.
    commands::finish(tx, key, decision.brain_id, &decision).await?;
    outcome(state, tx, decision).await
}

#[utoipa::path(post,path="/api/brains/{brain}/claims/{claim}/review",operation_id="reviewClaim",params(("brain"=Uuid,Path),("claim"=Uuid,Path)),request_body=ReviewInput,responses((status=200,body=ReviewOutcome)))]
pub async fn review(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(mut input): Json<ReviewInput>,
) -> Result<Json<ReviewOutcome>> {
    auth.require_browser()?;
    input.reason = input.reason.trim().into();
    reason(&input.reason)?;
    if !matches!(
        input.action.as_str(),
        "accept" | "reject" | "withdraw" | "correct" | "revalidate" | "restore"
    ) {
        return Err(Error::invalid("Choose a supported review action."));
    }
    if (input.content.is_some() && !matches!(input.action.as_str(), "correct" | "revalidate"))
        || (input.action == "correct" && input.content.is_none())
        || (input.action != "revalidate" && input.revalidation_basis.is_some())
    {
        return Err(Error::invalid(
            "Correct requires full content; only revalidation accepts a basis.",
        ));
    }
    if let Some(content) = &mut input.content {
        memory_policy::validate(content)?;
    }
    publication::safe_payload(&state, &json!(input))?;
    let key = commands::key(&headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    if let Some(decision) = commands::reserve::<ReviewDecision>(
        &mut tx,
        key.as_deref(),
        "claim.review",
        json!({"brain":brain,"claim":id,"input":input}),
    )
    .await?
    {
        let reply = outcome(&state, &mut tx, decision).await?;
        tx.commit().await?;
        return Ok(Json(reply));
    }
    let old = memory::current(&mut tx, brain, id).await?;
    check_revision(&mut tx, &old, input.base_revision).await?;
    jobs::capacity(&mut tx, brain).await?;
    if let Some(content) = &input.content {
        if !rules::family(&old.content, content) {
            return Err(Error::invalid(
                "A review must keep the same subject and predicate.",
            ));
        }
        memory::validate_evidence(&state, &mut tx, brain, content).await?;
    }
    let decision_id = Uuid::new_v4();
    let mut r = revised(&mut tx, &auth, &old, input.content.clone(), decision_id).await?;
    match input.action.as_str() {
        "accept" => {
            if old.review != "proposed" || old.lifecycle != "active" {
                return Err(conflict(
                    "Only active proposals can be accepted. Use the appropriate review action.",
                ));
            }
            require_unblocked(&mut tx, &r).await?;
            if !rules::conflicts(&mut tx, &r).await?.is_empty() {
                return Err(conflict(
                    "Resolve the overlapping conflicting claims before acceptance.",
                ));
            }
            accept_revision(&mut r);
        }
        "reject" => {
            if old.lifecycle != "active" || old.review == "rejected" {
                return Err(conflict("This claim is already rejected or withdrawn."));
            }
            r.review = "rejected".into();
            r.acceptance_policy = None;
        }
        "withdraw" => {
            if old.lifecycle != "active" {
                return Err(conflict("This claim is already withdrawn."));
            }
            r.lifecycle = "withdrawn".into();
        }
        "correct" => {
            if old.lifecycle != "active" {
                return Err(conflict("Restore or revalidate a withdrawn claim first."));
            }
            if rules::same_value(&old.content, &r.content) {
                return Err(Error::invalid(
                    "A correction must change the assertion value. Use revalidation for changed evidence or applicability.",
                ));
            }
            require_unblocked(&mut tx, &r).await?;
            accept_revision(&mut r);
        }
        "revalidate" => {
            if !rules::same_value(&old.content, &r.content) {
                return Err(Error::invalid(
                    "Revalidation keeps the assertion value; use correction to replace it.",
                ));
            }
            let valid = match input.revalidation_basis.as_deref() {
                Some("changed_evidence") => old.content.supports != r.content.supports,
                Some("changed_applicability") => {
                    old.content.selection != r.content.selection
                        || old.content.validity != r.content.validity
                        || old.content.manifest_revision_id != r.content.manifest_revision_id
                }
                Some("review_correction") => true,
                _ => false,
            };
            if !valid {
                return Err(Error::invalid(
                    "Provide changed evidence, changed applicability, or explicit review correction as the revalidation basis.",
                ));
            }
            accept_revision(&mut r);
        }
        "restore" => {
            if old.lifecycle != "withdrawn" {
                return Err(conflict("Only withdrawn claims can be restored."));
            }
            if rules::matching(&mut tx, &r, true)
                .await?
                .iter()
                .any(|rule| rule.kind != "withdrawal")
            {
                return Err(conflict(
                    "Rejection requires explicit revalidation before restoration.",
                ));
            }
            r.lifecycle = "active".into();
        }
        _ => unreachable!(),
    }
    if matches!(input.action.as_str(), "reject" | "correct" | "withdraw") {
        rules::add(
            &mut tx,
            &old,
            decision_id,
            if input.action == "withdraw" {
                "withdrawal"
            } else {
                "rejected_value"
            },
            r.recorded_at,
        )
        .await?;
    }
    memory::append(&mut tx, &r).await?;
    if matches!(input.action.as_str(), "revalidate" | "restore") {
        rules::exempt(&mut tx, &r, decision_id, input.action == "restore").await?;
    }
    let decision = ReviewDecision {
        id: decision_id,
        brain_id: brain,
        actor_id: auth.user.id,
        actor_name: auth.user.username.clone(),
        action: input.action,
        reason: input.reason,
        revalidation_basis: input.revalidation_basis,
        recorded_at: r.recorded_at,
        transitions: vec![ReviewTransition {
            claim_id: id,
            before_revision: Some(old.id),
            after_revision: r.id,
        }],
    };
    let reply = finish(&state, &auth, &mut tx, key.as_deref(), decision).await?;
    tx.commit().await?;
    Ok(Json(reply))
}

#[derive(Default, serde::Deserialize)]
pub struct ReviewQuery {
    pub offset: Option<i64>,
    pub operation_id: Option<Uuid>,
}

// A conflict resolution's reason can refer to every transition. Return it to a
// scoped reader only when all referenced revision bodies remain applicable and
// retained. Browser history keeps its existing Brain-wide read contract.
const SCOPED_DECISION: &str = "($4::jsonb IS NULL OR NOT EXISTS (
    SELECT 1 FROM jsonb_array_elements(d.decision->'transitions') t
    CROSS JOIN LATERAL (VALUES(t->>'before_revision'),(t->>'after_revision')) v(id)
    WHERE v.id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM claim_revisions r WHERE r.brain_id=d.brain_id AND r.id=v.id::uuid
        AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
        AND r.revision#>'{content,selection}' IS NOT NULL
        AND recollect_recall_scope(r.revision#>'{content,selection}',$4))))";

#[utoipa::path(get,path="/api/brains/{brain}/claims/{claim}/review",operation_id="claimReviewHistory",params(("brain"=Uuid,Path),("claim"=Uuid,Path),("offset"=Option<i64>,Query),("operation_id"=Option<Uuid>,Query)),responses((status=200,body=ReviewHistory)))]
pub async fn history(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ReviewQuery>,
) -> Result<Json<ReviewHistory>> {
    let offset = publication::offset(&publication::Page {
        offset: query.offset,
    })?;
    let mut tx = memory_evidence::read_tx(&state, &auth, brain).await?;
    let scope = memory::inspection::scope(&mut tx, &auth, brain, query.operation_id).await?;
    let r = memory::current(&mut tx, brain, id).await?;
    if let Some(scope) = &scope
        && memory::inspection::mask(&mut tx, scope, [&r.content.selection])
            .await?
            .is_empty()
    {
        return Err(Error::missing());
    }
    let from = format!("FROM memory_decisions d JOIN memory_decision_claims c ON c.decision_id=d.id AND c.brain_id=d.brain_id
        WHERE d.brain_id=$1 AND c.claim_id=$2 AND {SCOPED_DECISION}");
    let total = sqlx::query_scalar(&format!("SELECT count(*) {from} AND $3::bigint>=0"))
        .bind(brain)
        .bind(id)
        .bind(offset)
        .bind(scope.as_ref().map(SqlJson))
        .fetch_one(&mut *tx)
        .await?;
    let decisions = sqlx::query_scalar::<_, SqlJson<ReviewDecision>>(&format!(
        "SELECT d.decision {from} ORDER BY d.created_at DESC,d.id LIMIT 20 OFFSET $3"
    ))
    .bind(brain)
    .bind(id)
    .bind(offset)
    .bind(scope.as_ref().map(SqlJson))
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|d| d.0)
    .collect();
    let mut rules = rules::matching(&mut tx, &r, true).await?;
    if let Some(scope) = &scope {
        let allowed =
            memory::inspection::mask(&mut tx, scope, rules.iter().map(|r| &r.content.selection))
                .await?;
        rules = rules
            .into_iter()
            .enumerate()
            .filter_map(|(i, r)| allowed.contains(&i).then_some(r))
            .collect();
    }
    let blocking = rules::matching(&mut tx, &r, false).await?;
    let exempted_rule_ids = rules
        .iter()
        .filter(|r| !blocking.iter().any(|b| b.id == r.id))
        .map(|r| r.id)
        .collect();
    let mut conflicts = rules::conflicts(&mut tx, &r).await?;
    if let Some(scope) = &scope {
        let allowed = memory::inspection::mask(
            &mut tx,
            scope,
            conflicts.iter().map(|r| &r.content.selection),
        )
        .await?;
        conflicts = conflicts
            .into_iter()
            .enumerate()
            .filter_map(|(i, r)| allowed.contains(&i).then_some(r))
            .collect();
    }
    tx.commit().await?;
    Ok(Json(ReviewHistory {
        decisions,
        rules,
        exempted_rule_ids,
        conflicts,
        total,
        offset,
    }))
}
