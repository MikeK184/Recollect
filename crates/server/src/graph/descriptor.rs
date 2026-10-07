use super::*;
use serde_json::Value;

pub(super) fn capacity() -> Error {
    failure(
        "graph_input_too_large",
        "This graph input exceeds the generation limits. Its projection was not published.",
    )
}

pub(super) async fn available(tx: &mut Tx<'_>, g: &GraphGeneration) -> Result<()> {
    if g.kind == "combined" {
        if combined::origin_epoch(tx, g.brain_id).await? != g.input_epoch
            || g.adapter != combined::LINKER
        {
            return Err(failure(
                "graph_input_changed",
                "Repository origin bindings or linker settings changed. Current combined inputs will be rebuilt automatically.",
            ));
        }
        combined::available_inputs(tx, g.brain_id, &g.input_snapshot_ids).await?;
    } else if let Some(snapshot) = g.snapshot_id {
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM repository_snapshots s WHERE brain_id=$1 AND id=$2
          AND recollect_content_state(brain_id,'repository',privacy_state,created_at)='active'
          AND (SELECT count(*) FROM repository_facts f WHERE f.brain_id=s.brain_id AND f.snapshot_id=s.id)=s.fact_count
          AND EXISTS(SELECT 1 FROM repository_jobs r JOIN jobs j ON j.id=r.job_id WHERE r.brain_id=s.brain_id AND r.snapshot_id=s.id AND j.state='succeeded'))")
            .bind(g.brain_id).bind(snapshot).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(failure(
                "graph_input_unavailable",
                "This repository snapshot is unavailable or has not finished materialization.",
            ));
        }
    } else if epoch(tx, g.brain_id).await? != g.input_epoch {
        return Err(failure(
            "graph_input_changed",
            "Knowledge changed after this generation was queued. A current generation will be built automatically.",
        ));
    }
    Ok(())
}
pub(super) async fn build(tx: &mut Tx<'_>, g: &GraphGeneration) -> Result<Descriptor> {
    available(tx, g).await?;
    let mut d = if g.kind == "combined" {
        combined::build(tx, g).await?
    } else if let Some(snapshot) = g.snapshot_id {
        repository(tx, g.brain_id, snapshot).await?
    } else {
        knowledge(tx, g.brain_id).await?
    };
    d.expires_at = sqlx::query_scalar(include_str!("deadline.sql"))
        .bind(g.brain_id)
        .bind(Json(&d.nodes))
        .bind(g.snapshot_id)
        .fetch_one(&mut **tx)
        .await?;
    if g.kind == "combined" {
        let deadline:Option<chrono::DateTime<chrono::Utc>>=sqlx::query_scalar("SELECT min(recollect_retention_deadline(brain_id,'repository',created_at)) FROM repository_snapshots WHERE brain_id=$1 AND id=ANY($2)")
            .bind(g.brain_id).bind(&g.input_snapshot_ids).fetch_one(&mut **tx).await?;
        d.expires_at = [d.expires_at, deadline].into_iter().flatten().min();
    }
    if d.nodes.len() > 100000
        || d.edges.len() > 250000
        || serde_json::to_vec(&d).map_err(|_| capacity())?.len() > 64 * 1024 * 1024
    {
        return Err(capacity());
    }
    Ok(d)
}

pub(super) async fn retained(tx: &mut Tx<'_>, d: &Descriptor) -> Result<()> {
    let valid: bool = sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
        .bind(d.expires_at)
        .fetch_one(&mut **tx)
        .await?;
    if !valid {
        return Err(failure(
            "graph_input_expired",
            "A graph input expired during this attempt. Its generation was not published.",
        ));
    }
    Ok(())
}

async fn repository(tx: &mut Tx<'_>, brain: Uuid, snapshot: Uuid) -> Result<Descriptor> {
    let facts:Vec<(Uuid,Json<Value>)>=sqlx::query_as("SELECT id,record FROM repository_facts WHERE brain_id=$1 AND snapshot_id=$2 ORDER BY ordinal LIMIT 100001")
        .bind(brain).bind(snapshot).fetch_all(&mut **tx).await?;
    if facts.len() > 100000 {
        return Err(capacity());
    }
    let mut ids: BTreeMap<String, Vec<Uuid>> = BTreeMap::new();
    for (id, record) in &facts {
        if let Some(raw) = record.get("id").and_then(Value::as_str) {
            ids.entry(raw.to_string()).or_default().push(*id);
        }
    }
    let mut d = Descriptor::default();
    for (id, record) in &facts {
        d.nodes.push(Entity {
            kind: "repository_fact".into(),
            id: *id,
            revision_id: *id,
        });
        for (ordinal, relation) in record
            .get("relations")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let kind = relation.get("kind").and_then(Value::as_str).unwrap_or("");
            if !STRUCTURAL.contains(&kind) {
                d.unsupported += 1;
                continue;
            }
            let targets = relation
                .get("target_id")
                .and_then(Value::as_str)
                .and_then(|target| ids.get(target));
            let target = match targets.map(Vec::as_slice) {
                Some([target]) => *target,
                Some(targets) if targets.len() > 1 => {
                    d.ambiguous += 1;
                    continue;
                }
                _ => {
                    d.unresolved += 1;
                    continue;
                }
            };
            if d.edges.len() == 250000 {
                return Err(capacity());
            }
            d.edges.push(GraphEdge {
                id: Uuid::new_v4(),
                from: key("repository_fact", *id),
                to: key("repository_fact", target),
                family: "structural".into(),
                relation: kind.into(),
                evidence_kind: "repository_fact".into(),
                evidence_id: *id,
                evidence_ordinal: ordinal as i32,
            });
        }
    }
    Ok(d)
}

const CLAIMS: &str="WITH current_claims AS MATERIALIZED (
 SELECT r.* FROM claims c JOIN claim_revisions r ON r.id=c.current_revision AND r.brain_id=c.brain_id
 WHERE r.brain_id=$1 AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
 AND recollect_memory_supported(r.brain_id,r.id)
 AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
 AND r.revision#>>'{content,freshness}'<>'superseded'
 AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)
)";
async fn knowledge(tx: &mut Tx<'_>, brain: Uuid) -> Result<Descriptor> {
    let claims: Vec<(Uuid, Uuid)> = sqlx::query_as(&format!(
        "{CLAIMS} SELECT claim_id,id FROM current_claims ORDER BY id LIMIT 100001"
    ))
    .bind(brain)
    .fetch_all(&mut **tx)
    .await?;
    if claims.len() > 100000 {
        return Err(capacity());
    }
    let mut nodes: BTreeMap<String, Entity> = claims
        .iter()
        .map(|(id, revision_id)| {
            let e = Entity {
                kind: "claim".into(),
                id: *id,
                revision_id: *revision_id,
            };
            (e.key(), e)
        })
        .collect();
    let supports:Vec<(Uuid,i32,String,Uuid)>=sqlx::query_as(&format!("{CLAIMS}, supports AS (
      SELECT s.* FROM claim_supports s JOIN current_claims c ON c.id=s.revision_id AND c.brain_id=s.brain_id
    ), edges AS (
      SELECT s.revision_id,s.ordinal,'source_version'::text kind,v.id
      FROM supports s JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=s.brain_id
      WHERE recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
      UNION ALL
      SELECT s.revision_id,s.ordinal,'repository_fact',f.id FROM supports s
      JOIN repository_facts f ON f.id=s.fact_id AND f.brain_id=s.brain_id
      JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=f.brain_id
      WHERE recollect_content_state(p.brain_id,'repository',p.privacy_state,p.created_at)='active'
      UNION ALL
      SELECT s.revision_id,s.ordinal,'manifest_revision',m.id FROM supports s
      JOIN manifest_revisions m ON m.id=s.manifest_revision_id AND m.brain_id=s.brain_id WHERE m.privacy_state='active'
    ) SELECT * FROM edges ORDER BY revision_id,ordinal LIMIT 250001"))
        .bind(brain).fetch_all(&mut **tx).await?;
    if supports.len() > 250000 {
        return Err(capacity());
    }
    let mut d = Descriptor::default();
    let support_count: i64 = sqlx::query_scalar(&format!("{CLAIMS} SELECT count(*) FROM claim_supports s JOIN current_claims c ON c.id=s.revision_id AND c.brain_id=s.brain_id"))
        .bind(brain).fetch_one(&mut **tx).await?;
    d.unresolved = support_count - supports.len() as i64;
    for (claim, ordinal, kind, id) in supports {
        let entity = Entity {
            kind: kind.clone(),
            id,
            revision_id: id,
        };
        let target = entity.key();
        nodes.insert(target.clone(), entity);
        d.edges.push(GraphEdge {
            id: Uuid::new_v4(),
            from: key("claim", claim),
            to: target,
            family: "provenance".into(),
            relation: "supported_by".into(),
            evidence_kind: "claim_revision".into(),
            evidence_id: claim,
            evidence_ordinal: ordinal,
        });
    }
    let contributions:Vec<(Uuid,Uuid,i32)>=sqlx::query_as(&format!("{CLAIMS} SELECT r.id,c.input_revision_id,(requested.ordinality-1)::integer
      FROM current_claims r JOIN claim_contributions c ON c.brain_id=r.brain_id AND c.revision_id=r.id
      JOIN current_claims i ON i.id=c.input_revision_id
      CROSS JOIN LATERAL jsonb_array_elements_text(r.revision#>'{{content,handover,contributions}}') WITH ORDINALITY requested(id,ordinality)
      WHERE r.revision#>>'{{content,kind}}'='handover'
        AND requested.id=c.input_revision_id::text
      ORDER BY r.id,c.input_revision_id LIMIT 250001"))
        .bind(brain).fetch_all(&mut **tx).await?;
    for (output, input, ordinal) in contributions {
        if d.edges.len() == 250000 {
            return Err(capacity());
        }
        d.edges.push(GraphEdge {
            id: Uuid::new_v4(),
            from: key("claim", input),
            to: key("claim", output),
            family: "provenance".into(),
            relation: "contributed_to".into(),
            evidence_kind: "claim_revision".into(),
            evidence_id: output,
            evidence_ordinal: ordinal,
        });
    }
    if nodes.len() > 100000 {
        return Err(capacity());
    }
    d.nodes = nodes.into_values().collect();
    Ok(d)
}
