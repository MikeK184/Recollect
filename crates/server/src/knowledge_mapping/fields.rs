//! Typed canonical UUID fields with immutable origins, not source-span extraction.
use crate::{
    AppState,
    auth::Auth,
    db,
    error::{Error, Result},
    graph,
    memory_evidence::Tx,
    retrieval,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use recollect_protocol::*;
use serde_json::{Value, json};
use sqlx::{FromRow, types::Json as SqlJson};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
use uuid::Uuid;

mod refresh;
pub use refresh::{execute, run_once};
const WINDOW: usize = 250;
const DETAILS: usize = 80;

fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "organization_changed",
        "Canonical field inputs changed. Read the current page again.",
    )
}

#[derive(Clone, FromRow)]
struct Input {
    id: Uuid,
    origin_kind: String,
    logical_id: Uuid,
    revision_id: Uuid,
    context: SqlJson<Value>,
    evidence_kind: String,
    evidence_id: Uuid,
    evidence_revision: Uuid,
}
impl Input {
    fn evidence(&self) -> graph::Entity {
        graph::Entity {
            kind: self.evidence_kind.clone(),
            id: self.evidence_id,
            revision_id: self.evidence_revision,
        }
    }
    fn origin_kind(&self) -> Result<FieldOriginKind> {
        serde_json::from_value(json!(self.origin_kind)).map_err(|_| changed())
    }
}
#[derive(Clone, PartialEq, Eq, Debug)]
struct Reference {
    kind: &'static str,
    pointer: String,
    value: Uuid,
}

/// Only schema-declared canonical UUID fields participate. No free text is read.
fn references(kind: &str, context: &Value) -> Result<Vec<Reference>> {
    let mut out = vec![];
    let mut one = |kind, pointer: String, value: &Value| -> Result<()> {
        if value.is_null() {
            return Ok(());
        }
        let id = value
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .filter(|id| !id.is_nil())
            .ok_or_else(changed)?;
        out.push(Reference {
            kind,
            pointer,
            value: id,
        });
        Ok(())
    };
    match kind {
        "source_context" | "claim_revision" => {
            let prefix = if kind == "claim_revision" {
                "/content/selection"
            } else {
                "/selection"
            };
            if let Some(ids) = context
                .pointer(&format!("{prefix}/repository_ids"))
                .and_then(Value::as_array)
            {
                if ids.len() > 100 {
                    return Err(changed());
                }
                for (n, id) in ids.iter().enumerate() {
                    one("repository", format!("{prefix}/repository_ids/{n}"), id)?;
                }
            }
            if let Some(id) = context.pointer(&format!("{prefix}/environment_id")) {
                one("environment", format!("{prefix}/environment_id"), id)?;
            }
            if kind == "source_context" {
                if let Some(ids) = context["environments"].as_array() {
                    if ids.len() > 100 {
                        return Err(changed());
                    }
                    for (n, id) in ids.iter().enumerate() {
                        one("environment", format!("/environments/{n}"), id)?;
                    }
                }
            }
        }
        "repository_snapshot" => one(
            "repository",
            "/repository_id".into(),
            &context["repository_id"],
        )?,
        "manifest_revision" => {
            one(
                "environment",
                "/environment_id".into(),
                &context["environment_id"],
            )?;
            if let Some(entries) = context["entries"].as_array() {
                if entries.len() > 100 {
                    return Err(changed());
                }
                for (n, e) in entries.iter().enumerate() {
                    one(
                        "repository",
                        format!("/entries/{n}/repository_id"),
                        &e["repository_id"],
                    )?;
                }
            }
        }
        _ => return Err(changed()),
    }
    Ok(out)
}
fn in_scope(r: &Reference, scope: &GraphSelection) -> bool {
    if r.kind == "repository" {
        scope.selection.repository_ids.is_empty()
            || scope.selection.repository_ids.contains(&r.value)
    } else {
        scope
            .selection
            .environment_id
            .is_none_or(|id| id == r.value)
    }
}

#[derive(FromRow)]
struct Row {
    id: Uuid,
    input_id: Uuid,
    entity_id: Uuid,
    field_pointer: String,
    value: Uuid,
    kind: String,
    label: String,
}

async fn prepare(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    request: &FieldOrganizationRequest,
) -> Result<FieldOrganizationView> {
    if request.scope.kind != "knowledge" {
        return Err(Error::invalid(
            "Choose a knowledge scope for canonical fields.",
        ));
    }
    let mut scope = request.scope.clone();
    let mut recall = RecallRequest {
        query: "Canonical field selection".into(),
        selection: scope.selection.clone(),
        operation_id: scope.operation_id,
        collection_id: scope.collection_id,
        manifest_revision_id: scope.manifest_revision_id,
        fact_at: scope.fact_at,
        mode: scope.mode.clone(),
        ..Default::default()
    };
    retrieval::validate(&mut recall)?;
    scope.selection = recall.selection;
    let mut tx = auth.preparation_tx(&state.pool).await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL statement_timeout='1500ms'")
        .execute(&mut *tx)
        .await?;
    retrieval::graph::authorize(&mut tx, auth, brain, &scope).await?;
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    if let Some(c) = &request.cursor {
        if c.epoch != epoch
            || c.entity_id != request.entity_id
            || c.expires_at.is_some_and(|at| at <= Utc::now())
            || serde_json::to_value(&c.scope).ok() != serde_json::to_value(&scope).ok()
        {
            return Err(changed());
        }
    }
    let after = request.cursor.as_ref().and_then(|c| c.after_input);
    let after_origin = request.cursor.as_ref().and_then(|c| c.after_origin);
    let mut inputs:Vec<Input>=sqlx::query_as("SELECT i.id,i.origin_kind,i.logical_id,i.revision_id,i.context,i.evidence_kind,i.evidence_id,i.evidence_revision FROM knowledge_field_inputs i WHERE i.brain_id=$1 AND ($2::uuid IS NULL OR i.id>$2) AND ($3::uuid IS NULL OR EXISTS(SELECT 1 FROM knowledge_field_references r WHERE r.brain_id=i.brain_id AND r.input_id=i.id AND r.entity_id=$3)) ORDER BY i.id LIMIT 251")
        .bind(brain).bind(after).bind(request.entity_id).fetch_all(&mut *tx).await?;
    let more = inputs.len() > WINDOW;
    inputs.truncate(WINDOW);
    let examined = inputs.len();
    let window_after = inputs.last().map(|i| i.id);
    let ids: Vec<_> = inputs.iter().map(|i| i.id).collect();
    let current:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM knowledge_field_inputs WHERE brain_id=$1 AND id=ANY($2) AND recollect_field_input_current(brain_id,id)").bind(brain).bind(&ids).fetch_all(&mut *tx).await?;
    inputs.retain(|i| current.contains(&i.id));
    let entities: Vec<_> = inputs.iter().map(Input::evidence).collect();
    let qualified = retrieval::graph::qualify(
        state,
        &mut tx,
        auth,
        brain,
        &mut scope,
        &entities,
        (WINDOW, None),
    )
    .await?;
    inputs.retain(|i| qualified.nodes.contains_key(&i.evidence().key()));
    let qualified_count = inputs.len();
    let ids: Vec<_> = inputs.iter().map(|i| i.id).collect();
    let rows:Vec<Row>=sqlx::query_as("SELECT r.id,r.input_id,r.entity_id,r.field_pointer,r.value,e.kind,CASE e.kind WHEN 'repository' THEN p.canonical_origin ELSE g.name END AS label
     FROM knowledge_field_references r JOIN knowledge_field_entities e ON e.brain_id=r.brain_id AND e.id=r.entity_id
     LEFT JOIN repositories p ON p.brain_id=e.brain_id AND p.id=e.repository_id
     LEFT JOIN evidence_groups g ON g.brain_id=e.brain_id AND g.id=e.environment_id AND g.kind='environment'
     WHERE r.brain_id=$1 AND r.input_id=ANY($2) AND ($3::uuid IS NULL OR r.entity_id=$3)
      AND ((e.kind='repository' AND p.id=r.value) OR (e.kind='environment' AND g.id=r.value)) ORDER BY r.id")
        .bind(brain).bind(&ids).bind(request.entity_id).fetch_all(&mut *tx).await?;
    let by_input: BTreeMap<_, _> = inputs.iter().map(|i| (i.id, i)).collect();
    let expected: BTreeMap<_, _> = inputs
        .iter()
        .map(|i| Ok((i.id, references(&i.origin_kind, &i.context)?)))
        .collect::<Result<_>>()?;
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|r| {
            expected[&r.input_id].iter().any(|e| {
                e.kind == r.kind
                    && e.pointer == r.field_pointer
                    && e.value == r.value
                    && in_scope(e, &scope)
            })
        })
        .collect();
    let assigned = rows
        .iter()
        .map(|r| r.input_id)
        .collect::<BTreeSet<_>>()
        .len();
    let mut cards: BTreeMap<Uuid, FieldOrganizationEntity> = BTreeMap::new();
    let mut contributors: BTreeMap<Uuid, BTreeSet<Uuid>> = BTreeMap::new();
    for r in &rows {
        contributors
            .entry(r.entity_id)
            .or_default()
            .insert(r.input_id);
        cards.entry(r.entity_id).or_insert(FieldOrganizationEntity {
            id: r.entity_id,
            kind: if r.kind == "repository" {
                FieldReferenceKind::Repository
            } else {
                FieldReferenceKind::Environment
            },
            reference_id: r.value,
            label: r.label.clone(),
            label_pointer: if r.kind == "repository" {
                "/canonical_origin"
            } else {
                "/name"
            }
            .into(),
            contributor_count: 0,
        });
    }
    for (id, card) in &mut cards {
        card.contributor_count = contributors[id].len();
    }
    let mut detail: Vec<_> = rows
        .into_iter()
        .filter(|r| after_origin.is_none_or(|id| r.id > id))
        .collect();
    let more_detail = detail.len() > DETAILS;
    detail.truncate(DETAILS);
    let mut evidence_keys = BTreeSet::new();
    let mut origins = vec![];
    for r in &detail {
        let i = by_input[&r.input_id];
        let key = i.evidence().key();
        evidence_keys.insert(key.clone());
        origins.push(OrganizationFieldOrigin {
            id: r.id,
            entity_id: r.entity_id,
            input_id: r.input_id,
            origin_kind: i.origin_kind()?,
            logical_id: i.logical_id,
            revision_id: i.revision_id,
            field_pointer: r.field_pointer.clone(),
            value: r.value,
            evidence_key: key,
        });
    }
    // One qualified representative per card keeps every visible card actionable.
    let represented: BTreeSet<_> = origins.iter().map(|o| o.entity_id).collect();
    cards.retain(|id, _| represented.contains(id));
    let refresh: Option<(String, i64, i64)> = sqlx::query_as(
        "SELECT CASE WHEN NOT recollect_field_actor_current(brain_id) THEN 'blocked' ELSE state END AS state,revision,handled_revision FROM knowledge_field_outbox WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_optional(&mut *tx)
    .await?;
    let refresh_state = refresh.map_or_else(
        || "pending".into(),
        |(state, revision, handled)| {
            if revision > handled && state == "ready" {
                "pending".into()
            } else {
                state
            }
        },
    );
    let partial = more
        || more_detail
        || after.is_some()
        || after_origin.is_some()
        || refresh_state != "ready";
    let next_cursor = if more_detail {
        Some(FieldOrganizationCursor {
            after_input: after,
            after_origin: detail.last().map(|r| r.id),
            epoch,
            scope: scope.clone(),
            entity_id: request.entity_id,
            expires_at: qualified.deadline,
        })
    } else if more {
        Some(FieldOrganizationCursor {
            after_input: window_after,
            after_origin: None,
            epoch,
            scope: scope.clone(),
            entity_id: request.entity_id,
            expires_at: qualified.deadline,
        })
    } else {
        None
    };
    let out = FieldOrganizationView {
        brain_id: brain,
        scope,
        epoch,
        expires_at: qualified.deadline,
        entities: cards.into_values().collect(),
        origins,
        evidence: qualified
            .nodes
            .into_iter()
            .filter(|(k, _)| evidence_keys.contains(k))
            .map(|(_, v)| v)
            .collect(),
        coverage: FieldOrganizationCoverage {
            examined_inputs: examined,
            qualified_inputs: qualified_count,
            assigned_inputs: assigned,
            unassigned_inputs: qualified_count.saturating_sub(assigned),
            counts_are_lower_bounds: partial,
            refresh_state,
        },
        next_cursor,
    };
    tx.commit().await?;
    Ok(out)
}

#[utoipa::path(post,path="/api/brains/{brain}/organization/fields/view",operation_id="viewOrganizationFields",params(("brain"=Uuid,Path)),request_body=FieldOrganizationRequest,responses((status=200,body=FieldOrganizationView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(request): Json<FieldOrganizationRequest>,
) -> Result<Json<FieldOrganizationView>> {
    let _permit = graph::read_capacity().try_acquire().map_err(|_| {
        Error(
            StatusCode::TOO_MANY_REQUESTS,
            "graph_read_capacity",
            "Graph readers are occupied. Retry shortly.",
        )
    })?;
    let out = tokio::time::timeout(
        Duration::from_secs(5),
        prepare(&state, &auth, brain, &request),
    )
    .await
    .map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "organization_preparation_timeout",
            "Canonical field preparation exceeded its bounded window.",
        )
    })??;
    let mut tx = auth.publication_tx(&state.pool).await?;
    sqlx::query("SET LOCAL statement_timeout='750ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL lock_timeout='250ms'")
        .execute(&mut *tx)
        .await?;
    db::require_role(&mut tx, brain, false).await?;
    retrieval::graph::authorize(&mut tx, &auth, brain, &out.scope).await?;
    graph::preparation_unchanged(&mut tx, brain, out.epoch).await?;
    if out.expires_at.is_some_and(|at| at <= Utc::now()) {
        return Err(changed());
    }
    tx.commit().await?;
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untyped_claim_text_and_open_records_never_infer_entities() {
        let id = Uuid::new_v4();
        let context = json!({"content":{"subject":"payments-api","predicate":"uses","value":"PostgreSQL","selection":{"repository_ids":[id],"environment_id":null}}});
        let refs = references("claim_revision", &context)
            .unwrap_or_else(|e| panic!("field reference {}", e.1));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, "repository");
        assert_eq!(refs[0].pointer, "/content/selection/repository_ids/0");
        assert!(
            references(
                "enola_function",
                &json!({"kind":"service","name":"payments-api"})
            )
            .is_err()
        );
    }
}
