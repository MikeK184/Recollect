//! Typed source publication for the bounded, independently unqualified hybrid.
use super::*;

pub(super) async fn dependencies(
    tx: &mut Tx<'_>,
    brain: Uuid,
    report: Uuid,
    f: &source_features::Features,
) -> Result<()> {
    sqlx::query("INSERT INTO knowledge_organization_sources(brain_id,report_id,input_id) SELECT $1,$2,unnest($3::uuid[])")
        .bind(brain).bind(report).bind(f.sources.iter().map(|s|s.id).collect::<Vec<_>>()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO knowledge_organization_vectors(brain_id,report_id,entry_id,profile_id,request_id) SELECT $1,$2,entry,profile,request FROM jsonb_to_recordset($3) x(entry uuid,profile uuid,request uuid)")
        .bind(brain).bind(report).bind(Json(&f.vectors)).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn publish(
    state: &AppState,
    job: &ClaimedJob,
    p: &Prepared,
    d: &Frozen,
    output: native::Output,
) -> Result<()> {
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
        return Err(failure(
            "organization_engine_version_changed",
            "The graph engine does not match the frozen qualified recipe.",
        ));
    }
    if current.inputs != p.inputs
        || current.features != p.features
        || current.projection != p.projection
        || current.recipe != p.recipe
        || current.revision != p.revision
        || current.epoch != p.epoch
    {
        return Err(changed());
    }
    let features = p.features.as_ref().ok_or_else(changed)?;
    let source_ids: Vec<_> = features.sources.iter().map(|s| s.id).collect();
    let (old, changes) = intent_prepare(prepare_until, async {
    let mut tx = db::preparation_tx(&state.pool, job.actor_id, job.device_id).await?;
    // Legacy membership transfer is possible only through exact immutable
    // version AND captured context equality, never a title/entity resemblance.
    let rows:Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT t.id,m.input_id FROM knowledge_topics t JOIN knowledge_topic_sources m ON m.brain_id=t.brain_id AND m.topic_id=t.id WHERE t.brain_id=$1 AND t.kind='community' AND m.input_id=ANY($2) UNION SELECT t.id,s.id FROM knowledge_topics t JOIN knowledge_topic_memberships m ON m.brain_id=t.brain_id AND m.topic_id=t.id JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_source_inputs s ON s.brain_id=i.brain_id AND s.source_version_id=i.source_version_id AND s.context=c.context WHERE t.brain_id=$1 AND t.kind='community' AND s.id=ANY($2)")
        .bind(job.brain_id).bind(&source_ids).fetch_all(&mut *tx).await?;
    let mut old = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
    for (t, s) in rows {
        old.entry(t).or_default().insert(s);
    }
    for (topic, sources) in saved.source_ids() {
        if !saved.is_overlay(topic) {
            old.insert(topic, sources);
        }
    }
    let changes: BTreeMap<Uuid, Uuid> = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT id,change_id FROM knowledge_topics WHERE brain_id=$1 AND kind='community'",
    )
    .bind(job.brain_id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();
    tx.commit().await?;
    Ok((old, changes))
    }).await?;
    let indices: BTreeMap<_, _> = d
        .nodes
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    if output.scores.len() != d.nodes.len() {
        return Err(changed());
    }
    let mut grouped = BTreeMap::<i64, BTreeSet<Uuid>>::new();
    for score in &output.scores {
        let index = *indices.get(score.key.as_str()).ok_or_else(changed)?;
        if let Vertex::Input { id, .. } = p.projection.nodes[index] {
            grouped
                .entry(score.group.ok_or_else(changed)?)
                .or_default()
                .insert(id);
        }
    }
    let mut groups: Vec<_> = grouped.into_values().collect();
    groups.sort();
    let previous: Vec<_> = old.iter().map(|(t, s)| (*t, s.clone())).collect();
    let mut used = BTreeSet::new();
    let mut feature_basis = BTreeMap::<Uuid, (bool, bool)>::new();
    for edge in &p.projection.edges {
        match (&p.projection.nodes[edge.from], &p.projection.nodes[edge.to]) {
            (Vertex::Input { id, .. }, Vertex::Term { .. }) => {
                feature_basis.entry(*id).or_default().0 = true
            }
            (Vertex::Input { id: a, .. }, Vertex::Input { id: b, .. }) => {
                feature_basis.entry(*a).or_default().1 = true;
                feature_basis.entry(*b).or_default().1 = true;
            }
            _ => return Err(changed()),
        }
    }
    let mut reasons = vec![];
    let mut topic_ids = vec![];
    let mut input_ids = vec![];
    let mut topics = vec![];
    let mut change_ids = vec![];
    let mut previous_changes = vec![];
    let mut lineage = vec![];
    for inputs in groups {
        let retained = affinity::match_topic(&inputs, &previous, &used);
        let topic = retained.unwrap_or_else(Uuid::new_v4);
        used.insert(topic);
        topics.push(topic);
        change_ids.push(Uuid::new_v4());
        previous_changes.push(retained.and_then(|t| changes.get(&t).copied()));
        for (old_topic, old_inputs) in &old {
            if let Some(basis) = inputs.intersection(old_inputs).next() {
                lineage.push(
                    json!({"id":Uuid::new_v4(),"topic":topic,"previous":old_topic,"basis":basis}),
                );
            }
        }
        for input in inputs {
            topic_ids.push(topic);
            input_ids.push(input);
            let basis = feature_basis.get(&input).copied().ok_or_else(changed)?;
            reasons.push(match basis {
                (true, true) => "hybrid_entity_and_vector",
                (true, false) => "hybrid_entity_affinity",
                (false, true) => "hybrid_retained_vector",
                _ => return Err(changed()),
            });
        }
    }
    let edited: BTreeSet<_> = saved.topic_ids().into_iter().collect();
    let mut cohort_map: BTreeMap<Uuid, manual_intent::Cohort> = topics
        .iter()
        .filter(|t| edited.contains(t))
        .map(|t| {
            (
                *t,
                manual_intent::Cohort {
                    topic: *t,
                    mappings: vec![],
                    sources: vec![],
                },
            )
        })
        .collect();
    for (topic, input) in topic_ids.iter().zip(&input_ids) {
        if let Some(cohort) = cohort_map.get_mut(topic) {
            cohort.sources.push(*input);
        }
    }
    let mappings=intent_prepare(prepare_until,async {
        let mut membership_read = db::preparation_tx(&state.pool, job.actor_id, job.device_id).await?;
        sqlx::query("SET LOCAL statement_timeout='4000ms'").execute(&mut *membership_read).await?;
        sqlx::query("SET LOCAL recollect.bulk_qualification=on").execute(&mut *membership_read).await?;
        let mappings:Vec<(Uuid,Uuid,Uuid)>=sqlx::query_as("SELECT DISTINCT m.topic,i.id,n.entity_id FROM unnest($2::uuid[],$3::uuid[]) m(topic,input) JOIN knowledge_source_inputs s ON s.brain_id=$1 AND s.id=m.input JOIN knowledge_mapping_inputs i ON i.brain_id=s.brain_id AND i.source_version_id=s.source_version_id AND i.id=ANY($4) JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id AND c.context=s.context JOIN knowledge_mentions n ON n.brain_id=i.brain_id AND n.input_id=i.id JOIN knowledge_entities e ON e.brain_id=n.brain_id AND e.id=n.entity_id AND e.kind IN('technology','concept') WHERE m.topic=ANY($5)")
        .bind(job.brain_id).bind(&topic_ids).bind(&input_ids).bind(p.inputs.iter().map(|(i,_)|*i).collect::<Vec<_>>()).bind(cohort_map.keys().copied().collect::<Vec<_>>()).fetch_all(&mut *membership_read).await?;
        membership_read.commit().await?;
        Ok(mappings)
    }).await?;
    for (topic, input, entity) in mappings {
        cohort_map
            .get_mut(&topic)
            .ok_or_else(changed)?
            .mappings
            .push((input, entity));
    }
    let cohorts: Vec<_> = cohort_map.into_values().collect();
    let manual = intent_prepare(
        prepare_until,
        manual_intent::prepare_publication(state, &auth, job.brain_id, saved, &cohorts),
    )
    .await?;
    tokio::time::timeout(Duration::from_millis(1500),async {
        let(mut tx,r,_)=work::ready(state,job).await?;
        sqlx::query("SET LOCAL statement_timeout='1000ms'").execute(&mut *tx).await?;
        sqlx::query("SET LOCAL lock_timeout='250ms'").execute(&mut *tx).await?;
        fresh(&mut tx,job.brain_id,p).await?;if r.analytics_epoch!=p.epoch{return Err(changed());}
        manual_intent::apply_publication(&mut tx,job.brain_id,job.actor_id,&manual).await?;
        // Stable source-input identities make all explicit source overrides
        // survive a matched rebuild without transferring them across correction.
        sqlx::query("DELETE FROM knowledge_topic_sources m USING knowledge_topics t WHERE m.brain_id=$1 AND t.id=m.topic_id AND t.brain_id=m.brain_id AND t.kind='community' AND m.reason IN('hybrid_entity_affinity','hybrid_retained_vector','hybrid_entity_and_vector')")
            .bind(job.brain_id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM knowledge_topic_memberships m USING knowledge_topics t WHERE m.brain_id=$1 AND t.id=m.topic_id AND t.brain_id=m.brain_id AND t.kind='community' AND m.reason='weighted_leiden'")
            .bind(job.brain_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe,organization_report_id) SELECT id,$1,'community',change,$4,$5 FROM unnest($2::uuid[],$3::uuid[]) x(id,change) ON CONFLICT(id) DO UPDATE SET change_id=excluded.change_id,recipe=excluded.recipe,organization_report_id=excluded.organization_report_id")
            .bind(job.brain_id).bind(&topics).bind(&change_ids).bind(source_features::RECIPE).bind(job.target_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_topic_changes(id,brain_id,topic_id,actor_id,action,previous_change_id) SELECT change,$1,topic,$2,CASE WHEN previous IS NULL THEN 'create' ELSE 'rebuild' END,previous FROM unnest($3::uuid[],$4::uuid[],$5::uuid[]) x(change,topic,previous)")
            .bind(job.brain_id).bind(job.actor_id).bind(&change_ids).bind(&topics).bind(&previous_changes).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT $1,topic,input,reason,1 FROM unnest($2::uuid[],$3::uuid[],$4::text[]) x(topic,input,reason) ON CONFLICT(topic_id,input_id) DO UPDATE SET reason=excluded.reason,weight=excluded.weight")
            .bind(job.brain_id).bind(&topic_ids).bind(&input_ids).bind(&reasons).execute(&mut *tx).await?;
        // Only genuine backed NER terms retain the existing entity membership
        // API. Vector-only contributors never borrow an arbitrary entity.
        sqlx::query("INSERT INTO knowledge_topic_memberships(brain_id,topic_id,entity_id,input_id,reason,weight) SELECT DISTINCT $1,m.topic,mn.entity_id,i.id,'weighted_leiden',1 FROM unnest($2::uuid[],$3::uuid[]) m(topic,input) JOIN knowledge_source_inputs s ON s.id=m.input AND s.brain_id=$1 JOIN knowledge_mapping_inputs i ON i.brain_id=s.brain_id AND i.source_version_id=s.source_version_id AND i.id=ANY($4) JOIN knowledge_mapping_contexts c ON c.id=i.context_id AND c.brain_id=i.brain_id AND c.context=s.context JOIN knowledge_mentions mn ON mn.input_id=i.id AND mn.brain_id=i.brain_id JOIN knowledge_entities e ON e.id=mn.entity_id AND e.brain_id=mn.brain_id AND e.kind IN('technology','concept') ON CONFLICT DO NOTHING")
            .bind(job.brain_id).bind(&topic_ids).bind(&input_ids).bind(p.inputs.iter().map(|(i,_)|*i).collect::<Vec<_>>()).execute(&mut *tx).await?;
        // Transfer legacy explicit intent only through identical immutable
        // source+context, while the surviving canonical topic is still known.
        sqlx::query("INSERT INTO knowledge_topic_source_labels(brain_id,topic_id,basis_input,label) SELECT l.brain_id,l.topic_id,s.id,l.label FROM knowledge_topic_labels l JOIN knowledge_mapping_inputs i ON i.brain_id=l.brain_id AND i.id=l.basis_input JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_source_inputs s ON s.brain_id=i.brain_id AND s.source_version_id=i.source_version_id AND s.context=c.context JOIN knowledge_topic_sources m ON m.brain_id=s.brain_id AND m.topic_id=l.topic_id AND m.input_id=s.id WHERE l.brain_id=$1 AND l.topic_id=ANY($2) ON CONFLICT DO NOTHING")
            .bind(job.brain_id).bind(&topics).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id) SELECT e.brain_id,e.topic_id,s.id FROM knowledge_topic_exclusions e JOIN knowledge_mapping_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_source_inputs s ON s.brain_id=i.brain_id AND s.source_version_id=i.source_version_id AND s.context=c.context JOIN knowledge_topic_sources m ON m.brain_id=s.brain_id AND m.topic_id=e.topic_id AND m.input_id=s.id WHERE e.brain_id=$1 AND e.topic_id=ANY($2) ON CONFLICT DO NOTHING")
            .bind(job.brain_id).bind(&topics).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_topic_source_splits(brain_id,original_topic_id,split_topic_id,input_id) SELECT e.brain_id,e.original_topic_id,e.split_topic_id,s.id FROM knowledge_topic_splits e JOIN knowledge_mapping_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id JOIN knowledge_source_inputs s ON s.brain_id=i.brain_id AND s.source_version_id=i.source_version_id AND s.context=c.context JOIN knowledge_topic_sources m ON m.brain_id=s.brain_id AND m.topic_id=e.original_topic_id AND m.input_id=s.id WHERE e.brain_id=$1 AND e.original_topic_id=ANY($2) ON CONFLICT DO NOTHING")
            .bind(job.brain_id).bind(&topics).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight) SELECT e.brain_id,e.split_topic_id,e.input_id,'explicit_source_split',m.weight FROM knowledge_topic_source_splits e JOIN knowledge_topic_sources m ON m.brain_id=e.brain_id AND m.topic_id=e.original_topic_id AND m.input_id=e.input_id WHERE e.brain_id=$1 AND e.original_topic_id=ANY($2) ON CONFLICT DO NOTHING")
            .bind(job.brain_id).bind(&topics).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_organization_source_lineage(id,brain_id,topic_id,previous_topic_id,basis_input,report_id) SELECT id,$1,topic,previous,basis,$2 FROM jsonb_to_recordset($3) x(id uuid,topic uuid,previous uuid,basis uuid)")
            .bind(job.brain_id).bind(job.target_id).bind(Json(&lineage)).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM knowledge_topics t WHERE brain_id=$1 AND kind='community' AND NOT EXISTS(SELECT 1 FROM knowledge_topic_sources m WHERE m.topic_id=t.id AND m.brain_id=t.brain_id) AND NOT EXISTS(SELECT 1 FROM knowledge_topic_memberships m WHERE m.topic_id=t.id AND m.brain_id=t.brain_id)")
            .bind(job.brain_id).execute(&mut *tx).await?;
        let epoch:i64=sqlx::query_scalar("UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=$1 RETURNING analytics_epoch").bind(job.brain_id).fetch_one(&mut *tx).await?;
        sqlx::query("UPDATE analytics_reports SET state='ready',scores=$3,gds_version=$4,estimated_bytes=$5,projected_edge_count=$6,published_at=clock_timestamp(),analytics_epoch=$7,error_code=NULL WHERE brain_id=$1 AND id=$2")
            .bind(job.brain_id).bind(job.target_id).bind(Json(&output.scores)).bind(&output.version).bind(output.estimate).bind(output.projected_edges as i32).bind(epoch).execute(&mut *tx).await?;
        sqlx::query("UPDATE knowledge_organization_outbox SET state='ready',handled_revision=$2,report_id=$3,deadline=$4,vector_coverage=$5,updated_at=clock_timestamp() WHERE brain_id=$1")
            .bind(job.brain_id).bind(p.revision).bind(job.target_id).bind(p.deadline).bind(&features.coverage).execute(&mut *tx).await?;
        let lease:DateTime<Utc>=sqlx::query_scalar("SELECT lease_until FROM jobs WHERE id=$1 AND lease_token=$2").bind(job.id).bind(job.lease_token).fetch_one(&mut *tx).await?;
        let changed_job=sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL,error_code=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='running' AND lease_token=$2 AND lease_until>clock_timestamp()")
            .bind(job.id).bind(job.lease_token).execute(&mut *tx).await?;
        if changed_job.rows_affected()!=1{return Err(failure("analytics_lease_lost","The analytical publication lease expired."));}
        db::audit(&mut tx,job.actor_id,job.brain_id,"organization.publish",job.target_id,"ready").await?;
        deadline(&mut tx,[p.deadline,p.authority_deadline,manual.deadline(),Some(lease)].into_iter().flatten().min()).await?;
        tx.commit().await?;Ok(())
    }).await.map_err(|_|failure("analytics_publication_timeout","Organization publication exceeded its bounded writer window."))?
}
