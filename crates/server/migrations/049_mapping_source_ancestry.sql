-- Retained excerpts have independent expiry, but erasure/model-input fences
-- follow exact source ancestry. Natural ancestor expiry is not a fence.
CREATE INDEX source_excerpt_descendants ON source_excerpts(brain_id,parent_version_id);

CREATE FUNCTION recollect_source_model_fenced(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE lineage(id) AS (
  SELECT id FROM source_versions WHERE brain_id=b AND id=target
  UNION
  SELECT e.parent_version_id FROM source_excerpts e JOIN lineage l ON e.version_id=l.id
   WHERE e.brain_id=b
 )
 SELECT NOT EXISTS(SELECT 1 FROM lineage)
  OR NOT EXISTS(SELECT 1 FROM lineage l WHERE NOT EXISTS(
    SELECT 1 FROM source_excerpts e WHERE e.brain_id=b AND e.version_id=l.id))
  OR EXISTS(SELECT 1 FROM lineage l JOIN source_versions v ON v.brain_id=b AND v.id=l.id
    WHERE v.privacy_state='erased' OR EXISTS(
     SELECT 1 FROM model_input_fences f WHERE f.brain_id=b AND f.source_version_id=l.id))
  OR EXISTS(SELECT 1 FROM lineage l JOIN source_excerpts e ON e.brain_id=b AND e.version_id=l.id
    WHERE NOT EXISTS(SELECT 1 FROM source_versions p
     WHERE p.brain_id=b AND p.id=e.parent_version_id AND p.source_id=e.parent_source_id))
$$;
REVOKE ALL ON FUNCTION recollect_source_model_fenced(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_source_model_fenced(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_source_derivative_versions(b uuid,seeds uuid[]) RETURNS TABLE(id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE affected(id) AS (
  SELECT v.id FROM source_versions v WHERE v.brain_id=b AND v.id=ANY(seeds)
  UNION
  SELECT e.version_id FROM source_excerpts e JOIN affected a ON e.parent_version_id=a.id
   WHERE e.brain_id=b
 ) SELECT id FROM affected
$$;
REVOKE ALL ON FUNCTION recollect_source_derivative_versions(uuid,uuid[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_source_derivative_versions(uuid,uuid[]) TO recollect_app;

CREATE FUNCTION recollect_source_model_derivatives(b uuid,seeds uuid[]) RETURNS jsonb
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE versions AS MATERIALIZED (
  SELECT id FROM recollect_source_derivative_versions(b,seeds)
 ), affected(id) AS (
  SELECT revision_id FROM claim_supports WHERE brain_id=b AND source_version_id IN(SELECT id FROM versions)
  UNION SELECT c.revision_id FROM claim_contributions c JOIN affected a ON a.id=c.input_revision_id WHERE c.brain_id=b
 ), learning AS (
  SELECT r.id FROM learning_runs r WHERE r.brain_id=b AND (
   r.source_version_id IN(SELECT id FROM versions) OR EXISTS(
    SELECT 1 FROM learning_run_inputs i WHERE i.brain_id=b AND i.run_id=r.id AND i.revision_id IN(SELECT id FROM affected)))
 ), handovers AS (
  SELECT r.id FROM handover_runs r WHERE r.brain_id=b AND EXISTS(
   SELECT 1 FROM handover_run_inputs i WHERE i.brain_id=b AND i.run_id=r.id AND i.revision_id IN(SELECT id FROM affected))
 ), vectors AS (
  SELECT id FROM semantic_entries WHERE brain_id=b AND
   (source_version_id IN(SELECT id FROM versions) OR claim_revision_id IN(SELECT id FROM affected))
 ) SELECT jsonb_build_object(
  'model_input_sources',ARRAY(SELECT id FROM versions ORDER BY id),
  'learning_support_stages',ARRAY(SELECT id FROM learning ORDER BY id),
  'handover_support_stages',ARRAY(SELECT id FROM handovers ORDER BY id),
  'semantic_entries',ARRAY(SELECT id FROM vectors ORDER BY id),
  'semantic_batches',ARRAY(SELECT id FROM semantic_batches WHERE brain_id=b AND state IN ('queued','running','blocked') AND id IN(
   SELECT batch_id FROM semantic_batch_inputs WHERE brain_id=b AND entry_id IN(SELECT id FROM vectors)) ORDER BY id),
  'memory_support_assessments',ARRAY(SELECT id FROM memory_support_assessments WHERE brain_id=b AND revision_id IN(SELECT id FROM affected) ORDER BY id))
$$;
REVOKE ALL ON FUNCTION recollect_source_model_derivatives(uuid,uuid[]) FROM PUBLIC;

CREATE FUNCTION recollect_scrub_source_model_derivatives(b uuid,seeds uuid[]) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; versions uuid[]; learning uuid[]; handovers uuid[]; assessments uuid[]; vectors uuid[];
BEGIN
 m:=recollect_source_model_derivatives(b,seeds);
 versions:=recollect_privacy_ids(m,'model_input_sources');
 learning:=recollect_privacy_ids(m,'learning_support_stages');
 handovers:=recollect_privacy_ids(m,'handover_support_stages');
 assessments:=recollect_privacy_ids(m,'memory_support_assessments');
 vectors:=recollect_privacy_ids(m,'semantic_entries');
 UPDATE learning_support_stages SET payload='{}',verdicts=NULL,privacy_state='erased'
  WHERE brain_id=b AND run_id=ANY(learning);
 UPDATE learning_runs SET state='removed',selection='{}',error_code='model_input_fenced',finished_at=clock_timestamp()
  WHERE brain_id=b AND id=ANY(learning) AND state IN ('queued','running');
 UPDATE handover_support_stages SET payload='{}',verdict=NULL,privacy_state='erased'
  WHERE brain_id=b AND run_id=ANY(handovers);
 UPDATE handover_runs SET title='',selection='{}',state='removed',error_code='model_input_fenced',finished_at=clock_timestamp()
  WHERE brain_id=b AND id=ANY(handovers);
 UPDATE memory_support_assessments SET state='removed',disposition=NULL,reason=NULL,privacy_state='erased'
  WHERE brain_id=b AND id=ANY(assessments);
 UPDATE semantic_entries SET state='removed',embedding=NULL,error_code='model_input_fenced',updated_at=clock_timestamp()
  WHERE brain_id=b AND id=ANY(vectors);
 UPDATE semantic_batches SET state='removed',error_code='model_input_fenced',finished_at=clock_timestamp()
  WHERE brain_id=b AND state IN ('queued','running','blocked') AND id IN(
   SELECT batch_id FROM semantic_batch_inputs WHERE brain_id=b AND entry_id=ANY(vectors));
 UPDATE model_requests SET suppressed=true WHERE brain_id=b AND (
  id IN(SELECT request_id FROM model_request_inputs WHERE brain_id=b AND (
   (kind='source_version' AND input_id=ANY(versions))
   OR (kind='learning_support_stage' AND input_id=ANY(learning))
   OR (kind='handover_support_stage' AND input_id=ANY(handovers))
   OR (kind='claim_support_audit' AND input_id=ANY(assessments))))
  OR id IN(SELECT request_id FROM model_request_inputs WHERE brain_id=b AND kind='semantic_entry' AND input_id=ANY(vectors))
  OR operation_id=ANY(learning||handovers)
  OR id IN(SELECT assessment_request_id FROM learning_support_stages WHERE brain_id=b AND run_id=ANY(learning))
  OR id IN(SELECT assessment_request_id FROM handover_support_stages WHERE brain_id=b AND run_id=ANY(handovers))
  OR id IN(SELECT request_id FROM memory_support_assessments WHERE brain_id=b AND id=ANY(assessments))
  OR operation_id IN(SELECT assessment_operation_id FROM handover_support_stages WHERE brain_id=b AND run_id=ANY(handovers)));
 UPDATE jobs SET state='cancelled',error_code='model_input_fenced',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
  WHERE brain_id=b AND state IN ('queued','running') AND (
   (kind='source.learn' AND target_id=ANY(learning)) OR (kind='handover.generate' AND target_id=ANY(handovers))
   OR (kind='claim.support' AND target_id=ANY(assessments)) OR id IN(
    SELECT job_id FROM semantic_batches WHERE brain_id=b AND state='removed'));
 DELETE FROM knowledge_mapping_inputs i WHERE i.brain_id=b AND i.source_version_id=ANY(versions);
 DELETE FROM knowledge_entities e WHERE e.brain_id=b AND NOT EXISTS(
  SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=b AND m.entity_id=e.id);
 DELETE FROM knowledge_mapping_contexts c WHERE c.brain_id=b AND NOT EXISTS(
  SELECT 1 FROM knowledge_mapping_inputs i WHERE i.brain_id=b AND i.context_id=c.id);
END $$;
REVOKE ALL ON FUNCTION recollect_scrub_source_model_derivatives(uuid,uuid[]) FROM PUBLIC;

CREATE FUNCTION recollect_mapping_fenced_inputs() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; seeds uuid[];
BEGIN
 -- Statement triggers also run after INSERT ... ON CONFLICT inserts no rows.
 IF NOT EXISTS(SELECT 1 FROM mapping_new_fences) THEN RETURN NULL; END IF;
 -- Materialize every exact descendant fence so all existing canonical support,
 -- vector and worker checks retain their direct-fence meaning. The nested
 -- statement contains only conflicts and stops at the empty-table guard.
 INSERT INTO model_input_fences(brain_id,source_version_id,request_id)
 SELECT DISTINCT ON(f.brain_id,d.id) f.brain_id,d.id,f.request_id
 FROM (SELECT brain_id,request_id,array_agg(source_version_id) AS seeds
  FROM mapping_new_fences GROUP BY brain_id,request_id) f CROSS JOIN LATERAL
  recollect_source_derivative_versions(f.brain_id,f.seeds) d
 ORDER BY f.brain_id,d.id,f.request_id ON CONFLICT DO NOTHING;
 FOR b IN SELECT DISTINCT brain_id FROM mapping_new_fences LOOP
  SELECT array_agg(source_version_id) INTO seeds FROM mapping_new_fences WHERE brain_id=b;
  PERFORM recollect_scrub_source_model_derivatives(b,seeds);
 END LOOP;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_fenced_inputs() FROM PUBLIC;
CREATE TRIGGER mapping_source_fenced AFTER INSERT ON model_input_fences
 REFERENCING NEW TABLE AS mapping_new_fences
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_mapping_fenced_inputs();

-- A later retained copy must not bypass an already-fenced ancestor.
CREATE FUNCTION recollect_excerpt_model_fence() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE request uuid;
BEGIN
 SELECT request_id INTO request FROM model_input_fences
  WHERE brain_id=NEW.brain_id AND source_version_id=NEW.parent_version_id;
 IF request IS NOT NULL THEN
  INSERT INTO model_input_fences(brain_id,source_version_id,request_id)
   VALUES(NEW.brain_id,NEW.version_id,request) ON CONFLICT DO NOTHING;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_excerpt_model_fence() FROM PUBLIC;
CREATE TRIGGER excerpt_model_fence AFTER INSERT ON source_excerpts
 FOR EACH ROW EXECUTE FUNCTION recollect_excerpt_model_fence();

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_mapping_ancestry_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; versions uuid[]; inputs uuid[]; extra jsonb; field text; ids uuid[];
BEGIN
 m:=recollect_pre_mapping_ancestry_closure(b,k,t,c);
 versions:=recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources');
 IF c='erase' THEN
  extra:=recollect_source_model_derivatives(b,recollect_privacy_ids(m,'model_input_sources'));
  FOREACH field IN ARRAY ARRAY['model_input_sources','learning_support_stages','handover_support_stages','memory_support_assessments','semantic_entries','semantic_batches'] LOOP
   SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO ids FROM unnest(
    recollect_privacy_ids(m,field)||recollect_privacy_ids(extra,field)) id;
   m:=m||jsonb_build_object(field,ids);
  END LOOP;
  SELECT coalesce(array_agg(id ORDER BY id),'{}') INTO versions
   FROM recollect_source_derivative_versions(b,versions);
 END IF;
 SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO inputs FROM (
  SELECT unnest(recollect_privacy_ids(m,'knowledge_mapping_inputs')) AS id
  UNION
  SELECT i.id FROM knowledge_mapping_inputs i WHERE i.brain_id=b AND i.source_version_id=ANY(versions)
 ) x;
 RETURN m||jsonb_build_object('knowledge_mapping_inputs',inputs);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_mapping_ancestry_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; seeds uuid[]; versions uuid[];
BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 seeds:=recollect_privacy_ids(r.manifest,'source_versions')||recollect_privacy_ids(r.manifest,'model_input_sources');
 IF r.cause='erase' THEN
  -- Older journals may predate both mapping closure and model-input fields.
  SELECT coalesce(array_agg(DISTINCT id),'{}') INTO seeds FROM (
   SELECT unnest(seeds) AS id UNION
   SELECT source_version_id FROM claim_supports WHERE brain_id=r.brain_id
    AND revision_id=ANY(recollect_privacy_ids(r.manifest,'claim_revisions'))
    AND source_version_id IS NOT NULL UNION
   SELECT source_version_id FROM model_input_fences WHERE brain_id=r.brain_id AND request_id=r.id
  ) x;
  SELECT coalesce(array_agg(id),'{}') INTO versions
   FROM recollect_source_derivative_versions(r.brain_id,seeds);
  PERFORM recollect_scrub_source_model_derivatives(r.brain_id,seeds);
 ELSE versions:=seeds;
 END IF;
 DELETE FROM knowledge_mapping_inputs i WHERE i.brain_id=r.brain_id AND (
  i.source_version_id=ANY(versions)
  OR i.id=ANY(recollect_privacy_ids(r.manifest,'knowledge_mapping_inputs')));
 DELETE FROM knowledge_entities e WHERE e.brain_id=r.brain_id AND NOT EXISTS(
  SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=e.brain_id AND m.entity_id=e.id);
 DELETE FROM knowledge_mapping_contexts c WHERE c.brain_id=r.brain_id AND NOT EXISTS(
  SELECT 1 FROM knowledge_mapping_inputs i WHERE i.brain_id=c.brain_id AND i.context_id=c.id);
 PERFORM recollect_pre_mapping_ancestry_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;

ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_ancestry_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_ancestry_normalize(m)||jsonb_build_object(
  'semantic_entries',to_jsonb(recollect_privacy_ids(m,'semantic_entries')),
  'semantic_batches',to_jsonb(recollect_privacy_ids(m,'semantic_batches')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;

-- Upgrading an existing installation must remove already fenced derivatives.
INSERT INTO model_input_fences(brain_id,source_version_id,request_id)
 SELECT DISTINCT ON(f.brain_id,d.id) f.brain_id,d.id,f.request_id
 FROM (SELECT brain_id,request_id,array_agg(source_version_id) AS seeds
  FROM model_input_fences GROUP BY brain_id,request_id) f CROSS JOIN LATERAL
  recollect_source_derivative_versions(f.brain_id,f.seeds) d
 ORDER BY f.brain_id,d.id,f.request_id ON CONFLICT DO NOTHING;
DO $$ DECLARE b uuid; seeds uuid[]; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM model_input_fences LOOP
  SELECT array_agg(source_version_id) INTO seeds FROM model_input_fences WHERE brain_id=b;
  PERFORM recollect_scrub_source_model_derivatives(b,seeds);
 END LOOP;
END $$;
DELETE FROM knowledge_mapping_inputs i
 WHERE recollect_source_model_fenced(i.brain_id,i.source_version_id);
DELETE FROM knowledge_entities e WHERE NOT EXISTS(
 SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=e.brain_id AND m.entity_id=e.id);
DELETE FROM knowledge_mapping_contexts c WHERE NOT EXISTS(
 SELECT 1 FROM knowledge_mapping_inputs i WHERE i.brain_id=c.brain_id AND i.context_id=c.id);
