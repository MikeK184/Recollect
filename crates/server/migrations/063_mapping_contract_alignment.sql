-- Shared provider/native mapping contract. The source-mentions-1 wire shape
-- remains unchanged. No existing output/profile/head/catalogue row is rewritten
-- or qualified by this migration, and no producer definition is seeded.
ALTER TABLE knowledge_relation_candidates DROP CONSTRAINT knowledge_relation_candidates_relation_check;
ALTER TABLE knowledge_relation_candidates ADD CHECK(relation IN('uses','configured_by','depends_on','located_in','implements','related_to','does_not_use','has_owner','alias'));
ALTER TABLE knowledge_shadow_candidates DROP CONSTRAINT knowledge_shadow_candidates_adapter_revision_check;
ALTER TABLE knowledge_shadow_candidates ADD CHECK(adapter_revision IN('learning-quote-anchors-1','learning-quote-anchors-2'));

-- Only the current identity may be bootstrapped. An existing head is preserved;
-- legacy published outputs and exact legacy retained-profile matches survive.
CREATE OR REPLACE FUNCTION recollect_mapping_bootstrap(b uuid) RETURNS uuid
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
  AND c.prompt_label=CASE WHEN p.policy->>'autonomous_memory'='true' THEN 'source-reconciliation-3' ELSE 'source-facts-3' END
  AND c.schema_label=c.prompt_label AND c.adapter_revision='learning-quote-anchors-2';
 IF matches<>1 THEN RETURN NULL; END IF;
 SELECT c.* INTO STRICT chosen FROM knowledge_mapping_release_producers c
 WHERE c.provider=p.policy->>'provider' AND c.requested_model=p.policy->>'text_model'
  AND c.prompt_label=CASE WHEN p.policy->>'autonomous_memory'='true' THEN 'source-reconciliation-3' ELSE 'source-facts-3' END
  AND c.schema_label=c.prompt_label AND c.adapter_revision='learning-quote-anchors-2';
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


-- Route only an exact request/payload/profile adapter tuple. Legacy bytes are
-- never reinterpreted as adapter2, and a catalogue cannot select an old profile.
CREATE OR REPLACE FUNCTION recollect_mapping_stage_routes() RETURNS TABLE(brain_id uuid,actor_id uuid,device_id uuid,run_id uuid)
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
  AND s.payload->>'schema'='source-mentions-1'
  AND q.schema_label=q.prompt_label
  AND ((q.prompt_label IN('source-facts-2','source-reconciliation-2') AND s.payload->>'adapter_revision'='learning-quote-anchors-1')
    OR (q.prompt_label IN('source-facts-3','source-reconciliation-3') AND s.payload->>'adapter_revision'='learning-quote-anchors-2'))
  AND (EXISTS(SELECT 1 FROM knowledge_mapping_producer_heads h JOIN knowledge_mapping_producer_profiles f ON f.brain_id=h.brain_id AND f.change_id=h.change_id
    WHERE h.brain_id=b.id AND f.quality_state='qualified' AND f.provider=q.provider AND f.requested_model=q.model AND f.returned_model=q.returned_model
     AND f.prompt_label=q.prompt_label AND f.schema_label=q.schema_label AND f.adapter_revision=s.payload->>'adapter_revision' AND f.mapping_schema=s.payload->>'schema')
   OR (NOT EXISTS(SELECT 1 FROM knowledge_mapping_producer_heads h WHERE h.brain_id=b.id)
    AND EXISTS(SELECT 1 FROM knowledge_mapping_release_producers c WHERE c.provider=q.provider AND c.requested_model=q.model AND c.returned_model=q.returned_model
     AND c.prompt_label=q.prompt_label AND c.schema_label=q.schema_label AND c.adapter_revision=s.payload->>'adapter_revision' AND c.mapping_schema=s.payload->>'schema' AND c.adapter_revision='learning-quote-anchors-2' AND c.prompt_label=CASE WHEN p.policy->>'autonomous_memory'='true' THEN 'source-reconciliation-3' ELSE 'source-facts-3' END)))
$$;
REVOKE ALL ON FUNCTION recollect_mapping_stage_routes() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_stage_routes() TO recollect_app;
