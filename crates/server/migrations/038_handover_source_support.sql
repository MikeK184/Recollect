CREATE TABLE handover_support_stages (
 run_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 synthesis_request_id uuid NOT NULL,
 assessment_operation_id uuid NOT NULL UNIQUE,
 verifier_version text NOT NULL CHECK(verifier_version='source-support-3'),
 payload jsonb NOT NULL CHECK(jsonb_typeof(payload)='object' AND octet_length(payload::text)<=65536),
 verdict jsonb CHECK(verdict IS NULL OR (jsonb_typeof(verdict)='object' AND octet_length(verdict::text)<=5000)),
 assessment_request_id uuid,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 assessed_at timestamptz,
 privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased')),
 UNIQUE(run_id,brain_id),
 FOREIGN KEY(run_id,brain_id) REFERENCES handover_runs(id,brain_id),
 FOREIGN KEY(synthesis_request_id,brain_id) REFERENCES model_requests(id,brain_id),
 FOREIGN KEY(assessment_request_id,brain_id) REFERENCES model_requests(id,brain_id),
 CHECK(privacy_state='active' OR (payload='{}'::jsonb AND verdict IS NULL))
);
ALTER TABLE handover_support_stages ENABLE ROW LEVEL SECURITY;
ALTER TABLE handover_support_stages FORCE ROW LEVEL SECURITY;
CREATE POLICY handover_stage_read ON handover_support_stages FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY handover_stage_write ON handover_support_stages FOR ALL USING(recollect_role(brain_id) IN ('writer','admin')) WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT,UPDATE ON handover_support_stages TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(1002,'handover_support_stages');
ALTER TABLE memory_support_assessments ADD COLUMN source_handover_stage_id uuid;
ALTER TABLE memory_support_assessments ADD FOREIGN KEY(source_handover_stage_id,brain_id) REFERENCES handover_support_stages(run_id,brain_id);
ALTER TABLE memory_support_assessments ADD CHECK(source_stage_id IS NULL OR source_handover_stage_id IS NULL);
ALTER TABLE handover_runs ADD COLUMN local_recoveries integer NOT NULL DEFAULT 0 CHECK(local_recoveries BETWEEN 0 AND 2);
CREATE FUNCTION recollect_handover_run_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF (NEW.id,NEW.brain_id,NEW.contributions,NEW.policy_id,NEW.actor_id,NEW.device_id,NEW.operation_id,
     NEW.job_id,NEW.created_at,NEW.automatic,NEW.base_revision_id,NEW.automatic_attempt,NEW.retry_of)
 IS DISTINCT FROM
    (OLD.id,OLD.brain_id,OLD.contributions,OLD.policy_id,OLD.actor_id,OLD.device_id,OLD.operation_id,
     OLD.job_id,OLD.created_at,OLD.automatic,OLD.base_revision_id,OLD.automatic_attempt,OLD.retry_of) THEN
   RAISE EXCEPTION 'immutable handover run identity';
 END IF;
 IF NEW.local_recoveries<OLD.local_recoveries OR NEW.local_recoveries>OLD.local_recoveries+1 THEN
   RAISE EXCEPTION 'invalid handover recovery progression';
 END IF;
 IF NEW.local_recoveries>OLD.local_recoveries AND NOT (OLD.state='failed' AND NEW.state='queued') THEN
   RAISE EXCEPTION 'handover recovery requires failed queued transition';
 END IF;
 IF OLD.state='removed' AND NEW.state<>'removed' THEN RAISE EXCEPTION 'removed handover cannot resume'; END IF;
 IF NEW.title IS DISTINCT FROM OLD.title AND (NEW.title='' AND
    ((NEW.state='removed' AND NEW.error_code IN ('content_removed','model_input_fenced')) OR
     recollect_retention_deadline(OLD.brain_id,'audit',OLD.created_at)<=clock_timestamp())) IS NOT TRUE THEN
   RAISE EXCEPTION 'immutable handover title';
 END IF;
 IF NEW.selection IS DISTINCT FROM OLD.selection AND
    (NEW.selection='{}'::jsonb AND NEW.title='' AND NEW.state='removed' AND
     NEW.error_code IN ('content_removed','model_input_fenced')) IS NOT TRUE THEN
   RAISE EXCEPTION 'immutable handover scope';
 END IF;
 IF NEW.claim_id IS DISTINCT FROM OLD.claim_id AND NOT
    (OLD.claim_id IS NULL AND NEW.claim_id IS NOT NULL AND NEW.state='succeeded') THEN
   RAISE EXCEPTION 'immutable handover publication identity';
 END IF;
 IF OLD.request_id IS NOT NULL AND NEW.request_id IS DISTINCT FROM OLD.request_id THEN
   RAISE EXCEPTION 'immutable handover synthesis receipt';
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_handover_run_immutable() FROM PUBLIC;
CREATE TRIGGER immutable_handover_run BEFORE UPDATE ON handover_runs FOR EACH ROW EXECUTE FUNCTION recollect_handover_run_immutable();
CREATE FUNCTION recollect_handover_input_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'immutable handover input identity'; END IF;
 IF NOT EXISTS(SELECT 1 FROM handover_runs r WHERE r.id=NEW.run_id AND r.brain_id=NEW.brain_id
    AND NEW.revision_id=ANY(r.contributions) AND r.state='queued' AND r.request_id IS NULL
    AND NOT EXISTS(SELECT 1 FROM handover_support_stages s WHERE s.run_id=r.id)) THEN
   RAISE EXCEPTION 'handover input must match frozen queued run';
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_handover_input_immutable() FROM PUBLIC;
CREATE TRIGGER immutable_handover_input BEFORE INSERT OR UPDATE ON handover_run_inputs FOR EACH ROW EXECUTE FUNCTION recollect_handover_input_immutable();
ALTER TABLE model_request_inputs DROP CONSTRAINT model_request_inputs_kind_check;
ALTER TABLE model_request_inputs ADD CHECK(kind IN ('source_version','repository_fact','claim_revision','manifest_revision','semantic_entry','learning_support_stage','claim_support_audit','handover_support_stage'));

CREATE FUNCTION recollect_handover_support_deadline(b uuid,target uuid) RETURNS timestamptz
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT min(deadline) FROM (
   SELECT recollect_retention_deadline(brain_id,'claim',created_at) AS deadline FROM handover_support_stages WHERE brain_id=b AND run_id=target
   UNION ALL SELECT recollect_memory_deadline(brain_id,revision_id) FROM handover_run_inputs WHERE brain_id=b AND run_id=target
   UNION ALL SELECT recollect_memory_deadline(brain_id,base_revision_id) FROM handover_runs WHERE brain_id=b AND id=target AND base_revision_id IS NOT NULL
 ) deadlines
$$;
REVOKE ALL ON FUNCTION recollect_handover_support_deadline(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_handover_support_deadline(uuid,uuid) TO recollect_app;
CREATE FUNCTION recollect_handover_support_inspection(b uuid,target uuid) RETURNS jsonb
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT jsonb_build_object(
   'state',CASE WHEN s.privacy_state<>'active' THEN s.privacy_state
     WHEN recollect_handover_support_deadline(b,target)<=clock_timestamp() THEN 'expired'
     WHEN s.verdict IS NOT NULL THEN 'assessed'
     WHEN r.state='failed' THEN 'failed' ELSE 'pending' END,
   'disposition',CASE WHEN s.privacy_state='active' AND
     (recollect_handover_support_deadline(b,target) IS NULL OR recollect_handover_support_deadline(b,target)>clock_timestamp())
     THEN s.verdict->>'disposition' ELSE NULL END,
   'reason',CASE WHEN s.privacy_state='active' AND
     (recollect_handover_support_deadline(b,target) IS NULL OR recollect_handover_support_deadline(b,target)>clock_timestamp())
     THEN s.verdict->>'reason' ELSE NULL END,
   'assessment_request_id',coalesce(s.assessment_request_id,m.id),
   'request_state',m.state,'request_error_code',m.error_code,'verifier_version',s.verifier_version)
 FROM handover_support_stages s JOIN handover_runs r ON r.id=s.run_id AND r.brain_id=s.brain_id
 LEFT JOIN model_requests m ON m.brain_id=s.brain_id AND m.operation_id=s.assessment_operation_id
   AND m.actor_id=r.actor_id AND m.device_id IS NOT DISTINCT FROM r.device_id
   AND m.policy_id=r.policy_id AND m.purpose='extraction'
 WHERE s.brain_id=b AND s.run_id=target
$$;
REVOKE ALL ON FUNCTION recollect_handover_support_inspection(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_handover_support_inspection(uuid,uuid) TO recollect_app;
CREATE FUNCTION recollect_handover_stage_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF (NEW.run_id,NEW.brain_id,NEW.synthesis_request_id,NEW.assessment_operation_id,NEW.verifier_version,NEW.created_at)
 IS DISTINCT FROM (OLD.run_id,OLD.brain_id,OLD.synthesis_request_id,OLD.assessment_operation_id,OLD.verifier_version,OLD.created_at) THEN
   RAISE EXCEPTION 'immutable handover stage identity';
 END IF;
 IF NEW.privacy_state='active' THEN
   IF OLD.privacy_state<>'active' OR NEW.payload<>OLD.payload OR (OLD.verdict IS NOT NULL AND
     (NEW.verdict,NEW.assessment_request_id,NEW.assessed_at) IS DISTINCT FROM (OLD.verdict,OLD.assessment_request_id,OLD.assessed_at)) THEN
     RAISE EXCEPTION 'immutable handover stage content';
   END IF;
 ELSIF NEW.payload<>'{}'::jsonb OR NEW.verdict IS NOT NULL OR (OLD.privacy_state='erased' AND NEW.privacy_state<>'erased') THEN
   RAISE EXCEPTION 'removed handover stage cannot retain content';
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_handover_stage_immutable() FROM PUBLIC;
CREATE TRIGGER immutable_handover_stage BEFORE UPDATE ON handover_support_stages FOR EACH ROW EXECUTE FUNCTION recollect_handover_stage_immutable();
CREATE FUNCTION recollect_handover_lineage_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.source_handover_stage_id IS DISTINCT FROM OLD.source_handover_stage_id THEN RAISE EXCEPTION 'immutable handover support lineage'; END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_handover_lineage_immutable() FROM PUBLIC;
CREATE TRIGGER immutable_handover_lineage BEFORE UPDATE ON memory_support_assessments FOR EACH ROW EXECUTE FUNCTION recollect_handover_lineage_immutable();

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_handover_support_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; ids uuid[];
BEGIN
 m:=recollect_pre_handover_support_closure(b,k,t,c);
 SELECT coalesce(array_agg(DISTINCT r.id),'{}') INTO ids FROM handover_runs r
 WHERE r.brain_id=b AND ((k='handover_support_stage' AND r.id=t)
   OR r.base_revision_id=ANY(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions'))
   OR EXISTS(SELECT 1 FROM handover_run_inputs i CROSS JOIN LATERAL recollect_memory_dependencies(b,i.revision_id) d
     LEFT JOIN claim_supports s ON s.brain_id=b AND s.revision_id=d.revision_id
     LEFT JOIN claim_revisions rev ON rev.brain_id=b AND rev.id=d.revision_id
     WHERE i.brain_id=b AND i.run_id=r.id AND (
       d.revision_id=ANY(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions'))
       OR s.source_version_id=ANY(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources'))
       OR s.fact_id=ANY(recollect_privacy_ids(m,'facts')) OR s.manifest_revision_id=ANY(recollect_privacy_ids(m,'manifest_revisions'))
       OR (rev.revision#>>'{content,manifest_revision_id}')::uuid=ANY(recollect_privacy_ids(m,'manifest_revisions')))));
 RETURN m||jsonb_build_object('handover_support_stages',ids);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_handover_support_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; ids uuid[];
BEGIN
 PERFORM recollect_pre_handover_support_apply(rid);
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 ids:=recollect_privacy_ids(r.manifest,'handover_support_stages');
 UPDATE handover_support_stages SET payload='{}',verdict=NULL,
   privacy_state=CASE WHEN privacy_state='erased' OR r.cause='erase' THEN 'erased' ELSE 'expired' END WHERE brain_id=r.brain_id AND run_id=ANY(ids);
 UPDATE handover_runs SET title='',selection='{}',state='removed',error_code='content_removed',finished_at=clock_timestamp()
   WHERE brain_id=r.brain_id AND id=ANY(ids) AND state IN ('queued','running','failed');
 UPDATE model_requests SET suppressed=true WHERE brain_id=r.brain_id AND (operation_id=ANY(ids)
   OR operation_id IN(SELECT assessment_operation_id FROM handover_support_stages WHERE brain_id=r.brain_id AND run_id=ANY(ids)));
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
   WHERE brain_id=r.brain_id AND kind='handover.generate' AND target_id=ANY(ids) AND state IN ('queued','running');
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_handover_support_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_handover_support_normalize(m)||jsonb_build_object('handover_support_stages',to_jsonb(recollect_privacy_ids(m,'handover_support_stages')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
ALTER FUNCTION recollect_expire_one() RENAME TO recollect_pre_handover_support_expire_one;
CREATE FUNCTION recollect_expire_one() RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE rid uuid; candidate record; m jsonb;
BEGIN
 rid:=recollect_pre_handover_support_expire_one(); IF rid IS NOT NULL THEN RETURN rid; END IF;
 SELECT run_id,brain_id INTO candidate FROM handover_support_stages WHERE privacy_state='active'
   AND recollect_handover_support_deadline(brain_id,run_id)<=clock_timestamp() ORDER BY created_at,run_id LIMIT 1;
 IF NOT FOUND THEN RETURN NULL; END IF;
 PERFORM id FROM brains WHERE id=candidate.brain_id FOR UPDATE SKIP LOCKED; IF NOT FOUND THEN RETURN NULL; END IF;
 IF NOT EXISTS(SELECT 1 FROM handover_support_stages WHERE run_id=candidate.run_id AND privacy_state='active'
   AND recollect_handover_support_deadline(brain_id,run_id)<=clock_timestamp()) THEN RETURN NULL; END IF;
 rid:=gen_random_uuid(); m:=recollect_privacy_closure(candidate.brain_id,'handover_support_stage',candidate.run_id,'expire');
 INSERT INTO privacy_requests(id,brain_id,target,cause,manifest) VALUES(rid,candidate.brain_id,jsonb_build_object('kind','handover_support_stage','id',candidate.run_id),'expire',m);
 PERFORM recollect_privacy_apply(rid); RETURN rid;
END $$;
REVOKE ALL ON FUNCTION recollect_expire_one() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_one() TO recollect_app;

CREATE FUNCTION recollect_defer_handover_job(jid uuid,token uuid,reason text) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE target uuid;
BEGIN
 UPDATE jobs SET state='queued',error_code=reason,attempts=greatest(attempts-1,0),lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp(),
   not_before=CASE WHEN reason='model_budget_exhausted' THEN (date_trunc('day',clock_timestamp() AT TIME ZONE 'UTC')+interval '1 day') AT TIME ZONE 'UTC' ELSE clock_timestamp()+interval '10 seconds' END
 WHERE id=jid AND kind='handover.generate' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
   AND reason IN ('model_budget_exhausted','model_concurrency_full') RETURNING target_id INTO target;
 IF target IS NULL THEN RETURN false; END IF;
 UPDATE handover_runs SET state='queued',error_code=reason WHERE id=target AND state='running'; RETURN true;
END $$;
REVOKE ALL ON FUNCTION recollect_defer_handover_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_defer_handover_job(uuid,uuid,text) TO recollect_app;
