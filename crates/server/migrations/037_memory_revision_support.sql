-- One immutable input generation per revision, policy and verifier. Model
-- receipts are metadata; candidate/evidence bodies remain canonical elsewhere.
CREATE TABLE memory_support_assessments (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 revision_id uuid NOT NULL,
 policy_id uuid NOT NULL,
 verifier_version text NOT NULL CHECK(verifier_version='source-support-3'),
 actor_id uuid NOT NULL REFERENCES accounts(id),
 job_id uuid,
 source_stage_id uuid,
 request_id uuid,
 retry_of uuid,
 attempt integer NOT NULL DEFAULT 0 CHECK(attempt BETWEEN 0 AND 2),
 local_recoveries integer NOT NULL DEFAULT 0 CHECK(local_recoveries BETWEEN 0 AND 2),
 state text NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','succeeded','failed','removed')),
 disposition text CHECK(disposition IN ('supported','insufficient','contradicted')),
 reason text CHECK(octet_length(reason)<=4000),
 error_code text,
 privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased')),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 finished_at timestamptz,
 UNIQUE(id,brain_id),
 UNIQUE(brain_id,revision_id,policy_id,verifier_version,attempt),
 FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
 FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id),
 FOREIGN KEY(source_stage_id,brain_id) REFERENCES learning_support_stages(run_id,brain_id),
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),
 FOREIGN KEY(retry_of,brain_id) REFERENCES memory_support_assessments(id,brain_id),
 CHECK((state='succeeded')=(disposition IS NOT NULL)),
 CHECK(privacy_state='active' OR (state='removed' AND reason IS NULL AND disposition IS NULL))
);
CREATE INDEX memory_support_pending ON memory_support_assessments(brain_id,state,created_at);
ALTER TABLE memory_support_assessments ENABLE ROW LEVEL SECURITY;
ALTER TABLE memory_support_assessments FORCE ROW LEVEL SECURITY;
CREATE POLICY revision_support_read ON memory_support_assessments FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY revision_support_write ON memory_support_assessments FOR ALL USING(recollect_role(brain_id) IN ('writer','admin')) WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT,UPDATE ON memory_support_assessments TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(1000,'memory_support_assessments');
ALTER TABLE model_request_inputs DROP CONSTRAINT model_request_inputs_kind_check;
ALTER TABLE model_request_inputs ADD CONSTRAINT model_request_inputs_kind_check
 CHECK(kind IN ('source_version','repository_fact','claim_revision','manifest_revision','semantic_entry','learning_support_stage','claim_support_audit'));

-- Cached answers and projections created before the guard are stale at the
-- same atomic migration boundary, even before the first audit is queued.
INSERT INTO memory_epochs(brain_id,epoch) SELECT id,1 FROM brains
ON CONFLICT(brain_id) DO UPDATE SET epoch=memory_epochs.epoch+1;

CREATE FUNCTION recollect_reviewed_revision(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM claim_revisions r JOIN memory_decisions d
 ON d.brain_id=r.brain_id AND d.id=(r.revision->>'review_decision_id')::uuid
 WHERE r.brain_id=b AND r.id=target AND r.revision->>'review'='accepted'
 AND d.decision->>'actor_id'=r.revision->>'reviewer_id'
 AND EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(d.decision->'transitions','[]')) t
   WHERE t->>'after_revision'=r.id::text AND t->>'claim_id'=r.claim_id::text))
$$;
CREATE FUNCTION recollect_selection_valid(b uuid,s jsonb) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT (SELECT count(*) FROM repositories WHERE brain_id=b AND id=ANY(ARRAY(SELECT jsonb_array_elements_text(coalesce(s->'repository_ids','[]'))::uuid)))
    =jsonb_array_length(coalesce(s->'repository_ids','[]'))
 AND (SELECT count(*) FROM evidence_groups WHERE brain_id=b AND kind='area' AND id=ANY(ARRAY(SELECT jsonb_array_elements_text(coalesce(s->'area_ids','[]'))::uuid)))
    =jsonb_array_length(coalesce(s->'area_ids','[]'))
 AND (s->>'environment_id' IS NULL OR EXISTS(SELECT 1 FROM evidence_groups WHERE brain_id=b AND kind='environment' AND id=(s->>'environment_id')::uuid))
$$;
CREATE FUNCTION recollect_validity_range(v jsonb) RETURNS tstzrange
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN v->>'kind'='interval' THEN tstzrange((v->>'from')::timestamptz,(v->>'to')::timestamptz,'[)')
 WHEN v->>'kind'='point' AND v->>'from' IS NOT NULL AND v->>'precision' IN ('second','minute','hour','day') THEN
  (SELECT tstzrange(to_timestamp(start),to_timestamp(start+bucket),'[)') FROM
    (SELECT bucket,floor(extract(epoch FROM (v->>'from')::timestamptz)/bucket)*bucket AS start FROM
      (SELECT CASE v->>'precision' WHEN 'second' THEN 1 WHEN 'minute' THEN 60 WHEN 'hour' THEN 3600 ELSE 86400 END::double precision AS bucket) sizes) bounds)
 ELSE tstzrange(NULL,NULL,'[)') END
$$;
CREATE FUNCTION recollect_rule_blocks(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM claim_revisions r JOIN assertion_rules a
   ON a.brain_id=r.brain_id AND a.subject_key=r.subject_key AND a.predicate_key=r.predicate_key AND a.value_key=r.value_key
   WHERE r.brain_id=b AND r.id=target AND NOT a.payload_erased
   AND NOT EXISTS(SELECT 1 FROM memory_rule_exceptions e WHERE e.brain_id=b AND e.rule_id=a.id AND e.revision_id=r.id)
   AND recollect_recall_scope((a.rule#>'{content,selection}')-'area_ids',(r.revision#>'{content,selection}')-'area_ids')
   AND recollect_validity_range(a.rule#>'{content,validity}') && recollect_validity_range(r.revision#>'{content,validity}'))
$$;
CREATE FUNCTION recollect_memory_dependencies(b uuid,target uuid) RETURNS TABLE(revision_id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE edges(parent,child) AS (
   SELECT revision_id,input_revision_id FROM claim_contributions WHERE brain_id=b
   UNION SELECT d.revision_id,c.current_revision FROM claim_contributions d
     JOIN claim_revisions r ON r.brain_id=d.brain_id AND r.id=d.input_revision_id
     JOIN claims c ON c.brain_id=r.brain_id AND c.id=r.claim_id WHERE d.brain_id=b
 ), dependencies(id) AS (
   SELECT target UNION SELECT e.child FROM edges e JOIN dependencies d ON d.id=e.parent
 ) SELECT id FROM dependencies
$$;
CREATE FUNCTION recollect_memory_exact_dependencies(b uuid,target uuid) RETURNS TABLE(revision_id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE dependencies(id) AS (
   SELECT target UNION SELECT c.input_revision_id FROM claim_contributions c JOIN dependencies d ON d.id=c.revision_id WHERE c.brain_id=b
 ) SELECT id FROM dependencies
$$;
CREATE FUNCTION recollect_memory_exact_acyclic(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT NOT EXISTS(SELECT 1 FROM recollect_memory_exact_dependencies(b,target) d
   JOIN claim_contributions c ON c.brain_id=b AND c.revision_id=d.revision_id
   WHERE EXISTS(SELECT 1 FROM recollect_memory_exact_dependencies(b,c.input_revision_id) back WHERE back.revision_id=d.revision_id))
$$;
CREATE FUNCTION recollect_manifest_supported(b uuid,target uuid,selected uuid[] DEFAULT NULL) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM manifest_revisions m WHERE m.brain_id=b AND m.id=target AND m.privacy_state='active'
   AND (selected IS NULL OR NOT EXISTS(SELECT 1 FROM unnest(selected) wanted
     WHERE (SELECT count(*) FROM jsonb_array_elements(coalesce(m.revision->'entries','[]')) e WHERE (e->>'repository_id')::uuid=wanted)<>1))
   AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(m.revision->'entries','[]')) e
     LEFT JOIN repositories repo ON repo.brain_id=b AND repo.id=(e->>'repository_id')::uuid
     LEFT JOIN repository_snapshots p ON p.brain_id=b AND p.id=(e->>'snapshot_id')::uuid
     WHERE (selected IS NULL OR (e->>'repository_id')::uuid=ANY(selected)) AND
       (repo.id IS NULL OR (e->>'snapshot_id' IS NOT NULL AND
       (p.id IS NULL OR p.repository_id IS DISTINCT FROM repo.id OR p.revision IS DISTINCT FROM e->>'revision'
        OR recollect_content_state(b,'repository',p.privacy_state,p.created_at)<>'active')))))
$$;
CREATE FUNCTION recollect_revision_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM claim_revisions WHERE brain_id=b AND id=target)
 AND NOT EXISTS(SELECT 1 FROM claim_revisions r WHERE r.id=target AND r.brain_id=b
   AND (recollect_content_state(b,'claim',r.privacy_state,r.recorded_at)<>'active'
   OR r.revision->>'review'='rejected' OR coalesce(r.revision->>'lifecycle','active')<>'active'
   OR r.revision#>>'{content,freshness}'='superseded'
   OR NOT recollect_selection_valid(b,r.revision#>'{content,selection}')
   OR recollect_rule_blocks(b,r.id)
   OR EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=b AND f.revision_id=r.id)
   OR (r.revision#>>'{content,manifest_revision_id}' IS NOT NULL AND NOT recollect_manifest_supported(b,(r.revision#>>'{content,manifest_revision_id}')::uuid,
       ARRAY(SELECT jsonb_array_elements_text(r.revision#>'{content,selection,repository_ids}')::uuid)))
   OR (r.revision#>>'{content,manifest_revision_id}' IS NOT NULL AND NOT EXISTS(SELECT 1 FROM manifest_revisions m
     WHERE m.brain_id=b AND m.id=(r.revision#>>'{content,manifest_revision_id}')::uuid
       AND m.revision->>'environment_id' IS NOT DISTINCT FROM r.revision#>>'{content,selection,environment_id}'))
   OR NOT (recollect_reviewed_revision(b,r.id) OR EXISTS(
     SELECT 1 FROM memory_support_assessments a JOIN model_policy_heads p ON p.brain_id=a.brain_id AND p.policy_id=a.policy_id
     WHERE a.brain_id=b AND a.revision_id=r.id AND a.verifier_version='source-support-3'
       AND a.state='succeeded' AND a.disposition='supported' AND a.privacy_state='active'))
   OR EXISTS(SELECT 1 FROM claim_supports s LEFT JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=b
     LEFT JOIN repository_facts f ON f.id=s.fact_id AND f.brain_id=b
     LEFT JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=b
     LEFT JOIN manifest_revisions m ON m.id=s.manifest_revision_id AND m.brain_id=b
     WHERE s.brain_id=b AND s.revision_id=r.id AND (
       (s.source_version_id IS NOT NULL AND (v.id IS NULL OR (v.artifact_id IS NULL AND NOT recollect_reviewed_revision(b,r.id))
        OR (recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)<>'active'
          AND NOT (recollect_reviewed_revision(b,r.id)
            AND recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)='expired'))
        OR EXISTS(SELECT 1 FROM model_input_fences x WHERE x.brain_id=b AND x.source_version_id=v.id)))
       OR (s.fact_id IS NOT NULL AND (p.id IS NULL
         OR (recollect_content_state(b,'repository',p.privacy_state,p.created_at)<>'active'
           AND NOT (recollect_reviewed_revision(b,r.id)
             AND recollect_content_state(b,'repository',p.privacy_state,p.created_at)='expired'))))
       OR (s.manifest_revision_id IS NOT NULL AND NOT recollect_manifest_supported(b,s.manifest_revision_id))))))
$$;
CREATE FUNCTION recollect_memory_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_revision_supported(b,target) AND NOT EXISTS(
   SELECT 1 FROM recollect_memory_dependencies(b,target) d
   LEFT JOIN claim_revisions r ON r.brain_id=b AND r.id=d.revision_id
   WHERE NOT recollect_revision_supported(b,d.revision_id) OR NOT recollect_memory_exact_acyclic(b,d.revision_id)
     OR (d.revision_id<>target AND r.revision#>>'{content,kind}'='handover'))
$$;
CREATE FUNCTION recollect_memory_deadline(b uuid,target uuid) RETURNS timestamptz
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH dependencies(id) AS (SELECT revision_id FROM recollect_memory_dependencies(b,target)),
 manifests(id,selected) AS (
   SELECT (r.revision#>>'{content,manifest_revision_id}')::uuid,ARRAY(SELECT jsonb_array_elements_text(r.revision#>'{content,selection,repository_ids}')::uuid) FROM claim_revisions r JOIN dependencies d ON d.id=r.id WHERE r.brain_id=b
   UNION SELECT s.manifest_revision_id,NULL::uuid[] FROM claim_supports s JOIN dependencies d ON d.id=s.revision_id WHERE s.brain_id=b
 ), deadlines(deadline) AS (
   SELECT recollect_retention_deadline(r.brain_id,'claim',r.recorded_at) FROM claim_revisions r JOIN dependencies d ON d.id=r.id WHERE r.brain_id=b
   UNION ALL SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) FROM claim_supports s JOIN dependencies d ON d.id=s.revision_id JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=b WHERE s.brain_id=b
     AND (NOT recollect_reviewed_revision(b,s.revision_id) OR recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)='active')
   UNION ALL SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at) FROM claim_supports s JOIN dependencies d ON d.id=s.revision_id JOIN repository_facts f ON f.id=s.fact_id AND f.brain_id=b JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=b WHERE s.brain_id=b
     AND (NOT recollect_reviewed_revision(b,s.revision_id) OR recollect_content_state(b,'repository',p.privacy_state,p.created_at)='active')
   UNION ALL SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at) FROM manifest_revisions m JOIN manifests x ON x.id=m.id
     CROSS JOIN LATERAL jsonb_array_elements(coalesce(m.revision->'entries','[]')) e
     JOIN repository_snapshots p ON p.id=(e->>'snapshot_id')::uuid AND p.brain_id=b WHERE m.brain_id=b
       AND (x.selected IS NULL OR (e->>'repository_id')::uuid=ANY(x.selected))
 ) SELECT min(deadline) FROM deadlines
$$;
CREATE FUNCTION recollect_support_audit_target(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM claims c JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.current_revision
   WHERE c.brain_id=b AND recollect_content_state(b,'claim',r.privacy_state,r.recorded_at)='active'
     AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
     AND EXISTS(SELECT 1 FROM recollect_memory_dependencies(b,r.id) d WHERE d.revision_id=target))
$$;
REVOKE ALL ON FUNCTION recollect_manifest_supported(uuid,uuid,uuid[]),recollect_revision_supported(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_manifest_supported(uuid,uuid,uuid[]),recollect_revision_supported(uuid,uuid) TO recollect_app;
REVOKE ALL ON FUNCTION recollect_memory_exact_dependencies(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_memory_exact_dependencies(uuid,uuid) TO recollect_app;
REVOKE ALL ON FUNCTION recollect_memory_exact_acyclic(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_memory_exact_acyclic(uuid,uuid) TO recollect_app;
CREATE FUNCTION recollect_contribution_acyclic() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP='UPDATE' OR NEW.revision_id=NEW.input_revision_id OR EXISTS(
   SELECT 1 FROM recollect_memory_exact_dependencies(NEW.brain_id,NEW.input_revision_id) d WHERE d.revision_id=NEW.revision_id) THEN
   RAISE EXCEPTION 'immutable acyclic contribution lineage';
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_contribution_acyclic() FROM PUBLIC;
CREATE TRIGGER contribution_acyclic BEFORE INSERT OR UPDATE ON claim_contributions FOR EACH ROW EXECUTE FUNCTION recollect_contribution_acyclic();
REVOKE ALL ON FUNCTION recollect_reviewed_revision(uuid,uuid),recollect_memory_supported(uuid,uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION recollect_selection_valid(uuid,jsonb),recollect_validity_range(jsonb),recollect_rule_blocks(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_reviewed_revision(uuid,uuid),recollect_memory_supported(uuid,uuid),recollect_selection_valid(uuid,jsonb),recollect_validity_range(jsonb),recollect_rule_blocks(uuid,uuid) TO recollect_app;
REVOKE ALL ON FUNCTION recollect_memory_dependencies(uuid,uuid),recollect_memory_deadline(uuid,uuid),recollect_support_audit_target(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_memory_dependencies(uuid,uuid),recollect_memory_deadline(uuid,uuid),recollect_support_audit_target(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_revision_support_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP='UPDATE' THEN
   IF (NEW.id,NEW.brain_id,NEW.revision_id,NEW.policy_id,NEW.verifier_version,NEW.actor_id,NEW.source_stage_id,NEW.retry_of,NEW.attempt,NEW.created_at)
      IS DISTINCT FROM (OLD.id,OLD.brain_id,OLD.revision_id,OLD.policy_id,OLD.verifier_version,OLD.actor_id,OLD.source_stage_id,OLD.retry_of,OLD.attempt,OLD.created_at) THEN
     RAISE EXCEPTION 'immutable revision support identity';
   END IF;
   IF NEW.privacy_state='active' AND (OLD.privacy_state<>'active' OR (OLD.disposition IS NOT NULL AND
     (NEW.disposition,NEW.reason,NEW.request_id,NEW.finished_at) IS DISTINCT FROM (OLD.disposition,OLD.reason,OLD.request_id,OLD.finished_at))) THEN
     RAISE EXCEPTION 'immutable revision support verdict';
   END IF;
   IF OLD.privacy_state='erased' AND NEW.privacy_state<>'erased' THEN RAISE EXCEPTION 'erased support cannot return'; END IF;
   IF NEW.local_recoveries<OLD.local_recoveries OR NEW.local_recoveries>OLD.local_recoveries+1 THEN
     RAISE EXCEPTION 'monotonic bounded support recovery';
   END IF;
 END IF;
 INSERT INTO memory_epochs(brain_id,epoch) VALUES(NEW.brain_id,1)
 ON CONFLICT(brain_id) DO UPDATE SET epoch=memory_epochs.epoch+1;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_revision_support_changed() FROM PUBLIC;
CREATE TRIGGER revision_support_changed BEFORE INSERT OR UPDATE ON memory_support_assessments FOR EACH ROW EXECUTE FUNCTION recollect_revision_support_changed();

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_revision_support_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; ids uuid[];
BEGIN
 m:=recollect_pre_revision_support_closure(b,k,t,c);
 WITH RECURSIVE affected(id) AS (
   SELECT unnest(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions'))
   UNION SELECT s.revision_id FROM claim_supports s WHERE s.brain_id=b AND (
     s.source_version_id=ANY(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources'))
     OR s.fact_id=ANY(recollect_privacy_ids(m,'facts')) OR s.manifest_revision_id=ANY(recollect_privacy_ids(m,'manifest_revisions')))
   UNION SELECT c.revision_id FROM claim_contributions c JOIN affected a ON a.id=c.input_revision_id WHERE c.brain_id=b
 ) SELECT coalesce(array_agg(a.id),'{}') INTO ids FROM memory_support_assessments a WHERE a.brain_id=b AND a.revision_id IN(SELECT id FROM affected);
 RETURN m||jsonb_build_object('memory_support_assessments',ids);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_revision_support_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; ids uuid[];
BEGIN
 PERFORM recollect_pre_revision_support_apply(rid);
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 ids:=recollect_privacy_ids(r.manifest,'memory_support_assessments');
 UPDATE memory_support_assessments SET state='removed',disposition=NULL,reason=NULL,
   privacy_state=CASE WHEN privacy_state='erased' OR r.cause='erase' THEN 'erased' ELSE 'expired' END
 WHERE brain_id=r.brain_id AND id=ANY(ids);
 UPDATE model_requests SET suppressed=true WHERE brain_id=r.brain_id AND id IN(
   SELECT request_id FROM memory_support_assessments WHERE brain_id=r.brain_id AND id=ANY(ids));
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=r.brain_id AND kind='claim.support' AND target_id=ANY(ids) AND state IN ('queued','running');
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_revision_support_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_revision_support_normalize(m)||jsonb_build_object('memory_support_assessments',to_jsonb(recollect_privacy_ids(m,'memory_support_assessments')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;

CREATE FUNCTION recollect_defer_support_job(jid uuid,token uuid,reason text) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH changed AS (UPDATE jobs SET state='queued',error_code=reason,attempts=greatest(attempts-1,0),
 lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp(),
 not_before=CASE WHEN reason='model_budget_exhausted' THEN (date_trunc('day',clock_timestamp() AT TIME ZONE 'UTC')+interval '1 day') AT TIME ZONE 'UTC' ELSE clock_timestamp()+interval '10 seconds' END
 WHERE id=jid AND kind='claim.support' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
 AND reason IN ('model_budget_exhausted','model_concurrency_full') RETURNING id) SELECT EXISTS(SELECT 1 FROM changed)
$$;
REVOKE ALL ON FUNCTION recollect_defer_support_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_defer_support_job(uuid,uuid,text) TO recollect_app;

-- Worker exhaustion and lease expiry must not leave phantom pending audits.
-- A committed verdict remains usable even if a later job-status write failed.
CREATE FUNCTION recollect_support_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.kind='claim.support' AND NEW.state IN ('failed','cancelled') THEN
   UPDATE memory_support_assessments SET state='failed',error_code=coalesce(NEW.error_code,'job_failed'),finished_at=clock_timestamp()
   WHERE brain_id=NEW.brain_id AND id=NEW.target_id AND state IN ('queued','running') AND privacy_state='active';
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_support_job_state() FROM PUBLIC;
CREATE TRIGGER support_job_state AFTER UPDATE OF state ON jobs FOR EACH ROW EXECUTE FUNCTION recollect_support_job_state();

-- Keep assessment-owned bookkeeping through generic seven-day cleanup.
-- Other work retains its existing cleanup and bounded claiming behavior.
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
 DELETE FROM command_receipts WHERE (actor_id,key) IN (SELECT actor_id,key FROM command_receipts WHERE expires_at<=clock_timestamp() LIMIT 100);
 DELETE FROM jobs WHERE id IN (SELECT id FROM jobs WHERE state IN ('succeeded','failed','cancelled') AND updated_at<clock_timestamp()-interval '7 days'
  AND NOT EXISTS(SELECT 1 FROM repository_jobs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM learning_runs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM handover_runs r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM semantic_batches r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM graph_generations r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM analytics_reports r WHERE r.job_id=jobs.id)
  AND NOT EXISTS(SELECT 1 FROM memory_support_assessments r WHERE r.job_id=jobs.id) LIMIT 100);
 UPDATE jobs SET state='failed',error_code='lease_expired',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
  WHERE lane=selected_lane AND state='running' AND lease_until<=clock_timestamp() AND attempts>=max_attempts;
 IF (SELECT count(*) FROM jobs WHERE lane=selected_lane AND state='running' AND lease_until>clock_timestamp())>=slot_limit THEN RETURN; END IF;
 RETURN QUERY UPDATE jobs SET state='running',attempts=attempts+1,progress=1,lease_token=token,
  lease_until=clock_timestamp()+interval '20 seconds',error_code=NULL,updated_at=clock_timestamp()
  WHERE id=(SELECT id FROM jobs WHERE lane=selected_lane AND attempts<max_attempts AND
   ((state='queued' AND not_before<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp()))
   ORDER BY created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *;
END $$;
