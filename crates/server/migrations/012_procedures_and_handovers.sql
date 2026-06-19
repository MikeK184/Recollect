CREATE TABLE claim_contributions (
    revision_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    input_revision_id uuid NOT NULL,
    PRIMARY KEY(revision_id,input_revision_id),
    FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
    FOREIGN KEY(input_revision_id,brain_id) REFERENCES claim_revisions(id,brain_id)
);
CREATE INDEX contribution_dependents ON claim_contributions(brain_id,input_revision_id);
CREATE TABLE handover_runs (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    title text NOT NULL,
    contributions uuid[] NOT NULL,
    selection jsonb NOT NULL,
    policy_id uuid NOT NULL,
    actor_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    operation_id uuid,
    job_id uuid NOT NULL UNIQUE REFERENCES jobs(id),
    state text NOT NULL CHECK(state IN ('queued','running','succeeded','failed','cancelled','removed')),
    error_code text,
    request_id uuid,
    claim_id uuid,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    finished_at timestamptz,
    UNIQUE(id,brain_id),
    FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),
    FOREIGN KEY(claim_id,brain_id) REFERENCES claims(id,brain_id)
);
CREATE TABLE handover_run_inputs (
    run_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    revision_id uuid NOT NULL,
    PRIMARY KEY(run_id,revision_id),
    FOREIGN KEY(run_id,brain_id) REFERENCES handover_runs(id,brain_id),
    FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id)
);
CREATE INDEX handover_input_runs ON handover_run_inputs(brain_id,revision_id);
CREATE TABLE model_claim_fences (
    brain_id uuid NOT NULL REFERENCES brains(id),
    revision_id uuid NOT NULL,
    request_id uuid NOT NULL,
    PRIMARY KEY(brain_id,revision_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES privacy_requests(id,brain_id)
);
ALTER TABLE model_claim_fences ENABLE ROW LEVEL SECURITY;
ALTER TABLE model_claim_fences FORCE ROW LEVEL SECURITY;
CREATE POLICY read_fences ON model_claim_fences FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
GRANT SELECT ON model_claim_fences TO recollect_app;
DO $$
DECLARE tab text;
BEGIN
  FOREACH tab IN ARRAY ARRAY['claim_contributions','handover_runs','handover_run_inputs'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY read_rows ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY write_rows ON %I FOR ALL USING(recollect_role(brain_id) IN (''writer'',''admin'')) WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('GRANT SELECT,INSERT,UPDATE ON %I TO recollect_app',tab);
  END LOOP;
END $$;

CREATE FUNCTION recollect_handover_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NEW.kind='handover.generate' AND NEW.state IN ('failed','cancelled') THEN
    UPDATE handover_runs SET state=NEW.state,error_code=NEW.error_code,finished_at=clock_timestamp()
      WHERE job_id=NEW.id AND state IN ('queued','running');
  END IF;
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_handover_job_state() FROM PUBLIC;
CREATE TRIGGER handover_job_state AFTER UPDATE OF state ON jobs FOR EACH ROW EXECUTE FUNCTION recollect_handover_job_state();

CREATE FUNCTION recollect_fail_handover_job(jid uuid,token uuid,reason text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE target uuid;
BEGIN
  IF length(reason)>80 OR reason !~ '^[a-z_]+$' THEN RAISE EXCEPTION 'invalid handover failure'; END IF;
  UPDATE jobs SET state='failed',error_code=reason,progress=0,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE id=jid AND kind='handover.generate' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
    RETURNING target_id INTO target;
  IF target IS NOT NULL THEN
    UPDATE handover_runs SET request_id=(SELECT id FROM model_requests WHERE brain_id=handover_runs.brain_id AND actor_id=handover_runs.actor_id AND operation_id=handover_runs.id AND purpose='synthesis')
      WHERE id=target AND state='failed';
  END IF;
END $$;
REVOKE ALL ON FUNCTION recollect_fail_handover_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_fail_handover_job(uuid,uuid,text) TO recollect_app;

CREATE FUNCTION recollect_handover_input_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NEW.privacy_state=OLD.privacy_state OR NEW.privacy_state='active' THEN RETURN NEW; END IF;
  UPDATE handover_runs SET title='',selection='{}',state='removed',error_code='content_removed',finished_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND (claim_id=NEW.claim_id OR
      ((NEW.privacy_state='erased' OR state IN ('queued','running')) AND id IN
      (SELECT run_id FROM handover_run_inputs WHERE brain_id=NEW.brain_id AND revision_id=NEW.id)));
  UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND kind='handover.generate' AND state IN ('queued','running')
      AND target_id IN(SELECT id FROM handover_runs WHERE brain_id=NEW.brain_id AND state='removed');
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_handover_input_removed() FROM PUBLIC;
CREATE TRIGGER handover_input_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW EXECUTE FUNCTION recollect_handover_input_removed();

-- Extend the canonical closure before writing its journal; the old applier consumes
-- the expanded identities through the same existing removal transaction.
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_learning_privacy_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; cr uuid[]; claim_ids uuid[]; rules uuid[]; decisions uuid[]; inputs uuid[]; sources uuid[]; work uuid[];
BEGIN
  m:=recollect_learning_privacy_closure(b,k,t,c);
  cr:=recollect_privacy_ids(m,'claim_revisions');
  IF c='erase' THEN
    WITH RECURSIVE affected(id) AS (
      SELECT unnest(cr) UNION
      SELECT d.revision_id FROM claim_contributions d JOIN affected a ON d.input_revision_id=a.id WHERE d.brain_id=b
    ) SELECT coalesce(array_agg(id ORDER BY id),'{}') INTO cr FROM affected;
    SELECT coalesce(array_agg(DISTINCT claim_id ORDER BY claim_id),'{}') INTO claim_ids FROM claim_revisions WHERE brain_id=b AND id=ANY(cr);
    SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO rules FROM (
      SELECT unnest(recollect_privacy_ids(m,'rules')) AS id UNION
      SELECT id FROM assertion_rules WHERE brain_id=b AND (rule->>'revision_id')::uuid=ANY(cr)
    ) x;
    SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO decisions FROM (
      SELECT unnest(recollect_privacy_ids(m,'decisions')) AS id UNION
      SELECT decision_id FROM assertion_rules WHERE brain_id=b AND id=ANY(rules) UNION
      SELECT id FROM memory_decisions d WHERE d.brain_id=b AND EXISTS(
        SELECT 1 FROM jsonb_array_elements(coalesce(d.decision->'transitions','[]')) tr
        WHERE (tr->>'before_revision')::uuid=ANY(cr) OR (tr->>'after_revision')::uuid=ANY(cr))
    ) x;
    SELECT coalesce(array_agg(DISTINCT input_revision_id ORDER BY input_revision_id),'{}') INTO inputs
      FROM claim_contributions WHERE brain_id=b AND revision_id=ANY(cr);
    SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO sources FROM (
      SELECT unnest(recollect_privacy_ids(m,'model_input_sources')) AS id UNION
      SELECT source_version_id FROM claim_supports WHERE brain_id=b AND revision_id=ANY(cr) AND source_version_id IS NOT NULL
    ) x;
    m:=m||jsonb_build_object('claim_revisions',cr,'claim_ids',claim_ids,'rules',rules,'decisions',decisions,
      'independent_claim_revisions',(SELECT count(*) FROM claim_revisions WHERE brain_id=b AND privacy_state='active' AND claim_id=ANY(claim_ids) AND NOT id=ANY(cr)));
    IF cardinality(inputs)>0 THEN m:=m||jsonb_build_object('model_input_claim_revisions',inputs); END IF;
    IF cardinality(sources)>0 THEN m:=m||jsonb_build_object('model_input_sources',sources); END IF;
  END IF;
  SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO work FROM (
    SELECT unnest(recollect_privacy_ids(m,'jobs')) AS id UNION
    SELECT j.id FROM jobs j JOIN handover_runs r ON r.job_id=j.id
      WHERE r.brain_id=b AND j.state IN ('queued','running') AND EXISTS(
        SELECT 1 FROM handover_run_inputs i WHERE i.run_id=r.id AND i.revision_id=ANY(cr||coalesce(inputs,'{}')))
  ) x;
  RETURN jsonb_set(m,'{jobs}',to_jsonb(work));
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_learning_privacy_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; inputs uuid[];
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
  inputs:=recollect_privacy_ids(r.manifest,'model_input_claim_revisions');
  IF r.cause='erase' THEN
    INSERT INTO model_claim_fences(brain_id,revision_id,request_id)
      SELECT r.brain_id,unnest(inputs),r.id ON CONFLICT DO NOTHING;
    UPDATE handover_runs SET title='',selection='{}',state='removed',error_code='model_input_fenced',finished_at=clock_timestamp()
      WHERE brain_id=r.brain_id AND id IN (SELECT run_id FROM handover_run_inputs WHERE brain_id=r.brain_id AND revision_id=ANY(inputs));
    UPDATE jobs SET state='cancelled',error_code='model_input_fenced',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
      WHERE brain_id=r.brain_id AND kind='handover.generate' AND state IN ('queued','running')
      AND target_id IN(SELECT id FROM handover_runs WHERE brain_id=r.brain_id AND state='removed');
    UPDATE model_requests SET suppressed=true WHERE brain_id=r.brain_id AND id IN
      (SELECT request_id FROM model_request_inputs WHERE brain_id=r.brain_id AND kind='claim_revision' AND input_id=ANY(inputs));
  END IF;
  PERFORM recollect_learning_privacy_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;

ALTER FUNCTION recollect_expire_model_details() RENAME TO recollect_expire_learning_details;
CREATE FUNCTION recollect_expire_model_details() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  PERFORM recollect_expire_learning_details();
  UPDATE handover_runs SET title='' WHERE id IN (
    SELECT id FROM handover_runs WHERE title<>'' AND recollect_retention_deadline(brain_id,'audit',created_at)<=clock_timestamp() LIMIT 100);
END $$;
REVOKE ALL ON FUNCTION recollect_expire_model_details() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_model_details() TO recollect_app;

-- Keep the job identities referenced by durable publication/model history.
CREATE OR REPLACE FUNCTION recollect_claim_job(selected_lane text, token uuid) RETURNS SETOF jobs
LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp AS $$
DECLARE slot_limit integer; lane_lock integer;
BEGIN
    CASE selected_lane
        WHEN 'interactive' THEN slot_limit := 2; lane_lock := 73241101;
        WHEN 'capture' THEN slot_limit := 2; lane_lock := 73241102;
        WHEN 'model' THEN slot_limit := 1; lane_lock := 73241103;
        WHEN 'heavy' THEN slot_limit := 1; lane_lock := 73241104;
        ELSE RETURN;
    END CASE;
    PERFORM pg_advisory_xact_lock(lane_lock);
    DELETE FROM command_receipts WHERE (actor_id,key) IN (SELECT actor_id,key FROM command_receipts WHERE expires_at<=now() LIMIT 100);
    DELETE FROM jobs WHERE id IN (SELECT id FROM jobs WHERE state IN ('succeeded','failed','cancelled') AND updated_at < now() - interval '7 days' AND NOT EXISTS(SELECT 1 FROM repository_jobs r WHERE r.job_id=jobs.id) AND NOT EXISTS(SELECT 1 FROM learning_runs r WHERE r.job_id=jobs.id) AND NOT EXISTS(SELECT 1 FROM handover_runs r WHERE r.job_id=jobs.id) LIMIT 100);
    UPDATE jobs SET state='failed', error_code='lease_expired', lease_token=NULL, lease_until=NULL, updated_at=now()
      WHERE lane=selected_lane AND state='running' AND lease_until<=clock_timestamp() AND attempts>=max_attempts;
    IF (SELECT count(*) FROM jobs WHERE lane=selected_lane AND state='running' AND lease_until>clock_timestamp()) >= slot_limit THEN RETURN; END IF;
    RETURN QUERY UPDATE jobs SET state='running', attempts=attempts+1, progress=1, lease_token=token,
        lease_until=clock_timestamp()+interval '20 seconds', error_code=NULL, updated_at=now()
    WHERE id = (
        SELECT id FROM jobs WHERE lane=selected_lane AND attempts<max_attempts AND
            ((state='queued' AND not_before<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp()))
        ORDER BY created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED
    ) RETURNING *;
END
$$;
