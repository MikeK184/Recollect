//! Exact retained evidence joins. Parsing stays in the companion and path
//! computation stays in Neo4j; labels never stand in for repository identity.
use super::*;
use serde_json::Value;

pub(super) const LINKER: &str = "terraform-git-module-1";

pub(super) async fn origin_epoch(tx: &mut Tx<'_>, brain: Uuid) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT graph_link_epoch FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&mut **tx)
            .await?,
    )
}

pub(super) async fn manifest_inputs(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<Vec<Uuid>> {
    let manifest = crate::memory::manifest(tx, brain, id).await?;
    if !(2..=100).contains(&manifest.entries.len()) {
        return Err(Error::invalid(
            "Choose a manifest with two to 100 exact repository snapshots.",
        ));
    }
    let mut ids: Vec<_> = manifest
        .entries
        .iter()
        .filter_map(|e| e.snapshot_id)
        .collect();
    ids.sort();
    ids.dedup();
    if ids.len() != manifest.entries.len() {
        return Err(failure(
            "graph_input_unavailable",
            "Every combined manifest entry must have an exact available snapshot.",
        ));
    }
    available_inputs(tx, brain, &ids).await?;
    Ok(ids)
}

pub(super) async fn available_inputs(tx: &mut Tx<'_>, brain: Uuid, ids: &[Uuid]) -> Result<()> {
    let (count, total):(i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(fact_count),0)::bigint FROM repository_snapshots WHERE brain_id=$1 AND id=ANY($2)")
        .bind(brain).bind(ids).fetch_one(&mut **tx).await?;
    if count != ids.len() as i64 {
        return Err(failure(
            "graph_input_unavailable",
            "A combined snapshot is unavailable in this Brain.",
        ));
    }
    if total > 100000 {
        return Err(descriptor::capacity());
    }
    let valid:bool=sqlx::query_scalar("SELECT coalesce(bool_and(recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'
      AND (SELECT count(*) FROM repository_facts f WHERE f.brain_id=s.brain_id AND f.snapshot_id=s.id)=s.fact_count
      AND EXISTS(SELECT 1 FROM repository_jobs r JOIN jobs j ON j.id=r.job_id WHERE r.brain_id=s.brain_id AND r.snapshot_id=s.id AND j.state='succeeded')),false)
      FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id=ANY($2)")
        .bind(brain).bind(ids).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(failure(
            "graph_input_unavailable",
            "A combined input expired or has not finished materialization.",
        ));
    }
    Ok(())
}

pub(super) async fn discover(
    tx: &mut Tx<'_>,
    brain: Uuid,
    actor: Uuid,
    room: usize,
) -> Result<usize> {
    if room == 0 {
        return Ok(0);
    }
    let current = origin_epoch(tx, brain).await?;
    let manifests:Vec<Uuid>=sqlx::query_scalar("SELECT DISTINCT ON (i.snapshots) r.id
      FROM manifest_revisions r CROSS JOIN LATERAL (
        SELECT array_agg((e->>'snapshot_id')::uuid ORDER BY (e->>'snapshot_id')::uuid) AS snapshots
        FROM jsonb_array_elements(r.revision->'entries') e) i
      WHERE r.brain_id=$1 AND cardinality(i.snapshots) BETWEEN 2 AND 100
        AND array_position(i.snapshots,NULL) IS NULL
        AND (SELECT coalesce(sum(s.fact_count),0) FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id=ANY(i.snapshots))<=100000
        AND NOT EXISTS(SELECT 1 FROM unnest(i.snapshots) x WHERE NOT EXISTS(
          SELECT 1 FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id=x
           AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'
           AND EXISTS(SELECT 1 FROM repository_jobs j JOIN jobs w ON w.id=j.job_id WHERE j.brain_id=s.brain_id AND j.snapshot_id=s.id AND w.state='succeeded')))
        AND NOT EXISTS(SELECT 1 FROM graph_generations g WHERE g.brain_id=$1 AND g.kind='combined' AND g.input_snapshot_ids=i.snapshots AND g.input_epoch=$2 AND g.adapter=$4 AND g.state NOT IN ('superseded','removed'))
      ORDER BY i.snapshots,r.created_at,r.id LIMIT $3")
        .bind(brain).bind(current).bind(room as i64).bind(LINKER).fetch_all(&mut **tx).await?;
    let mut count = 0;
    for id in manifests {
        queue::enqueue(
            tx,
            brain,
            actor,
            &GraphRebuild {
                kind: "combined".into(),
                snapshot_id: None,
                manifest_revision_id: Some(id),
            },
        )
        .await?;
        count += 1;
    }
    Ok(count)
}

pub(super) async fn remove_unavailable(tx: &mut Tx<'_>, brain: Uuid) -> Result<()> {
    sqlx::query("UPDATE jobs SET state='cancelled',error_code='graph_input_unavailable',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
      WHERE brain_id=$1 AND kind='graph.project' AND state IN ('queued','running') AND target_id IN (
        SELECT g.id FROM graph_generations g WHERE g.brain_id=$1 AND g.kind='combined' AND EXISTS(
          SELECT 1 FROM unnest(g.input_snapshot_ids) x WHERE NOT EXISTS(SELECT 1 FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id=x AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active')))")
        .bind(brain).execute(&mut **tx).await?;
    sqlx::query("UPDATE graph_generations g SET state='removed',error_code='graph_input_unavailable' WHERE brain_id=$1 AND kind='combined' AND state<>'removed' AND EXISTS(
      SELECT 1 FROM unnest(g.input_snapshot_ids) x WHERE NOT EXISTS(SELECT 1 FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id=x AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'))")
        .bind(brain).execute(&mut **tx).await?;
    Ok(())
}

struct Fact {
    id: Uuid,
    snapshot: Uuid,
    repository: Uuid,
    record: Value,
}
fn issue(d: &mut Descriptor, source: &Fact, code: &str) {
    d.unresolved += 1;
    d.issues.push(GraphLinkIssue {
        source: key("repository_fact", source.id),
        source_snapshot_id: source.snapshot,
        code: code.into(),
    });
}
fn witness(fact: &Fact) -> std::result::Result<GitModuleSource, &'static str> {
    let value = fact
        .record
        .pointer("/props/recollect_module_source")
        .cloned()
        .ok_or("source_not_verified")?;
    let witness: ModuleSourceWitness =
        serde_json::from_value(value).map_err(|_| "source_not_verified")?;
    if witness.state != "verified_literal" {
        return Err(match witness.state.as_str() {
            "local_module" => "local_module",
            "source_expression" => "source_expression",
            "unresolved_source_ref" => "unresolved_source_ref",
            "unsupported_source_options" => "unsupported_source_options",
            "unsupported_source_transport" => "unsupported_source_transport",
            "ambiguous_module_declaration"
            | "ambiguous_source_fact"
            | "ambiguous_source_attribute" => "ambiguous_source",
            "module_override_unsupported" => "module_override_unsupported",
            "terraform_json_unexamined" => "terraform_json_unexamined",
            "source_parse_error" | "source_parse_timeout" | "source_file_unavailable" => {
                "source_not_parsed"
            }
            _ => "source_not_verified",
        });
    }
    let target = witness.target.ok_or("source_not_verified")?;
    if witness.parser.is_empty()
        || witness.parser.len() > 200
        || witness.line_from == 0
        || witness.line_to < witness.line_from
        || witness.line_to > 10_000_000
        || !git_object_id(&target.revision)
        || canonical_origin(&target.origin).as_ref() != Ok(&target.origin)
        || target.directory.contains('%')
        || (target.directory != "." && !repository_path(&target.directory))
    {
        return Err("source_not_verified");
    }
    Ok(target)
}

pub(super) async fn build(tx: &mut Tx<'_>, g: &GraphGeneration) -> Result<Descriptor> {
    let rows:Vec<(Uuid,Uuid,Uuid,Json<Value>)>=sqlx::query_as("SELECT f.id,f.snapshot_id,s.repository_id,f.record FROM repository_facts f JOIN repository_snapshots s ON s.brain_id=f.brain_id AND s.id=f.snapshot_id WHERE f.brain_id=$1 AND f.snapshot_id=ANY($2) ORDER BY f.snapshot_id,f.ordinal LIMIT 100001")
        .bind(g.brain_id).bind(&g.input_snapshot_ids).fetch_all(&mut **tx).await?;
    if rows.len() > 100000 {
        return Err(descriptor::capacity());
    }
    let facts: Vec<_> = rows
        .into_iter()
        .map(|(id, snapshot, repository, Json(record))| Fact {
            id,
            snapshot,
            repository,
            record,
        })
        .collect();
    let snapshots:Vec<(Uuid,Uuid,String)>=sqlx::query_as("SELECT repository_id,id,revision FROM repository_snapshots WHERE brain_id=$1 AND id=ANY($2)")
        .bind(g.brain_id).bind(&g.input_snapshot_ids).fetch_all(&mut **tx).await?;
    let by_repository: BTreeMap<_, _> =
        snapshots.into_iter().map(|(r, s, c)| (r, (s, c))).collect();
    if by_repository.len() != g.input_snapshot_ids.len() {
        return Err(failure(
            "graph_input_unavailable",
            "Combined inputs must select exactly one snapshot of each repository.",
        ));
    }
    let origins: Vec<(String, Uuid)> =
        sqlx::query_as("SELECT origin,repository_id FROM repository_origins WHERE brain_id=$1")
            .bind(g.brain_id)
            .fetch_all(&mut **tx)
            .await?;
    let origins: BTreeMap<_, _> = origins.into_iter().collect();
    let mut modules = BTreeMap::<(Uuid, String), Vec<&Fact>>::new();
    for fact in &facts {
        if fact.record["kind"] == "module"
            && fact.record["props"]["language"] == "hcl"
            && fact.record["file"] == fact.record["name"]
            && let Some(directory) = fact.record["name"].as_str()
        {
            modules
                .entry((fact.snapshot, directory.into()))
                .or_default()
                .push(fact);
        }
    }
    let mut d = Descriptor::default();
    let mut endpoints = BTreeSet::new();
    for source in &facts {
        if source.record["kind"] != "symbol" || source.record["props"]["hcl_block"] != "module" {
            continue;
        }
        // The witness describes even unsupported expressions. Old local-only
        // sources are already covered by the ordinary structural extractor.
        if source
            .record
            .pointer("/props/recollect_module_source")
            .is_none()
            && source.record["props"]["external"] != true
        {
            continue;
        }
        let target = match witness(source) {
            Ok(target) => target,
            Err("local_module") => continue,
            Err(reason) => {
                issue(&mut d, source, reason);
                continue;
            }
        };
        let Some(repository) = origins.get(&target.origin) else {
            issue(&mut d, source, "target_origin_unregistered");
            continue;
        };
        if *repository == source.repository {
            issue(&mut d, source, "target_is_same_repository");
            continue;
        }
        let Some((snapshot, revision)) = by_repository.get(repository) else {
            issue(&mut d, source, "target_outside_manifest");
            continue;
        };
        if *revision != target.revision {
            issue(&mut d, source, "target_revision_mismatch");
            continue;
        }
        let candidates = modules
            .get(&(*snapshot, target.directory.clone()))
            .map(Vec::as_slice)
            .unwrap_or_default();
        let [destination] = candidates else {
            issue(
                &mut d,
                source,
                if candidates.len() > 1 {
                    "ambiguous_target_module"
                } else {
                    "target_module_unavailable"
                },
            );
            continue;
        };
        if destination.record["props"]["recollect_hcl_directory"]["state"] != "parsed"
            || destination.record["props"]["recollect_hcl_directory"]["directory"]
                != target.directory
        {
            issue(&mut d, source, "target_directory_not_parsed");
            continue;
        }
        endpoints.insert(source.id);
        endpoints.insert(destination.id);
        d.edges.push(GraphEdge {
            id: Uuid::new_v4(),
            from: key("repository_fact", source.id),
            to: key("repository_fact", destination.id),
            family: "cross_repository".into(),
            relation: "terraform_module".into(),
            evidence_kind: "repository_fact".into(),
            evidence_id: source.id,
            evidence_ordinal: 0,
        });
    }
    d.nodes = endpoints
        .into_iter()
        .map(|id| Entity {
            kind: "repository_fact".into(),
            id,
            revision_id: id,
        })
        .collect();
    Ok(d)
}
