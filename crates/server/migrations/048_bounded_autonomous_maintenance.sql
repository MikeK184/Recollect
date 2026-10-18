-- Scheduling progress contains opaque identities only, never copied content or
-- cached eligibility. Each policy starts its own fair, restart-safe walk.
CREATE TABLE autonomous_maintenance_cursors (
 brain_id uuid NOT NULL REFERENCES brains(id),
 policy_id uuid NOT NULL,
 kind text NOT NULL CHECK(kind IN ('source','digest_discovery','digest','handover')),
 last_id uuid,
 version bigint NOT NULL CHECK(version>0),
 PRIMARY KEY(brain_id,policy_id,kind),
 FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id)
);
ALTER TABLE autonomous_maintenance_cursors ENABLE ROW LEVEL SECURITY;
ALTER TABLE autonomous_maintenance_cursors FORCE ROW LEVEL SECURITY;
CREATE POLICY maintenance_cursor_read ON autonomous_maintenance_cursors FOR SELECT
 USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY maintenance_cursor_write ON autonomous_maintenance_cursors FOR ALL
 USING(recollect_role(brain_id) IN ('writer','admin'))
 WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT,UPDATE ON autonomous_maintenance_cursors TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation)
 VALUES(681,'autonomous_maintenance_cursors');
CREATE FUNCTION recollect_maintenance_cursor_changed() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.brain_id<>OLD.brain_id OR NEW.policy_id<>OLD.policy_id
  OR NEW.kind<>OLD.kind OR NEW.version<>OLD.version+1 THEN
  RAISE EXCEPTION 'Immutable maintenance identity and monotonic progress required';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER maintenance_cursor_changed BEFORE UPDATE ON autonomous_maintenance_cursors
 FOR EACH ROW EXECUTE FUNCTION recollect_maintenance_cursor_changed();

-- The existing all-Brain enumerator remains available for compatibility. The
-- autonomous scheduler admits only a bounded exact event window instead.
CREATE FUNCTION recollect_digest_discover_events(b uuid, inputs uuid[]) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF recollect_role(b) IS NULL OR recollect_role(b) NOT IN ('writer','admin')
  OR inputs IS NULL OR cardinality(inputs) NOT BETWEEN 1 AND 16 THEN
  RAISE EXCEPTION 'Bounded digest writer inputs required';
 END IF;
 WITH candidates AS (
  SELECT DISTINCT cb.brain_id,cb.id binding_id,o.task_id,cb.actor_id,cb.device_id,
   cb.selection,e.metadata->>'host_session_id' session,e.metadata->>'agent_id' agent,
   md5(jsonb_build_array(cb.id,e.metadata->>'host_session_id',e.metadata->>'agent_id',cb.selection,NULL)::text) identity
  FROM unnest(inputs) input(id) JOIN capture_events e ON e.id=input.id AND e.brain_id=b
  JOIN capture_bindings cb ON cb.id=e.binding_id AND cb.brain_id=e.brain_id
  JOIN operation_bindings o ON o.id=cb.operation_id AND o.brain_id=cb.brain_id
  WHERE e.state='accepted' AND length(e.metadata->>'host_session_id')>0
 )
 INSERT INTO session_digest_partitions(id,brain_id,binding_id,task_id,actor_id,device_id,
  selection,host_session_id,agent_id,identity_key)
 SELECT gen_random_uuid(),brain_id,binding_id,task_id,actor_id,device_id,selection,session,agent,identity
 FROM candidates ON CONFLICT DO NOTHING;
 INSERT INTO session_digest_events(event_id,partition_id,brain_id)
 SELECT e.id,p.id,p.brain_id FROM unnest(inputs) input(id)
 JOIN capture_events e ON e.id=input.id AND e.brain_id=b AND e.state='accepted'
 JOIN session_digest_partitions p ON p.brain_id=e.brain_id AND p.binding_id=e.binding_id
  AND e.metadata->>'host_session_id'=p.host_session_id
  AND e.metadata->>'agent_id' IS NOT DISTINCT FROM p.agent_id AND p.privacy_state='active'
 ON CONFLICT DO NOTHING;
 UPDATE session_digest_partitions p SET observed_receipt=greatest(p.observed_receipt,selected.receipt)
 FROM (SELECT d.partition_id,max(e.receipt_sequence) receipt
  FROM unnest(inputs) input(id) JOIN capture_events e ON e.id=input.id AND e.brain_id=b
  JOIN session_digest_events d ON d.event_id=e.id AND d.brain_id=b GROUP BY d.partition_id) selected
 WHERE p.id=selected.partition_id AND p.brain_id=b AND p.privacy_state='active'
  AND p.observed_receipt<selected.receipt;
END $$;
REVOKE ALL ON FUNCTION recollect_digest_discover_events(uuid,uuid[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_digest_discover_events(uuid,uuid[]) TO recollect_app;
CREATE INDEX maintenance_capture_walk ON capture_events(brain_id,id);
CREATE INDEX maintenance_digest_walk ON session_digest_partitions(brain_id,id);

-- Native failures retain their existing three-attempt ceiling. A claim that
-- loses preparation qualification before any native call has a separate finite
-- contention budget. The committed marker proves this exact lease never entered
-- outbound setup/import; stale or already-used leases cannot refund attempts.
ALTER TABLE jobs ADD COLUMN preparation_deferrals integer NOT NULL DEFAULT 0
 CHECK(preparation_deferrals BETWEEN 0 AND 8);
ALTER TABLE graph_generations ADD COLUMN native_attempt_token uuid;
CREATE OR REPLACE FUNCTION recollect_fail_job(job_id uuid, token uuid, outcome text, reason text) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH changed AS (
  UPDATE jobs SET state=CASE
   WHEN outcome='queued' AND reason='graph_preparation_contended' THEN
    CASE WHEN preparation_deferrals>=8 THEN 'failed' ELSE 'queued' END
   WHEN outcome='queued' AND attempts>=max_attempts THEN 'failed' ELSE outcome END,
   attempts=CASE WHEN reason='graph_preparation_contended' AND preparation_deferrals<8 THEN greatest(attempts-1,0) ELSE attempts END,
   preparation_deferrals=CASE WHEN reason='graph_preparation_contended' AND preparation_deferrals<8 THEN preparation_deferrals+1 ELSE preparation_deferrals END,
   error_code=reason,progress=0,lease_token=NULL,lease_until=NULL,
   not_before=clock_timestamp()+make_interval(secs=>least(attempts,2)),updated_at=clock_timestamp()
  WHERE id=job_id AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
   AND outcome IN ('queued','failed','cancelled')
   AND (reason IN ('database_error','permission_revoked','unsupported_kind','input_missing','artifact_unavailable')
    OR (kind='graph.project' AND reason IN ('graph_input_changed','graph_input_unavailable','graph_input_expired',
     'graph_input_too_large','graph_generation_changed','graph_configuration_invalid','graph_query_invalid',
     'graph_backend_transient','graph_query_failed','graph_unavailable','graph_attempt_timeout',
     'graph_response_incomplete','graph_response_too_large','graph_response_invalid','graph_import_mismatch',
     'graph_preparation_changed'))
    OR (kind='graph.project' AND reason='graph_preparation_contended' AND outcome='queued'
     AND attempts>0 AND EXISTS(SELECT 1 FROM graph_generations g WHERE g.job_id=jobs.id AND g.brain_id=jobs.brain_id
      AND g.state IN ('queued','running') AND g.native_attempt_token IS DISTINCT FROM token)))
  RETURNING id
 ) SELECT EXISTS(SELECT 1 FROM changed)
$$;
