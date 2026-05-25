-- Canonical inputs and profile identity own compatibility; vectors are a
-- disposable projection. The first representation preserves all 3072 components.
CREATE TABLE semantic_profiles (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    provider text NOT NULL,
    model text NOT NULL,
    dimensions integer NOT NULL CHECK(dimensions=3072),
    representation text NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    policy_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id),
    FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id)
);
CREATE TABLE semantic_heads (
    brain_id uuid PRIMARY KEY REFERENCES brains(id),
    profile_id uuid NOT NULL,
    FOREIGN KEY(profile_id,brain_id) REFERENCES semantic_profiles(id,brain_id)
);
CREATE TABLE semantic_batches (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    profile_id uuid NOT NULL,
    actor_id uuid NOT NULL REFERENCES accounts(id),
    policy_id uuid NOT NULL,
    job_id uuid NOT NULL REFERENCES jobs(id),
    request_id uuid,
    state text NOT NULL CHECK(state IN ('queued','running','succeeded','blocked','failed','removed')),
    input_count integer NOT NULL CHECK(input_count BETWEEN 1 AND 20),
    error_code text,
    retry_of uuid UNIQUE,
    automatic_attempt integer NOT NULL DEFAULT 0 CHECK(automatic_attempt BETWEEN 0 AND 2),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    finished_at timestamptz,
    UNIQUE(id,brain_id),
    FOREIGN KEY(profile_id,brain_id) REFERENCES semantic_profiles(id,brain_id),
    FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),
    FOREIGN KEY(retry_of,brain_id) REFERENCES semantic_batches(id,brain_id)
);
ALTER TABLE source_chunks ADD CONSTRAINT semantic_chunk_identity UNIQUE(id,version_id,brain_id);
CREATE TABLE semantic_entries (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    profile_id uuid NOT NULL,
    kind text NOT NULL CHECK(kind IN ('source_chunk','claim_revision','repository_fact','manifest_revision')),
    input_id uuid NOT NULL,
    chunk_id uuid,
    source_version_id uuid,
    claim_revision_id uuid,
    fact_id uuid,
    manifest_revision_id uuid,
    state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','queued','running','ready','blocked','failed','removed')),
    batch_id uuid,
    request_id uuid,
    error_code text,
    truncated boolean NOT NULL DEFAULT false,
    embedding vector(3072),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id),
    UNIQUE(profile_id,kind,input_id),
    CHECK(num_nonnulls(source_version_id,claim_revision_id,fact_id,manifest_revision_id)=1),
    CHECK(coalesce((kind='source_chunk' AND chunk_id IS NOT NULL AND chunk_id=input_id AND source_version_id IS NOT NULL)
       OR (kind='source_chunk' AND state='removed' AND chunk_id IS NULL AND source_version_id IS NOT NULL)
       OR (kind='claim_revision' AND claim_revision_id=input_id AND chunk_id IS NULL)
       OR (kind='repository_fact' AND fact_id=input_id AND chunk_id IS NULL)
       OR (kind='manifest_revision' AND manifest_revision_id=input_id AND chunk_id IS NULL),false)),
    CHECK((state='ready')=(embedding IS NOT NULL)),
    CHECK(embedding IS NULL OR vector_norm(embedding)>0),
    FOREIGN KEY(profile_id,brain_id) REFERENCES semantic_profiles(id,brain_id),
    FOREIGN KEY(chunk_id,source_version_id,brain_id) REFERENCES source_chunks(id,version_id,brain_id) ON DELETE SET NULL (chunk_id),
    FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id),
    FOREIGN KEY(claim_revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
    FOREIGN KEY(fact_id,brain_id) REFERENCES repository_facts(id,brain_id),
    FOREIGN KEY(manifest_revision_id,brain_id) REFERENCES manifest_revisions(id,brain_id),
    FOREIGN KEY(batch_id,brain_id) REFERENCES semantic_batches(id,brain_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
CREATE TABLE semantic_batch_inputs (
    brain_id uuid NOT NULL,
    batch_id uuid NOT NULL,
    entry_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK(ordinal BETWEEN 0 AND 19),
    PRIMARY KEY(batch_id,entry_id),
    UNIQUE(batch_id,ordinal),
    FOREIGN KEY(batch_id,brain_id) REFERENCES semantic_batches(id,brain_id),
    FOREIGN KEY(entry_id,brain_id) REFERENCES semantic_entries(id,brain_id)
);
CREATE INDEX semantic_entry_pending ON semantic_entries(brain_id,profile_id,state,created_at,id);
CREATE INDEX semantic_entry_source ON semantic_entries(brain_id,source_version_id) WHERE source_version_id IS NOT NULL;
CREATE INDEX semantic_entry_claim ON semantic_entries(brain_id,claim_revision_id) WHERE claim_revision_id IS NOT NULL;
CREATE INDEX semantic_entry_fact ON semantic_entries(brain_id,fact_id) WHERE fact_id IS NOT NULL;
CREATE INDEX semantic_entry_manifest ON semantic_entries(brain_id,manifest_revision_id) WHERE manifest_revision_id IS NOT NULL;
CREATE INDEX semantic_batch_recent ON semantic_batches(brain_id,created_at DESC,id DESC);

DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['semantic_profiles','semantic_heads','semantic_batches','semantic_entries','semantic_batch_inputs'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
    -- Uncorrelated Brain sets preserve current statement/RLS authority without
    -- repeating account/group/device resolution for every vector or chunk.
    EXECUTE format('CREATE POLICY read_rows ON %I FOR SELECT USING(brain_id IN (SELECT id FROM brains))',tab);
    EXECUTE format('CREATE POLICY write_rows ON %I FOR ALL USING(brain_id IN (SELECT b.id FROM brains b WHERE recollect_role(b.id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN (SELECT b.id FROM brains b WHERE recollect_role(b.id) IN (''writer'',''admin'')))',tab);
    EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
  END LOOP;
END $$;

ALTER TABLE model_request_inputs DROP CONSTRAINT model_request_inputs_kind_check;
ALTER TABLE model_request_inputs ADD CONSTRAINT model_request_inputs_kind_check
  CHECK(kind IN ('source_version','repository_fact','claim_revision','manifest_revision','semantic_entry'));

-- Privacy replay already changes the canonical privacy state. The same trigger
-- clears both live vectors and restored older projections and cancels publication.
CREATE FUNCTION recollect_semantic_input_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE removed_brain uuid; removed_id uuid; affected uuid[];
BEGIN
  IF TG_OP='DELETE' THEN
    removed_brain:=OLD.brain_id; removed_id:=OLD.id;
  ELSE
    IF NEW.privacy_state='active' THEN RETURN NEW; END IF;
    removed_brain:=NEW.brain_id; removed_id:=NEW.id;
  END IF;
  WITH cleared AS (
    UPDATE semantic_entries SET state='removed',embedding=NULL,error_code='content_removed',updated_at=clock_timestamp()
    WHERE brain_id=removed_brain AND
      ((TG_TABLE_NAME='source_versions' AND source_version_id=removed_id)
       OR (TG_TABLE_NAME='source_chunks' AND chunk_id=removed_id)
       OR (TG_TABLE_NAME='claim_revisions' AND claim_revision_id=removed_id)
       OR (TG_TABLE_NAME='manifest_revisions' AND manifest_revision_id=removed_id)
       OR (TG_TABLE_NAME='repository_snapshots' AND fact_id IN
         (SELECT id FROM repository_facts WHERE brain_id=removed_brain AND snapshot_id=removed_id)))
    RETURNING id
  ) SELECT coalesce(array_agg(id),'{}') INTO affected FROM cleared;
  UPDATE model_requests SET suppressed=true WHERE brain_id=removed_brain AND id IN
    (SELECT request_id FROM model_request_inputs WHERE brain_id=removed_brain
      AND kind='semantic_entry' AND input_id=ANY(affected));
  UPDATE semantic_batches SET state='removed',error_code='content_removed',finished_at=clock_timestamp()
    WHERE brain_id=removed_brain AND state IN ('queued','running','blocked') AND id IN
      (SELECT batch_id FROM semantic_batch_inputs WHERE brain_id=removed_brain AND entry_id=ANY(affected));
  UPDATE semantic_entries SET state='blocked',embedding=NULL,error_code='batch_input_removed',updated_at=clock_timestamp()
    WHERE brain_id=removed_brain AND state IN ('queued','running') AND batch_id IN
      (SELECT id FROM semantic_batches WHERE brain_id=removed_brain AND state='removed');
  UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE brain_id=removed_brain AND state IN ('queued','running') AND id IN
      (SELECT job_id FROM semantic_batches WHERE brain_id=removed_brain AND state='removed');
  IF TG_OP='DELETE' THEN RETURN OLD; END IF;
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_semantic_input_removed() FROM PUBLIC;
CREATE TRIGGER source_semantic_removed AFTER UPDATE OF privacy_state ON source_versions
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_input_removed();
CREATE TRIGGER chunk_semantic_removed BEFORE DELETE ON source_chunks
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_input_removed();
CREATE TRIGGER claim_semantic_removed AFTER UPDATE OF privacy_state ON claim_revisions
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_input_removed();
CREATE TRIGGER snapshot_semantic_removed AFTER UPDATE OF privacy_state ON repository_snapshots
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_input_removed();
CREATE TRIGGER manifest_semantic_removed AFTER UPDATE OF privacy_state ON manifest_revisions
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_input_removed();

-- A model input fence can remove a derivative while leaving its independently
-- retained source intact. Apply the same projection disposition on journal replay.
CREATE FUNCTION recollect_semantic_fenced() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE affected uuid[];
BEGIN
  WITH cleared AS (
    UPDATE semantic_entries SET state='removed',embedding=NULL,error_code='model_input_fenced',updated_at=clock_timestamp()
      WHERE brain_id=NEW.brain_id AND
        ((TG_TABLE_NAME='model_input_fences' AND source_version_id=(to_jsonb(NEW)->>'source_version_id')::uuid)
        OR (TG_TABLE_NAME='model_claim_fences' AND claim_revision_id=(to_jsonb(NEW)->>'revision_id')::uuid))
      RETURNING id
  ) SELECT coalesce(array_agg(id),'{}') INTO affected FROM cleared;
  UPDATE model_requests SET suppressed=true WHERE brain_id=NEW.brain_id AND id IN
    (SELECT request_id FROM model_request_inputs WHERE brain_id=NEW.brain_id AND kind='semantic_entry' AND input_id=ANY(affected));
  UPDATE semantic_batches SET state='removed',error_code='model_input_fenced',finished_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND state IN ('queued','running','blocked') AND id IN
      (SELECT batch_id FROM semantic_batch_inputs WHERE brain_id=NEW.brain_id AND entry_id=ANY(affected));
  UPDATE jobs SET state='cancelled',error_code='model_input_fenced',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND state IN ('queued','running') AND id IN
      (SELECT job_id FROM semantic_batches WHERE brain_id=NEW.brain_id AND state='removed');
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_semantic_fenced() FROM PUBLIC;
CREATE TRIGGER semantic_source_fenced AFTER INSERT ON model_input_fences
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_fenced();
CREATE TRIGGER semantic_claim_fenced AFTER INSERT ON model_claim_fences
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_fenced();

CREATE FUNCTION recollect_semantic_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE recorded uuid; outcome text;
BEGIN
  IF NEW.kind<>'semantic.generate' OR NEW.state NOT IN ('cancelled','failed') THEN RETURN NEW; END IF;
  SELECT r.id INTO recorded FROM model_requests r JOIN semantic_batches b ON b.id=NEW.target_id
    WHERE r.brain_id=b.brain_id AND r.actor_id=b.actor_id AND r.operation_id=b.id AND r.purpose='embedding';
  outcome:=CASE WHEN recorded IS NULL AND NEW.error_code IN
    ('model_budget_exhausted','model_concurrency_full','model_policy_denied',
     'model_policy_changed','permission_or_policy_denied','semantic_input_too_large','model_input_unavailable')
    THEN 'blocked' ELSE 'failed' END;
  UPDATE semantic_batches SET state=outcome,error_code=NEW.error_code,request_id=recorded,finished_at=clock_timestamp()
    WHERE id=NEW.target_id AND state IN ('queued','running');
  UPDATE semantic_entries SET state=outcome,embedding=NULL,error_code=NEW.error_code,request_id=recorded,updated_at=clock_timestamp()
    WHERE brain_id=NEW.brain_id AND batch_id=NEW.target_id AND state IN ('queued','running');
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_semantic_job_state() FROM PUBLIC;
CREATE TRIGGER semantic_job_state AFTER UPDATE OF state ON jobs
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_job_state();

CREATE FUNCTION recollect_fail_semantic_job(jid uuid,token uuid,reason text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF length(reason)>80 OR reason !~ '^[a-z_]+$' THEN RAISE EXCEPTION 'invalid semantic failure'; END IF;
  UPDATE jobs SET state='failed',error_code=reason,progress=0,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE id=jid AND kind='semantic.generate' AND state='running' AND lease_token=token AND lease_until>clock_timestamp();
END $$;
REVOKE ALL ON FUNCTION recollect_fail_semantic_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_fail_semantic_job(uuid,uuid,text) TO recollect_app;

CREATE FUNCTION recollect_semantic_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT b.id,p.created_by FROM brains b
    JOIN model_policy_heads h ON h.brain_id=b.id
    JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id
    JOIN accounts a ON a.id=p.created_by
    WHERE NOT b.archived AND a.enabled AND
      (p.policy->>'automatic_embedding'='true' OR EXISTS(SELECT 1 FROM semantic_heads s WHERE s.brain_id=b.id))
    ORDER BY b.id
$$;
REVOKE ALL ON FUNCTION recollect_semantic_brains() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_semantic_brains() TO recollect_app;

-- Retained semantic batches own their job history just as learning/handover do.
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
    AND NOT EXISTS(SELECT 1 FROM semantic_batches r WHERE r.job_id=jobs.id) LIMIT 100);
  UPDATE jobs SET state='failed',error_code='lease_expired',lease_token=NULL,lease_until=NULL,updated_at=now()
    WHERE lane=selected_lane AND state='running' AND lease_until<=clock_timestamp() AND attempts>=max_attempts;
  IF (SELECT count(*) FROM jobs WHERE lane=selected_lane AND state='running' AND lease_until>clock_timestamp())>=slot_limit THEN RETURN; END IF;
  RETURN QUERY UPDATE jobs SET state='running',attempts=attempts+1,progress=1,lease_token=token,
    lease_until=clock_timestamp()+interval '20 seconds',error_code=NULL,updated_at=now()
    WHERE id=(SELECT id FROM jobs WHERE lane=selected_lane AND attempts<max_attempts AND
      ((state='queued' AND not_before<=clock_timestamp()) OR (state='running' AND lease_until<=clock_timestamp()))
      ORDER BY created_at,id LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *;
END $$;
