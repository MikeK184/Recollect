ALTER TABLE brains ADD COLUMN change_id uuid NOT NULL DEFAULT gen_random_uuid();
CREATE TABLE command_receipts (
    actor_id uuid NOT NULL REFERENCES accounts(id), key text NOT NULL,
    operation text NOT NULL, input jsonb NOT NULL,
    brain_id uuid REFERENCES brains(id), response jsonb,
    expires_at timestamptz NOT NULL DEFAULT (now() + interval '24 hours'),
    PRIMARY KEY(actor_id, key)
);
ALTER TABLE command_receipts ENABLE ROW LEVEL SECURITY;
CREATE POLICY receipt_scope ON command_receipts FOR ALL
    USING (actor_id = recollect_actor() AND (brain_id IS NULL OR recollect_role(brain_id) IS NOT NULL))
    WITH CHECK (actor_id = recollect_actor() AND (brain_id IS NULL OR recollect_role(brain_id) IS NOT NULL));
GRANT SELECT, INSERT, UPDATE, DELETE ON command_receipts TO recollect_app;

CREATE FUNCTION recollect_forget_expired_receipt(selected_key text) RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path = public, pg_temp AS $$
    DELETE FROM command_receipts WHERE actor_id=recollect_actor() AND key=selected_key AND expires_at<=now()
$$;
REVOKE ALL ON FUNCTION recollect_forget_expired_receipt(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_forget_expired_receipt(text) TO recollect_app;

CREATE TABLE jobs (
    id uuid PRIMARY KEY, brain_id uuid NOT NULL REFERENCES brains(id),
    actor_id uuid NOT NULL REFERENCES accounts(id),
    audit_id uuid NOT NULL REFERENCES mutation_audit(id), target_id uuid NOT NULL,
    kind text NOT NULL,
    lane text NOT NULL CHECK (lane IN ('interactive','capture','model','heavy')),
    state text NOT NULL DEFAULT 'queued' CHECK (state IN ('queued','running','succeeded','failed','cancelled')),
    progress smallint NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
    attempts integer NOT NULL DEFAULT 0, max_attempts integer NOT NULL DEFAULT 3,
    not_before timestamptz NOT NULL DEFAULT now(), lease_token uuid,
    lease_until timestamptz, error_code text,
    created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX jobs_claim ON jobs(lane, state, not_before, created_at);
CREATE INDEX jobs_brain ON jobs(brain_id, created_at DESC);
ALTER TABLE jobs ENABLE ROW LEVEL SECURITY;
CREATE POLICY job_read ON jobs FOR SELECT USING (recollect_role(brain_id) IS NOT NULL);
CREATE POLICY job_create ON jobs FOR INSERT WITH CHECK (actor_id = recollect_actor() AND recollect_role(brain_id) IN ('writer','admin'));
CREATE POLICY job_update ON jobs FOR UPDATE USING (recollect_role(brain_id) IN ('writer','admin'))
    WITH CHECK (recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT, INSERT, UPDATE ON jobs TO recollect_app;

CREATE TABLE brain_directory (
    brain_id uuid PRIMARY KEY REFERENCES brains(id), name text NOT NULL,
    description text NOT NULL, archived boolean NOT NULL, source_change uuid NOT NULL,
    refreshed_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE brain_directory ENABLE ROW LEVEL SECURITY;
CREATE POLICY directory_read ON brain_directory FOR SELECT USING (recollect_role(brain_id) IS NOT NULL);
CREATE POLICY directory_create ON brain_directory FOR INSERT WITH CHECK (recollect_role(brain_id) IN ('writer','admin'));
CREATE POLICY directory_update ON brain_directory FOR UPDATE USING (recollect_role(brain_id) IN ('writer','admin'))
    WITH CHECK (recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT, INSERT, UPDATE ON brain_directory TO recollect_app;

-- Only the bounded claim function enumerates cross-Brain work. A consumer must
-- rebind to the persisted actor and validate access before reading source data.
CREATE FUNCTION recollect_claim_job(selected_lane text, token uuid) RETURNS SETOF jobs
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
    DELETE FROM jobs WHERE id IN (SELECT id FROM jobs WHERE state IN ('succeeded','failed','cancelled') AND updated_at < now() - interval '7 days' LIMIT 100);
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
CREATE FUNCTION recollect_renew_job(job_id uuid, token uuid) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path = public, pg_temp AS $$
    WITH renewed AS (
        UPDATE jobs SET lease_until=clock_timestamp()+interval '20 seconds'
        WHERE id=job_id AND state='running' AND lease_token=token AND lease_until>clock_timestamp() RETURNING id
    ) SELECT EXISTS(SELECT 1 FROM renewed)
$$;
CREATE FUNCTION recollect_fail_job(job_id uuid, token uuid, outcome text, reason text) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path = public, pg_temp AS $$
    WITH changed AS (
        UPDATE jobs SET state=CASE WHEN outcome='queued' AND attempts>=max_attempts THEN 'failed' ELSE outcome END,
            error_code=reason, progress=0, lease_token=NULL, lease_until=NULL,
            not_before=clock_timestamp()+make_interval(secs=>least(attempts,2)), updated_at=now()
        WHERE id=job_id AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
          AND outcome IN ('queued','failed','cancelled')
          AND reason IN ('database_error','permission_revoked','unsupported_kind','input_missing')
        RETURNING id
    ) SELECT EXISTS(SELECT 1 FROM changed)
$$;
REVOKE ALL ON FUNCTION recollect_claim_job(text,uuid), recollect_renew_job(uuid,uuid), recollect_fail_job(uuid,uuid,text,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_claim_job(text,uuid), recollect_renew_job(uuid,uuid), recollect_fail_job(uuid,uuid,text,text) TO recollect_app;

INSERT INTO mutation_audit (id,brain_id,actor_id,action,target_id,disposition)
SELECT gen_random_uuid(),id,owner_id,'brain.projection_requested',id,'migration_backfill' FROM brains;
INSERT INTO jobs (id,brain_id,actor_id,audit_id,target_id,kind,lane)
SELECT gen_random_uuid(),brain_id,actor_id,id,target_id,'brain.refresh','interactive'
FROM mutation_audit WHERE action='brain.projection_requested' AND disposition='migration_backfill';
