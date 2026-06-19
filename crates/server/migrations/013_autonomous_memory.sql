ALTER TABLE learning_runs ADD COLUMN reconciliation_inputs uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE learning_runs ADD COLUMN revised integer NOT NULL DEFAULT 0;
ALTER TABLE learning_runs ADD COLUMN retired integer NOT NULL DEFAULT 0;
ALTER TABLE learning_runs ADD COLUMN automatic_attempt integer NOT NULL DEFAULT 0 CHECK(automatic_attempt BETWEEN 0 AND 2);
ALTER TABLE learning_runs ADD COLUMN retry_of uuid UNIQUE;
ALTER TABLE learning_runs ADD CONSTRAINT learning_retry_parent FOREIGN KEY(retry_of,brain_id) REFERENCES learning_runs(id,brain_id);
DROP INDEX learning_automatic_once;
CREATE UNIQUE INDEX learning_automatic_once ON learning_runs(brain_id,source_version_id,policy_id) WHERE automatic AND retry_of IS NULL;
CREATE TABLE learning_run_inputs (
    run_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    revision_id uuid NOT NULL,
    PRIMARY KEY(run_id,revision_id),
    FOREIGN KEY(run_id,brain_id) REFERENCES learning_runs(id,brain_id),
    FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id)
);
ALTER TABLE learning_run_inputs ENABLE ROW LEVEL SECURITY;
ALTER TABLE learning_run_inputs FORCE ROW LEVEL SECURITY;
CREATE POLICY read_inputs ON learning_run_inputs FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY write_inputs ON learning_run_inputs FOR INSERT WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT ON learning_run_inputs TO recollect_app;
ALTER TABLE handover_runs ADD COLUMN automatic boolean NOT NULL DEFAULT false;
ALTER TABLE handover_runs ADD COLUMN automatic_attempt integer NOT NULL DEFAULT 0 CHECK(automatic_attempt BETWEEN 0 AND 2);
ALTER TABLE handover_runs ADD COLUMN retry_of uuid UNIQUE;
ALTER TABLE handover_runs ADD CONSTRAINT handover_retry_parent FOREIGN KEY(retry_of,brain_id) REFERENCES handover_runs(id,brain_id);
ALTER TABLE handover_runs ADD COLUMN base_revision_id uuid;
ALTER TABLE handover_runs ADD CONSTRAINT handover_base FOREIGN KEY(base_revision_id,brain_id) REFERENCES claim_revisions(id,brain_id);
CREATE INDEX automatic_handover_base ON handover_runs(brain_id,base_revision_id) WHERE automatic;

-- The native scheduler needs only routing identities. All work re-enters the
-- policy author's current account/Brain transaction before reading any content.
CREATE FUNCTION recollect_autonomous_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT b.id,p.created_by FROM brains b
    JOIN model_policy_heads h ON h.brain_id=b.id
    JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id
    JOIN accounts a ON a.id=p.created_by
    WHERE NOT b.archived AND a.enabled
      AND p.policy->>'enabled'='true' AND p.policy->>'autonomous_memory'='true'
    ORDER BY b.id
$$;
REVOKE ALL ON FUNCTION recollect_autonomous_brains() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_autonomous_brains() TO recollect_app;

CREATE FUNCTION recollect_reconciliation_input_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NEW.privacy_state=OLD.privacy_state OR NEW.privacy_state='active' THEN RETURN NEW; END IF;
  UPDATE learning_runs SET state='removed',error_code='content_removed',finished_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND state IN ('queued','running')
      AND id IN(SELECT run_id FROM learning_run_inputs WHERE brain_id=NEW.brain_id AND revision_id=NEW.id);
  UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND kind='source.learn' AND state IN ('queued','running')
      AND target_id IN(SELECT id FROM learning_runs WHERE brain_id=NEW.brain_id AND state='removed');
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_reconciliation_input_removed() FROM PUBLIC;
CREATE TRIGGER reconciliation_input_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW EXECUTE FUNCTION recollect_reconciliation_input_removed();

-- An admitted reconciliation already depends on its old memory inputs even
-- before it publishes a revision. Fence its new source too when those inputs
-- are explicitly erased, including replay into a backup predating that source.
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_handover_privacy_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; revisions uuid[]; sources uuid[]; work uuid[];
BEGIN
  m:=recollect_handover_privacy_closure(b,k,t,c);
  IF c='erase' THEN
    revisions:=recollect_privacy_ids(m,'claim_revisions');
    SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO sources FROM (
      SELECT unnest(recollect_privacy_ids(m,'model_input_sources')) AS id UNION
      SELECT r.source_version_id FROM learning_runs r JOIN learning_run_inputs i ON i.run_id=r.id
        WHERE i.brain_id=b AND i.revision_id=ANY(revisions)
    ) x;
    SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO work FROM (
      SELECT unnest(recollect_privacy_ids(m,'jobs')) AS id UNION
      SELECT j.id FROM jobs j JOIN learning_runs r ON r.job_id=j.id JOIN learning_run_inputs i ON i.run_id=r.id
        WHERE i.brain_id=b AND i.revision_id=ANY(revisions) AND j.state IN ('queued','running')
    ) x;
    m:=m||jsonb_build_object('model_input_sources',sources,'jobs',work);
  END IF;
  RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

CREATE OR REPLACE FUNCTION recollect_finish_model_request(rid uuid,token uuid,outcome text,reason text,returned text,it bigint,ot bigint,tt bigint,dims integer)
RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  WITH changed AS (
    UPDATE model_requests SET state=outcome,error_code=reason,returned_model=returned,
      input_tokens=it,output_tokens=ot,total_tokens=tt,dimensions=dims,
      charged_tokens=coalesce(tt,reserved_tokens),finished_at=clock_timestamp()
    WHERE id=rid AND call_token=token AND state='running'
      AND outcome IN ('succeeded','failed','uncertain')
      AND (reason IS NULL OR reason IN ('provider_http','provider_rate_limited','provider_unavailable','provider_timeout','provider_transport','provider_refusal','provider_incomplete','provider_shape','provider_body_limit'))
      AND (returned IS NULL OR length(returned)<=120)
      AND coalesce(it>=0,true) AND coalesce(ot>=0,true) AND coalesce(tt>=0,true)
    RETURNING id
  ) SELECT EXISTS(SELECT 1 FROM changed)
$$;
