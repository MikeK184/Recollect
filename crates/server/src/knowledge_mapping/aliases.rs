//! Independently attributed, reversible source-described equivalence links.
//! A supported alias never merges original entity IDs or creates accepted truth.
use crate::db::admission::{self, BrainAccess};
use crate::{
    AppState,
    auth::Auth,
    commands, db,
    error::{Error, Result},
    graph, model_gateway, retrieval,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use recollect_protocol::*;
use serde_json::{Value, json};
use sqlx::FromRow;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
use uuid::Uuid;

#[derive(FromRow)]
struct Candidate {
    id: Uuid,
    input_id: Uuid,
    source_id: Uuid,
    source_version_id: Uuid,
    from_entity: Uuid,
    to_entity: Uuid,
    from_kind: String,
    to_kind: String,
    from_realm: Value,
    to_realm: Value,
    from_quote: String,
    to_quote: String,
    from_start: i32,
    from_end: i32,
    to_start: i32,
    to_end: i32,
    byte_start: i32,
    byte_end: i32,
    quote: String,
    disposition: String,
    change_id: Option<Uuid>,
}
fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "organization_changed",
        "Organization inputs changed. Read the current page again.",
    )
}
fn key(id: Uuid) -> String {
    graph::key("source_version", id)
}
fn scope(mut scope: GraphSelection) -> Result<GraphSelection> {
    if scope.kind != "knowledge" {
        return Err(Error::invalid(
            "Choose a knowledge scope for alias evidence.",
        ));
    }
    let mut input = RecallRequest {
        query: "Alias evidence".into(),
        selection: scope.selection.clone(),
        operation_id: scope.operation_id,
        collection_id: scope.collection_id,
        manifest_revision_id: scope.manifest_revision_id,
        fact_at: scope.fact_at,
        mode: scope.mode.clone(),
        ..Default::default()
    };
    retrieval::validate(&mut input)?;
    scope.selection = input.selection;
    Ok(scope)
}
fn compatible(kind_a: &str, realm_a: &Value, kind_b: &str, realm_b: &Value) -> bool {
    if kind_a != kind_b {
        return false;
    }
    if matches!(kind_a, "technology" | "concept") {
        return realm_a == &json!({"kind":"brain_term"}) && realm_a == realm_b;
    }
    let valid = |realm: &Value| {
        realm["kind"] == "source_occurrence"
            && ["source_id", "context_id", "version_id"].iter().all(|key| {
                realm[*key]
                    .as_str()
                    .and_then(|v| Uuid::parse_str(v).ok())
                    .is_some_and(|v| !v.is_nil())
            })
            && realm["byte_start"].as_i64().is_some_and(|start| {
                start >= 0 && realm["byte_end"].as_i64().is_some_and(|end| end > start)
            })
    };
    if !valid(realm_a) || !valid(realm_b) {
        return false;
    }
    let strip = |realm: &Value| {
        let mut realm = realm.clone();
        if let Some(object) = realm.as_object_mut() {
            object.remove("byte_start");
            object.remove("byte_end");
        }
        realm
    };
    strip(realm_a) == strip(realm_b)
}

/// Deliberately narrow English recognition, followed by an explicit human
/// command. The ENTIRE immutable input must be a standalone positive statement;
/// a clipped positive-looking substring of a negative, history, example, quote,
/// conditional or multi-sentence document is unsupported. This does not claim
/// general English interpretation or silently select automatic equivalence.
fn positive_statement(text: &str, c: &Candidate) -> bool {
    let statement = text.trim();
    let statement = statement.strip_suffix('.').unwrap_or(statement);
    if text
        .chars()
        .any(|ch| matches!(ch, '\'' | '"' | '`' | '“' | '”' | '‘' | '’'))
    {
        return false;
    }
    let words: Vec<_> = text
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::to_lowercase)
        .collect();
    if words.iter().any(|word| {
        matches!(
            word.as_str(),
            "not"
                | "no"
                | "never"
                | "formerly"
                | "previously"
                | "historically"
                | "was"
                | "were"
                | "might"
                | "may"
                | "could"
                | "would"
                | "if"
                | "example"
                | "hypothetical"
                | "retracted"
                | "obsolete"
                | "until"
                | "before"
                | "after"
        )
    }) {
        return false;
    }
    let leading = text.len() - text.trim_start().len();
    if c.byte_start as usize > leading
        || (c.byte_end as usize) < text.trim_end().len()
        || text.get(c.byte_start as usize..c.byte_end as usize) != Some(c.quote.as_str())
    {
        return false;
    }
    for (a, a_start, a_end, b, b_start, b_end) in [
        (
            &c.from_quote,
            c.from_start,
            c.from_end,
            &c.to_quote,
            c.to_start,
            c.to_end,
        ),
        (
            &c.to_quote,
            c.to_start,
            c.to_end,
            &c.from_quote,
            c.from_start,
            c.from_end,
        ),
    ] {
        for connector in [" is also known as ", " is known as ", " is an alias for "] {
            let expected = format!("{a}{connector}{b}");
            if statement == expected
                && a_start as usize == leading
                && a_end as usize == leading + a.len()
                && b_start as usize == leading + a.len() + connector.len()
                && b_end as usize == leading + expected.len()
            {
                return true;
            }
        }
    }
    false
}
fn card(id: Uuid, kind: &str, label: &str) -> OrganizationCard {
    OrganizationCard {
        id,
        kind: kind.into(),
        topic_kind: None,
        label: label.into(),
        source_count: 1,
        entity_count: 1,
        pinned: false,
        change_id: None,
    }
}
async fn prepare(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: &AliasRequest,
    exact: Option<Uuid>,
) -> Result<AliasView> {
    if !(1..=50).contains(&input.limit) {
        return Err(Error::invalid("Choose 1 to 50 alias candidates."));
    }
    let mut scope = scope(input.scope.clone())?;
    let mut tx = auth.preparation_tx(&state.pool).await?;
    sqlx::query("SET LOCAL statement_timeout='1500ms'")
        .execute(&mut *tx)
        .await?;
    retrieval::graph::authorize(&mut tx, auth, brain, &scope).await?;
    let epoch = graph::preparation_epoch(&mut tx, brain).await?;
    if let Some(cursor) = &input.cursor {
        if cursor.epoch != epoch
            || cursor.expires_at.is_some_and(|d| d <= Utc::now())
            || serde_json::to_value(&cursor.scope).ok() != serde_json::to_value(&scope).ok()
        {
            return Err(changed());
        }
    }
    let mut rows:Vec<Candidate>=sqlx::query_as(
        "SELECT r.id,r.input_id,v.source_id,i.source_version_id,a.entity_id AS from_entity,b.entity_id AS to_entity,
         ea.kind AS from_kind,eb.kind AS to_kind,ea.realm AS from_realm,eb.realm AS to_realm,
         a.quote AS from_quote,b.quote AS to_quote,a.byte_start AS from_start,a.byte_end AS from_end,b.byte_start AS to_start,b.byte_end AS to_end,
         r.byte_start,r.byte_end,r.quote,r.disposition,h.change_id
         FROM knowledge_relation_candidates r JOIN knowledge_mapping_inputs i ON i.brain_id=r.brain_id AND i.id=r.input_id
         JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id
         JOIN knowledge_mentions a ON a.input_id=r.input_id AND a.brain_id=r.brain_id AND a.id=r.from_mention
         JOIN knowledge_mentions b ON b.input_id=r.input_id AND b.brain_id=r.brain_id AND b.id=r.to_mention
         JOIN knowledge_entities ea ON ea.brain_id=a.brain_id AND ea.id=a.entity_id
         JOIN knowledge_entities eb ON eb.brain_id=b.brain_id AND eb.id=b.entity_id
         LEFT JOIN knowledge_alias_heads h ON h.brain_id=r.brain_id AND h.candidate_id=r.id
         WHERE r.brain_id=$1 AND r.relation='alias' AND i.state='published'
          AND ($2::uuid IS NULL OR r.id>$2) AND ($3::uuid IS NULL OR r.id=$3)
         ORDER BY r.id LIMIT $4")
        .bind(brain).bind(input.cursor.as_ref().map(|c|c.after_id)).bind(exact).bind((input.limit+1) as i64).fetch_all(&mut *tx).await?;
    let more = rows.len() > input.limit;
    rows.truncate(input.limit);
    let after = rows.last().map(|r| r.id);
    let sources: Vec<_> = rows
        .iter()
        .map(|r| (r.source_version_id, r.source_id))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .map(|(revision_id, id)| graph::Entity {
            kind: "source_version".into(),
            id,
            revision_id,
        })
        .collect();
    let qualified = retrieval::graph::qualify(
        state,
        &mut tx,
        auth,
        brain,
        &mut scope,
        &sources,
        (50, None),
    )
    .await?;
    rows.retain(|r| qualified.nodes.contains_key(&key(r.source_version_id)));
    let inputs: Vec<_> = rows
        .iter()
        .map(|r| r.input_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let current: Vec<Uuid> =
        sqlx::query_scalar("SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)")
            .bind(brain)
            .bind(&inputs)
            .fetch_all(&mut *tx)
            .await?;
    rows.retain(|r| current.contains(&r.input_id));
    let mut texts = BTreeMap::new();
    let mut evidence = BTreeSet::new();
    let mut candidates = vec![];
    for row in rows {
        if !texts.contains_key(&row.source_version_id) {
            let text = model_gateway::source(
                state,
                &mut tx,
                brain,
                row.source_version_id,
                2 * 1024 * 1024,
            )
            .await
            .ok()
            .map(|s| s.text);
            texts.insert(row.source_version_id, text);
        }
        let text = texts[&row.source_version_id].as_deref();
        let acceptance_state = if text.is_none() {
            "source_unavailable"
        } else if row.from_entity == row.to_entity {
            "same_identity"
        } else if !compatible(&row.from_kind, &row.from_realm, &row.to_kind, &row.to_realm) {
            "incompatible_identity_scope"
        } else if positive_statement(text.unwrap_or_default(), &row) {
            "explicit_positive_statement"
        } else {
            "unsupported_context"
        };
        evidence.insert(key(row.source_version_id));
        candidates.push(AliasCandidateView {
            id: row.id,
            input_id: row.input_id,
            from: card(row.from_entity, &row.from_kind, &row.from_quote),
            to: card(row.to_entity, &row.to_kind, &row.to_quote),
            source_key: key(row.source_version_id),
            byte_start: row.byte_start,
            byte_end: row.byte_end,
            quote: row.quote,
            disposition: row.disposition,
            change_id: row.change_id,
            acceptance_state: acceptance_state.into(),
        });
    }
    let next_cursor = if more {
        after.map(|after_id| AliasCursor {
            after_id,
            epoch,
            scope: scope.clone(),
            expires_at: qualified.deadline,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(AliasView {
        brain_id: brain,
        scope,
        epoch,
        expires_at: qualified.deadline,
        candidates,
        evidence: qualified
            .nodes
            .into_iter()
            .filter(|(key, _)| evidence.contains(key))
            .map(|(_, node)| node)
            .collect(),
        next_cursor,
    })
}
async fn fresh(
    tx: &mut crate::memory_evidence::Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    prepared: &AliasView,
) -> Result<()> {
    retrieval::graph::authorize(tx, auth, brain, &prepared.scope).await?;
    let current: bool =
        sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
            .bind(prepared.expires_at)
            .fetch_one(&mut **tx)
            .await?;
    if !current || graph::preparation_epoch(tx, brain).await? != prepared.epoch {
        return Err(changed());
    }
    Ok(())
}
#[utoipa::path(post,path="/api/brains/{brain}/organization/aliases/view",operation_id="organizationAliases",params(("brain"=Uuid,Path)),request_body=AliasRequest,responses((status=200,body=AliasView)))]
pub async fn view(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<AliasRequest>,
) -> Result<Json<AliasView>> {
    let _permit = graph::read_capacity().try_acquire().map_err(|_| {
        Error(
            StatusCode::TOO_MANY_REQUESTS,
            "graph_busy",
            "Graph reads are busy. Try again shortly.",
        )
    })?;
    tokio::time::timeout(Duration::from_secs(5), async {
        let prepared = prepare(&state, &auth, brain, &input, None).await?;
        let mut tx = auth.publication_tx(&state.pool).await?;
        sqlx::query("SET LOCAL statement_timeout='750ms'")
            .execute(&mut *tx)
            .await?;
        fresh(&mut tx, &auth, brain, &prepared).await?;
        tx.commit().await?;
        Ok(Json(prepared))
    })
    .await
    .map_err(|_| {
        Error(
            StatusCode::GATEWAY_TIMEOUT,
            "organization_timeout",
            "Alias review exceeded its bounded read deadline.",
        )
    })?
}
#[utoipa::path(put,path="/api/brains/{brain}/organization/aliases/{id}",operation_id="organizationAlias",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=AliasCommand,responses((status=200,body=AliasCommandResult)))]
pub async fn decide(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<AliasCommand>,
) -> Result<Json<AliasCommandResult>> {
    auth.require_browser()?;
    let key = commands::key(&headers)?
        .ok_or_else(|| Error::invalid("Supply an Idempotency-Key for an alias decision."))?;
    let prepared = prepare(
        &state,
        &auth,
        brain,
        &AliasRequest {
            scope: GraphSelection {
                kind: "knowledge".into(),
                ..Default::default()
            },
            cursor: None,
            limit: 1,
        },
        Some(id),
    )
    .await?;
    let candidate = prepared.candidates.first().ok_or_else(changed)?;
    if matches!(input.action, AliasAction::Accept)
        && candidate.acceptance_state != "explicit_positive_statement"
    {
        return Err(Error(
            StatusCode::CONFLICT,
            "alias_evidence_unsupported",
            "This exact canonical source does not match the supported standalone positive-equivalence form. Keep it pending or reject it.",
        ));
    }
    let mut tx = admission::brain_tx(
        &state.pool,
        &auth,
        brain,
        BrainAccess::WRITE
            .with_statement_timeout(Duration::from_millis(750))
            .with_session_recheck(),
    )
    .await?
    .tx;
    fresh(&mut tx, &auth, brain, &prepared).await?;
    let payload = json!({"brain_id":brain,"candidate_id":id,"command":input});
    if let Some(saved) =
        commands::reserve(&mut tx, Some(&key), "organization.alias", payload).await?
    {
        tx.commit().await?;
        return Ok(Json(saved));
    }
    let base:Option<Option<Uuid>>=sqlx::query_scalar("SELECT h.change_id FROM knowledge_relation_candidates r LEFT JOIN knowledge_alias_heads h ON h.brain_id=r.brain_id AND h.candidate_id=r.id WHERE r.brain_id=$1 AND r.id=$2 AND r.relation='alias' FOR UPDATE OF r")
        .bind(brain).bind(id).fetch_optional(&mut *tx).await?;
    if base != Some(input.base_change_id) {
        return Err(changed());
    }
    let (action, disposition) = match input.action {
        AliasAction::Accept => ("accept", "accepted"),
        AliasAction::Reject => ("reject", "rejected"),
        AliasAction::Revoke => ("revoke", "pending"),
    };
    let change_id = Uuid::new_v4();
    sqlx::query("INSERT INTO knowledge_alias_changes(id,brain_id,candidate_id,actor_id,action,previous_change_id) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(change_id).bind(brain).bind(id).bind(auth.user.id).bind(action).bind(input.base_change_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_alias_heads(brain_id,candidate_id,change_id) VALUES($1,$2,$3) ON CONFLICT(candidate_id) DO UPDATE SET change_id=excluded.change_id")
        .bind(brain).bind(id).bind(change_id).execute(&mut *tx).await?;
    sqlx::query(
        "UPDATE knowledge_relation_candidates SET disposition=$3 WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(id)
    .bind(disposition)
    .execute(&mut *tx)
    .await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "organization.alias",
        id,
        action,
    )
    .await?;
    sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1")
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT recollect_organization_override_changed($1)")
        .bind(brain)
        .execute(&mut *tx)
        .await?;
    let result = AliasCommandResult {
        id,
        change_id,
        disposition: disposition.into(),
    };
    commands::finish(&mut tx, Some(&key), brain, &result).await?;
    tx.commit().await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candidate(text: &str, phrase: &str) -> Candidate {
        let start = text.find(phrase).unwrap();
        let from_start = text.find("app").unwrap();
        let to_start = text.find("service").unwrap();
        Candidate {
            id: Uuid::nil(),
            input_id: Uuid::nil(),
            source_id: Uuid::nil(),
            source_version_id: Uuid::nil(),
            from_entity: Uuid::nil(),
            to_entity: Uuid::nil(),
            from_kind: "service".into(),
            to_kind: "service".into(),
            from_realm: json!({}),
            to_realm: json!({}),
            from_quote: "app".into(),
            to_quote: "service".into(),
            from_start: from_start as i32,
            from_end: (from_start + 3) as i32,
            to_start: to_start as i32,
            to_end: (to_start + 7) as i32,
            byte_start: start as i32,
            byte_end: (start + phrase.len()) as i32,
            quote: phrase.into(),
            disposition: "pending".into(),
            change_id: None,
        }
    }
    #[test]
    fn alias_acceptance_requires_complete_positive_canonical_context() {
        for positive in [
            "app is also known as service.",
            " app is an alias for service.\n",
            "app is known as service",
        ] {
            let c = candidate(positive, positive.trim());
            assert!(positive_statement(positive, &c), "{positive}");
        }
        for text in [
            "It is no longer true that app is also known as service.",
            "Previously app is also known as service.",
            "Example: app is also known as service.",
            "If deployed, app is also known as service.",
            "\"app is also known as service\"",
            "app is also known as service. This statement was retracted.",
            "History\n\napp is also known as service.",
            "app is also known as service, if enabled.",
        ] {
            let phrase = "app is also known as service";
            let c = candidate(text, phrase);
            assert!(!positive_statement(text, &c), "{text}");
        }
    }
    #[test]
    fn aliases_preserve_kind_and_canonical_instance_boundaries() {
        let a = json!({"kind":"source_occurrence","source_id":Uuid::new_v4(),"context_id":Uuid::new_v4(),"version_id":Uuid::new_v4(),"byte_start":0,"byte_end":3});
        let mut b = a.clone();
        b["byte_start"] = json!(20);
        b["byte_end"] = json!(27);
        assert!(compatible("service", &a, "service", &b));
        b["context_id"] = json!("other");
        assert!(!compatible("service", &a, "service", &b));
        assert!(!compatible("service", &a, "person", &a));
        assert!(!compatible("service", &json!({}), "service", &json!({})));
    }
}
