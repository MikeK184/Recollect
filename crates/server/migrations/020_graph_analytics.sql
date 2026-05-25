ALTER TABLE brains ADD COLUMN analytics_epoch bigint NOT NULL DEFAULT 0;
ALTER TABLE privacy_installation ADD COLUMN analytics_initialized boolean NOT NULL DEFAULT false;
CREATE TABLE analytics_reports (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 actor_id uuid NOT NULL REFERENCES accounts(id),
 job_id uuid NOT NULL UNIQUE REFERENCES jobs(id),
 algorithm text NOT NULL CHECK(algorithm IN ('pagerank','leiden','wcc')),
 direction text NOT NULL CHECK(direction IN ('outgoing','incoming','both')),
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','ready','stale','removed','failed','cancelled')),
 selection jsonb,
 provenance jsonb,
 descriptor jsonb,
 scores jsonb,
 parameters jsonb NOT NULL,
 gds_version text,
 analytics_epoch bigint NOT NULL,
 privacy_sequence bigint NOT NULL,
 node_count integer NOT NULL CHECK(node_count BETWEEN 1 AND 10000),
 edge_count integer NOT NULL CHECK(edge_count BETWEEN 0 AND 50000),
 projected_edge_count integer,
 estimated_bytes bigint,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 published_at timestamptz,
 expires_at timestamptz,
 error_code text,
 UNIQUE(id,brain_id)
);
CREATE INDEX analytics_recent ON analytics_reports(brain_id,created_at DESC,id DESC);
CREATE TABLE analytics_attempts (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 report_id uuid NOT NULL,
 job_id uuid NOT NULL REFERENCES jobs(id),
 lease_token uuid NOT NULL,
 privacy_sequence bigint NOT NULL,
 created_at timestamptz NOT NULL,
 deadline timestamptz NOT NULL,
 cleaned_at timestamptz,
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE
);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['analytics_reports','analytics_attempts'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY analytics_read ON %I FOR SELECT USING(brain_id IN (SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY analytics_write ON %I FOR ALL USING(brain_id IN (SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN (SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;

CREATE FUNCTION recollect_analytics_invalidate(affected uuid[]) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[];
BEGIN
 UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=ANY(affected);
 SELECT array_agg(id) INTO reports FROM analytics_reports WHERE brain_id=ANY(affected) AND state IN ('queued','running','ready');
 -- The durable claimant locks jobs before its report-state trigger. Keep that
 -- order here too; report-first invalidation deadlocks a simultaneous claim.
 UPDATE jobs SET state='cancelled',error_code='analytics_inputs_changed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=ANY(affected) AND kind='graph.analyze' AND state IN ('queued','running')
  AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='stale',scores=NULL,error_code='analytics_inputs_changed' WHERE id=ANY(reports);
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_invalidate(uuid[]) FROM PUBLIC;

-- Once per statement/Brain, not once per extracted fact or chunk. Analytics
-- does not advance memory_epoch or change autonomous model scheduling.
CREATE FUNCTION recollect_analytics_inputs_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE affected uuid[];
BEGIN
 IF TG_TABLE_NAME='graph_generations' THEN
  IF TG_OP='INSERT' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM new_rows WHERE state='ready';
  ELSIF TG_OP='DELETE' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM old_rows WHERE state='ready';
  ELSE SELECT array_agg(DISTINCT brain_id) INTO affected FROM (
   SELECT brain_id FROM new_rows WHERE state='ready' UNION SELECT brain_id FROM old_rows WHERE state='ready') x;
  END IF;
 ELSIF TG_OP='INSERT' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM new_rows;
 ELSIF TG_OP='DELETE' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM old_rows;
 ELSE SELECT array_agg(DISTINCT brain_id) INTO affected FROM (
  SELECT brain_id FROM new_rows UNION SELECT brain_id FROM old_rows) x;
 END IF;
 IF affected IS NOT NULL THEN PERFORM recollect_analytics_invalidate(affected); END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_inputs_changed() FROM PUBLIC;
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['sources','source_versions','source_chunks','evidence_groups','evidence_memberships',
  'claims','claim_revisions','claim_supports','claim_contributions','memory_epochs','assertion_rules','memory_rule_exceptions',
  'repositories','repository_origins','repository_snapshots','repository_facts','manifest_revisions','revision_manifests',
  'retention_policies','graph_generations'] LOOP
  EXECUTE format('CREATE TRIGGER analytics_insert AFTER INSERT ON %I REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
  EXECUTE format('CREATE TRIGGER analytics_update AFTER UPDATE ON %I REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
  EXECUTE format('CREATE TRIGGER analytics_delete AFTER DELETE ON %I REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
 END LOOP;
END $$;

CREATE FUNCTION recollect_analytics_removed(b uuid, sequence_before bigint) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[];
BEGIN
 -- Whole aggregates can contain influence from a removed input absent in the
 -- displayed top results. Conservative removal avoids partial score salvage.
 PERFORM id FROM brains WHERE id=b FOR UPDATE;
 SELECT array_agg(id) INTO reports FROM analytics_reports WHERE brain_id=b AND privacy_sequence<sequence_before;
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=b AND kind='graph.analyze' AND state IN ('queued','running')
 AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='content_removed' WHERE id=ANY(reports);
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_removed(uuid,bigint) FROM PUBLIC;
CREATE FUNCTION recollect_analytics_remove_request(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
 SELECT * INTO r FROM privacy_requests WHERE id=rid;
 IF FOUND THEN PERFORM recollect_analytics_removed(r.brain_id,r.sequence);
 ELSIF NOT EXISTS(SELECT 1 FROM privacy_replayed_entries WHERE id=rid) THEN RAISE insufficient_privilege;
 END IF;
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_remove_request(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_remove_request(uuid) TO recollect_app;
CREATE FUNCTION recollect_analytics_privacy_request() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN PERFORM recollect_analytics_removed(NEW.brain_id,NEW.sequence); RETURN NEW; END $$;
REVOKE ALL ON FUNCTION recollect_analytics_privacy_request() FROM PUBLIC;
CREATE TRIGGER analytics_privacy AFTER INSERT ON privacy_requests
 FOR EACH ROW EXECUTE FUNCTION recollect_analytics_privacy_request();

CREATE FUNCTION recollect_analytics_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.kind='graph.analyze' AND NEW.state IN ('queued','running','failed','cancelled') THEN
  UPDATE analytics_reports SET state=NEW.state,error_code=NEW.error_code
   WHERE job_id=NEW.id AND state NOT IN ('ready','stale','removed');
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_job_state() FROM PUBLIC;
CREATE TRIGGER analytics_job_state AFTER UPDATE OF state ON jobs
 FOR EACH ROW EXECUTE FUNCTION recollect_analytics_job_state();

CREATE FUNCTION recollect_fail_analytics_job(jid uuid, token uuid, outcome text, reason text) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH changed AS (
  UPDATE jobs SET state=CASE WHEN outcome='queued' AND attempts>=max_attempts THEN 'failed' ELSE outcome END,
   error_code=reason,progress=0,lease_token=NULL,lease_until=NULL,
   not_before=clock_timestamp()+make_interval(secs=>least(attempts,2)),updated_at=clock_timestamp()
  WHERE id=jid AND kind='graph.analyze' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
   AND outcome IN ('queued','failed','cancelled') AND length(reason)<=80 AND reason ~ '^[a-z_]+$'
  RETURNING id
 ) SELECT EXISTS(SELECT 1 FROM changed)
$$;
REVOKE ALL ON FUNCTION recollect_fail_analytics_job(uuid,uuid,text,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_fail_analytics_job(uuid,uuid,text,text) TO recollect_app;

CREATE FUNCTION recollect_analytics_attempt_live(jid uuid,token uuid) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM jobs j JOIN analytics_reports r ON r.job_id=j.id
 JOIN brains b ON b.id=j.brain_id WHERE j.id=jid AND j.lease_token=token AND j.state='running'
 AND j.lease_until>clock_timestamp() AND r.state='running' AND r.analytics_epoch=b.analytics_epoch
 AND (r.expires_at IS NULL OR r.expires_at>clock_timestamp())
 AND recollect_effective_role(j.brain_id,j.actor_id) IN ('writer','admin')
 AND (j.device_id IS NULL OR EXISTS(SELECT 1 FROM devices d WHERE d.id=j.device_id AND d.revoked_at IS NULL AND d.expires_at>clock_timestamp())))
$$;
REVOKE ALL ON FUNCTION recollect_analytics_attempt_live(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_attempt_live(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_analytics_cleaned(aid uuid,token uuid) RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 UPDATE analytics_attempts SET cleaned_at=clock_timestamp() WHERE id=aid AND lease_token=token
$$;
REVOKE ALL ON FUNCTION recollect_analytics_cleaned(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_cleaned(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_analytics_expected_entries() RETURNS SETOF analytics_attempts
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT * FROM analytics_attempts WHERE cleaned_at IS NULL OR deadline>clock_timestamp() ORDER BY created_at,id LIMIT 1001
$$;
REVOKE ALL ON FUNCTION recollect_analytics_expected_entries() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_expected_entries() TO recollect_app;

CREATE FUNCTION recollect_analytics_observed_stale(b uuid,rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF recollect_role(b) IS NULL THEN RAISE insufficient_privilege; END IF;
 UPDATE analytics_reports SET state='stale',scores=NULL,error_code='analytics_inputs_changed'
 WHERE brain_id=b AND id=rid AND state='ready';
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_observed_stale(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_observed_stale(uuid,uuid) TO recollect_app;

-- Ordinary readers cannot take row locks through the writer-only UPDATE RLS
-- policy. This exact, authorized metadata read holds SHARE through commit so
-- another reader cannot invalidate scores after their final state check.
CREATE FUNCTION recollect_analytics_read_final(b uuid,rid uuid) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE result jsonb;
BEGIN
 IF recollect_role(b) IS NULL THEN RAISE insufficient_privilege; END IF;
 SELECT to_jsonb(r)-'descriptor'-'scores' INTO result FROM analytics_reports r
 WHERE r.brain_id=b AND r.id=rid FOR SHARE;
 RETURN result;
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_read_final(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_read_final(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_analytics_maintain() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; reports uuid[];
BEGIN
 FOR b IN SELECT id FROM brains WHERE EXISTS(SELECT 1 FROM analytics_reports r WHERE r.brain_id=brains.id AND r.state IN ('queued','running','ready') AND r.expires_at<=clock_timestamp()) ORDER BY id LIMIT 100 FOR UPDATE SKIP LOCKED LOOP
  SELECT array_agg(id) INTO reports FROM analytics_reports WHERE brain_id=b AND state IN ('queued','running','ready') AND expires_at<=clock_timestamp();
  UPDATE jobs SET state='cancelled',error_code='analytics_inputs_expired',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
  WHERE brain_id=b AND kind='graph.analyze' AND state IN ('queued','running') AND target_id=ANY(reports);
  UPDATE analytics_reports SET state='stale',scores=NULL,error_code='analytics_inputs_expired' WHERE id=ANY(reports);
 END LOOP;
 DELETE FROM analytics_reports WHERE id IN (SELECT r.id FROM analytics_reports r
  WHERE r.state NOT IN ('queued','running') AND r.created_at<clock_timestamp()-interval '7 days'
  AND NOT EXISTS(SELECT 1 FROM analytics_attempts a WHERE a.report_id=r.id AND a.cleaned_at IS NULL) LIMIT 100);
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_maintain() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_analytics_maintain() TO recollect_app;

-- Preserve durable analytical job history while its report/attempt is retained.
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
  AND NOT EXISTS(SELECT 1 FROM graph_generations r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM analytics_reports r WHERE r.job_id=jobs.id) LIMIT 100);
 UPDATE jobs SET state='failed',error_code='lease_expired',lease_token=NULL,lease_until=NULL,updated_at=now()
  WHERE lane=selected_lane AND state='running' AND lease_until<=clock_timestamp() AND attempts>=max_attempts;
 IF (SELECT count(*) FROM jobs WHERE lane=selected_lane AND state='running' AND lease_until>clock_timestamp())>=slot_limit THEN RETURN; END IF;
 RETURN QUERY UPDATE jobs SET state='running',attempts=attempts+1,progress=1,lease_token=token,
  lease_until=clock_timestamp()+interval '20 seconds',error_code=NULL,updated_at=now()
  WHERE id=(SELECT id FROM jobs WHERE lane=selected_lane AND attempts<max_attempts AND
   ((state='queued' AND not_before<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp()))
   ORDER BY created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *;
END $$;
