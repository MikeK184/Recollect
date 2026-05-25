CREATE TABLE graph_generations (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 kind text NOT NULL CHECK(kind IN ('repository','knowledge')),
 snapshot_id uuid,
 input_epoch bigint NOT NULL,
 adapter text NOT NULL DEFAULT 'canonical-graph-1',
 actor_id uuid NOT NULL REFERENCES accounts(id),
 job_id uuid NOT NULL UNIQUE REFERENCES jobs(id),
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','ready','failed','cancelled','superseded','removed')),
 descriptor jsonb,
 node_count bigint NOT NULL DEFAULT 0,
 edge_count bigint NOT NULL DEFAULT 0,
 unresolved bigint NOT NULL DEFAULT 0,
 ambiguous bigint NOT NULL DEFAULT 0,
 unsupported bigint NOT NULL DEFAULT 0,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 published_at timestamptz,
 error_code text,
 cleaned_at timestamptz,
 UNIQUE(id,brain_id),
 FOREIGN KEY(snapshot_id,brain_id) REFERENCES repository_snapshots(id,brain_id),
 CHECK((kind='repository')=(snapshot_id IS NOT NULL)),
 CHECK(node_count BETWEEN 0 AND 100000 AND edge_count BETWEEN 0 AND 250000)
);
CREATE INDEX graph_input_history ON graph_generations(brain_id,kind,snapshot_id,created_at DESC,id DESC);
ALTER TABLE graph_generations ENABLE ROW LEVEL SECURITY;
ALTER TABLE graph_generations FORCE ROW LEVEL SECURITY;
CREATE POLICY graph_read ON graph_generations FOR SELECT USING(brain_id IN (SELECT id FROM brains));
CREATE POLICY graph_write ON graph_generations FOR ALL
 USING(brain_id IN (SELECT id FROM brains WHERE recollect_role(id) IN ('writer','admin')))
 WITH CHECK(brain_id IN (SELECT id FROM brains WHERE recollect_role(id) IN ('writer','admin')));
GRANT SELECT,INSERT,UPDATE,DELETE ON graph_generations TO recollect_app;

-- Same Brain read authority, evaluated as a visible-Brain set instead of a
-- role lookup for each of up to 100,000 facts. Insert authority is unchanged.
ALTER POLICY publication_read ON repository_facts
 USING(brain_id IN (SELECT id FROM brains));
ALTER POLICY evidence_read ON source_chunks
 USING(brain_id IN (SELECT id FROM brains));

ALTER TABLE brains ADD COLUMN graph_scanned_at timestamptz NOT NULL DEFAULT 'epoch';
CREATE FUNCTION recollect_graph_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT b.id,a.id FROM brains b CROSS JOIN LATERAL (
   SELECT id FROM accounts a WHERE recollect_effective_role(b.id,a.id) IN ('writer','admin')
   ORDER BY (a.id=b.owner_id) DESC,a.id LIMIT 1
 ) a WHERE NOT b.archived AND (
   EXISTS(SELECT 1 FROM claims c WHERE c.brain_id=b.id)
   OR EXISTS(SELECT 1 FROM repository_snapshots s WHERE s.brain_id=b.id)
   OR EXISTS(SELECT 1 FROM graph_generations g WHERE g.brain_id=b.id))
 ORDER BY b.graph_scanned_at,b.id LIMIT 100
$$;
REVOKE ALL ON FUNCTION recollect_graph_brains() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_graph_brains() TO recollect_app;

CREATE FUNCTION recollect_graph_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.kind='graph.project' AND NEW.state IN ('queued','running','failed','cancelled') THEN
  UPDATE graph_generations SET state=NEW.state,error_code=NEW.error_code
   WHERE job_id=NEW.id AND state NOT IN ('ready','superseded','removed');
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_graph_job_state() FROM PUBLIC;
CREATE TRIGGER graph_job_state AFTER UPDATE OF state ON jobs
 FOR EACH ROW EXECUTE FUNCTION recollect_graph_job_state();

-- Keep the existing retry schedule and lease fence. Graph adapters supply only
-- static, bounded failure codes, never Neo4j messages or response bodies.
CREATE OR REPLACE FUNCTION recollect_fail_job(job_id uuid, token uuid, outcome text, reason text) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH changed AS (
  UPDATE jobs SET state=CASE WHEN outcome='queued' AND attempts>=max_attempts THEN 'failed' ELSE outcome END,
   error_code=reason,progress=0,lease_token=NULL,lease_until=NULL,
   not_before=clock_timestamp()+make_interval(secs=>least(attempts,2)),updated_at=clock_timestamp()
  WHERE id=job_id AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
   AND outcome IN ('queued','failed','cancelled')
   AND (reason IN ('database_error','permission_revoked','unsupported_kind','input_missing','artifact_unavailable')
    OR (kind='graph.project' AND reason IN ('graph_input_changed','graph_input_unavailable','graph_input_expired',
     'graph_input_too_large','graph_generation_changed','graph_configuration_invalid','graph_query_invalid',
     'graph_backend_transient','graph_query_failed','graph_unavailable','graph_attempt_timeout',
     'graph_response_incomplete','graph_response_too_large','graph_response_invalid','graph_import_mismatch')))
  RETURNING id
 ) SELECT EXISTS(SELECT 1 FROM changed)
$$;

-- Graph identities were not stored before this migration. New requests must
-- complete the derived store cleanup; startup replay also cleans old snapshots.
ALTER TABLE privacy_requests ADD COLUMN graph_pending boolean NOT NULL DEFAULT false;
ALTER TABLE privacy_requests ALTER COLUMN graph_pending SET DEFAULT true;
CREATE FUNCTION recollect_privacy_graph_done(rid uuid) RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 UPDATE privacy_requests SET graph_pending=false WHERE id=rid
$$;
REVOKE ALL ON FUNCTION recollect_privacy_graph_done(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_privacy_graph_done(uuid) TO recollect_app;
CREATE OR REPLACE FUNCTION recollect_privacy_progress(rid uuid,exported boolean,removed uuid[],failure text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF failure IS NOT NULL AND failure NOT IN('journal_unavailable','artifact_unavailable','graph_unavailable') THEN RAISE invalid_parameter_value; END IF;
 UPDATE privacy_artifacts SET removed_at=clock_timestamp() WHERE request_id=rid AND artifact_id=ANY(removed) AND removed_at IS NULL;
 UPDATE privacy_requests SET journaled=journaled OR exported,error_code=failure,
 state=CASE WHEN failure IS NOT NULL THEN 'error'
  WHEN (journaled OR exported) AND NOT graph_pending AND NOT EXISTS(SELECT 1 FROM privacy_artifacts WHERE request_id=rid AND removed_at IS NULL) THEN 'complete'
  ELSE 'pending' END WHERE id=rid;
END $$;

-- Retained generation history owns its job; do not let ordinary pruning break it.
CREATE OR REPLACE FUNCTION recollect_claim_job(selected_lane text, token uuid) RETURNS SETOF jobs
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE slot_limit integer; lane_lock integer;
BEGIN
 CASE selected_lane
  WHEN 'interactive' THEN slot_limit:=2; lane_lock:=73241101;
  WHEN 'capture' THEN slot_limit:=2; lane_lock:=73241102;
  WHEN 'model' THEN slot_limit:=1; lane_lock:=73241103;
  WHEN 'heavy' THEN slot_limit:=1; lane_lock:=73241104;
  ELSE RETURN;
 END CASE;
 PERFORM pg_advisory_xact_lock(lane_lock);
 DELETE FROM command_receipts WHERE (actor_id,key) IN (SELECT actor_id,key FROM command_receipts WHERE expires_at<=now() LIMIT 100);
 DELETE FROM jobs WHERE id IN (SELECT id FROM jobs WHERE state IN ('succeeded','failed','cancelled') AND updated_at<now()-interval '7 days'
  AND NOT EXISTS(SELECT 1 FROM repository_jobs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM learning_runs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM handover_runs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM semantic_batches r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM graph_generations r WHERE r.job_id=jobs.id) LIMIT 100);
 UPDATE jobs SET state='failed',error_code='lease_expired',lease_token=NULL,lease_until=NULL,updated_at=now()
  WHERE lane=selected_lane AND state='running' AND lease_until<=clock_timestamp() AND attempts>=max_attempts;
 IF (SELECT count(*) FROM jobs WHERE lane=selected_lane AND state='running' AND lease_until>clock_timestamp())>=slot_limit THEN RETURN; END IF;
 RETURN QUERY UPDATE jobs SET state='running',attempts=attempts+1,progress=1,lease_token=token,
  lease_until=clock_timestamp()+interval '20 seconds',error_code=NULL,updated_at=now()
  WHERE id=(SELECT id FROM jobs WHERE lane=selected_lane AND attempts<max_attempts AND
   ((state='queued' AND not_before<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp()))
   ORDER BY created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *;
END $$;
