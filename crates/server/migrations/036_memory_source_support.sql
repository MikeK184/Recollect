-- Typed candidate stages are derived memory, never raw provider-response caches.
CREATE TABLE learning_support_stages (
  run_id uuid PRIMARY KEY,
  brain_id uuid NOT NULL,
  extraction_request_id uuid NOT NULL,
  assessment_operation_id uuid NOT NULL UNIQUE,
  assessment_request_id uuid,
  verifier_version text NOT NULL CHECK(verifier_version='source-support-3'),
  payload jsonb NOT NULL CHECK(octet_length(payload::text)<=65536),
  verdicts jsonb CHECK(verdicts IS NULL OR octet_length(verdicts::text)<=32768),
  privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased')),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  assessed_at timestamptz,
  UNIQUE(run_id,brain_id),
  FOREIGN KEY(run_id,brain_id) REFERENCES learning_runs(id,brain_id),
  FOREIGN KEY(extraction_request_id,brain_id) REFERENCES model_requests(id,brain_id),
  FOREIGN KEY(assessment_request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
ALTER TABLE learning_support_stages ENABLE ROW LEVEL SECURITY;
ALTER TABLE learning_support_stages FORCE ROW LEVEL SECURITY;
CREATE POLICY support_stage_read ON learning_support_stages FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY support_stage_write ON learning_support_stages FOR ALL USING(recollect_role(brain_id) IN ('writer','admin')) WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT,UPDATE ON learning_support_stages TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(1001,'learning_support_stages');

CREATE FUNCTION recollect_learning_support_deadline(b uuid,target uuid) RETURNS timestamptz
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH run AS (SELECT * FROM learning_runs WHERE brain_id=b AND id=target),
 inputs AS (SELECT revision_id FROM learning_run_inputs WHERE brain_id=b AND run_id=target),
 manifests AS (
   SELECT manifest_revision_id id FROM run
   UNION SELECT s.manifest_revision_id FROM claim_supports s JOIN inputs i ON i.revision_id=s.revision_id WHERE s.brain_id=b
   UNION SELECT (r.revision#>>'{content,manifest_revision_id}')::uuid FROM claim_revisions r JOIN inputs i ON i.revision_id=r.id WHERE r.brain_id=b
 ),
 deadlines AS (
   SELECT recollect_retention_deadline(brain_id,'claim',created_at) deadline FROM learning_support_stages WHERE brain_id=b AND run_id=target
   UNION ALL SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) FROM source_versions v JOIN run r ON r.source_version_id=v.id AND r.brain_id=v.brain_id
   UNION ALL SELECT recollect_retention_deadline(r.brain_id,'claim',r.recorded_at) FROM claim_revisions r JOIN inputs i ON i.revision_id=r.id WHERE r.brain_id=b
   UNION ALL SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) FROM claim_supports s JOIN inputs i ON i.revision_id=s.revision_id JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=s.brain_id WHERE s.brain_id=b
   UNION ALL SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at) FROM claim_supports s JOIN inputs i ON i.revision_id=s.revision_id JOIN repository_facts f ON f.id=s.fact_id AND f.brain_id=s.brain_id JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=f.brain_id WHERE s.brain_id=b
   -- Manifests have no independent repository TTL. Their exact snapshots do.
   UNION ALL SELECT recollect_retention_deadline(s.brain_id,'repository',s.created_at)
     FROM manifest_revisions m JOIN manifests x ON x.id=m.id
     CROSS JOIN LATERAL jsonb_array_elements(coalesce(m.revision->'entries','[]')) e
     JOIN repository_snapshots s ON s.brain_id=m.brain_id AND s.id=(e->>'snapshot_id')::uuid
     WHERE m.brain_id=b
 ) SELECT min(deadline) FROM deadlines
$$;
REVOKE ALL ON FUNCTION recollect_learning_support_deadline(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_learning_support_deadline(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_support_stage_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF (NEW.run_id,NEW.brain_id,NEW.extraction_request_id,NEW.assessment_operation_id,NEW.verifier_version,NEW.created_at)
   IS DISTINCT FROM (OLD.run_id,OLD.brain_id,OLD.extraction_request_id,OLD.assessment_operation_id,OLD.verifier_version,OLD.created_at) THEN
   RAISE EXCEPTION 'immutable support stage identity';
 END IF;
 IF NEW.privacy_state='active' THEN
   IF OLD.privacy_state<>'active' OR NEW.payload<>OLD.payload OR (OLD.verdicts IS NOT NULL AND
     (NEW.verdicts,NEW.assessment_request_id,NEW.assessed_at) IS DISTINCT FROM (OLD.verdicts,OLD.assessment_request_id,OLD.assessed_at)) THEN
     RAISE EXCEPTION 'immutable support stage content';
   END IF;
 ELSE
   IF NEW.payload<>'{}'::jsonb OR NEW.verdicts IS NOT NULL OR (OLD.privacy_state='erased' AND NEW.privacy_state<>'erased') THEN
     RAISE EXCEPTION 'removed support stage cannot retain content';
   END IF;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_support_stage_immutable() FROM PUBLIC;
CREATE TRIGGER immutable_support_stage BEFORE UPDATE ON learning_support_stages FOR EACH ROW EXECUTE FUNCTION recollect_support_stage_immutable();

ALTER TABLE model_request_inputs DROP CONSTRAINT model_request_inputs_kind_check;
ALTER TABLE model_request_inputs ADD CONSTRAINT model_request_inputs_kind_check
 CHECK(kind IN ('source_version','repository_fact','claim_revision','manifest_revision','semantic_entry','learning_support_stage'));

-- Input relations already freeze the run's source and offered revision identities.
-- Scrub stages on canonical removal even when no claim has yet been published.
CREATE FUNCTION recollect_support_stage_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.privacy_state=OLD.privacy_state OR NEW.privacy_state='active' THEN RETURN NEW; END IF;
 UPDATE learning_support_stages s SET payload='{}',verdicts=NULL,
   privacy_state=CASE WHEN s.privacy_state='erased' THEN 'erased' ELSE NEW.privacy_state END
 FROM learning_runs r WHERE s.run_id=r.id AND s.brain_id=NEW.brain_id AND (
   (TG_TABLE_NAME='source_versions' AND (r.source_version_id=NEW.id OR EXISTS(SELECT 1 FROM learning_run_inputs i JOIN claim_supports c ON c.revision_id=i.revision_id AND c.brain_id=i.brain_id WHERE i.run_id=r.id AND c.source_version_id=NEW.id))) OR
   (TG_TABLE_NAME='claim_revisions' AND EXISTS(SELECT 1 FROM learning_run_inputs i WHERE i.run_id=r.id AND i.revision_id=NEW.id)) OR
   (TG_TABLE_NAME='repository_snapshots' AND EXISTS(SELECT 1 FROM learning_run_inputs i JOIN claim_supports c ON c.revision_id=i.revision_id AND c.brain_id=i.brain_id JOIN repository_facts f ON f.id=c.fact_id AND f.brain_id=c.brain_id WHERE i.run_id=r.id AND f.snapshot_id=NEW.id)) OR
   (TG_TABLE_NAME='manifest_revisions' AND (r.manifest_revision_id=NEW.id OR EXISTS(SELECT 1 FROM learning_run_inputs i JOIN claim_supports c ON c.revision_id=i.revision_id AND c.brain_id=i.brain_id WHERE i.run_id=r.id AND c.manifest_revision_id=NEW.id))));
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_support_stage_removed() FROM PUBLIC;
CREATE TRIGGER support_source_removed AFTER UPDATE OF privacy_state ON source_versions FOR EACH ROW EXECUTE FUNCTION recollect_support_stage_removed();
CREATE TRIGGER support_revision_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW EXECUTE FUNCTION recollect_support_stage_removed();
CREATE TRIGGER support_snapshot_removed AFTER UPDATE OF privacy_state ON repository_snapshots FOR EACH ROW EXECUTE FUNCTION recollect_support_stage_removed();
CREATE TRIGGER support_manifest_removed AFTER UPDATE OF privacy_state ON manifest_revisions FOR EACH ROW EXECUTE FUNCTION recollect_support_stage_removed();

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_support_privacy_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; stages uuid[];
BEGIN
 m:=recollect_pre_support_privacy_closure(b,k,t,c);
 SELECT coalesce(array_agg(DISTINCT r.id ORDER BY r.id),'{}') INTO stages FROM learning_runs r
 WHERE r.brain_id=b AND ((k='learning_support_stage' AND r.id=t) OR
   r.source_version_id=ANY(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')) OR
   r.manifest_revision_id=ANY(recollect_privacy_ids(m,'manifest_revisions')) OR
   EXISTS(SELECT 1 FROM learning_run_inputs i LEFT JOIN claim_supports s ON s.revision_id=i.revision_id AND s.brain_id=i.brain_id WHERE i.run_id=r.id AND (
     i.revision_id=ANY(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions')) OR
     s.source_version_id=ANY(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')) OR
     s.fact_id=ANY(recollect_privacy_ids(m,'facts')) OR s.manifest_revision_id=ANY(recollect_privacy_ids(m,'manifest_revisions')))));
 RETURN m||jsonb_build_object('learning_support_stages',stages);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_support_privacy_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; stages uuid[];
BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 stages:=recollect_privacy_ids(r.manifest,'learning_support_stages');
 -- Canonical input fencing sets the stronger removed run disposition before
 -- stage-only cancellation can trigger the generic cancelled job projection.
 PERFORM recollect_pre_support_privacy_apply(rid);
 UPDATE learning_support_stages SET payload='{}',verdicts=NULL,
   privacy_state=CASE WHEN privacy_state='erased' OR r.cause='erase' THEN 'erased' ELSE 'expired' END
 WHERE brain_id=r.brain_id AND run_id=ANY(stages);
 UPDATE model_requests SET suppressed=true WHERE brain_id=r.brain_id AND id IN
   (SELECT request_id FROM model_request_inputs WHERE brain_id=r.brain_id AND kind='learning_support_stage' AND input_id=ANY(stages));
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=r.brain_id AND target_id=ANY(stages) AND kind='source.learn' AND state IN ('queued','running');
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
CREATE OR REPLACE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE plpgsql IMMUTABLE SET search_path=public,pg_temp AS $$
DECLARE k text;
BEGIN
 FOREACH k IN ARRAY ARRAY['model_input_sources','model_input_claim_revisions','capture_event_fences','learning_support_stages'] LOOP
   IF m->k='[]'::jsonb THEN m:=m-k; END IF;
 END LOOP;
 RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;

-- Admission denial happens before a model request exists. Requeue the SAME key
-- without consuming worker recovery or charged replacement-attempt capacity.
CREATE FUNCTION recollect_defer_learning_job(jid uuid,token uuid,reason text) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE target uuid;
BEGIN
 IF reason NOT IN ('model_budget_exhausted','model_concurrency_full') THEN RETURN false; END IF;
 UPDATE jobs SET state='queued',error_code=reason,progress=0,attempts=greatest(attempts-1,0),
   lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp(),
   not_before=CASE WHEN reason='model_budget_exhausted'
     THEN (date_trunc('day',clock_timestamp() AT TIME ZONE 'UTC')+interval '1 day') AT TIME ZONE 'UTC'
     ELSE clock_timestamp()+interval '10 seconds' END
 WHERE id=jid AND kind='source.learn' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
 RETURNING target_id INTO target;
 IF target IS NULL THEN RETURN false; END IF;
 UPDATE learning_runs SET state='queued',error_code=reason WHERE id=target AND state IN ('queued','running');
 RETURN true;
END $$;
REVOKE ALL ON FUNCTION recollect_defer_learning_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_defer_learning_job(uuid,uuid,text) TO recollect_app;

-- A local publication fault requeues the same work and its committed verdict.
-- Keep the observable run state aligned with that native recovery state.
CREATE OR REPLACE FUNCTION recollect_learning_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.kind='source.learn' AND NEW.state IN ('queued','cancelled','failed') THEN
  UPDATE learning_runs SET state=NEW.state,error_code=NEW.error_code,
    finished_at=CASE WHEN NEW.state='queued' THEN NULL ELSE clock_timestamp() END
    WHERE job_id=NEW.id AND state IN ('queued','running');
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_learning_job_state() FROM PUBLIC;

-- A staged assertion/reason follows derived-memory retention even when its
-- original document is durable. Removal is journaled without expiring that
-- independent source or a subsequently published canonical revision.
ALTER FUNCTION recollect_expire_one() RENAME TO recollect_pre_support_expire_one;
CREATE FUNCTION recollect_expire_one() RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE rid uuid; candidate record; m jsonb;
BEGIN
 rid:=recollect_pre_support_expire_one();
 IF rid IS NOT NULL THEN RETURN rid; END IF;
 SELECT run_id,brain_id INTO candidate FROM learning_support_stages
 WHERE privacy_state='active' AND recollect_learning_support_deadline(brain_id,run_id)<=clock_timestamp()
 ORDER BY created_at,run_id LIMIT 1;
 IF NOT FOUND THEN RETURN NULL; END IF;
 PERFORM id FROM brains WHERE id=candidate.brain_id FOR UPDATE SKIP LOCKED;
 IF NOT FOUND THEN RETURN NULL; END IF;
 IF NOT EXISTS(SELECT 1 FROM learning_support_stages WHERE run_id=candidate.run_id AND privacy_state='active'
   AND recollect_learning_support_deadline(brain_id,run_id)<=clock_timestamp()) THEN RETURN NULL; END IF;
 rid:=gen_random_uuid();
 m:=recollect_privacy_closure(candidate.brain_id,'learning_support_stage',candidate.run_id,'expire');
 INSERT INTO privacy_requests(id,brain_id,target,cause,manifest)
 VALUES(rid,candidate.brain_id,jsonb_build_object('kind','learning_support_stage','id',candidate.run_id),'expire',m);
 PERFORM recollect_privacy_apply(rid);
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=candidate.brain_id AND target_id=candidate.run_id AND state IN ('queued','running');
 RETURN rid;
END $$;
REVOKE ALL ON FUNCTION recollect_expire_one() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_one() TO recollect_app;
