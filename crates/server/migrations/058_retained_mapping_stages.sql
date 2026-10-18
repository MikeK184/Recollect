-- Release-owned, reviewed producer definitions. No fixture or unmeasured model
-- is seeded. Application principals can read definitions, never certify one.
CREATE TABLE knowledge_mapping_release_producers (
 id text PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 128),
 provider text NOT NULL CHECK(provider IN ('openai','openrouter')),
 requested_model text NOT NULL CHECK(length(requested_model) BETWEEN 1 AND 256),
 returned_model text NOT NULL CHECK(length(returned_model) BETWEEN 1 AND 256),
 prompt_label text NOT NULL CHECK(length(prompt_label) BETWEEN 1 AND 128),
 schema_label text NOT NULL CHECK(length(schema_label) BETWEEN 1 AND 128),
 adapter_revision text NOT NULL,
 mapping_schema text NOT NULL CHECK(mapping_schema='source-mentions-1'),
 quality_receipt_sha256 text NOT NULL CHECK(quality_receipt_sha256 ~ '^[a-f0-9]{64}$'),
 fixture_sha256 text NOT NULL CHECK(fixture_sha256 ~ '^[a-f0-9]{64}$'),
 recipe_sha256 text NOT NULL CHECK(recipe_sha256 ~ '^[a-f0-9]{64}$'),
 held_out_documents integer NOT NULL CHECK(held_out_documents>0),
 entity_precision double precision NOT NULL CHECK(entity_precision>=0.95 AND entity_precision<=1),
 entity_recall double precision NOT NULL CHECK(entity_recall>=0.80 AND entity_recall<=1),
 endpoint_precision double precision NOT NULL CHECK(endpoint_precision>=0.95 AND endpoint_precision<=1),
 endpoint_recall double precision NOT NULL CHECK(endpoint_recall>=0.80 AND endpoint_recall<=1)
);
GRANT SELECT ON knowledge_mapping_release_producers TO recollect_app;

-- One exact normalized payload, not the provider's raw response. It remains
-- unavailable to organization readers until normal production publication.
CREATE TABLE knowledge_mapping_stages (
 run_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 request_id uuid NOT NULL,
 source_version_id uuid NOT NULL,
 context jsonb NOT NULL CHECK(octet_length(context::text)<=65536),
 payload jsonb NOT NULL CHECK(octet_length(payload::text)<=262144),
 expires_at timestamptz NOT NULL,
 next_check_at timestamptz NOT NULL DEFAULT '-infinity',
 maintenance_checked_at timestamptz NOT NULL DEFAULT '-infinity',
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 FOREIGN KEY(run_id,brain_id) REFERENCES learning_runs(id,brain_id),
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),
 FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id)
);
CREATE INDEX mapping_stage_source ON knowledge_mapping_stages(brain_id,source_version_id);
CREATE INDEX mapping_stage_request ON knowledge_mapping_stages(brain_id,request_id);
CREATE INDEX mapping_stage_route_walk ON knowledge_mapping_stages(next_check_at,created_at,run_id);
CREATE INDEX mapping_stage_maintenance_walk ON knowledge_mapping_stages(maintenance_checked_at,created_at,run_id);
ALTER TABLE knowledge_mapping_stages ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_mapping_stages FORCE ROW LEVEL SECURITY;
CREATE POLICY mapping_stage_read ON knowledge_mapping_stages FOR SELECT
 USING(brain_id IN(SELECT id FROM brains));
CREATE POLICY mapping_stage_write ON knowledge_mapping_stages FOR ALL
 USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN ('writer','admin')))
 WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN ('writer','admin')));
GRANT SELECT,INSERT,DELETE ON knowledge_mapping_stages TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(607,'knowledge_mapping_stages');
ALTER TABLE knowledge_mapping_coverage DROP CONSTRAINT knowledge_mapping_coverage_state_check;
ALTER TABLE knowledge_mapping_coverage ADD CHECK(state IN ('missing','invalid','staged','blocked_profile','blocked_capacity','published'));

-- Fixed-function bootstrap accepts no profile/receipt/quality data. Its only
-- source is the release-owned catalogue and the current enabled Brain policy.
CREATE FUNCTION recollect_mapping_bootstrap(b uuid) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE chosen knowledge_mapping_release_producers; p model_policies; result uuid; matches integer;
BEGIN
 PERFORM id FROM brains WHERE id=b FOR UPDATE;
 IF NOT coalesce(recollect_role(b) IN ('writer','admin'),false) THEN RETURN NULL; END IF;
 SELECT change_id INTO result FROM knowledge_mapping_producer_heads WHERE brain_id=b;
 IF FOUND THEN RETURN result; END IF;
 SELECT v.* INTO p FROM model_policy_heads h JOIN model_policies v ON v.brain_id=h.brain_id AND v.id=h.policy_id WHERE h.brain_id=b;
 IF p.id IS NULL OR p.policy->>'enabled' IS DISTINCT FROM 'true' OR NOT coalesce(p.policy->'purposes' ? 'extraction',false) THEN RETURN NULL; END IF;
 SELECT count(*) INTO matches FROM knowledge_mapping_release_producers c
 WHERE c.provider=p.policy->>'provider' AND c.requested_model=p.policy->>'text_model'
  AND c.prompt_label=CASE WHEN p.policy->>'autonomous_memory'='true' THEN 'source-reconciliation-2' ELSE 'source-facts-2' END
  AND c.schema_label=c.prompt_label AND c.adapter_revision='learning-quote-anchors-1';
 IF matches<>1 THEN RETURN NULL; END IF;
 SELECT c.* INTO STRICT chosen FROM knowledge_mapping_release_producers c
 WHERE c.provider=p.policy->>'provider' AND c.requested_model=p.policy->>'text_model'
  AND c.prompt_label=CASE WHEN p.policy->>'autonomous_memory'='true' THEN 'source-reconciliation-2' ELSE 'source-facts-2' END
  AND c.schema_label=c.prompt_label AND c.adapter_revision='learning-quote-anchors-1';
 result:=gen_random_uuid();
 INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256)
 VALUES(result,b,chosen.provider,chosen.requested_model,chosen.returned_model,chosen.prompt_label,chosen.schema_label,chosen.adapter_revision,chosen.mapping_schema,'qualified',chosen.quality_receipt_sha256);
 INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES(b,result);
 INSERT INTO mutation_audit(id,actor_id,brain_id,action,target_id,disposition)
 VALUES(gen_random_uuid(),recollect_actor(),b,'knowledge.producer.select',result,'release_qualified');
 RETURN result;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_bootstrap(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_bootstrap(uuid) TO recollect_app;

-- Routing returns identities only, in a bounded fair order. A worker must still
-- authenticate the exact original actor/device and requalify every payload.
CREATE FUNCTION recollect_mapping_stage_routes() RETURNS TABLE(brain_id uuid,actor_id uuid,device_id uuid,run_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH wanted AS MATERIALIZED (
 SELECT s.run_id FROM knowledge_mapping_stages s WHERE s.next_check_at<=clock_timestamp()
  AND (EXISTS(SELECT 1 FROM knowledge_mapping_producer_heads h WHERE h.brain_id=s.brain_id)
   OR EXISTS(SELECT 1 FROM knowledge_mapping_release_producers))
 ORDER BY s.next_check_at,s.created_at,s.run_id LIMIT 20 FOR UPDATE OF s SKIP LOCKED
 ), checked AS (
 UPDATE knowledge_mapping_stages s SET next_check_at=clock_timestamp()+interval '30 seconds'
 FROM wanted w WHERE s.run_id=w.run_id RETURNING s.*
 ) SELECT s.brain_id,r.actor_id,r.device_id,s.run_id FROM checked s
 JOIN learning_runs r ON r.brain_id=s.brain_id AND r.id=s.run_id
 JOIN model_requests q ON q.brain_id=s.brain_id AND q.id=s.request_id
 JOIN brains b ON b.id=s.brain_id JOIN accounts a ON a.id=r.actor_id
 JOIN model_policy_heads ph ON ph.brain_id=s.brain_id JOIN model_policies p ON p.brain_id=ph.brain_id AND p.id=ph.policy_id
 WHERE NOT b.archived AND a.enabled AND r.state='succeeded' AND q.state='succeeded'
  AND NOT q.suppressed AND NOT q.detail_expired AND s.expires_at>clock_timestamp() AND p.id=r.policy_id
  AND recollect_effective_role(b.id,r.actor_id) IN ('writer','admin')
  AND p.policy->>'enabled'='true' AND p.policy->'purposes' ? 'extraction'
  AND (EXISTS(SELECT 1 FROM knowledge_mapping_producer_heads h JOIN knowledge_mapping_producer_profiles f ON f.brain_id=h.brain_id AND f.change_id=h.change_id
    WHERE h.brain_id=b.id AND f.quality_state='qualified' AND f.provider=q.provider AND f.requested_model=q.model AND f.returned_model=q.returned_model
     AND f.prompt_label=q.prompt_label AND f.schema_label=q.schema_label AND f.adapter_revision='learning-quote-anchors-1' AND f.mapping_schema='source-mentions-1')
   OR (NOT EXISTS(SELECT 1 FROM knowledge_mapping_producer_heads h WHERE h.brain_id=b.id)
    AND EXISTS(SELECT 1 FROM knowledge_mapping_release_producers c WHERE c.provider=q.provider AND c.requested_model=q.model AND c.returned_model=q.returned_model
     AND c.prompt_label=q.prompt_label AND c.schema_label=q.schema_label AND c.adapter_revision='learning-quote-anchors-1')))
$$;
REVOKE ALL ON FUNCTION recollect_mapping_stage_routes() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_stage_routes() TO recollect_app;

CREATE FUNCTION recollect_mapping_stage_requests_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 DELETE FROM knowledge_mapping_stages s USING mapping_stage_new_requests q
 WHERE s.brain_id=q.brain_id AND s.request_id=q.id AND (q.suppressed OR q.detail_expired);
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_stage_requests_removed() FROM PUBLIC;
CREATE TRIGGER mapping_stage_requests_removed AFTER UPDATE ON model_requests
 REFERENCING NEW TABLE AS mapping_stage_new_requests
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_mapping_stage_requests_removed();

CREATE FUNCTION recollect_mapping_stage_fenced() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; seeds uuid[]; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM mapping_stage_new_fences LOOP
  SELECT array_agg(source_version_id) INTO seeds FROM mapping_stage_new_fences WHERE brain_id=b;
  DELETE FROM knowledge_mapping_stages WHERE brain_id=b AND source_version_id IN(SELECT id FROM recollect_source_derivative_versions(b,seeds));
 END LOOP;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_stage_fenced() FROM PUBLIC;
CREATE TRIGGER mapping_stage_fenced AFTER INSERT ON model_input_fences
 REFERENCING NEW TABLE AS mapping_stage_new_fences
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_mapping_stage_fenced();

CREATE FUNCTION recollect_mapping_stage_maintain() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE cohort uuid[]; BEGIN
 SELECT coalesce(array_agg(run_id),'{}') INTO cohort FROM (
  SELECT s.run_id FROM knowledge_mapping_stages s ORDER BY s.maintenance_checked_at,s.created_at,s.run_id LIMIT 100 FOR UPDATE OF s SKIP LOCKED
 ) wanted;
 UPDATE knowledge_mapping_stages SET maintenance_checked_at=clock_timestamp() WHERE run_id=ANY(cohort);
 DELETE FROM knowledge_mapping_stages WHERE run_id IN(
  SELECT s.run_id FROM knowledge_mapping_stages s JOIN model_requests q ON q.brain_id=s.brain_id AND q.id=s.request_id
  JOIN learning_runs r ON r.brain_id=s.brain_id AND r.id=s.run_id
  JOIN sources v ON v.brain_id=s.brain_id AND v.id=(SELECT source_id FROM source_versions WHERE brain_id=s.brain_id AND id=s.source_version_id)
  WHERE s.run_id=ANY(cohort) AND (s.expires_at<=clock_timestamp() OR q.suppressed OR q.detail_expired OR r.state IN ('failed','cancelled','removed') OR v.current_version<>s.source_version_id
   OR NOT EXISTS(SELECT 1 FROM model_policy_heads h WHERE h.brain_id=s.brain_id AND h.policy_id=r.policy_id)
   OR recollect_retention_deadline(q.brain_id,'audit',q.created_at)<=clock_timestamp()
   OR s.context IS DISTINCT FROM jsonb_build_object(
    'selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=s.brain_id AND version_id=s.source_version_id),'{}'::jsonb),
    'areas',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=s.brain_id AND m.source_id=v.id AND g.kind='area' ORDER BY g.id),
    'environments',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=s.brain_id AND m.source_id=v.id AND g.kind='environment' ORDER BY g.id)))
  );
END
$$;
REVOKE ALL ON FUNCTION recollect_mapping_stage_maintain() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_stage_maintain() TO recollect_app;

CREATE FUNCTION recollect_mapping_stage_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 UPDATE knowledge_mapping_coverage SET state='invalid' WHERE brain_id=OLD.brain_id AND run_id=OLD.run_id AND state='blocked_profile';
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_stage_removed() FROM PUBLIC;
CREATE TRIGGER mapping_stage_removed AFTER DELETE ON knowledge_mapping_stages
 FOR EACH ROW EXECUTE FUNCTION recollect_mapping_stage_removed();

CREATE FUNCTION recollect_mapping_stages_affected(b uuid,m jsonb,descendants boolean) RETURNS uuid[]
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE seed_claims(id) AS (
  SELECT unnest(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions'))
  UNION SELECT c.revision_id FROM claim_contributions c JOIN seed_claims a ON a.id=c.input_revision_id WHERE c.brain_id=b
 ), seeds AS MATERIALIZED (
  SELECT unnest(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')) AS id
  UNION SELECT source_version_id FROM claim_supports WHERE descendants AND brain_id=b AND revision_id IN(SELECT id FROM seed_claims) AND source_version_id IS NOT NULL
 ), versions AS MATERIALIZED (
  SELECT id FROM seeds UNION SELECT id FROM recollect_source_derivative_versions(b,ARRAY(SELECT id FROM seeds)) WHERE descendants
 ), affected_claims(id) AS (
  SELECT id FROM seed_claims UNION SELECT revision_id FROM claim_supports WHERE brain_id=b AND source_version_id IN(SELECT id FROM versions)
  UNION SELECT c.revision_id FROM claim_contributions c JOIN affected_claims a ON a.id=c.input_revision_id WHERE c.brain_id=b
 ) SELECT coalesce(array_agg(s.run_id ORDER BY s.run_id),'{}') FROM knowledge_mapping_stages s
 WHERE s.brain_id=b AND (s.source_version_id IN(SELECT id FROM versions) OR EXISTS(
  SELECT 1 FROM model_request_inputs i WHERE i.brain_id=b AND i.request_id=s.request_id AND (
   i.kind='source_version' AND i.input_id IN(SELECT id FROM versions)
   OR i.kind='claim_revision' AND i.input_id IN(SELECT id FROM affected_claims)
   OR i.kind='repository_fact' AND i.input_id=ANY(recollect_privacy_ids(m,'facts')))))
$$;
REVOKE ALL ON FUNCTION recollect_mapping_stages_affected(uuid,jsonb,boolean) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_mapping_stages_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; ids uuid[]; BEGIN
 m:=recollect_pre_mapping_stages_closure(b,k,t,c);
 IF k='brain' THEN SELECT coalesce(array_agg(run_id ORDER BY run_id),'{}') INTO ids FROM knowledge_mapping_stages WHERE brain_id=b;
 ELSE ids:=recollect_mapping_stages_affected(b,m,c='erase'); END IF;
 RETURN m||jsonb_build_object('knowledge_mapping_stages',ids);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_mapping_stages_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_mapping_stages_normalize(m)||jsonb_build_object('knowledge_mapping_stages',recollect_privacy_ids(m,'knowledge_mapping_stages'))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_mapping_stages_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; ids uuid[]; BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 ids:=recollect_privacy_ids(r.manifest,'knowledge_mapping_stages')||recollect_mapping_stages_affected(r.brain_id,r.manifest,r.cause='erase');
 DELETE FROM knowledge_mapping_stages WHERE brain_id=r.brain_id AND run_id=ANY(ids);
 PERFORM recollect_pre_mapping_stages_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
