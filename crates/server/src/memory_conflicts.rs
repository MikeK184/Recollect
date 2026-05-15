use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs, memory, memory_policy, memory_review as review, memory_rules as rules, publication,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use recollect_protocol::*;
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

#[utoipa::path(post,path="/api/brains/{brain}/claim-conflicts/resolve",operation_id="resolveClaimConflict",params(("brain"=Uuid,Path)),request_body=ConflictResolution,responses((status=200,body=ReviewOutcome)))]
pub async fn resolve(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(mut input): Json<ConflictResolution>,
) -> Result<Json<ReviewOutcome>> {
    auth.require_browser()?;
    input.reason = input.reason.trim().into();
    review::reason(&input.reason)?;
    if !matches!(
        input.disposition.as_str(),
        "keep_selected" | "keep_both" | "retract" | "replace"
    ) || !(2..=20).contains(&input.participants.len())
        || input
            .participants
            .iter()
            .map(|p| p.claim_id)
            .collect::<HashSet<_>>()
            .len()
            != input.participants.len()
        || ((input.disposition == "keep_selected") != input.selected_id.is_some())
        || ((input.disposition == "replace") != input.replacement.is_some())
        || input
            .selected_id
            .is_some_and(|id| !input.participants.iter().any(|p| p.claim_id == id))
        || input
            .participants
            .iter()
            .any(|p| (input.disposition == "keep_both") != p.content.is_some())
    {
        return Err(Error::invalid(
            "Select 2–20 distinct claims and the inputs required by the conflict disposition.",
        ));
    }
    input.participants.sort_by_key(|p| p.claim_id);
    for content in input
        .participants
        .iter_mut()
        .filter_map(|p| p.content.as_mut())
        .chain(input.replacement.iter_mut())
    {
        memory_policy::validate(content)?;
    }
    publication::safe_payload(&state, &json!(input))?;
    let key = commands::key(&headers)?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_writer(&mut tx, brain).await?;
    if let Some(decision) = commands::reserve::<ReviewDecision>(
        &mut tx,
        key.as_deref(),
        "claim.resolve",
        json!({"brain":brain,"input":input}),
    )
    .await?
    {
        let reply = review::outcome(&state, &mut tx, decision).await?;
        tx.commit().await?;
        return Ok(Json(reply));
    }
    jobs::capacity(&mut tx, brain).await?;
    let mut previous = Vec::new();
    for participant in &input.participants {
        let old = memory::current(&mut tx, brain, participant.claim_id).await?;
        review::check_revision(&mut tx, &old, participant.base_revision).await?;
        if old.lifecycle != "active"
            || old.review == "rejected"
            || old.content.freshness == "superseded"
            || !rules::matching(&mut tx, &old, false).await?.is_empty()
        {
            return Err(review::conflict(
                "A participant is inactive or blocked by a review rule. Review it individually first.",
            ));
        }
        previous.push(old);
    }
    if previous
        .iter()
        .any(|r| !rules::family(&previous[0].content, &r.content))
    {
        return Err(Error::invalid(
            "Conflict participants must share a subject and predicate.",
        ));
    }
    if !previous.iter().enumerate().any(|(i, a)| {
        previous.iter().skip(i + 1).any(|b| {
            !rules::same_value(&a.content, &b.content) && rules::overlaps(&a.content, &b.content)
        })
    }) {
        return Err(review::conflict(
            "The selected claims no longer have an overlapping value conflict.",
        ));
    }
    let decision_id = Uuid::new_v4();
    let mut revisions = Vec::new();
    for (old, participant) in previous.iter().zip(&input.participants) {
        if let Some(content) = &participant.content {
            if !rules::family(&old.content, content) || !rules::same_value(&old.content, content) {
                return Err(Error::invalid(
                    "Keeping both changes applicability, while preserving each assertion value.",
                ));
            }
            memory::validate_evidence(&state, &mut tx, brain, content).await?;
        }
        let mut r = review::revised(
            &mut tx,
            &auth,
            old,
            participant.content.clone(),
            decision_id,
        )
        .await?;
        match input.disposition.as_str() {
            "keep_selected" if Some(old.claim_id) == input.selected_id => {
                if previous.iter().any(|p| {
                    p.claim_id != old.claim_id && rules::same_value(&p.content, &old.content)
                }) {
                    return Err(Error::invalid(
                        "The selected value must differ from every rejected participant.",
                    ));
                }
                review::require_unblocked(&mut tx, &r).await?;
                review::accept_revision(&mut r);
            }
            "keep_both" => {
                review::require_unblocked(&mut tx, &r).await?;
                review::accept_revision(&mut r);
            }
            "retract" => r.lifecycle = "withdrawn".into(),
            _ => {
                r.review = "rejected".into();
                r.acceptance_policy = None;
            }
        }
        revisions.push(r);
    }
    if input.disposition == "keep_both"
        && revisions.iter().enumerate().any(|(i, a)| {
            revisions
                .iter()
                .skip(i + 1)
                .any(|b| rules::overlaps(&a.content, &b.content))
        })
    {
        return Err(Error::invalid(
            "Specify disjoint repositories, environments or fact-time intervals before keeping both.",
        ));
    }
    let replacement = if let Some(content) = input.replacement {
        if !rules::family(&previous[0].content, &content)
            || previous
                .iter()
                .any(|r| rules::same_value(&r.content, &content))
        {
            return Err(Error::invalid(
                "A replacement keeps the subject/property and differs from every rejected value.",
            ));
        }
        memory::validate_evidence(&state, &mut tx, brain, &content).await?;
        rules::check_family(&mut tx, brain, &content, None).await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
        if count >= 5000 {
            return Err(rules::capacity());
        }
        let mut r =
            review::revised(&mut tx, &auth, &previous[0], Some(content), decision_id).await?;
        r.claim_id = Uuid::new_v4();
        review::require_unblocked(&mut tx, &r).await?;
        review::accept_revision(&mut r);
        Some(r)
    } else {
        None
    };
    // All participant bases and complete replacement inputs are validated before appending.
    let mut transitions = Vec::new();
    for (old, r) in previous.iter().zip(&revisions) {
        if r.review == "rejected" || r.lifecycle == "withdrawn" {
            rules::add(
                &mut tx,
                old,
                decision_id,
                if r.lifecycle == "withdrawn" {
                    "withdrawal"
                } else {
                    "rejected_value"
                },
                r.recorded_at,
            )
            .await?;
        }
        memory::append(&mut tx, r).await?;
        transitions.push(ReviewTransition {
            claim_id: r.claim_id,
            before_revision: Some(old.id),
            after_revision: r.id,
        });
    }
    if let Some(r) = replacement {
        sqlx::query("INSERT INTO claims(id,brain_id,created_by) VALUES($1,$2,$3)")
            .bind(r.claim_id)
            .bind(brain)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await?;
        memory::append(&mut tx, &r).await?;
        transitions.push(ReviewTransition {
            claim_id: r.claim_id,
            before_revision: None,
            after_revision: r.id,
        });
    }
    let recorded_at = review::next_time(&mut tx, None).await?;
    let decision = ReviewDecision {
        id: decision_id,
        brain_id: brain,
        actor_id: auth.user.id,
        actor_name: auth.user.username.clone(),
        action: input.disposition,
        reason: input.reason,
        revalidation_basis: None,
        transitions,
        recorded_at,
    };
    let reply = review::finish(&state, &auth, &mut tx, key.as_deref(), decision).await?;
    tx.commit().await?;
    Ok(Json(reply))
}
