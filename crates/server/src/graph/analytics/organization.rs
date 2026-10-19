//! Native semantic communities. The scheduler prepares an exact qualified
//! source cohort without a writer lock; GDS returns provisional group numbers.
//! Only a freshly qualified, leased publication creates canonical topics.
use super::*;
mod hybrid;
use crate::db::admission::{self, BrainAccess, Role};
use crate::knowledge_mapping::{
    EntityKind,
    affinity::{self, Association, Projection, Vertex},
};
use crate::knowledge_mapping::{manual_intent, source_features};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

#[derive(Clone)]
struct Prepared {
    epoch: i64,
    revision: i64,
    recipe: Option<Uuid>,
    gds_version: Option<String>,
    inputs: Vec<(Uuid, Uuid)>,
    projection: Projection,
    deadline: Option<DateTime<Utc>>,
    authority_deadline: Option<DateTime<Utc>>,
    features: Option<source_features::Features>,
    vector_coverage: Option<String>,
}

async fn prepare(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
) -> Result<Prepared> {
    tokio::time::timeout(
        Duration::from_secs(5),
        prepare_inner(state, brain, actor, device),
    )
    .await
    .map_err(|_| {
        failure(
            "analytics_preparation_timeout",
            "Organization preparation exceeded its bounded window.",
        )
    })?
}

async fn prepare_inner(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
    device: Option<Uuid>,
) -> Result<Prepared> {
    let mut tx = admission::brain_preparation_tx(
        &state.pool,
        actor,
        device,
        brain,
        BrainAccess::READ.with_role(Role::Writer),
    )
    .await?
    .tx;
    sqlx::query("SET LOCAL statement_timeout='4000ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL jit=off").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL recollect.bulk_qualification=on")
        .execute(&mut *tx)
        .await?;
    let epoch = analytics_epoch(&mut tx, brain).await?;
    let authority_deadline:Option<DateTime<Utc>>=sqlx::query_scalar("SELECT least(CASE WHEN a.auth_kind<>'local' THEN a.membership_until END,d.expires_at) FROM accounts a LEFT JOIN devices d ON d.id=$2 AND d.account_id=a.id WHERE a.id=$1")
        .bind(actor).bind(device).fetch_one(&mut *tx).await?;
    let Json(user) =
        sqlx::query_scalar("SELECT to_jsonb(a) FROM accounts a WHERE id=$1 AND enabled")
            .bind(actor)
            .fetch_one(&mut *tx)
            .await?;
    let auth = Auth {
        user,
        device_id: device,
        session: Uuid::nil(),
        csrf: Uuid::nil(),
    };
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM knowledge_organization_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *tx)
            .await?;
    let selected: Option<(Uuid,String,String,Option<String>,Option<String>,Option<i32>,Option<String>,Json<source_features::HybridParameters>)>=sqlx::query_as("SELECT h.change_id,p.gds_version,p.recipe,p.embedding_provider,p.embedding_model,p.embedding_dimensions,p.representation,p.feature_parameters FROM knowledge_organization_heads h JOIN knowledge_organization_recipes p ON p.change_id=h.change_id AND p.brain_id=h.brain_id WHERE h.brain_id=$1 AND p.quality_state='qualified' AND p.recipe=ANY($2)")
        .bind(brain).bind(vec![affinity::RECIPE,source_features::RECIPE]).fetch_optional(&mut *tx).await?;
    let hybrid_profile = selected
        .as_ref()
        .filter(|r| r.2 == source_features::RECIPE)
        .map(|r| {
            (
                r.3.clone().unwrap_or_default(),
                r.4.clone().unwrap_or_default(),
                r.5.unwrap_or_default(),
                r.6.clone().unwrap_or_default(),
                r.7.0,
            )
        });
    let (recipe, gds_version) = selected.map_or((None, None), |r| (Some(r.0), Some(r.1)));
    if recipe.is_none() {
        tx.commit().await?;
        return Ok(Prepared {
            epoch,
            revision,
            recipe,
            gds_version,
            inputs: vec![],
            projection: affinity::prepare(&[], &[]).map_err(|_| changed())?,
            deadline: None,
            authority_deadline,
            features: None,
            vector_coverage: None,
        });
    }
    let candidates: Vec<Uuid>=sqlx::query_scalar("SELECT i.id FROM knowledge_mapping_inputs i JOIN sources s ON s.brain_id=i.brain_id AND s.current_version=i.source_version_id JOIN knowledge_mapping_producer_heads h ON h.brain_id=i.brain_id AND h.change_id=i.producer_change_id WHERE i.brain_id=$1 AND i.state='published' ORDER BY i.id LIMIT 10001")
        .bind(brain).fetch_all(&mut *tx).await?;
    if candidates.len() > NODES {
        return Err(Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytics_scope_too_large",
            "The organization input cohort exceeds its complete bounded envelope.",
        ));
    }
    let eligible: Vec<Uuid> =
        sqlx::query_scalar("SELECT input_id FROM recollect_mapping_current_input_ids($1,$2)")
            .bind(brain)
            .bind(&candidates)
            .fetch_all(&mut *tx)
            .await?;
    let mut inputs: Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT i.id,i.source_version_id FROM knowledge_mapping_inputs i WHERE i.brain_id=$1 AND i.id=ANY($2) ORDER BY i.id")
        .bind(brain).bind(&eligible).fetch_all(&mut *tx).await?;
    let sources: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT source_id,id FROM source_versions WHERE brain_id=$1 AND id=ANY($2) ORDER BY id",
    )
    .bind(brain)
    .bind(inputs.iter().map(|(_, v)| *v).collect::<Vec<_>>())
    .fetch_all(&mut *tx)
    .await?;
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let mut qualified_versions = BTreeSet::new();
    let mut qualified_deadline = None;
    let mut scope: GraphSelection =
        serde_json::from_value(json!({"kind":"knowledge"})).map_err(|_| changed())?;
    crate::retrieval::graph::authorize(&mut tx, &auth, brain, &scope).await?;
    for page in sources.chunks(250) {
        let entities: Vec<_> = page
            .iter()
            .map(|(source, version)| Entity {
                kind: "source_version".into(),
                id: *source,
                revision_id: *version,
            })
            .collect();
        let qualified = crate::retrieval::graph::qualify(
            state,
            &mut tx,
            &auth,
            brain,
            &mut scope,
            &entities,
            (250, Some(at)),
        )
        .await?;
        for (_, version) in page {
            if qualified
                .nodes
                .contains_key(&key("source_version", *version))
            {
                qualified_versions.insert(*version);
            }
        }
        qualified_deadline = [qualified_deadline, qualified.deadline]
            .into_iter()
            .flatten()
            .min();
    }
    inputs.retain(|(_, v)| qualified_versions.contains(v));
    let eligible: Vec<_> = inputs.iter().map(|(i, _)| *i).collect();
    let rows: Vec<(Uuid,Uuid,Uuid,String)>=sqlx::query_as("SELECT DISTINCT m.input_id,i.source_version_id,m.entity_id,e.kind FROM knowledge_mentions m JOIN knowledge_mapping_inputs i ON i.id=m.input_id AND i.brain_id=m.brain_id JOIN knowledge_entities e ON e.id=m.entity_id AND e.brain_id=m.brain_id WHERE m.brain_id=$1 AND m.input_id=ANY($2) AND e.kind IN ('technology','concept') ORDER BY m.input_id,m.entity_id LIMIT 50001")
        .bind(brain).bind(&eligible).fetch_all(&mut *tx).await?;
    if rows.len() > EDGES {
        return Err(Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytics_scope_too_large",
            "The complete organization affinity projection exceeds its edge limit.",
        ));
    }
    let associations: Vec<_> = rows
        .into_iter()
        .map(
            |(input_id, source_version_id, entity_id, kind)| Association {
                input_id,
                source_version_id,
                entity_id,
                kind: if kind == "technology" {
                    EntityKind::Technology
                } else {
                    EntityKind::Concept
                },
                boilerplate: false,
            },
        )
        .collect();
    let mut projection = affinity::prepare(&inputs, &associations).map_err(|_| {
        Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "analytics_scope_too_large",
            "The exact affinity projection exceeds its complete bounded envelope.",
        )
    })?;
    let mut features = None;
    let mut vector_coverage = None;
    if let Some((provider, model, dimensions, representation, parameters)) = hybrid_profile {
        let (prepared, reason) = source_features::prepare(
            state,
            &mut tx,
            &auth,
            brain,
            &inputs,
            (&provider, &model, dimensions, &representation),
            parameters,
        )
        .await?;
        vector_coverage = Some(reason);
        if let Some((f, p)) = prepared {
            vector_coverage = Some(f.coverage.clone());
            projection = p;
            features = Some(f);
        }
    }
    let source_deadline: Option<DateTime<Utc>>=sqlx::query_scalar("SELECT min(recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)) FROM source_versions v WHERE v.brain_id=$1 AND v.id=ANY($2)")
        .bind(brain).bind(inputs.iter().map(|(_,v)|*v).collect::<Vec<_>>()).fetch_one(&mut *tx).await?;
    let deadline = [
        source_deadline,
        qualified_deadline,
        features.as_ref().and_then(|f| f.deadline),
    ]
    .into_iter()
    .flatten()
    .min();
    super::deadline(&mut tx, deadline).await?;
    tx.commit().await?;
    Ok(Prepared {
        epoch,
        revision,
        recipe,
        gds_version,
        inputs,
        projection,
        deadline,
        authority_deadline,
        features,
        vector_coverage,
    })
}

pub(super) fn frozen(projection: &Projection) -> Frozen {
    let nodes: Vec<_> = projection
        .nodes
        .iter()
        .map(|n| match n {
            Vertex::Input { id, .. } => format!("input:{id}"),
            Vertex::Term { id } => format!("term:{id}"),
        })
        .collect();
    let edges = projection
        .edges
        .iter()
        .enumerate()
        .map(|(index, e)| GraphEdge {
            id: Uuid::from_u128(index as u128 + 1),
            from: nodes[e.from].clone(),
            to: nodes[e.to].clone(),
            family: "semantic_affinity".into(),
            relation: if matches!(projection.nodes[e.to], Vertex::Term { .. }) {
                "entity_term"
            } else {
                "retained_source_cosine"
            }
            .into(),
            evidence_kind: if projection.recipe == source_features::RECIPE {
                "source_feature_input"
            } else {
                "mapping_input"
            }
            .into(),
            evidence_id: match projection.nodes[e.from] {
                Vertex::Input { id, .. } => id,
                _ => Uuid::nil(),
            },
            evidence_ordinal: index as i32,
        })
        .collect();
    Frozen {
        nodes,
        edges,
        generations: vec![],
    }
}

async fn fresh(tx: &mut Tx<'_>, brain: Uuid, p: &Prepared) -> Result<()> {
    let current: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM brains b JOIN knowledge_organization_outbox o ON o.brain_id=b.id WHERE b.id=$1 AND b.analytics_epoch=$2 AND o.revision=$3 AND (SELECT h.change_id FROM knowledge_organization_heads h JOIN knowledge_organization_recipes r ON r.change_id=h.change_id AND r.brain_id=h.brain_id WHERE h.brain_id=b.id AND r.quality_state='qualified') IS NOT DISTINCT FROM $4)")
        .bind(brain).bind(p.epoch).bind(p.revision).bind(p.recipe).fetch_one(&mut **tx).await?;
    if !current {
        return Err(changed());
    }
    deadline(
        tx,
        [p.deadline, p.authority_deadline]
            .into_iter()
            .flatten()
            .min(),
    )
    .await
}

async fn schedule_brain(state: &AppState, brain: Uuid, actor: Uuid) -> Result<usize> {
    let mut routing = db::preparation_tx(&state.pool, actor, None).await?;
    let due: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM knowledge_organization_outbox WHERE brain_id=$1 AND (revision>handled_revision OR deadline<=clock_timestamp()) AND NOT EXISTS(SELECT 1 FROM analytics_reports WHERE brain_id=$1 AND purpose='organization' AND state IN ('queued','running')))")
        .bind(brain).fetch_one(&mut *routing).await?;
    let routed_revision: Option<i64> =
        sqlx::query_scalar("SELECT revision FROM knowledge_organization_outbox WHERE brain_id=$1")
            .bind(brain)
            .fetch_optional(&mut *routing)
            .await?;
    routing.commit().await?;
    if !due {
        return Ok(0);
    }
    let prepare_until = tokio::time::Instant::now() + Duration::from_secs(5);
    let prepared = tokio::time::timeout_at(prepare_until, prepare(state, brain, actor, None))
        .await
        .map_err(|_| {
            failure(
                "analytics_preparation_timeout",
                "Organization preparation exceeded its bounded window.",
            )
        })?;
    let p = match prepared {
        Ok(p) => p,
        Err(e) if e.1 == "organization_vector_corrupt" => {
            reconcile_without_machine(
                state,
                brain,
                actor,
                routed_revision,
                prepare_until,
                "failed",
                Some("corrupt"),
            )
            .await?;
            return Err(e);
        }
        Err(e) => return Err(e),
    };
    if p.recipe.is_none() || p.projection.nodes.is_empty() {
        return reconcile_without_machine(
            state,
            brain,
            actor,
            Some(p.revision),
            prepare_until,
            if p.recipe.is_none() {
                "blocked_recipe"
            } else {
                "empty"
            },
            p.vector_coverage.as_deref(),
        )
        .await;
    }
    tokio::time::timeout(Duration::from_millis(1500),async {
    let mut tx = admission::brain_tx_as(&state.pool, actor, None, brain, BrainAccess::WRITE).await?.tx;
    sqlx::query("SET LOCAL statement_timeout='1000ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL lock_timeout='250ms'")
        .execute(&mut *tx)
        .await?;
    fresh(&mut tx, brain, &p).await?;
    if let Some(features) = &p.features {
        source_features::persist(&mut tx, brain, actor, features).await?;
    }
    let active: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM analytics_reports WHERE brain_id=$1 AND purpose='organization' AND state IN ('queued','running'))").bind(brain).fetch_one(&mut *tx).await?;
    if active {
        tx.commit().await?;
        return Ok(0);
    }
    let id = Uuid::new_v4();
    let audit = db::audit(
        &mut tx,
        actor,
        brain,
        "organization.calculate",
        id,
        "queued",
    )
    .await?;
    let job =
        jobs::enqueue_work(&mut tx, actor, brain, audit, id, "graph.analyze", "heavy").await?;
    let sequence = privacy_sequence(&mut tx, brain).await?;
    sqlx::query("INSERT INTO analytics_reports(id,brain_id,actor_id,job_id,purpose,algorithm,direction,descriptor,parameters,analytics_epoch,privacy_sequence,node_count,edge_count,expires_at) VALUES($1,$2,$3,$4,'organization','leiden','both',$5,$6,$7,$8,$9,$10,$11)")
        .bind(id).bind(brain).bind(actor).bind(job).bind(Json(frozen(&p.projection))).bind(Json(parameters("leiden")?)).bind(p.epoch).bind(sequence).bind(p.projection.nodes.len() as i32).bind(p.projection.edges.len() as i32).bind(p.deadline).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_organization_calculations(report_id,brain_id,recipe_change_id,outbox_revision,projection,source_features) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(id).bind(brain).bind(p.recipe).bind(p.revision).bind(Json(&p.projection)).bind(p.features.as_ref().map(Json)).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_organization_inputs(brain_id,report_id,input_id) SELECT $1,$2,unnest($3::uuid[])")
        .bind(brain).bind(id).bind(p.inputs.iter().map(|(i,_)|*i).collect::<Vec<_>>()).execute(&mut *tx).await?;
    if let Some(features) = &p.features {
        hybrid::dependencies(&mut tx, brain, id, features).await?;
    }
    sqlx::query("UPDATE knowledge_organization_outbox SET state='queued',handled_revision=$2,report_id=$3,deadline=$4,vector_coverage=$5,updated_at=clock_timestamp() WHERE brain_id=$1")
        .bind(brain).bind(p.revision).bind(id).bind(p.deadline).bind(&p.vector_coverage).execute(&mut *tx).await?;
    deadline(
        &mut tx,
        [p.deadline, p.authority_deadline]
            .into_iter()
            .flatten()
            .min(),
    )
    .await?;
    tx.commit().await?;
    Ok(1)
    }).await.map_err(|_|failure("analytics_publication_timeout","Organization scheduling exceeded its bounded writer window."))?
}

async fn reconcile_without_machine(
    state: &AppState,
    brain: Uuid,
    actor: Uuid,
    revision: Option<i64>,
    until: tokio::time::Instant,
    status: &str,
    vector_coverage: Option<&str>,
) -> Result<usize> {
    let auth = intent_prepare(until, actor_auth(state, actor, None)).await?;
    let (epoch,current_revision)=intent_prepare(until,async {
        let mut read = db::preparation_tx(&state.pool, actor, None).await?;
        let snapshot:(i64,i64)=sqlx::query_as("SELECT b.analytics_epoch,o.revision FROM brains b JOIN knowledge_organization_outbox o ON o.brain_id=b.id WHERE b.id=$1").bind(brain).fetch_one(&mut *read).await?;
        read.commit().await?;
        Ok(snapshot)
    }).await?;
    if revision != Some(current_revision) {
        return Err(changed());
    }
    let saved = intent_prepare(
        until,
        manual_intent::prepare_saved(state, &auth, brain, epoch),
    )
    .await?;
    let changed_topics = !saved.topic_ids().is_empty();
    let manual = intent_prepare(
        until,
        manual_intent::prepare_publication(state, &auth, brain, saved, &[]),
    )
    .await?;
    tokio::time::timeout(Duration::from_millis(1500),async {
        let mut tx = admission::brain_tx_as(&state.pool, actor, None, brain, BrainAccess::WRITE).await?.tx;
        sqlx::query("SET LOCAL statement_timeout='750ms'").execute(&mut *tx).await?;
        sqlx::query("SET LOCAL lock_timeout='250ms'").execute(&mut *tx).await?;
        let fresh:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM brains b JOIN knowledge_organization_outbox o ON o.brain_id=b.id WHERE b.id=$1 AND b.analytics_epoch=$2 AND o.revision=$3)").bind(brain).bind(epoch).bind(current_revision).fetch_one(&mut *tx).await?;
        if !fresh{return Err(changed());}
        manual_intent::apply_publication(&mut tx,brain,actor,&manual).await?;
        if changed_topics {
            sqlx::query("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1").bind(brain).execute(&mut *tx).await?;
            db::audit(&mut tx,actor,brain,"organization.reconcile",brain,"human_intent").await?;
        }
        sqlx::query("UPDATE knowledge_organization_outbox SET handled_revision=$2,state=$3,report_id=NULL,deadline=$4,vector_coverage=$5,updated_at=clock_timestamp() WHERE brain_id=$1")
            .bind(brain).bind(current_revision).bind(status).bind(manual.deadline()).bind(vector_coverage).execute(&mut *tx).await?;
        deadline(&mut tx,manual.deadline()).await?;
        tx.commit().await?;
        Ok(0)
    }).await.map_err(|_|failure("analytics_publication_timeout","Human organization reconciliation exceeded its bounded writer window."))?
}

pub(super) async fn schedule(state: &AppState) -> Result<usize> {
    let brains: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT brain_id,actor_id FROM recollect_organization_brains()")
            .fetch_all(&state.pool)
            .await?;
    let mut count = 0;
    for (brain, actor) in brains {
        match schedule_brain(state, brain, actor).await {
            Ok(n) => count += n,
            Err(e) => tracing::warn!(brain_id=%brain,code=e.1,"Weighted organization is waiting"),
        }
    }
    Ok(count)
}

pub(super) async fn run(
    state: &AppState,
    job: &ClaimedJob,
    armed: &mut Option<journal::Entry>,
) -> Result<()> {
    let p = prepare(state, job.brain_id, job.actor_id, job.device_id).await?;
    let mut read = db::preparation_tx(&state.pool, job.actor_id, job.device_id).await?;
    let saved: Option<(Json<Projection>,Uuid,i64)>=sqlx::query_as("SELECT projection,recipe_change_id,outbox_revision FROM knowledge_organization_calculations WHERE brain_id=$1 AND report_id=$2")
        .bind(job.brain_id).bind(job.target_id).fetch_optional(&mut *read).await?;
    let Some((Json(projection), recipe, revision)) = saved else {
        return Err(changed());
    };
    if projection != p.projection || Some(recipe) != p.recipe || revision != p.revision {
        return Err(changed());
    }
    read.commit().await?;
    let entry = journal::arm(state, job).await?;
    *armed = Some(entry.clone());
    let (mut tx, r, _) = work::ready(state, job).await?;
    fresh(&mut tx, job.brain_id, &p).await?;
    if r.analytics_epoch != p.epoch {
        return Err(changed());
    }
    journal::insert(&mut tx, &entry).await?;
    tx.commit().await?;
    let weights: Vec<_> = projection.edges.iter().map(|e| e.weight).collect();
    let d = frozen(&projection);
    let mut calculation = Box::pin(native::calculate_weighted(state, &r, &d, &entry, &weights));
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.tick().await;
    let output = loop {
        tokio::select! {result=&mut calculation=>break result?,_=interval.tick()=>{tokio::select!{biased;result=&mut calculation=>break result?,checked=work::progress(state,job,&entry)=>checked?,}}}
    };
    journal::close(&state.config, &state.pool, &state.http, &entry).await?;
    *armed = None;
    // Native scores are provisional. The publication implementation rechecks
    // the entire exact cohort, then uses a fresh short authenticated writer.
    if p.features.is_some() {
        hybrid::publish(state, job, &p, &d, output).await
    } else {
        publish(state, job, &p, &d, output).await
    }
}

async fn actor_auth(state: &AppState, actor: Uuid, device: Option<Uuid>) -> Result<Auth> {
    let mut tx = db::preparation_tx(&state.pool, actor, device).await?;
    let Json(user) =
        sqlx::query_scalar("SELECT to_jsonb(a) FROM accounts a WHERE id=$1 AND enabled")
            .bind(actor)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Auth {
        user,
        device_id: device,
        session: Uuid::nil(),
        csrf: Uuid::nil(),
    })
}
async fn intent_prepare<T>(
    until: tokio::time::Instant,
    f: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    tokio::time::timeout_at(until, f).await.map_err(|_| {
        failure(
            "analytics_preparation_timeout",
            "Human organization preparation exceeded its bounded window.",
        )
    })?
}

async fn publish(
    state: &AppState,
    job: &ClaimedJob,
    p: &Prepared,
    d: &Frozen,
    output: native::Output,
) -> Result<()> {
    // Reprepare complete canonical inputs after native scratch has been evicted.
    // Neither GDS group numbers nor a formerly valid epoch establish currency.
    let prepare_until = tokio::time::Instant::now() + Duration::from_secs(5);
    let current = intent_prepare(
        prepare_until,
        prepare(state, job.brain_id, job.actor_id, job.device_id),
    )
    .await?;
    let auth = intent_prepare(
        prepare_until,
        actor_auth(state, job.actor_id, job.device_id),
    )
    .await?;
    let saved = intent_prepare(
        prepare_until,
        manual_intent::prepare_saved(state, &auth, job.brain_id, p.epoch),
    )
    .await?;
    if p.gds_version.as_deref() != Some(output.version.as_str()) {
        return Err(Error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "organization_engine_version_changed",
            "The graph engine does not match this recipe's independently qualified version.",
        ));
    }
    if current.inputs != p.inputs
        || current.projection != p.projection
        || current.recipe != p.recipe
        || current.revision != p.revision
        || current.epoch != p.epoch
    {
        return Err(changed());
    }
    let (old, old_changes) = intent_prepare(prepare_until, async {
    let mut read = db::preparation_tx(&state.pool, job.actor_id, job.device_id).await?;
    let rows: Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT DISTINCT t.id,i.source_version_id FROM knowledge_topics t JOIN knowledge_topic_memberships m ON m.topic_id=t.id AND m.brain_id=t.brain_id JOIN knowledge_mapping_inputs i ON i.id=m.input_id AND i.brain_id=m.brain_id WHERE t.brain_id=$1 AND t.kind='community' AND m.input_id=ANY($2) ORDER BY t.id,i.source_version_id")
        .bind(job.brain_id).bind(p.inputs.iter().map(|(i,_)|*i).collect::<Vec<_>>()).fetch_all(&mut *read).await?;
    let mut old = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
    for (id, version) in rows {
        old.entry(id).or_default().insert(version);
    }
    for (topic, versions) in saved.versions() {
        if !saved.is_overlay(topic) {
            old.insert(topic, versions);
        }
    }
    let old_changes: BTreeMap<Uuid, Uuid> = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id,change_id FROM knowledge_topics WHERE brain_id=$1 AND kind='community'",
    )
    .bind(job.brain_id)
    .fetch_all(&mut *read)
    .await?
    .into_iter()
    .collect();
    read.commit().await?;
    Ok((old, old_changes))
    }).await?;
    let indices: BTreeMap<_, _> = d
        .nodes
        .iter()
        .enumerate()
        .map(|(index, key)| (key.as_str(), index))
        .collect();
    let mut grouped = BTreeMap::<i64, BTreeSet<Uuid>>::new();
    if output.scores.len() != d.nodes.len() {
        return Err(changed());
    }
    for score in &output.scores {
        let index = *indices.get(score.key.as_str()).ok_or_else(changed)?;
        if let Vertex::Input { id, .. } = p.projection.nodes[index] {
            grouped
                .entry(score.group.ok_or_else(changed)?)
                .or_default()
                .insert(id);
        }
    }
    // Sort by canonical contributors, never by ephemeral native community ID.
    let mut groups: Vec<_> = grouped.into_values().collect();
    groups.sort();
    let versions: BTreeMap<_, _> = p.inputs.iter().copied().collect();
    let previous: Vec<_> = old.iter().map(|(id, v)| (*id, v.clone())).collect();
    let mut used = BTreeSet::new();
    let mut assignments = vec![];
    for inputs in groups {
        let contributors = inputs.iter().map(|id| versions[id]).collect();
        let retained = affinity::match_topic(&contributors, &previous, &used);
        let topic = retained.unwrap_or_else(Uuid::new_v4);
        used.insert(topic);
        assignments.push((topic, retained.is_some(), inputs, contributors));
    }
    let mut topic_ids = vec![];
    let mut entity_ids = vec![];
    let mut input_ids = vec![];
    let mut weights = vec![];
    let mut topics = vec![];
    let mut changes = vec![];
    let mut previous_changes = vec![];
    let mut lineage_topics = vec![];
    let mut lineage_previous = vec![];
    let mut lineage_basis = vec![];
    let mut lineage_ids = vec![];
    for (topic, retained, inputs, contributors) in &assignments {
        let change = Uuid::new_v4();
        topics.push(*topic);
        changes.push(change);
        let previous_change = old_changes.get(topic).copied();
        if *retained != previous_change.is_some() {
            return Err(changed());
        }
        previous_changes.push(previous_change);
        for (prior, prior_versions) in &old {
            let surviving: BTreeSet<_> =
                contributors.intersection(prior_versions).copied().collect();
            if surviving.is_empty() {
                continue;
            }
            // Every lineage edge is bound to an input supported by BOTH the
            // old and new communities; arbitrary surviving inputs cannot retain
            // an unsupported association after erasure.
            let basis = *inputs
                .iter()
                .find(|input| surviving.contains(&versions[input]))
                .ok_or_else(changed)?;
            lineage_ids.push(Uuid::new_v4());
            lineage_topics.push(*topic);
            lineage_previous.push(*prior);
            lineage_basis.push(basis);
        }
    }
    let assigned: BTreeMap<Uuid, Uuid> = assignments
        .iter()
        .flat_map(|(topic, _, inputs, _)| inputs.iter().map(move |input| (*input, *topic)))
        .collect();
    for edge in &p.projection.edges {
        let (Vertex::Input { id: input, .. }, Vertex::Term { id: entity }) =
            (&p.projection.nodes[edge.from], &p.projection.nodes[edge.to])
        else {
            return Err(changed());
        };
        let topic = *assigned.get(input).ok_or_else(changed)?;
        topic_ids.push(topic);
        entity_ids.push(*entity);
        input_ids.push(*input);
        weights.push(edge.weight);
    }
    let edited: BTreeSet<_> = saved.topic_ids().into_iter().collect();
    let mut cohorts: BTreeMap<_, _> = assignments
        .iter()
        .filter(|(topic, _, _, _)| edited.contains(topic))
        .map(|(topic, _, _, _)| {
            (
                *topic,
                manual_intent::Cohort {
                    topic: *topic,
                    sources: vec![],
                    mappings: vec![],
                },
            )
        })
        .collect();
    for ((topic, input), entity) in topic_ids.iter().zip(&input_ids).zip(&entity_ids) {
        if let Some(cohort) = cohorts.get_mut(topic) {
            cohort.mappings.push((*input, *entity));
        }
    }
    let cohorts: Vec<_> = cohorts.into_values().collect();
    let manual = intent_prepare(
        prepare_until,
        manual_intent::prepare_publication(state, &auth, job.brain_id, saved, &cohorts),
    )
    .await?;
    tokio::time::timeout(Duration::from_millis(1500),async {
    let (mut tx,r,_)=work::ready(state,job).await?;
    sqlx::query("SET LOCAL statement_timeout='1000ms'").execute(&mut *tx).await?;
    sqlx::query("SET LOCAL lock_timeout='250ms'").execute(&mut *tx).await?;
    fresh(&mut tx,job.brain_id,p).await?;
    if r.analytics_epoch!=p.epoch {return Err(changed());}
    manual_intent::apply_publication(&mut tx,job.brain_id,job.actor_id,&manual).await?;
    // Replace only native memberships. Explicit split membership and all
    // authenticated overrides remain separate canonical records.
    sqlx::query("DELETE FROM knowledge_topic_sources m USING knowledge_topics t WHERE m.brain_id=$1 AND t.brain_id=m.brain_id AND t.id=m.topic_id AND t.kind='community' AND m.reason IN('hybrid_entity_affinity','hybrid_retained_vector','hybrid_entity_and_vector')").bind(job.brain_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM knowledge_topic_memberships m USING knowledge_topics t WHERE m.brain_id=$1 AND t.brain_id=m.brain_id AND t.id=m.topic_id AND t.kind='community' AND m.reason='weighted_leiden'")
        .bind(job.brain_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe,organization_report_id) SELECT id,$1,'community',change,$4,$5 FROM unnest($2::uuid[],$3::uuid[]) x(id,change) ON CONFLICT(id) DO UPDATE SET change_id=excluded.change_id,recipe=excluded.recipe,organization_report_id=excluded.organization_report_id")
        .bind(job.brain_id).bind(&topics).bind(&changes).bind(affinity::RECIPE).bind(job.target_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action,previous_change_id) SELECT change,$1,topic,$2,CASE WHEN previous IS NULL THEN 'create' ELSE 'rebuild' END,previous FROM unnest($3::uuid[],$4::uuid[],$5::uuid[]) x(change,topic,previous)")
        .bind(job.brain_id).bind(job.actor_id).bind(&changes).bind(&topics).bind(&previous_changes).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT $1,topic,entity,input,'weighted_leiden',weight FROM unnest($2::uuid[],$3::uuid[],$4::uuid[],$5::float8[]) x(topic,entity,input,weight) ON CONFLICT(topic_id,entity_id,input_id) DO UPDATE SET weight=excluded.weight")
        .bind(job.brain_id).bind(&topic_ids).bind(&entity_ids).bind(&input_ids).bind(&weights).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO knowledge_organization_lineage(id,brain_id,topic_id,previous_topic_id,basis_input,report_id) SELECT id,$1,topic,previous,basis,$2 FROM unnest($3::uuid[],$4::uuid[],$5::uuid[],$6::uuid[]) x(id,topic,previous,basis)")
        .bind(job.brain_id).bind(job.target_id).bind(&lineage_ids).bind(&lineage_topics).bind(&lineage_previous).bind(&lineage_basis).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM knowledge_topics t WHERE brain_id=$1 AND kind='community' AND NOT EXISTS(SELECT 1 FROM knowledge_topic_memberships m WHERE m.topic_id=t.id AND m.brain_id=t.brain_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_sources m WHERE m.topic_id=t.id AND m.brain_id=t.brain_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_sources m WHERE m.topic_id=t.id AND m.brain_id=t.brain_id)")
        .bind(job.brain_id).execute(&mut *tx).await?;
    let epoch:i64=sqlx::query_scalar("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1 RETURNING analytics_epoch").bind(job.brain_id).fetch_one(&mut *tx).await?;
    sqlx::query("UPDATE analytics_reports SET state='ready',scores=$3,gds_version=$4,estimated_bytes=$5,projected_edge_count=$6,published_at=clock_timestamp(),analytics_epoch=$7,error_code=NULL WHERE brain_id=$1 AND id=$2")
        .bind(job.brain_id).bind(job.target_id).bind(Json(&output.scores)).bind(output.version).bind(output.estimate).bind(output.projected_edges as i32).bind(epoch).execute(&mut *tx).await?;
    sqlx::query("UPDATE knowledge_organization_outbox SET state='ready',handled_revision=$2,report_id=$3,deadline=$4,updated_at=clock_timestamp() WHERE brain_id=$1")
        .bind(job.brain_id).bind(p.revision).bind(job.target_id).bind(p.deadline).execute(&mut *tx).await?;
    let lease_until:DateTime<Utc>=sqlx::query_scalar("SELECT lease_until FROM jobs WHERE id=$1 AND lease_token=$2").bind(job.id).bind(job.lease_token).fetch_one(&mut *tx).await?;
    let changed_job=sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,error_code=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()")
        .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
    if changed_job.rows_affected()!=1 {return Err(failure("analytics_lease_lost","The analytical publication lease expired."));}
    db::audit(&mut tx,job.actor_id,job.brain_id,"organization.publish",job.target_id,"ready").await?;
    deadline(&mut tx,[p.deadline,p.authority_deadline,manual.deadline(),Some(lease_until)].into_iter().flatten().min()).await?;
    tx.commit().await?;
    Ok(())
    }).await.map_err(|_|failure("analytics_publication_timeout","Organization publication exceeded its bounded writer window."))?
}
