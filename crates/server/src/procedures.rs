use crate::auth::Auth;
use crate::{
    AppState,
    error::{Error, Result},
    memory,
    memory_evidence::Tx,
    memory_policy,
};
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

#[utoipa::path(get,path="/api/brains/{brain}/handover-inputs/{revision}",operation_id="handoverInput",params(("brain"=Uuid,Path),("revision"=Uuid,Path)),responses((status=200,body=ClaimView)))]
pub async fn get_input(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ClaimView>> {
    let mut tx = crate::memory_evidence::read_tx(&state, &auth, brain).await?;
    let r = exact(&mut tx, brain, id)
        .await?
        .ok_or_else(crate::retention::unavailable)?;
    if r.content.kind == "handover" {
        return Err(Error::invalid(
            "A handover cannot be used as a contribution.",
        ));
    }
    let view = memory::view(&state, &mut tx, r, Utc::now(), None).await?;
    tx.commit().await?;
    Ok(Json(view))
}

fn strings(values: &[String], max: usize, required: bool) -> Result<()> {
    if values.len() > max || (required && values.is_empty()) {
        return Err(Error::invalid(
            "The memory form has too few or too many entries.",
        ));
    }
    for value in values {
        memory_policy::text(value, 1000, true)?;
    }
    Ok(())
}
pub(crate) fn validate(content: &mut ClaimContent) -> Result<()> {
    if (content.kind == "procedure") != content.procedure.is_some()
        || (content.kind == "handover") != content.handover.is_some()
    {
        return Err(Error::invalid(
            "Select details matching the procedure or handover kind.",
        ));
    }
    if let Some(p) = &mut content.procedure {
        memory_policy::text(&p.conditions, 2000, true)?;
        memory_policy::text(&p.expected_outcome, 2000, true)?;
        strings(&p.steps, 20, true)?;
        if p.observations.len() > 20 {
            return Err(Error::invalid("Record at most 20 procedure observations."));
        }
        for o in &mut p.observations {
            if !matches!(o.result.as_str(), "success" | "failure")
                || o.support_ids.is_empty()
                || o.support_ids.len() > 20
                || o.support_ids
                    .iter()
                    .any(|id| !content.supports.iter().any(|s| s.id == *id))
            {
                return Err(Error::invalid(
                    "Each observation requires success/failure and exact supporting evidence.",
                ));
            }
            memory_policy::timestamp(o.observed_at)?;
            memory_policy::text(&o.conditions, 2000, true)?;
            memory_policy::text(&o.summary, 2000, true)?;
            o.support_ids.sort_unstable();
            o.support_ids.dedup();
        }
        p.observations.sort_by_key(|o| o.observed_at);
    }
    if let Some(h) = &mut content.handover {
        if content.operational != "declared" {
            return Err(Error::invalid(
                "A handover is a declared synthesis, not operational verification.",
            ));
        }
        strings(&h.completed, 12, false)?;
        strings(&h.next_steps, 12, false)?;
        strings(&h.risks, 12, false)?;
        contribution_ids(&mut h.contributions)?;
    }
    Ok(())
}
pub(crate) fn contribution_ids(ids: &mut Vec<Uuid>) -> Result<()> {
    let len = ids.len();
    ids.sort_unstable();
    ids.dedup();
    if len != ids.len() || len == 0 || len > 12 {
        return Err(Error::invalid(
            "Select 1–12 distinct contributing revisions.",
        ));
    }
    Ok(())
}
pub(crate) async fn exact(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<Option<ClaimRevision>> {
    let value: Option<SqlJson<ClaimRevision>> = sqlx::query_scalar("SELECT CASE WHEN recollect_content_state(brain_id,'claim',privacy_state,recorded_at)='active' THEN revision END FROM claim_revisions WHERE brain_id=$1 AND id=$2")
        .bind(brain).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    Ok(value.map(|v| v.0))
}
pub(crate) async fn inputs(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    ids: &[Uuid],
) -> Result<Vec<ClaimRevision>> {
    dependency_inputs(state, tx, brain, ids, true).await
}
async fn dependency_inputs(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    ids: &[Uuid],
    require_current: bool,
) -> Result<Vec<ClaimRevision>> {
    let mut revisions = Vec::new();
    for id in ids {
        let revision = exact(tx, brain, *id)
            .await?
            .ok_or_else(crate::retention::unavailable)?;
        if revision.content.kind == "handover"
            || (require_current && memory::current(tx, brain, revision.claim_id).await?.id != *id)
        {
            return Err(Error::invalid(
                "Use current claims, decisions or procedures as handover contributions.",
            ));
        }
        if !require_current {
            let supported: bool = sqlx::query_scalar("SELECT recollect_memory_exact_acyclic($1,$2) AND NOT EXISTS(SELECT 1 FROM recollect_memory_exact_dependencies($1,$2) d WHERE NOT recollect_revision_supported($1,d.revision_id))")
                .bind(brain).bind(revision.id).fetch_one(&mut **tx).await?;
            if !supported {
                return Err(Error::invalid(
                    "An exact contribution is no longer supported.",
                ));
            }
            revisions.push(revision);
            continue;
        }
        let view = memory::base_view(state, tx, revision, Utc::now(), None).await?;
        if !view.eligibility.investigation {
            return Err(Error::invalid(
                "A contribution is no longer eligible for this handover.",
            ));
        }
        revisions.push(view.revision);
    }
    Ok(revisions)
}
pub(crate) fn combined(revisions: &[ClaimRevision]) -> Result<(ScopeSelection, Vec<ClaimSupport>)> {
    let first = revisions
        .first()
        .ok_or_else(|| Error::invalid("A handover needs contributions."))?;
    let mut scope = first.content.selection.clone();
    let mut supports: Vec<ClaimSupport> = Vec::new();
    for r in revisions {
        if r.content.selection.environment_id != scope.environment_id {
            return Err(Error::invalid(
                "Contributions from different environments require separate handovers.",
            ));
        }
        scope
            .repository_ids
            .extend(&r.content.selection.repository_ids);
        scope.area_ids.extend(&r.content.selection.area_ids);
        for support in &r.content.supports {
            if let Some(previous) = supports
                .iter()
                .find(|s| s.kind == support.kind && s.id == support.id)
            {
                if previous != support {
                    return Err(Error::invalid(
                        "Contributions use conflicting spans for the same evidence.",
                    ));
                }
            } else {
                supports.push(support.clone());
            }
        }
    }
    scope.repository_ids.sort_unstable();
    scope.repository_ids.dedup();
    scope.area_ids.sort_unstable();
    scope.area_ids.dedup();
    supports.sort_by_key(|s| (s.kind.clone(), s.id));
    if supports.len() > 20 {
        return Err(Error::invalid(
            "The combined handover exceeds 20 evidence supports.",
        ));
    }
    Ok((scope, supports))
}
pub(crate) async fn validate_dependencies(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
) -> Result<()> {
    validate_dependency_content(state, tx, brain, content, true).await
}
pub(crate) async fn validate_audit_dependencies(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
) -> Result<()> {
    validate_dependency_content(state, tx, brain, content, false).await
}
async fn validate_dependency_content(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
    require_current: bool,
) -> Result<()> {
    if let Some(handover) = &content.handover {
        let rows =
            dependency_inputs(state, tx, brain, &handover.contributions, require_current).await?;
        let (scope, supports) = combined(&rows)?;
        if scope != content.selection || supports.iter().any(|s| !content.supports.contains(s)) {
            return Err(Error::invalid(
                "Preserve every contribution's combined scope and exact evidence.",
            ));
        }
    }
    Ok(())
}
pub(crate) async fn append(tx: &mut Tx<'_>, r: &ClaimRevision) -> Result<()> {
    if let Some(h) = &r.content.handover {
        for id in &h.contributions {
            let input = exact(tx, r.brain_id, *id)
                .await?
                .ok_or_else(crate::retention::unavailable)?;
            if input.claim_id == r.claim_id || input.content.kind == "handover" {
                return Err(Error::invalid(
                    "Handovers cannot reference themselves or other handovers.",
                ));
            }
            sqlx::query("INSERT INTO claim_contributions(revision_id,brain_id,input_revision_id) VALUES($1,$2,$3)")
                .bind(r.id).bind(r.brain_id).bind(id).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
fn qualify(eligibility: &mut ClaimEligibility, reason: &str, exclude: bool) {
    eligibility.strict_accepted = false;
    eligibility.strict_operational = false;
    if exclude {
        eligibility.investigation = false;
    }
    if eligibility.effective_freshness != "superseded" {
        eligibility.effective_freshness = "needs_verification".into();
    }
    if !eligibility.reasons.iter().any(|r| r == reason) {
        eligibility.reasons.push(reason.into());
    }
}
pub(crate) async fn assess(
    state: &AppState,
    tx: &mut Tx<'_>,
    view: &mut ClaimView,
    at: DateTime<Utc>,
    fact_at: Option<DateTime<Utc>>,
) -> Result<()> {
    if let Some(p) = &view.revision.content.procedure
        && p.observations.last().is_none_or(|o| o.result != "success")
    {
        view.eligibility.strict_operational = false;
        view.eligibility
            .reasons
            .push("procedure_success_not_recorded".into());
    }
    let Some(h) = &view.revision.content.handover else {
        return Ok(());
    };
    let ids = h.contributions.clone();
    let brain = view.revision.brain_id;
    for id in ids {
        let state_row: Option<(Uuid, String)> = sqlx::query_as("SELECT claim_id,recollect_content_state(brain_id,'claim',privacy_state,recorded_at) FROM claim_revisions WHERE brain_id=$1 AND id=$2")
            .bind(brain).bind(id).fetch_optional(&mut **tx).await?;
        let Some((claim, privacy)) = state_row else {
            qualify(&mut view.eligibility, "contribution_unavailable", true);
            view.contributions.push(ClaimContribution {
                revision_id: id,
                claim_id: None,
                state: "unavailable".into(),
                revision: None,
                eligibility: None,
            });
            continue;
        };
        if privacy != "active" {
            qualify(
                &mut view.eligibility,
                "contribution_unavailable",
                privacy == "erased",
            );
            view.contributions.push(ClaimContribution {
                revision_id: id,
                claim_id: Some(claim),
                state: privacy,
                revision: None,
                eligibility: None,
            });
            continue;
        }
        let original = exact(tx, brain, id)
            .await?
            .ok_or_else(crate::retention::unavailable)?;
        if original.content.kind == "handover" {
            qualify(&mut view.eligibility, "invalid_contribution", true);
            continue;
        }
        let original = memory::base_view(state, tx, original, at, fact_at).await?;
        let latest: Option<Uuid> = sqlx::query_scalar("SELECT id FROM claim_revisions WHERE brain_id=$1 AND claim_id=$2 AND recorded_at<=$3 ORDER BY recorded_at DESC LIMIT 1")
            .bind(brain).bind(claim).bind(at).fetch_optional(&mut **tx).await?;
        let changed = latest != Some(id);
        if changed {
            qualify(&mut view.eligibility, "contribution_changed", false);
        }
        if !original.eligibility.investigation {
            qualify(&mut view.eligibility, "contribution_ineligible", true);
        } else if !original.eligibility.strict_accepted {
            view.eligibility.strict_accepted = false;
            view.eligibility.strict_operational = false;
            if !view
                .eligibility
                .reasons
                .iter()
                .any(|r| r == "contribution_not_strictly_accepted")
            {
                view.eligibility
                    .reasons
                    .push("contribution_not_strictly_accepted".into());
            }
            if original.eligibility.effective_freshness != "current" {
                qualify(
                    &mut view.eligibility,
                    "contribution_needs_verification",
                    false,
                );
            }
        }
        if let Some(latest) = latest.filter(|latest| *latest != id) {
            if let Some(current) = exact(tx, brain, latest).await? {
                if current.content.kind == "handover"
                    || !memory::base_view(state, tx, current, at, fact_at)
                        .await?
                        .eligibility
                        .investigation
                {
                    qualify(&mut view.eligibility, "contribution_ineligible", true);
                }
            } else {
                qualify(&mut view.eligibility, "contribution_unavailable", true);
            }
        }
        view.contributions.push(ClaimContribution {
            revision_id: id,
            claim_id: Some(claim),
            state: if changed { "changed" } else { "current" }.into(),
            revision: Some(original.revision),
            eligibility: Some(original.eligibility),
        });
    }
    Ok(())
}
