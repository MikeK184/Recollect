use crate::db::admission::{self, BrainAccess};
use crate::{
    AppState, artifacts,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    jobs,
    memory_evidence::{self as evidence, Tx},
    memory_policy::{self as policy, Assessment},
    memory_rules, publication, workspace,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

fn conflict(message: &'static str) -> Error {
    Error(StatusCode::CONFLICT, "claim_conflict", message)
}
fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "claim_capacity",
        "Claim or history capacity reached.",
    )
}
pub(crate) async fn manifest(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ManifestRevision> {
    let r: Option<SqlJson<ManifestRevision>> =
        sqlx::query_scalar("SELECT CASE WHEN privacy_state='active' THEN revision END FROM manifest_revisions WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(Error::missing)?;
    Ok(r.ok_or_else(crate::retention::unavailable)?.0)
}
pub(crate) async fn current(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<ClaimRevision> {
    let r: Option<SqlJson<ClaimRevision>> = sqlx::query_scalar("SELECT CASE WHEN recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active' THEN r.revision END FROM claims c JOIN claim_revisions r ON r.id=c.current_revision WHERE c.brain_id=$1 AND c.id=$2")
        .bind(brain).bind(id).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    Ok(r.ok_or_else(crate::retention::unavailable)?.0)
}
pub(crate) async fn validate_evidence(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
) -> Result<()> {
    validate_supports(state, tx, brain, content).await?;
    crate::procedures::validate_dependencies(state, tx, brain, content).await
}
pub(crate) async fn validate_evidence_for_audit(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
) -> Result<()> {
    validate_supports(state, tx, brain, content).await?;
    crate::procedures::validate_audit_dependencies(state, tx, brain, content).await
}
async fn validate_supports(
    state: &AppState,
    tx: &mut Tx<'_>,
    brain: Uuid,
    content: &ClaimContent,
) -> Result<()> {
    if !workspace::selection_valid(tx, brain, &content.selection).await? {
        return Err(Error::missing());
    }
    let selected = if let Some(id) = content.manifest_revision_id {
        let selected = manifest(tx, brain, id).await?;
        if Some(selected.environment_id) != content.selection.environment_id
            || content
                .selection
                .repository_ids
                .iter()
                .any(|id| !selected.entries.iter().any(|e| e.repository_id == *id))
        {
            return Err(Error::invalid(
                "The manifest must match this environment and contain every selected repository.",
            ));
        }
        Some(selected)
    } else {
        None
    };
    for support in &content.supports {
        let row = evidence::row(tx, brain, &support.kind, support.id).await?;
        if row.privacy() != "active" {
            return Err(crate::retention::unavailable());
        }
        if let Some(repo) = row.repository_id {
            if !content.selection.repository_ids.contains(&repo) {
                return Err(Error::invalid(
                    "Include each supporting repository in the claim's applicability.",
                ));
            }
            if selected.as_ref().is_some_and(|m| {
                !m.entries
                    .iter()
                    .any(|e| e.repository_id == repo && e.snapshot_id == row.snapshot_id)
            }) {
                return Err(Error::invalid(
                    "Supporting facts must use the manifest's exact selected snapshots.",
                ));
            }
        }
        if let Some(to) = support.line_to {
            let artifact = row
                .artifact_id
                .ok_or_else(|| Error::invalid("A line span requires retained source text."))?;
            let text =
                artifacts::read(&state.config.artifact_dir, brain, artifact, row.byte_length)
                    .await
                    .map_err(|_| {
                        Error::invalid("The source text for this line span is unavailable.")
                    })?;
            if to as usize > text.lines().count() {
                return Err(Error::invalid(
                    "The line span extends past the retained source.",
                ));
            }
        }
    }
    Ok(())
}
#[utoipa::path(post,path="/api/brains/{brain}/claims",operation_id="createClaim",params(("brain"=Uuid,Path)),request_body=ClaimInput,responses((status=200,body=ClaimRevision)))]
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<ClaimInput>,
) -> Result<Json<ClaimRevision>> {
    save(&state, &auth, brain, None, &headers, input)
        .await
        .map(Json)
}
#[utoipa::path(put,path="/api/brains/{brain}/claims/{claim}",operation_id="updateClaim",params(("brain"=Uuid,Path),("claim"=Uuid,Path)),request_body=ClaimInput,responses((status=200,body=ClaimRevision)))]
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ClaimInput>,
) -> Result<Json<ClaimRevision>> {
    save(&state, &auth, brain, Some(id), &headers, input)
        .await
        .map(Json)
}
async fn save(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    existing: Option<Uuid>,
    headers: &HeaderMap,
    mut input: ClaimInput,
) -> Result<ClaimRevision> {
    policy::validate(&mut input.content)?;
    publication::safe_payload(state, &json!(input))?;
    let key = commands::key(headers)?;
    let mut tx = admission::brain_tx(&state.pool, auth, brain, BrainAccess::WRITE)
        .await?
        .tx;
    validate_evidence(state, &mut tx, brain, &input.content).await?;
    if auth.device_id.is_some() && input.operation_id.is_none() {
        return Err(Error::forbidden());
    }
    if let Some(id) = input.operation_id {
        let op = publication::operation(
            &mut tx,
            brain,
            id,
            auth.user.id,
            auth.device_id,
            &input.content.selection,
            true,
        )
        .await?;
        if op.kind != "write" || op.scope.selection != input.content.selection {
            return Err(Error::forbidden());
        }
    }
    if let Some(reply) = commands::reserve::<ClaimRevision>(
        &mut tx,
        key.as_deref(),
        "claim.propose",
        json!({"brain":brain,"claim":existing,"input":input}),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(reply);
    }
    let previous = if let Some(id) = existing {
        let previous = current(&mut tx, brain, id).await?;
        if input.base_revision != Some(previous.id) {
            return Err(conflict(
                "This claim changed. Reload its latest revision before saving.",
            ));
        }
        if previous.review != "proposed" || previous.lifecycle != "active" {
            return Err(conflict(
                "A reviewed claim requires an explicit review or correction command.",
            ));
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM claim_revisions WHERE claim_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if count >= 1000 {
            return Err(capacity());
        }
        Some(previous)
    } else {
        if input.base_revision.is_some() {
            return Err(Error::invalid("A new claim has no base revision."));
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM claims WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
        if count >= 5000 {
            return Err(capacity());
        }
        None
    };
    jobs::capacity(&mut tx, brain).await?;
    crate::memory_rules::check_family(&mut tx, brain, &input.content, existing).await?;
    let claim_id = previous
        .as_ref()
        .map(|p| p.claim_id)
        .unwrap_or_else(Uuid::new_v4);
    let recorded_at: DateTime<Utc> = sqlx::query_scalar("SELECT greatest(clock_timestamp(),coalesce($1::timestamptz + interval '1 microsecond',clock_timestamp()))")
        .bind(previous.as_ref().map(|p| p.recorded_at)).fetch_one(&mut *tx).await?;
    let mut revision = ClaimRevision {
        id: Uuid::new_v4(),
        claim_id,
        brain_id: brain,
        actor_id: auth.user.id,
        actor_name: auth.user.username.clone(),
        device_id: auth.device_id,
        operation_id: input.operation_id,
        origin: if auth.device_id.is_some() {
            "device_authored"
        } else {
            "browser_authored"
        }
        .into(),
        recorded_at,
        content: input.content,
        review: "proposed".into(),
        reviewer_id: None,
        acceptance_policy: None,
        derivation: None,
        lifecycle: "active".into(),
        review_decision_id: None,
        admission: "proposed".into(),
    };
    if !crate::memory_rules::matching(&mut tx, &revision, false)
        .await?
        .is_empty()
    {
        revision.admission = "blocked_by_rule".into();
    }
    if previous.is_none() {
        sqlx::query("INSERT INTO claims(id,brain_id,created_by) VALUES($1,$2,$3)")
            .bind(claim_id)
            .bind(brain)
            .bind(auth.user.id)
            .execute(&mut *tx)
            .await?;
    }
    append(&mut tx, &revision).await?;
    let audit = db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "claim.propose",
        claim_id,
        if previous.is_some() {
            "revision_proposed"
        } else {
            "proposed"
        },
    )
    .await?;
    jobs::enqueue(&mut tx, auth.user.id, brain, audit).await?;
    commands::finish(&mut tx, key.as_deref(), brain, &revision).await?;
    tx.commit().await?;
    Ok(revision)
}
pub(crate) async fn append(tx: &mut Tx<'_>, r: &ClaimRevision) -> Result<()> {
    sqlx::query("INSERT INTO claim_revisions(id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(r.id).bind(r.claim_id).bind(r.brain_id).bind(r.recorded_at).bind(SqlJson(r))
        .bind(policy::assertion_key(&r.content.subject,true)).bind(policy::assertion_key(&r.content.predicate,true)).bind(policy::assertion_key(&r.content.value,false)).execute(&mut **tx).await?;
    for (ordinal, s) in r.content.supports.iter().enumerate() {
        sqlx::query("INSERT INTO claim_supports(revision_id,brain_id,ordinal,source_version_id,fact_id,manifest_revision_id) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(r.id).bind(r.brain_id).bind(ordinal as i32)
            .bind((s.kind == "source_version").then_some(s.id)).bind((s.kind == "repository_fact").then_some(s.id)).bind((s.kind == "manifest_revision").then_some(s.id)).execute(&mut **tx).await?;
    }
    crate::procedures::append(tx, r).await?;
    sqlx::query("UPDATE claims SET current_revision=$2 WHERE id=$1 AND brain_id=$3")
        .bind(r.claim_id)
        .bind(r.id)
        .bind(r.brain_id)
        .execute(&mut **tx)
        .await?;
    crate::memory_rules::advance_epoch(tx, r.brain_id).await?;
    Ok(())
}

pub(crate) mod inspection;

#[derive(Default, Deserialize)]
pub struct ClaimQuery {
    pub q: Option<String>,
    pub operation_id: Option<Uuid>,
    pub kind: Option<String>,
    pub repository_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub fact_at: Option<DateTime<Utc>>,
    pub knowledge_at: Option<DateTime<Utc>>,
    pub mode: Option<String>,
    pub offset: Option<i64>,
}
async fn query_time(tx: &mut Tx<'_>, query: &ClaimQuery) -> Result<DateTime<Utc>> {
    if query.kind.as_ref().is_some_and(|kind| {
        !matches!(
            kind.as_str(),
            "claim" | "decision" | "procedure" | "handover"
        )
    }) {
        return Err(Error::invalid(
            "Choose claim, decision, procedure or handover.",
        ));
    }
    for at in [query.fact_at, query.knowledge_at].into_iter().flatten() {
        policy::timestamp(at)?;
    }
    if query.mode.as_ref().is_some_and(|m| {
        !matches!(
            m.as_str(),
            "investigation" | "strict_accepted" | "strict_operational" | "history"
        )
    }) {
        return Err(Error::invalid(
            "Choose investigation, strict accepted, strict operational or history.",
        ));
    }
    if let Some(at) = query.knowledge_at {
        Ok(at)
    } else {
        Ok(sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut **tx)
            .await?)
    }
}
async fn query_scope(tx: &mut Tx<'_>, brain: Uuid, query: &ClaimQuery) -> Result<()> {
    let scope = ScopeSelection {
        repository_ids: query.repository_id.into_iter().collect(),
        area_ids: query.area_id.into_iter().collect(),
        environment_id: query.environment_id,
    };
    if !workspace::selection_valid(tx, brain, &scope).await? {
        return Err(Error::missing());
    }
    Ok(())
}
pub(crate) async fn base_view(
    state: &AppState,
    tx: &mut Tx<'_>,
    r: ClaimRevision,
    at: DateTime<Utc>,
    fact_at: Option<DateTime<Utc>>,
) -> Result<ClaimView> {
    base_views(
        state,
        tx,
        &mut evidence::EvidenceCache::default(),
        vec![r],
        at,
        fact_at,
        false,
    )
    .await?
    .pop()
    .expect("one view per revision")
}
/// Canonical eligibility for many revisions of one Brain in a fixed number of
/// statements (supports, source/manifest currency, support audit, rules,
/// conflicts, knowledge end), independent of the number of revisions. The
/// outer error is a database failure; each inner result is that revision's own
/// outcome (for example a support that no longer exists), exactly what
/// `base_view` returned for it alone. `presupported` is for callers whose
/// revisions were selected by the canonical candidate chain, which already
/// requires `recollect_memory_supported` in this transaction: the audit
/// function is expensive under row-level security and is not asked twice.
pub(crate) async fn base_views(
    state: &AppState,
    tx: &mut Tx<'_>,
    cache: &mut evidence::EvidenceCache,
    revisions: Vec<ClaimRevision>,
    at: DateTime<Utc>,
    fact_at: Option<DateTime<Utc>>,
    presupported: bool,
) -> Result<Vec<Result<ClaimView>>> {
    let Some(brain) = revisions.first().map(|r| r.brain_id) else {
        return Ok(Vec::new());
    };
    let scopes: Vec<&ScopeSelection> = revisions.iter().map(|r| &r.content.selection).collect();
    let scope_valid = workspace::selections_valid(tx, brain, &scopes).await?;
    cache
        .load(
            tx,
            brain,
            revisions
                .iter()
                .flat_map(|r| &r.content.supports)
                .map(|s| (s.kind.clone(), s.id)),
        )
        .await?;
    // A source support is current when it is still the newest known version of
    // its source at this time; a manifest when its entries are unchanged.
    let sources: Vec<Uuid> = revisions
        .iter()
        .flat_map(|r| &r.content.supports)
        .filter_map(|s| cache.row(&s.kind, s.id)?.source_id)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let latest_sources: std::collections::HashMap<Uuid, Uuid> = if sources.is_empty() {
        Default::default()
    } else {
        sqlx::query_as::<_, (Uuid, Uuid)>("SELECT DISTINCT ON(source_id) source_id,id FROM recollect_source_knowledge WHERE brain_id=$1 AND source_id=ANY($2) AND recorded_at<=$3 ORDER BY source_id,recorded_at DESC,id DESC")
            .bind(brain).bind(&sources).bind(at).fetch_all(&mut **tx).await?.into_iter().collect()
    };
    let manifest_ids: Vec<Uuid> = revisions
        .iter()
        .filter_map(|r| r.content.manifest_revision_id)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut originals: std::collections::HashMap<Uuid, Option<ManifestRevision>> =
        Default::default();
    let mut latest_manifests: std::collections::HashMap<Uuid, Option<ManifestRevision>> =
        Default::default();
    if !manifest_ids.is_empty() {
        let rows: Vec<(Uuid, Option<SqlJson<ManifestRevision>>)> = sqlx::query_as("SELECT id,CASE WHEN privacy_state='active' THEN revision END FROM manifest_revisions WHERE brain_id=$1 AND id=ANY($2)")
            .bind(brain).bind(&manifest_ids).fetch_all(&mut **tx).await?;
        originals.extend(rows.into_iter().map(|(id, m)| (id, m.map(|m| m.0))));
        let logical: Vec<Uuid> = originals
            .values()
            .flatten()
            .map(|m| m.manifest_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let rows: Vec<(Uuid, Option<SqlJson<ManifestRevision>>)> = sqlx::query_as("SELECT DISTINCT ON(manifest_id) manifest_id,CASE WHEN privacy_state='active' THEN revision END FROM manifest_revisions WHERE brain_id=$1 AND manifest_id=ANY($2) AND created_at<=$3 ORDER BY manifest_id,created_at DESC,id DESC")
            .bind(brain).bind(&logical).bind(at).fetch_all(&mut **tx).await?;
        latest_manifests.extend(rows.into_iter().map(|(id, m)| (id, m.map(|m| m.0))));
    }
    let ids: Vec<Uuid> = revisions.iter().map(|r| r.id).collect();
    let supported: std::collections::HashMap<Uuid, bool> = if presupported {
        ids.iter().map(|id| (*id, true)).collect()
    } else {
        memory_rules::supported(tx, brain, &ids).await?
    };
    let refs: Vec<&ClaimRevision> = revisions.iter().collect();
    let rules = memory_rules::matching_many(tx, &refs, false).await?;
    let blocked: Vec<bool> = rules.iter().map(|r| !r.is_empty()).collect();
    let conflicts = memory_rules::conflicts_many(tx, &refs, &blocked).await?;
    let ends: std::collections::HashMap<Uuid, Option<DateTime<Utc>>> = sqlx::query_as::<_, (Uuid, Option<DateTime<Utc>>)>(
        "SELECT t.id,(SELECT min(recorded_at) FROM claim_revisions c WHERE c.claim_id=t.claim_id AND c.recorded_at>t.recorded_at) FROM unnest($1::uuid[],$2::uuid[],$3::timestamptz[]) AS t(id,claim_id,recorded_at)",
    )
    .bind(&ids)
    .bind(revisions.iter().map(|r| r.claim_id).collect::<Vec<_>>())
    .bind(revisions.iter().map(|r| r.recorded_at).collect::<Vec<_>>())
    .fetch_all(&mut **tx).await?.into_iter().collect();
    let mut views = Vec::with_capacity(revisions.len());
    for (index, r) in revisions.into_iter().enumerate() {
        let mut choices = Vec::new();
        let mut changed = false;
        let mut missing = false;
        let mut available = true;
        let mut failure = None;
        for support in &r.content.supports {
            let Some(row) = cache.row(&support.kind, support.id) else {
                failure = Some(Error::missing());
                break;
            };
            let source_id = row.source_id;
            let row_id = row.id;
            let choice = cache
                .choice(state, brain, &support.kind, support.id)
                .await
                .expect("loaded evidence row");
            available &= choice.availability == "retained";
            missing |= !matches!(choice.availability.as_str(), "retained" | "reference_only");
            if let Some(source) = source_id {
                changed |= latest_sources.get(&source) != Some(&row_id);
            }
            choices.push(choice);
        }
        if let Some(e) = failure {
            views.push(Err(e));
            continue;
        }
        if let Some(id) = r.content.manifest_revision_id {
            let original = match originals.get(&id) {
                None => {
                    views.push(Err(Error::missing()));
                    continue;
                }
                Some(None) => {
                    views.push(Err(crate::retention::unavailable()));
                    continue;
                }
                Some(Some(original)) => original,
            };
            let latest = latest_manifests
                .get(&original.manifest_id)
                .and_then(Option::as_ref);
            changed |= latest.is_none_or(|latest| {
                r.content.selection.repository_ids.iter().any(|repo| {
                    let before = original.entries.iter().find(|e| e.repository_id == *repo);
                    let after = latest.entries.iter().find(|e| e.repository_id == *repo);
                    match (before, after) {
                        (Some(a), Some(b)) => {
                            a.revision != b.revision
                                || a.snapshot_id != b.snapshot_id
                                || a.config_paths != b.config_paths
                        }
                        _ => true,
                    }
                })
            });
        }
        let mut eligibility = policy::eligibility(
            &r,
            Assessment {
                scope_valid: scope_valid[index],
                support_available: available,
                evidence_changed: changed,
                evidence_missing: missing,
                fact_time: policy::fact_match(&r.content.validity, fact_at),
            },
        );
        if supported.get(&r.id) != Some(&true) {
            eligibility.investigation = false;
            eligibility.strict_accepted = false;
            eligibility.strict_operational = false;
            eligibility
                .reasons
                .push("source_support_not_current".into());
        }
        eligibility.rule_ids = rules[index].iter().map(|rule| rule.id).collect();
        if !eligibility.rule_ids.is_empty() {
            eligibility.investigation = false;
            eligibility.strict_accepted = false;
            eligibility.strict_operational = false;
            eligibility.reasons.push("blocked_by_review_rule".into());
        }
        eligibility.conflicting_claim_ids = conflicts[index].iter().map(|c| c.claim_id).collect();
        if !eligibility.conflicting_claim_ids.is_empty() {
            eligibility.strict_accepted = false;
            eligibility.strict_operational = false;
            eligibility.reasons.push("unresolved_conflict".into());
        }
        views.push(Ok(ClaimView {
            knowledge_until: ends.get(&r.id).copied().flatten(),
            revision: r,
            eligibility,
            evidence: choices,
            contributions: Vec::new(),
        }));
    }
    Ok(views)
}
/// `view` for many revisions: set-based base eligibility, then each revision's
/// procedure/handover assessment. Per-revision failures stay per revision.
pub(crate) async fn views(
    state: &AppState,
    tx: &mut Tx<'_>,
    cache: &mut evidence::EvidenceCache,
    revisions: Vec<ClaimRevision>,
    at: DateTime<Utc>,
    fact_at: Option<DateTime<Utc>>,
    presupported: bool,
) -> Result<Vec<Result<ClaimView>>> {
    let mut views = Vec::new();
    for view in base_views(state, tx, cache, revisions, at, fact_at, presupported).await? {
        views.push(match view {
            Ok(mut view) => {
                match crate::procedures::assess(state, tx, &mut view, at, fact_at).await {
                    Ok(()) => Ok(view),
                    Err(e) if matches!(e.0, StatusCode::NOT_FOUND | StatusCode::GONE) => Err(e),
                    Err(e) => return Err(e),
                }
            }
            Err(e) => Err(e),
        });
    }
    Ok(views)
}
pub(crate) async fn view(
    state: &AppState,
    tx: &mut Tx<'_>,
    r: ClaimRevision,
    at: DateTime<Utc>,
    fact_at: Option<DateTime<Utc>>,
) -> Result<ClaimView> {
    let mut view = base_view(state, tx, r, at, fact_at).await?;
    crate::procedures::assess(state, tx, &mut view, at, fact_at).await?;
    Ok(view)
}
const PAGE: i64 = 20;
const MAX_SCANNED_PAGES: usize = 25;
fn include(view: &ClaimView, mode: Option<&str>) -> bool {
    match mode.unwrap_or("investigation") {
        "strict_accepted" => view.eligibility.strict_accepted,
        "strict_operational" => view.eligibility.strict_operational,
        "history" => true,
        _ => view.eligibility.investigation,
    }
}
#[utoipa::path(get,path="/api/brains/{brain}/claims",operation_id="claims",params(("brain"=Uuid,Path),("operation_id"=Option<Uuid>,Query),("repository_id"=Option<Uuid>,Query),("environment_id"=Option<Uuid>,Query),("area_id"=Option<Uuid>,Query),("fact_at"=Option<DateTime<Utc>>,Query),("knowledge_at"=Option<DateTime<Utc>>,Query),("mode"=Option<String>,Query),("kind"=Option<String>,Query),("q"=Option<String>,Query),("offset"=Option<i64>,Query)),responses((status=200,body=ClaimPage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<ClaimQuery>,
) -> Result<Json<ClaimPage>> {
    let search = publication::list_query(query.q.as_deref())?;
    let offset = publication::offset(&publication::Page {
        offset: query.offset,
    })?;
    let mut tx = evidence::read_tx(&state, &auth, brain).await?;
    let at = query_time(&mut tx, &query).await?;
    let scope = inspection::scope(&mut tx, &auth, brain, query.operation_id).await?;
    query_scope(&mut tx, brain, &query).await?;
    // Choose the interval first: filtering privacy inside DISTINCT would resurrect an older revision.
    let show_removed = query.mode.as_deref() == Some("history")
        && query.environment_id.is_none()
        && query.repository_id.is_none()
        && query.area_id.is_none();
    let privacy_filter = if show_removed {
        ""
    } else {
        "AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'"
    };
    let filtered = format!(
        "FROM (SELECT DISTINCT ON(claim_id) * FROM claim_revisions WHERE brain_id=$1 AND recorded_at<=$2 ORDER BY claim_id,recorded_at DESC) r WHERE ($3::uuid IS NULL OR r.revision#>>'{{content,selection,environment_id}}' IS NULL OR r.revision#>>'{{content,selection,environment_id}}'=$3::text) AND ($4::uuid IS NULL OR r.revision#>'{{content,selection,repository_ids}}'='[]'::jsonb OR r.revision#>'{{content,selection,repository_ids}}' ? $4::text) AND ($5::uuid IS NULL OR r.revision#>'{{content,selection,area_ids}}'='[]'::jsonb OR r.revision#>'{{content,selection,area_ids}}' ? $5::text) {privacy_filter} AND ($8::text IS NULL OR r.revision#>>'{{content,kind}}'=$8)"
    );
    let filtered = format!(
        "{filtered} AND ($6::jsonb IS NULL OR (r.revision#>'{{content,selection}}' IS NOT NULL
        AND recollect_recall_scope(r.revision#>'{{content,selection}}',$6)))
        AND ($7='' OR (recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
          AND position(lower($7) in lower(concat_ws(' ',r.revision#>>'{{content,subject}}',r.revision#>>'{{content,predicate}}',r.revision#>>'{{content,value}}',r.revision#>>'{{content,rationale}}')))>0))"
    );
    // Unless history is requested, support is evaluated once per statement for
    // the revisions that passed every cheap filter, sharing one dependency
    // closure, instead of once per candidate row.
    let (gate, filtered) = if query.mode.as_deref() == Some("history") {
        (String::new(), filtered)
    } else {
        (
            format!(
                "WITH base AS MATERIALIZED (SELECT r.* {filtered}), gate AS MATERIALIZED (SELECT s.revision_id FROM recollect_memory_supported_many($1,ARRAY(SELECT b.id FROM base b)) s WHERE s.supported) "
            ),
            "FROM base r WHERE r.id IN (SELECT revision_id FROM gate)".to_string(),
        )
    };
    let total: i64 = sqlx::query_scalar(&format!("{gate}SELECT count(*) {filtered}"))
        .bind(brain)
        .bind(at)
        .bind(query.environment_id)
        .bind(query.repository_id)
        .bind(query.area_id)
        .bind(scope.as_ref().map(SqlJson))
        .bind(&search)
        .bind(&query.kind)
        .fetch_one(&mut *tx)
        .await?;
    let sql = format!(
        "{gate}SELECT r.id,r.claim_id,r.recorded_at,recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at) AS state,CASE WHEN recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active' THEN r.revision END AS revision {filtered} ORDER BY recorded_at DESC,id LIMIT {PAGE} OFFSET $9"
    );
    // Eligibility for the requested mode is canonical Rust policy over each view,
    // so it cannot move into the paging SQL. A page of candidates that are all
    // ineligible would otherwise be empty while more remain: keep reading
    // candidate pages (bounded) until something can be shown, and advance
    // `next_offset` past everything examined.
    let mut cache = evidence::EvidenceCache::default();
    let mut items = Vec::new();
    let mut unavailable = Vec::new();
    let mut scanned = offset;
    for _ in 0..MAX_SCANNED_PAGES {
        let revisions: Vec<PrivateRevision> = sqlx::query_as(&sql)
            .bind(brain)
            .bind(at)
            .bind(query.environment_id)
            .bind(query.repository_id)
            .bind(query.area_id)
            .bind(scope.as_ref().map(SqlJson))
            .bind(&search)
            .bind(&query.kind)
            .bind(scanned)
            .fetch_all(&mut *tx)
            .await?;
        let fetched = revisions.len() as i64;
        let mut live = Vec::new();
        for r in revisions {
            if let Some(revision) = r.revision {
                live.push(revision.0);
            } else {
                unavailable.push(UnavailableClaim {
                    claim_id: r.claim_id,
                    revision: UnavailableRevision {
                        id: r.id,
                        recorded_at: r.recorded_at,
                        state: r.state,
                    },
                });
            }
        }
        // Unless history is requested the SQL above already required support.
        let presupported = query.mode.as_deref() != Some("history");
        for view in views(
            &state,
            &mut tx,
            &mut cache,
            live,
            at,
            query.fact_at,
            presupported,
        )
        .await?
        {
            let view = view?;
            if include(&view, query.mode.as_deref()) {
                items.push(view);
            }
        }
        scanned += PAGE;
        if !items.is_empty() || !unavailable.is_empty() || fetched < PAGE || scanned >= total {
            break;
        }
    }
    // The page includes claim bodies and supporting-evidence labels. Advertise
    // their earliest canonical deadline so a long-lived display cannot retain
    // content past that deadline between authorized refreshes. Already removed
    // support is represented by unavailable metadata, not its former content.
    // Membership follows the emitted labels: a deadline that passes during
    // page construction still belongs to content already read into this page.
    let revisions: Vec<_> = items.iter().map(|v| v.revision.id).collect();
    let sources: Vec<_> = items
        .iter()
        .flat_map(|v| &v.evidence)
        .filter(|e| {
            e.kind == "source_version" && !matches!(e.availability.as_str(), "expired" | "erased")
        })
        .map(|e| e.id)
        .collect();
    let facts: Vec<_> = items
        .iter()
        .flat_map(|v| &v.evidence)
        .filter(|e| {
            e.kind == "repository_fact" && !matches!(e.availability.as_str(), "expired" | "erased")
        })
        .map(|e| e.id)
        .collect();
    let expires_at = sqlx::query_scalar(
        "WITH deadlines AS (
          SELECT recollect_memory_deadline(brain_id,id) AS deadline
          FROM claim_revisions WHERE brain_id=$1 AND id=ANY($2)
          UNION ALL
          SELECT recollect_retention_deadline(brain_id,retention_class,created_at)
          FROM source_versions WHERE brain_id=$1 AND id=ANY($3)
          UNION ALL
          SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at)
          FROM repository_facts f JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=f.brain_id
          WHERE f.brain_id=$1 AND f.id=ANY($4)
        ) SELECT min(deadline) FROM deadlines",
    )
    .bind(brain).bind(&revisions).bind(&sources).bind(&facts)
    .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(ClaimPage {
        items,
        expires_at,
        unavailable,
        total_candidates: total,
        offset,
        next_offset: (scanned < total).then_some(scanned),
        knowledge_at: at,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/claims/{claim}",operation_id="claim",params(("brain"=Uuid,Path),("operation_id"=Option<Uuid>,Query),("claim"=Uuid,Path),("fact_at"=Option<DateTime<Utc>>,Query),("knowledge_at"=Option<DateTime<Utc>>,Query),("offset"=Option<i64>,Query)),responses((status=200,body=ClaimDetail)))]
pub async fn detail(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ClaimQuery>,
) -> Result<Json<ClaimDetail>> {
    let offset = publication::offset(&publication::Page {
        offset: query.offset,
    })?;
    let mut tx = evidence::read_tx(&state, &auth, brain).await?;
    let scope = inspection::scope(&mut tx, &auth, brain, query.operation_id).await?;
    let current_revision: Uuid =
        sqlx::query_scalar("SELECT current_revision FROM claims WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(Error::missing)?;
    let at = query_time(&mut tx, &query).await?;
    let fields = "id,claim_id,recorded_at,p.state,CASE WHEN p.state='active' THEN revision END AS revision FROM claim_revisions CROSS JOIN LATERAL (SELECT recollect_content_state(brain_id,'claim',privacy_state,recorded_at) AS state) p";
    let selected:Option<PrivateRevision> = sqlx::query_as(&format!("SELECT {fields} WHERE brain_id=$1 AND claim_id=$2 AND recorded_at<=$3 ORDER BY recorded_at DESC LIMIT 1")).bind(brain).bind(id).bind(at).fetch_optional(&mut *tx).await?;
    let selection_state = selected
        .as_ref()
        .map_or("not_recorded", |r| r.state.as_str())
        .to_owned();
    let selected = if let Some(r) = selected.and_then(|r| r.revision) {
        if let Some(scope) = &scope
            && inspection::mask(&mut tx, scope, [&r.0.content.selection])
                .await?
                .is_empty()
        {
            return Err(Error::missing());
        }
        Some(view(&state, &mut tx, r.0, at, query.fact_at).await?)
    } else {
        None
    };
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM claim_revisions WHERE brain_id=$1 AND claim_id=$2
         AND ($3::jsonb IS NULL OR (revision#>'{content,selection}' IS NOT NULL
              AND recollect_recall_scope(revision#>'{content,selection}',$3)))",
    )
    .bind(brain)
    .bind(id)
    .bind(scope.as_ref().map(SqlJson))
    .fetch_one(&mut *tx)
    .await?;
    let records: Vec<PrivateRevision> = sqlx::query_as(&format!(
        "SELECT {fields} WHERE brain_id=$1 AND claim_id=$2
        AND ($4::jsonb IS NULL OR (revision#>'{{content,selection}}' IS NOT NULL
             AND recollect_recall_scope(revision#>'{{content,selection}}',$4)))
        ORDER BY recorded_at DESC LIMIT 20 OFFSET $3"
    ))
    .bind(brain)
    .bind(id)
    .bind(offset)
    .bind(scope.as_ref().map(SqlJson))
    .fetch_all(&mut *tx)
    .await?;
    let mut history = Vec::new();
    let mut unavailable_history = Vec::new();
    for row in records {
        if let Some(revision) = row.revision {
            history.push(revision.0)
        } else {
            unavailable_history.push(UnavailableRevision {
                id: row.id,
                recorded_at: row.recorded_at,
                state: row.state,
            })
        }
    }
    tx.commit().await?;
    Ok(Json(ClaimDetail {
        selected,
        current_revision,
        history,
        total,
        offset,
        knowledge_at: at,
        selection_state,
        unavailable_history,
    }))
}

#[derive(sqlx::FromRow)]
struct PrivateRevision {
    id: Uuid,
    claim_id: Uuid,
    recorded_at: DateTime<Utc>,
    state: String,
    revision: Option<SqlJson<ClaimRevision>>,
}
