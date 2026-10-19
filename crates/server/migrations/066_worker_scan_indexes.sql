-- Worker polling and housekeeping must not scan whole tables.
-- Receipts expire by time alone; finished jobs are only ever cleaned by age.
CREATE INDEX command_receipts_expiry ON command_receipts(expires_at);
CREATE INDEX jobs_finished_cleanup ON jobs(updated_at) WHERE state IN ('succeeded','failed','cancelled');
-- Cleanup and foreign-key checks look up owners by job.
CREATE INDEX semantic_batches_job ON semantic_batches(job_id);
CREATE INDEX memory_support_assessments_job ON memory_support_assessments(job_id) WHERE job_id IS NOT NULL;
CREATE INDEX analytics_attempts_job ON analytics_attempts(job_id);
-- Semantic discovery anti-joins every chunk of one Brain against its entries.
CREATE INDEX source_chunks_brain ON source_chunks(brain_id,id);

-- Expired receipts and finished jobs are removed once per maintenance cycle
-- instead of inside every lane claim (six lanes poll every 500 ms).
CREATE FUNCTION recollect_cleanup_expired() RETURNS integer
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE removed integer:=0; step integer;
BEGIN
 DELETE FROM command_receipts WHERE (actor_id,key) IN (SELECT actor_id,key FROM command_receipts WHERE expires_at<=clock_timestamp() ORDER BY expires_at LIMIT 500);
 GET DIAGNOSTICS step=ROW_COUNT; removed:=removed+step;
 DELETE FROM jobs WHERE id IN (SELECT id FROM jobs WHERE state IN ('succeeded','failed','cancelled') AND updated_at<clock_timestamp()-interval '7 days'
  AND NOT EXISTS(SELECT 1 FROM repository_jobs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM learning_runs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM handover_runs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM semantic_batches r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM graph_generations r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM analytics_reports r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM memory_support_assessments r WHERE r.job_id=jobs.id)
  ORDER BY updated_at LIMIT 100);
 GET DIAGNOSTICS step=ROW_COUNT; removed:=removed+step;
 RETURN removed;
END $$;
REVOKE ALL ON FUNCTION recollect_cleanup_expired() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_cleanup_expired() TO recollect_app;

-- Claiming keeps lease expiry and bounded lane admission but no longer scans
-- for deletable rows; see recollect_cleanup_expired.
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
 UPDATE jobs SET state='failed',error_code='lease_expired',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
  WHERE lane=selected_lane AND state='running' AND lease_until<=clock_timestamp() AND attempts>=max_attempts;
 IF (SELECT count(*) FROM jobs WHERE lane=selected_lane AND state='running' AND lease_until>clock_timestamp())>=slot_limit THEN RETURN; END IF;
 RETURN QUERY UPDATE jobs SET state='running',attempts=attempts+1,progress=1,lease_token=token,
  lease_until=clock_timestamp()+interval '20 seconds',error_code=NULL,updated_at=clock_timestamp()
  WHERE id=(SELECT id FROM jobs WHERE lane=selected_lane AND attempts<max_attempts AND
   ((state='queued' AND not_before<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp()))
   ORDER BY created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *;
END $$;
