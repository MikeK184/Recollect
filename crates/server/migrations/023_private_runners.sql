CREATE TABLE mcp_private_runners (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    device_id uuid NOT NULL REFERENCES devices(id),
    name text NOT NULL CHECK(length(name) BETWEEN 1 AND 120),
    enabled boolean NOT NULL DEFAULT true,
    revision uuid NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(brain_id,device_id)
);
CREATE UNIQUE INDEX mcp_private_name ON mcp_private_runners(brain_id,lower(name));
ALTER TABLE mcp_private_runners ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_private_read ON mcp_private_runners FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY mcp_private_insert ON mcp_private_runners FOR INSERT WITH CHECK(
    recollect_role(brain_id)='admin' AND created_by=recollect_actor() AND EXISTS(
      SELECT 1 FROM devices d WHERE d.id=device_id AND d.account_id=recollect_actor()
        AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now()));
CREATE POLICY mcp_private_update ON mcp_private_runners FOR UPDATE USING(recollect_role(brain_id)='admin')
    WITH CHECK(recollect_role(brain_id)='admin');
GRANT SELECT,INSERT ON mcp_private_runners TO recollect_app;
GRANT UPDATE(name,enabled,revision,updated_at) ON mcp_private_runners TO recollect_app;

-- Runner storage stays private to the coordinator. Readers can observe only
-- the last lease of a registration in a Brain they can currently read.
CREATE FUNCTION recollect_mcp_private_lease(registration uuid) RETURNS timestamptz
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT r.lease_until FROM mcp_private_runners p JOIN mcp_runners r ON r.reference='private:'||p.id::text
      WHERE p.id=registration AND recollect_role(p.brain_id) IS NOT NULL
$$;
REVOKE ALL ON FUNCTION recollect_mcp_private_lease(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_private_lease(uuid) TO recollect_app;

CREATE FUNCTION recollect_mcp_private_eligible(brain uuid,ref text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT EXISTS(SELECT 1 FROM mcp_private_runners p JOIN devices d ON d.id=p.device_id
      JOIN accounts a ON a.id=d.account_id JOIN brains b ON b.id=p.brain_id
      WHERE p.brain_id=brain AND ref='private:'||p.id::text AND p.enabled AND NOT b.archived
        AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now() AND a.enabled
        AND (a.auth_kind='local' OR a.membership_until>now())
        AND recollect_effective_role(brain,a.id)='admin')
$$;
-- Execution uses the caller's authority transaction while locking the separate
-- runner host identity. No device credential value crosses this function.
CREATE FUNCTION recollect_mcp_private_lock(ref text,runner_device uuid,runner_actor uuid) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE brain uuid;
BEGIN
    SELECT p.brain_id INTO brain FROM mcp_private_runners p JOIN devices d ON d.id=p.device_id
      JOIN accounts a ON a.id=d.account_id
      WHERE ref='private:'||p.id::text AND d.id=runner_device AND a.id=runner_actor
        AND recollect_role(p.brain_id) IS NOT NULL FOR SHARE OF p,d,a;
    RETURN brain IS NOT NULL AND recollect_mcp_private_eligible(brain,ref);
END $$;
CREATE FUNCTION recollect_mcp_runner_brain(ref text,brain uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT ref='central' OR ref LIKE 'device:%' OR EXISTS(
      SELECT 1 FROM mcp_private_runners p WHERE ref='private:'||p.id::text AND p.brain_id=brain)
$$;
REVOKE ALL ON FUNCTION recollect_mcp_private_eligible(uuid,text),recollect_mcp_private_lock(text,uuid,uuid),recollect_mcp_runner_brain(text,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_private_eligible(uuid,text),recollect_mcp_private_lock(text,uuid,uuid),recollect_mcp_runner_brain(text,uuid) TO recollect_app;

ALTER TABLE mcp_runners DROP CONSTRAINT mcp_runners_check;
ALTER TABLE mcp_runners ADD CONSTRAINT mcp_runners_check CHECK(
  (reference='central' AND actor_id IS NULL AND device_id IS NULL) OR
  (actor_id IS NOT NULL AND device_id IS NOT NULL AND
    (reference='device:'||device_id::text OR reference ~ '^private:[0-9a-f-]{36}$')));
CREATE OR REPLACE FUNCTION recollect_mcp_runner_owned(ref text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT (ref='central' AND recollect_actor() IS NULL AND recollect_device() IS NULL) OR
      EXISTS(SELECT 1 FROM devices d JOIN accounts a ON a.id=d.account_id
        WHERE d.id=recollect_device() AND d.account_id=recollect_actor()
          AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now() AND a.enabled
          AND (a.auth_kind='local' OR a.membership_until>now()) AND
          (ref='device:'||d.id::text OR EXISTS(SELECT 1 FROM mcp_private_runners p
            WHERE ref='private:'||p.id::text AND p.device_id=d.id
              AND recollect_mcp_private_eligible(p.brain_id,ref))))
$$;
CREATE OR REPLACE FUNCTION recollect_mcp_next(ref text,target_epoch uuid)
RETURNS TABLE(id uuid,brain_id uuid,actor_id uuid,device_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT c.id,c.brain_id,c.actor_id,c.device_id FROM mcp_calls c
      WHERE recollect_mcp_runner_owned(ref) AND recollect_mcp_runner_alive(ref,target_epoch)
        AND recollect_mcp_runner_brain(ref,c.brain_id)
        AND c.runner_reference=ref AND c.state='queued' AND c.queue_expires_at>clock_timestamp() AND c.not_before<=clock_timestamp()
        AND (ref='central' OR ref LIKE 'private:%' OR c.actor_id=recollect_actor())
        AND (SELECT count(*) FROM mcp_calls a WHERE a.runner_reference=ref AND a.state IN ('starting','running'))<16
      ORDER BY c.created_at,c.id LIMIT 1
$$;
CREATE OR REPLACE FUNCTION recollect_mcp_attempt(ref text,target_epoch uuid,target_call uuid,target_token uuid)
RETURNS TABLE(id uuid,brain_id uuid,actor_id uuid,device_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT c.id,c.brain_id,c.actor_id,c.device_id FROM mcp_calls c
      WHERE recollect_mcp_runner_owned(ref) AND recollect_mcp_runner_brain(ref,c.brain_id)
        AND c.runner_reference=ref AND c.runner_epoch=target_epoch
        AND c.id=target_call AND c.attempt_token=target_token
$$;
CREATE OR REPLACE FUNCTION recollect_mcp_runner_status(brain uuid,ref text) RETURNS jsonb
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT jsonb_build_object('runner_reference',ref,'available',coalesce(r.lease_until>clock_timestamp(),false)
      AND (ref NOT LIKE 'private:%' OR recollect_mcp_private_eligible(brain,ref)),'lease_until',r.lease_until)
    FROM (SELECT 1) single LEFT JOIN mcp_runners r ON r.reference=ref
    WHERE recollect_role(brain) IS NOT NULL AND recollect_mcp_runner_brain(ref,brain)
      AND EXISTS(SELECT 1 FROM mcp_connections c WHERE c.brain_id=brain
        AND CASE WHEN c.placement='central' THEN ref='central' ELSE c.runner_reference=ref END)
$$;
CREATE OR REPLACE FUNCTION recollect_mcp_reject_queued(ref text,target_epoch uuid,target_call uuid,reason text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c mcp_calls%ROWTYPE;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) OR NOT recollect_mcp_runner_alive(ref,target_epoch) OR reason !~ '^[a-z0-9_]{1,100}$' THEN RETURN; END IF;
    UPDATE mcp_calls SET state='failed',code=reason,completed_at=clock_timestamp(),payload_expires_at=clock_timestamp()+interval '1 hour'
      WHERE id=target_call AND runner_reference=ref AND state='queued' AND recollect_mcp_runner_brain(ref,brain_id)
        AND (ref='central' OR ref LIKE 'private:%' OR actor_id=recollect_actor()) RETURNING * INTO c;
    IF FOUND THEN INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
        VALUES(gen_random_uuid(),c.actor_id,c.device_id,c.brain_id,'mcp.admission_recheck',c.id,reason); END IF;
END $$;
DROP POLICY mcp_call_insert ON mcp_calls;
CREATE POLICY mcp_call_insert ON mcp_calls FOR INSERT WITH CHECK(
    actor_id=recollect_actor() AND device_id IS NOT DISTINCT FROM recollect_device()
    AND recollect_mcp_can(profile_id,'use') AND recollect_mcp_runner_brain(runner_reference,brain_id));

-- Credential expiry before send is a known no-send failure. An expiry after
-- send remains unknown and cannot be retried as a tool operation.
CREATE OR REPLACE FUNCTION recollect_mcp_complete(ref text, target_epoch uuid, target_call uuid, target_token uuid,
    outcome text, reason text, body jsonb, resolution_id uuid) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c mcp_calls%ROWTYPE; prior jsonb; receipt_value jsonb;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) OR outcome NOT IN ('succeeded','tool_error','failed','cancelled','unknown')
       OR reason !~ '^[a-z0-9_]{1,100}$' OR octet_length(coalesce(body::text,''))>524288 THEN RETURN false; END IF;
    SELECT * INTO c FROM mcp_calls WHERE id=target_call AND runner_reference=ref AND runner_epoch=target_epoch
      AND attempt_token=target_token FOR UPDATE;
    IF NOT FOUND THEN RETURN false; END IF;
    receipt_value=jsonb_build_object('state',outcome,'code',reason,'result',body);
    IF c.receipt_state IS NOT NULL THEN
        SELECT receipt INTO prior FROM mcp_call_payloads WHERE call_id=c.id;
        RETURN c.receipt_state=outcome AND c.receipt_code=reason AND (prior IS NULL OR prior=receipt_value);
    END IF;
    IF c.state='running' AND c.lease_until<=clock_timestamp() THEN
        UPDATE mcp_calls SET state='unknown',code='lease_expired',completed_at=clock_timestamp(),
            payload_expires_at=clock_timestamp()+interval '1 hour' WHERE id=c.id RETURNING * INTO c;
        INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
            VALUES(gen_random_uuid(),c.actor_id,c.device_id,c.brain_id,'mcp.recovery',c.id,'unknown');
    END IF;
    IF c.state='starting' AND (outcome NOT IN ('failed','cancelled','unknown') OR body IS NOT NULL) THEN RETURN false; END IF;
    IF c.state='running' AND outcome NOT IN ('succeeded','tool_error','unknown')
       AND NOT (outcome='failed' AND body IS NULL AND reason IN ('provider_protocol_rejected','provider_instance_stale','tool_arguments_invalid','provider_tool_unavailable','cancelled_before_send','deadline_before_send','credential_expired')) THEN RETURN false; END IF;
    IF c.state NOT IN ('starting','running','unknown') THEN RETURN false; END IF;
    IF c.state='unknown' THEN
        -- Preserve the original unknown attempt; this is a separately attributed receipt.
        IF outcome IN ('succeeded','tool_error') OR (outcome='failed' AND body IS NULL
            AND reason IN ('provider_protocol_rejected','provider_instance_stale','tool_arguments_invalid','provider_tool_unavailable','cancelled_before_send','deadline_before_send','credential_expired')) THEN
            INSERT INTO mcp_call_resolutions(id,call_id,brain_id,actor_id,kind,outcome)
                VALUES(resolution_id,c.id,c.brain_id,c.actor_id,'late_receipt',outcome) ON CONFLICT DO NOTHING;
        ELSIF outcome<>'unknown' THEN RETURN false;
        END IF;
    ELSE
        UPDATE mcp_calls SET state=CASE WHEN c.state='starting' AND outcome='unknown' THEN 'failed' ELSE outcome END,
            code=CASE WHEN c.state='starting' AND outcome='unknown' THEN 'dispatch_not_committed' ELSE reason END,completed_at=clock_timestamp(),
            payload_expires_at=clock_timestamp()+interval '1 hour' WHERE id=c.id;
    END IF;
    UPDATE mcp_calls SET receipt_state=outcome,receipt_code=reason WHERE id=c.id;
    UPDATE mcp_call_payloads SET result=body,receipt=receipt_value WHERE call_id=c.id
      AND EXISTS(SELECT 1 FROM mcp_calls x WHERE x.id=c.id AND x.payload_expires_at>clock_timestamp());
    INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
        VALUES(gen_random_uuid(),c.actor_id,c.device_id,c.brain_id,'mcp.completion',c.id,
          CASE WHEN c.state='unknown' THEN 'late_receipt' ELSE outcome END);
    IF c.reconciles_call_id IS NOT NULL AND outcome='succeeded'
       AND body->'structuredContent'->>'outcome' IN ('succeeded','failed','not_executed','unknown') THEN
        INSERT INTO mcp_call_resolutions(id,call_id,brain_id,actor_id,kind,outcome,receipt_call_id)
          VALUES(gen_random_uuid(),c.reconciles_call_id,c.brain_id,c.actor_id,'connector_receipt',body->'structuredContent'->>'outcome',c.id)
          ON CONFLICT DO NOTHING;
        INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
          VALUES(gen_random_uuid(),c.actor_id,c.device_id,c.brain_id,'mcp.reconcile',c.reconciles_call_id,'connector_receipt');
    END IF;
    RETURN true;
END $$;
