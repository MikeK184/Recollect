-- Brain deletion closure in one pass per Brain (ADR 0016, contract
-- docs/contracts/platform-brain-deletion.md#closure-computation).
--
-- The 029 closure called the whole per-target recollect_privacy_closure chain
-- once for every source, claim, snapshot and manifest of the Brain and merged
-- each result into a growing document. Every layer reads Brain-wide tables, so
-- cost grew with targets times Brain size: 960 targets took 187 s (174 s of it
-- in recollect_source_model_derivatives), 570 sources with 1,137 claims more
-- than 120 s, and preview and DELETE failed at the 10 s pool statement timeout.
--
-- Every per-target rule below is a union of "some identity of this target's
-- closure reaches the row" tests, and the source_version expansion in 024 only
-- re-adds what a member source already closes (the version, its source and the
-- excerpts share the Brain by foreign key). The union over all member targets
-- therefore equals each layer applied once, in the same order, to the
-- Brain-wide identity sets. Arrays are emitted exactly as
-- recollect_brain_merge emits them (distinct jsonb values in jsonb order),
-- scalars keep the 029 overrides, and a Brain without member targets keeps the
-- 029 template, so the document and its fingerprint are unchanged.
--
-- The iterated chain stays as recollect_iterated_brain_closure, callable only
-- by the migration owner. crates/server/tests/brain_deletion.rs compares both
-- documents on the rich fixture, empty Brains and the generated scale Brain. A
-- migration that extends recollect_privacy_closure must extend this function
-- too; a new always-present key makes that comparison fail otherwise.
ALTER FUNCTION recollect_brain_closure(uuid) RENAME TO recollect_iterated_brain_closure;
REVOKE ALL ON FUNCTION recollect_iterated_brain_closure(uuid) FROM PUBLIC, recollect_app;

-- recollect_brain_merge's array rule for one complete set of values.
CREATE FUNCTION recollect_brain_set(items jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
  SELECT coalesce(jsonb_agg(v ORDER BY v),'[]') FROM
    (SELECT DISTINCT value v FROM jsonb_array_elements(coalesce(items,'[]'))) merged
$$;
REVOKE ALL ON FUNCTION recollect_brain_set(jsonb) FROM PUBLIC;

-- Excerpt descendants of the seed versions that belong to the Brain, as
-- recollect_source_derivative_versions computes them.
CREATE FUNCTION recollect_brain_derivatives(b uuid,seeds uuid[]) RETURNS uuid[]
LANGUAGE plpgsql STABLE SET search_path=public,pg_temp SET plan_cache_mode=force_custom_plan AS $$
DECLARE ids uuid[];
BEGIN
  WITH RECURSIVE affected(id) AS (
    SELECT v.id FROM source_versions v WHERE v.brain_id=b AND v.id=ANY(seeds)
    UNION
    SELECT e.version_id FROM source_excerpts e JOIN affected a ON e.parent_version_id=a.id WHERE e.brain_id=b
  ) SELECT coalesce(array_agg(id),'{}') INTO ids FROM affected;
  RETURN ids;
END $$;
REVOKE ALL ON FUNCTION recollect_brain_derivatives(uuid,uuid[]) FROM PUBLIC;

-- Claim revisions reached from seeds through contribution edges, as the
-- recursive "affected" sets of 012, 037, 049, 056 and 058 compute them.
CREATE FUNCTION recollect_brain_contributions(b uuid,seeds uuid[]) RETURNS uuid[]
LANGUAGE plpgsql STABLE SET search_path=public,pg_temp SET plan_cache_mode=force_custom_plan AS $$
DECLARE ids uuid[];
BEGIN
  WITH RECURSIVE affected(id) AS (
    SELECT unnest(seeds)
    UNION
    SELECT c.revision_id FROM claim_contributions c JOIN affected a ON a.id=c.input_revision_id WHERE c.brain_id=b
  ) SELECT coalesce(array_agg(id),'{}') INTO ids FROM affected;
  RETURN ids;
END $$;
REVOKE ALL ON FUNCTION recollect_brain_contributions(uuid,uuid[]) FROM PUBLIC;

-- Each block names the migration whose per-target rule it applies. Here and
-- in the two helpers, identity arrays become constants of a fresh plan for
-- every statement, so membership tests are hashed lookups rather than per-row
-- array scans.
CREATE FUNCTION recollect_brain_closure(b uuid) RETURNS jsonb
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path=public,pg_temp SET plan_cache_mode=force_custom_plan AS $$
DECLARE
  m jsonb:='{"source_versions":[],"source_ids":[],"claim_revisions":[],"claim_ids":[],"snapshots":[],"facts":[],"manifest_revisions":[],"rules":[],"decisions":[],"artifacts":[],"jobs":[],"model_input_sources":[],"model_input_claim_revisions":[],"capture_event_fences":[],"publication_fences":[],"independent_claim_revisions":0,"shared_sources":false}'::jsonb;
  v uuid[]; sn uuid[]; f uuid[]; mr uuid[]; cr uuid[]; rl uuid[]; dc uuid[]; art uuid[]; work uuid[];
  mis uuid[]; micr uuid[]; fences jsonb; calls uuid[]; lss uuid[]; msa uuid[]; hss uuid[];
  partitions uuid[]; generations uuid[]; kmi uuid[]; seeds uuid[]; affected uuid[]; derived uuid[];
  se uuid[]; sb uuid[]; kac uuid[]; kch uuid[]; rfc uuid[]; kfi uuid[]; kms uuid[]; ksi uuid[];
  ksc uuid[]; ksr uuid[]; kts uuid[]; ktm uuid[];
BEGIN
  IF recollect_role(b) IS NULL THEN RETURN NULL; END IF;
  IF EXISTS(SELECT 1 FROM sources WHERE brain_id=b) OR EXISTS(SELECT 1 FROM claims WHERE brain_id=b)
     OR EXISTS(SELECT 1 FROM repository_snapshots WHERE brain_id=b)
     OR EXISTS(SELECT 1 FROM revision_manifests WHERE brain_id=b) THEN
    -- 010: every member source's versions and their excerpts, every snapshot
    -- and manifest, and the revisions, rules and decisions they reach.
    v:=recollect_brain_derivatives(b,ARRAY(SELECT x.id FROM source_versions x
      WHERE x.brain_id=b AND x.source_id IN(SELECT s.id FROM sources s WHERE s.brain_id=b)));
    SELECT coalesce(array_agg(id),'{}') INTO sn FROM repository_snapshots WHERE brain_id=b;
    SELECT coalesce(array_agg(id),'{}') INTO f FROM repository_facts WHERE brain_id=b AND snapshot_id=ANY(sn);
    SELECT coalesce(array_agg(r.id),'{}') INTO mr FROM manifest_revisions r WHERE r.brain_id=b AND
      (r.manifest_id IN(SELECT x.id FROM revision_manifests x WHERE x.brain_id=b) OR EXISTS(
        SELECT 1 FROM jsonb_array_elements(coalesce(r.revision->'entries','[]')) e WHERE (e->>'snapshot_id')::uuid=ANY(sn)));
    SELECT coalesce(array_agg(r.id),'{}') INTO cr FROM claim_revisions r WHERE r.brain_id=b AND
      (r.claim_id IN(SELECT x.id FROM claims x WHERE x.brain_id=b) OR EXISTS(
        SELECT 1 FROM claim_supports s WHERE s.brain_id=b AND s.revision_id=r.id AND
          (s.source_version_id=ANY(v) OR s.fact_id=ANY(f) OR s.manifest_revision_id=ANY(mr)))
       OR (r.revision#>>'{content,manifest_revision_id}')::uuid=ANY(mr));
    SELECT coalesce(array_agg(a.id),'{}') INTO rl FROM assertion_rules a WHERE a.brain_id=b AND (
      (a.rule->>'revision_id')::uuid=ANY(cr) OR (a.rule#>>'{content,manifest_revision_id}')::uuid=ANY(mr)
      OR EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(a.rule#>'{content,supports}','[]')) s WHERE
        (s->>'kind'='source_version' AND (s->>'id')::uuid=ANY(v))
        OR (s->>'kind'='repository_fact' AND (s->>'id')::uuid=ANY(f))
        OR (s->>'kind'='manifest_revision' AND (s->>'id')::uuid=ANY(mr))));
    SELECT coalesce(array_agg(d.id),'{}') INTO dc FROM memory_decisions d WHERE d.brain_id=b AND
      (d.id IN(SELECT a.decision_id FROM assertion_rules a WHERE a.brain_id=b AND a.id=ANY(rl)) OR EXISTS(
        SELECT 1 FROM jsonb_array_elements(coalesce(d.decision->'transitions','[]')) tr
        WHERE (tr->>'before_revision')::uuid=ANY(cr) OR (tr->>'after_revision')::uuid=ANY(cr)));
    SELECT coalesce(array_agg(id),'{}') INTO art FROM (
      SELECT x.artifact_id AS id FROM source_versions x WHERE x.brain_id=b AND x.id=ANY(v) AND x.artifact_id IS NOT NULL
      UNION SELECT x.id FROM repository_artifacts x WHERE x.brain_id=b AND x.snapshot_id=ANY(sn)
      UNION SELECT x.artifact_id FROM repository_files x WHERE x.brain_id=b AND x.snapshot_id=ANY(sn) AND x.artifact_id IS NOT NULL) a;
    SELECT coalesce(array_agg(j.id),'{}') INTO work FROM jobs j
      WHERE j.brain_id=b AND (j.target_id=ANY(v) OR j.target_id=ANY(sn)) AND j.state IN('queued','running');

    -- 011: sources behind those revisions and learning work over them.
    SELECT coalesce(array_agg(DISTINCT s.source_version_id),'{}') INTO mis FROM claim_supports s
      WHERE s.brain_id=b AND s.revision_id=ANY(cr) AND s.source_version_id IS NOT NULL;
    SELECT work||coalesce(array_agg(j.id),'{}') INTO work FROM jobs j JOIN learning_runs r ON r.job_id=j.id
      WHERE r.brain_id=b AND (r.source_version_id=ANY(mis) OR r.source_version_id=ANY(v)) AND j.state IN('queued','running');

    -- 012: derived revisions, their rules, decisions, inputs, sources and
    -- handover work.
    cr:=recollect_brain_contributions(b,cr);
    SELECT rl||coalesce(array_agg(a.id),'{}') INTO rl FROM assertion_rules a
      WHERE a.brain_id=b AND (a.rule->>'revision_id')::uuid=ANY(cr);
    SELECT dc||coalesce(array_agg(x.id),'{}') INTO dc FROM (
      SELECT a.decision_id AS id FROM assertion_rules a WHERE a.brain_id=b AND a.id=ANY(rl)
      UNION SELECT d.id FROM memory_decisions d WHERE d.brain_id=b AND EXISTS(
        SELECT 1 FROM jsonb_array_elements(coalesce(d.decision->'transitions','[]')) tr
        WHERE (tr->>'before_revision')::uuid=ANY(cr) OR (tr->>'after_revision')::uuid=ANY(cr))) x;
    SELECT coalesce(array_agg(DISTINCT c.input_revision_id),'{}') INTO micr FROM claim_contributions c
      WHERE c.brain_id=b AND c.revision_id=ANY(cr);
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO mis FROM (
      SELECT unnest(mis) AS id UNION
      SELECT s.source_version_id FROM claim_supports s WHERE s.brain_id=b AND s.revision_id=ANY(cr) AND s.source_version_id IS NOT NULL) x;
    SELECT work||coalesce(array_agg(j.id),'{}') INTO work FROM jobs j JOIN handover_runs r ON r.job_id=j.id
      WHERE r.brain_id=b AND j.state IN('queued','running') AND EXISTS(
        SELECT 1 FROM handover_run_inputs i WHERE i.run_id=r.id AND (i.revision_id=ANY(cr) OR i.revision_id=ANY(micr)));

    -- 013: sources and work of learning runs that used those revisions.
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO mis FROM (
      SELECT unnest(mis) AS id UNION
      SELECT r.source_version_id FROM learning_runs r JOIN learning_run_inputs i ON i.run_id=r.id
        WHERE i.brain_id=b AND i.revision_id=ANY(cr)) x;
    SELECT work||coalesce(array_agg(j.id),'{}') INTO work FROM jobs j JOIN learning_runs r ON r.job_id=j.id
      JOIN learning_run_inputs i ON i.run_id=r.id
      WHERE i.brain_id=b AND i.revision_id=ANY(cr) AND j.state IN('queued','running');

    -- 014: capture event fences.
    SELECT coalesce(jsonb_agg(jsonb_build_object('event_id',e.id,'binding_id',e.binding_id,'native_key',e.native_key)),'[]')
      INTO fences FROM capture_events e WHERE e.brain_id=b AND e.source_version_id=ANY(v);

    -- 024: managed calls behind fenced or reported versions. The member
    -- sources already close each such version, so only the call fences add.
    SELECT coalesce(array_agg(DISTINCT cb.id),'{}') INTO calls FROM capture_bindings cb
      WHERE cb.brain_id=b AND cb.host='managed_mcp' AND (
        cb.id IN(SELECT e.binding_id FROM capture_events e WHERE e.brain_id=b AND e.source_version_id=ANY(v))
        OR EXISTS(SELECT 1 FROM mcp_call_resolutions r WHERE r.call_id=cb.id AND r.source_version_id=ANY(v)));
    IF cardinality(calls)>0 THEN
      fences:=fences
        ||coalesce((SELECT jsonb_agg(jsonb_build_object('event_id',o.id,'binding_id',o.call_id,'native_key',NULL))
            FROM mcp_observation_outbox o WHERE o.brain_id=b AND o.call_id=ANY(calls)),'[]')
        ||(SELECT jsonb_agg(jsonb_build_object('event_id',id,'binding_id',id,'native_key',NULL)) FROM unnest(calls) id);
    END IF;

    -- 036: learning support stages.
    SELECT coalesce(array_agg(r.id),'{}') INTO lss FROM learning_runs r WHERE r.brain_id=b AND (
      r.source_version_id=ANY(v) OR r.source_version_id=ANY(mis) OR r.manifest_revision_id=ANY(mr) OR EXISTS(
        SELECT 1 FROM learning_run_inputs i LEFT JOIN claim_supports s ON s.revision_id=i.revision_id AND s.brain_id=i.brain_id
        WHERE i.run_id=r.id AND (i.revision_id=ANY(cr) OR i.revision_id=ANY(micr)
          OR s.source_version_id=ANY(v) OR s.source_version_id=ANY(mis)
          OR s.fact_id=ANY(f) OR s.manifest_revision_id=ANY(mr))));

    -- 037: support assessments of every affected revision.
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO seeds FROM (
      SELECT unnest(cr||micr) AS id UNION
      SELECT s.revision_id FROM claim_supports s WHERE s.brain_id=b AND (
        s.source_version_id=ANY(v) OR s.source_version_id=ANY(mis) OR s.fact_id=ANY(f) OR s.manifest_revision_id=ANY(mr))) x;
    affected:=recollect_brain_contributions(b,seeds);
    SELECT coalesce(array_agg(a.id),'{}') INTO msa FROM memory_support_assessments a
      WHERE a.brain_id=b AND a.revision_id=ANY(affected);

    -- 038: handover support stages over each input's dependency walk.
    SELECT coalesce(array_agg(r.id),'{}') INTO hss FROM handover_runs r WHERE r.brain_id=b AND (
      r.base_revision_id=ANY(cr) OR r.base_revision_id=ANY(micr) OR EXISTS(
        SELECT 1 FROM handover_run_inputs i CROSS JOIN LATERAL recollect_memory_dependencies(b,i.revision_id) d
        LEFT JOIN claim_supports s ON s.brain_id=b AND s.revision_id=d.revision_id
        LEFT JOIN claim_revisions rev ON rev.brain_id=b AND rev.id=d.revision_id
        WHERE i.brain_id=b AND i.run_id=r.id AND (
          d.revision_id=ANY(cr) OR d.revision_id=ANY(micr)
          OR s.source_version_id=ANY(v) OR s.source_version_id=ANY(mis)
          OR s.fact_id=ANY(f) OR s.manifest_revision_id=ANY(mr)
          OR (rev.revision#>>'{content,manifest_revision_id}')::uuid=ANY(mr))));

    -- 039: pending excerpt artifacts and session digest partitions and
    -- generations (the erase branch admits every reached partition).
    SELECT art||coalesce(array_agg(i.artifact_id),'{}') INTO art FROM excerpt_artifact_intents i
      WHERE i.brain_id=b AND i.parent_version_id=ANY(v);
    seeds:=ARRAY(SELECT (x->>'event_id')::uuid FROM jsonb_array_elements(fences) x);
    SELECT coalesce(array_agg(p.id),'{}') INTO partitions FROM session_digest_partitions p WHERE p.brain_id=b AND (
      EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
        WHERE d.partition_id=p.id AND (e.source_version_id=ANY(v) OR e.id=ANY(seeds)))
      OR EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
        JOIN learning_runs l ON l.brain_id=e.brain_id AND l.source_version_id=e.source_version_id
        JOIN claims current_claim ON current_claim.brain_id=l.brain_id AND current_claim.id=ANY(l.claim_ids)
        WHERE d.partition_id=p.id AND current_claim.current_revision=ANY(cr))
      OR EXISTS(SELECT 1 FROM session_digest_generations g WHERE g.partition_id=p.id AND g.contributors && cr)
      OR EXISTS(SELECT 1 FROM session_digest_generations g JOIN claim_supports cs ON cs.brain_id=g.brain_id AND cs.revision_id=ANY(g.contributors)
        WHERE g.partition_id=p.id AND cs.source_version_id=ANY(v)));
    SELECT coalesce(array_agg(g.id),'{}') INTO generations FROM session_digest_generations g
      WHERE g.brain_id=b AND (g.partition_id=ANY(partitions) OR g.contributors && cr
        OR EXISTS(SELECT 1 FROM session_digest_claims d JOIN claim_revisions r ON r.claim_id=d.claim_id AND r.brain_id=b
          WHERE d.generation_id=g.id AND r.id=ANY(cr)
            AND r.id=(SELECT current_revision FROM claims WHERE id=r.claim_id AND brain_id=b)));

    -- 045: mapping inputs of reached versions.
    SELECT coalesce(array_agg(i.id),'{}') INTO kmi FROM knowledge_mapping_inputs i
      WHERE i.brain_id=b AND (i.source_version_id=ANY(v) OR i.source_version_id=ANY(mis));

    -- 049: model derivatives of the input sources (the 049
    -- recollect_source_model_derivatives sets), then mapping inputs of every
    -- excerpt descendant of the reached and input versions.
    derived:=recollect_brain_derivatives(b,mis);
    SELECT coalesce(array_agg(DISTINCT s.revision_id),'{}') INTO seeds FROM claim_supports s
      WHERE s.brain_id=b AND s.source_version_id=ANY(derived);
    affected:=recollect_brain_contributions(b,seeds);
    SELECT lss||coalesce(array_agg(r.id),'{}') INTO lss FROM learning_runs r WHERE r.brain_id=b AND (
      r.source_version_id=ANY(derived) OR EXISTS(
        SELECT 1 FROM learning_run_inputs i WHERE i.brain_id=b AND i.run_id=r.id AND i.revision_id=ANY(affected)));
    SELECT hss||coalesce(array_agg(r.id),'{}') INTO hss FROM handover_runs r WHERE r.brain_id=b AND EXISTS(
      SELECT 1 FROM handover_run_inputs i WHERE i.brain_id=b AND i.run_id=r.id AND i.revision_id=ANY(affected));
    SELECT coalesce(array_agg(id),'{}') INTO se FROM semantic_entries
      WHERE brain_id=b AND (source_version_id=ANY(derived) OR claim_revision_id=ANY(affected));
    SELECT coalesce(array_agg(id),'{}') INTO sb FROM semantic_batches
      WHERE brain_id=b AND state IN('queued','running','blocked') AND id IN(
        SELECT batch_id FROM semantic_batch_inputs WHERE brain_id=b AND entry_id=ANY(se));
    SELECT msa||coalesce(array_agg(id),'{}') INTO msa FROM memory_support_assessments
      WHERE brain_id=b AND revision_id=ANY(affected);
    seeds:=recollect_brain_derivatives(b,v||mis);
    mis:=mis||derived;
    SELECT kmi||coalesce(array_agg(i.id),'{}') INTO kmi FROM knowledge_mapping_inputs i
      WHERE i.brain_id=b AND i.source_version_id=ANY(seeds);

    -- 054: alias candidates over those inputs and their decisions.
    SELECT coalesce(array_agg(r.id),'{}') INTO kac FROM knowledge_relation_candidates r
      WHERE r.brain_id=b AND r.relation='alias' AND r.input_id=ANY(kmi);
    SELECT coalesce(array_agg(id),'{}') INTO kch FROM knowledge_alias_changes WHERE brain_id=b AND candidate_id=ANY(kac);

    -- 056: retrieval feedback contexts (recollect_feedback_affected).
    derived:=recollect_brain_derivatives(b,v||mis);
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO seeds FROM (
      SELECT unnest(cr||micr) AS id UNION
      SELECT revision_id FROM claim_supports WHERE brain_id=b AND source_version_id=ANY(derived)) x;
    affected:=recollect_brain_contributions(b,seeds);
    SELECT coalesce(array_agg(id),'{}') INTO rfc FROM retrieval_feedback_contexts
      WHERE brain_id=b AND (source_version_id=ANY(derived) OR claim_revision_id=ANY(affected));

    -- 057: structured field inputs by origin.
    SELECT coalesce(array_agg(i.id),'{}') INTO kfi FROM knowledge_field_inputs i WHERE i.brain_id=b AND (
      (i.origin_kind='source_context' AND (i.revision_id=ANY(v) OR i.revision_id=ANY(mis)))
      OR (i.origin_kind='claim_revision' AND (i.revision_id=ANY(cr) OR i.revision_id=ANY(micr)))
      OR (i.origin_kind='repository_snapshot' AND i.revision_id=ANY(sn))
      OR (i.origin_kind='manifest_revision' AND i.revision_id=ANY(mr)));

    -- 058: mapping stages (recollect_mapping_stages_affected).
    affected:=recollect_brain_contributions(b,cr||micr);
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO seeds FROM (
      SELECT unnest(v||mis) AS id UNION
      SELECT source_version_id FROM claim_supports WHERE brain_id=b AND revision_id=ANY(affected) AND source_version_id IS NOT NULL) x;
    seeds:=seeds||recollect_brain_derivatives(b,seeds);
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO affected FROM (
      SELECT unnest(affected) AS id UNION
      SELECT revision_id FROM claim_supports WHERE brain_id=b AND source_version_id=ANY(seeds)) x;
    affected:=recollect_brain_contributions(b,affected);
    SELECT coalesce(array_agg(s.run_id),'{}') INTO kms FROM knowledge_mapping_stages s
      WHERE s.brain_id=b AND (s.source_version_id=ANY(seeds) OR EXISTS(
        SELECT 1 FROM model_request_inputs i WHERE i.brain_id=b AND i.request_id=s.request_id AND (
          i.kind='source_version' AND i.input_id=ANY(seeds)
          OR i.kind='claim_revision' AND i.input_id=ANY(affected)
          OR i.kind='repository_fact' AND i.input_id=ANY(f))));

    -- 060: organization source inputs of reached versions and descendants.
    SELECT coalesce(array_agg(id),'{}') INTO ksi FROM knowledge_source_inputs
      WHERE brain_id=b AND source_version_id=ANY(derived);

    -- 061: shadow candidates (recollect_shadow_candidates_affected) and the
    -- reports over them.
    SELECT coalesce(array_agg(c.id),'{}') INTO ksc FROM knowledge_shadow_candidates c WHERE c.brain_id=b AND (
      c.source_version_id=ANY(derived) OR EXISTS(SELECT 1 FROM model_request_inputs i
        WHERE i.brain_id=b AND i.request_id=c.request_id AND (
          (i.kind='source_version' AND i.input_id=ANY(derived))
          OR (i.kind='claim_revision' AND (i.input_id=ANY(cr) OR i.input_id=ANY(micr))))));
    SELECT coalesce(array_agg(DISTINCT s.report_id),'{}') INTO ksr FROM knowledge_shadow_sources s
      WHERE s.brain_id=b AND (s.source_version_id=ANY(v) OR s.source_version_id=ANY(mis) OR s.candidate_id=ANY(ksc));

    -- 062: manual topic intents (recollect_topic_intent_affected). No member
    -- closure carries knowledge_topics or knowledge_entities.
    SELECT coalesce(array_agg(s.id),'{}') INTO kts FROM knowledge_topic_intent_sources s
      JOIN knowledge_source_inputs i ON i.brain_id=s.brain_id AND i.id=s.input_id
      WHERE s.brain_id=b AND (s.input_id=ANY(ksi) OR i.source_version_id=ANY(derived));
    SELECT coalesce(array_agg(s.id),'{}') INTO ktm FROM knowledge_topic_intent_mappings s
      JOIN knowledge_mapping_inputs i ON i.brain_id=s.brain_id AND i.id=s.input_id
      WHERE s.brain_id=b AND (s.input_id=ANY(kmi) OR i.source_version_id=ANY(derived));

    -- 029: one merged document; nothing is independently retained and no
    -- shared source reaches another Brain.
    m:=m||jsonb_build_object(
      'source_versions',recollect_brain_set(to_jsonb(v)),
      'source_ids',recollect_brain_set(to_jsonb(ARRAY(
        SELECT x.source_id FROM source_versions x WHERE x.brain_id=b AND x.id=ANY(v)))),
      'claim_revisions',recollect_brain_set(to_jsonb(cr)),
      'claim_ids',recollect_brain_set(to_jsonb(ARRAY(
        SELECT x.claim_id FROM claim_revisions x WHERE x.brain_id=b AND x.id=ANY(cr)))),
      'snapshots',recollect_brain_set(to_jsonb(sn)),
      'facts',recollect_brain_set(to_jsonb(f)),
      'manifest_revisions',recollect_brain_set(to_jsonb(mr)),
      'rules',recollect_brain_set(to_jsonb(rl)),
      'decisions',recollect_brain_set(to_jsonb(dc)),
      'artifacts',recollect_brain_set(to_jsonb(art)),
      'jobs',recollect_brain_set(to_jsonb(work)),
      'model_input_sources',recollect_brain_set(to_jsonb(mis)),
      'model_input_claim_revisions',recollect_brain_set(to_jsonb(micr)),
      'capture_event_fences',recollect_brain_set(fences),
      'publication_fences',recollect_brain_set((SELECT jsonb_agg(jsonb_build_object('repository_id',x.repository_id,'revision',x.revision))
        FROM repository_snapshots x WHERE x.brain_id=b AND x.id=ANY(sn))),
      'learning_support_stages',recollect_brain_set(to_jsonb(lss)),
      'memory_support_assessments',recollect_brain_set(to_jsonb(msa)),
      'handover_support_stages',recollect_brain_set(to_jsonb(hss)),
      'session_digest_partitions',recollect_brain_set(to_jsonb(partitions)),
      'session_digest_generations',recollect_brain_set(to_jsonb(generations)),
      'knowledge_mapping_inputs',recollect_brain_set(to_jsonb(kmi)),
      'semantic_entries',recollect_brain_set(to_jsonb(se)),
      'semantic_batches',recollect_brain_set(to_jsonb(sb)),
      'knowledge_alias_candidates',recollect_brain_set(to_jsonb(kac)),
      'knowledge_alias_changes',recollect_brain_set(to_jsonb(kch)),
      'retrieval_feedback_contexts',recollect_brain_set(to_jsonb(rfc)),
      'knowledge_field_inputs',recollect_brain_set(to_jsonb(kfi)),
      'knowledge_mapping_stages',recollect_brain_set(to_jsonb(kms)),
      'knowledge_source_inputs',recollect_brain_set(to_jsonb(ksi)),
      'knowledge_shadow_candidates',recollect_brain_set(to_jsonb(ksc)),
      'knowledge_shadow_reports',recollect_brain_set(to_jsonb(ksr)),
      'knowledge_topic_intent_sources',recollect_brain_set(to_jsonb(kts)),
      'knowledge_topic_intent_mappings',recollect_brain_set(to_jsonb(ktm)));
  END IF;
  -- 050 and 051: Brain-owned skills and coordination records, in the order
  -- those wrappers wrote them.
  RETURN m||jsonb_build_object(
    'managed_skills',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM managed_skills WHERE brain_id=b),'[]'),
    'managed_skill_versions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM managed_skill_versions WHERE brain_id=b),'[]'),
    'coordination_sessions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_sessions WHERE brain_id=b),'[]'),
    'coordination_messages',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_messages WHERE brain_id=b),'[]'),
    'coordination_events',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_events WHERE brain_id=b),'[]'),
    'coordination_overlaps',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_overlaps WHERE brain_id=b),'[]'));
END $$;
REVOKE ALL ON FUNCTION recollect_brain_closure(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_closure(uuid) TO recollect_app;
