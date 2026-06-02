-- MCP effects are deliberately separate from retryable background jobs.
CREATE TABLE mcp_calls (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    actor_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    request_id uuid NOT NULL,
    profile_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    profile_revision uuid NOT NULL,
    connection_revision uuid NOT NULL,
    definition_revision timestamptz NOT NULL,
    tool_name text NOT NULL CHECK(length(tool_name) BETWEEN 1 AND 128),
    environment_id uuid,
    operation_id uuid,
    scope jsonb,
    client_session_id uuid NOT NULL,
    runner_reference text NOT NULL CHECK(length(runner_reference) BETWEEN 1 AND 200),
    state text NOT NULL CHECK(state IN ('queued','starting','running','succeeded','tool_error','failed','cancelled','unknown')),
    code text CHECK(length(code)<=100),
    timeout_seconds integer NOT NULL CHECK(timeout_seconds BETWEEN 1 AND 3600),
    cancel_requested boolean NOT NULL DEFAULT false,
    runner_epoch uuid,
    attempt_token uuid,
    instance_id uuid,
    lease_until timestamptz,
    deadline timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    queue_expires_at timestamptz NOT NULL DEFAULT clock_timestamp()+interval '10 minutes',
    not_before timestamptz NOT NULL DEFAULT clock_timestamp(),
    started_at timestamptz,
    dispatched_at timestamptz,
    completed_at timestamptz,
    payload_expires_at timestamptz,
    receipt_state text,
    receipt_code text,
    reconciles_call_id uuid REFERENCES mcp_calls(id),
    UNIQUE(brain_id,actor_id,request_id),
    UNIQUE(id,brain_id),
    FOREIGN KEY(profile_id,brain_id) REFERENCES mcp_profiles(id,brain_id),
    FOREIGN KEY(connection_id,brain_id) REFERENCES mcp_connections(id,brain_id),
    CHECK((state IN ('queued','starting','running'))=(completed_at IS NULL)),
    CHECK((completed_at IS NULL)=(payload_expires_at IS NULL)),
    CHECK((runner_epoch IS NULL)=(attempt_token IS NULL))
);
CREATE INDEX mcp_pending ON mcp_calls(runner_reference,created_at,id) WHERE state='queued';
CREATE INDEX mcp_active ON mcp_calls(lease_until) WHERE state IN ('starting','running');
CREATE INDEX mcp_call_history ON mcp_calls(brain_id,created_at DESC,id DESC);
CREATE TABLE mcp_call_payloads (
    call_id uuid PRIMARY KEY REFERENCES mcp_calls(id),
    brain_id uuid NOT NULL,
    request jsonb NOT NULL CHECK(octet_length(request::text)<=65536),
    arguments jsonb NOT NULL CHECK(octet_length(arguments::text)<=65536),
    result jsonb CHECK(octet_length(result::text)<=524288),
    receipt jsonb CHECK(octet_length(receipt::text)<=524288),
    FOREIGN KEY(call_id,brain_id) REFERENCES mcp_calls(id,brain_id)
);
CREATE TABLE mcp_call_resolutions (
    id uuid PRIMARY KEY,
    call_id uuid NOT NULL REFERENCES mcp_calls(id),
    brain_id uuid NOT NULL,
    actor_id uuid NOT NULL REFERENCES accounts(id),
    request_id uuid,
    kind text NOT NULL CHECK(kind IN ('late_receipt','connector_receipt','evidence')),
    outcome text NOT NULL CHECK(outcome IN ('succeeded','failed','not_executed','unknown','tool_error')),
    receipt_call_id uuid REFERENCES mcp_calls(id),
    source_version_id uuid,
    explanation text CHECK(length(explanation)<=2000),
    explanation_expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(call_id,brain_id) REFERENCES mcp_calls(id,brain_id)
);
CREATE UNIQUE INDEX mcp_resolution_request ON mcp_call_resolutions(brain_id,actor_id,request_id) WHERE request_id IS NOT NULL;
CREATE UNIQUE INDEX mcp_connector_resolution ON mcp_call_resolutions(call_id,receipt_call_id) WHERE kind='connector_receipt';
CREATE UNIQUE INDEX mcp_late_resolution ON mcp_call_resolutions(call_id) WHERE kind='late_receipt';

CREATE TABLE mcp_runners (
    reference text PRIMARY KEY,
    actor_id uuid REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    epoch uuid NOT NULL,
    lease_until timestamptz NOT NULL,
    CHECK((reference='central' AND actor_id IS NULL AND device_id IS NULL) OR
          (actor_id IS NOT NULL AND device_id IS NOT NULL AND reference='device:'||device_id::text))
);
CREATE TABLE mcp_instances (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    profile_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    actor_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    client_session_id uuid NOT NULL,
    runner_reference text NOT NULL REFERENCES mcp_runners(reference),
    runner_epoch uuid NOT NULL,
    connection_revision uuid NOT NULL,
    definition_revision timestamptz NOT NULL,
    credential_generation uuid NOT NULL,
    state text NOT NULL CHECK(state IN ('starting','ready','draining','stopped','lost')),
    active_calls integer NOT NULL CHECK(active_calls BETWEEN 0 AND 4),
    idle_seconds bigint NOT NULL CHECK(idle_seconds>=0),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(profile_id,brain_id) REFERENCES mcp_profiles(id,brain_id),
    FOREIGN KEY(connection_id,brain_id) REFERENCES mcp_connections(id,brain_id)
);
CREATE TABLE mcp_session_releases (
    brain_id uuid NOT NULL REFERENCES brains(id),
    actor_id uuid NOT NULL REFERENCES accounts(id),
    client_session_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(brain_id,actor_id,client_session_id)
);

ALTER TABLE mcp_calls ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_call_read ON mcp_calls FOR SELECT USING(
    recollect_role(brain_id) IS NOT NULL AND (actor_id=recollect_actor() OR recollect_role(brain_id)='admin'));
CREATE POLICY mcp_call_insert ON mcp_calls FOR INSERT WITH CHECK(
    actor_id=recollect_actor() AND device_id IS NOT DISTINCT FROM recollect_device() AND recollect_mcp_can(profile_id,'use'));
CREATE POLICY mcp_call_update ON mcp_calls FOR UPDATE USING(
    recollect_role(brain_id) IS NOT NULL AND (actor_id=recollect_actor() OR recollect_role(brain_id)='admin')) WITH CHECK(
    recollect_role(brain_id) IS NOT NULL AND (actor_id=recollect_actor() OR recollect_role(brain_id)='admin'));
ALTER TABLE mcp_call_payloads ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_payload_read ON mcp_call_payloads FOR SELECT USING(EXISTS(
    SELECT 1 FROM mcp_calls c WHERE c.id=call_id AND recollect_mcp_can(c.profile_id,'use')
       AND (c.payload_expires_at IS NULL OR c.payload_expires_at>clock_timestamp())));
CREATE POLICY mcp_payload_insert ON mcp_call_payloads FOR INSERT WITH CHECK(EXISTS(
    SELECT 1 FROM mcp_calls c WHERE c.id=call_id AND c.actor_id=recollect_actor() AND recollect_mcp_can(c.profile_id,'use')));
ALTER TABLE mcp_call_resolutions ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_resolution_read ON mcp_call_resolutions FOR SELECT USING(EXISTS(
    SELECT 1 FROM mcp_calls c WHERE c.id=call_id AND recollect_mcp_can(c.profile_id,'use')));
CREATE POLICY mcp_resolution_insert ON mcp_call_resolutions FOR INSERT WITH CHECK(
    actor_id=recollect_actor() AND EXISTS(SELECT 1 FROM mcp_calls c WHERE c.id=call_id AND recollect_mcp_can(c.profile_id,'use')));
ALTER TABLE mcp_instances ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_instance_read ON mcp_instances FOR SELECT USING(
    recollect_role(brain_id) IS NOT NULL AND (actor_id=recollect_actor() OR recollect_role(brain_id)='admin'));
ALTER TABLE mcp_session_releases ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_release ON mcp_session_releases FOR ALL USING(
    recollect_role(brain_id) IS NOT NULL AND actor_id=recollect_actor()) WITH CHECK(
    recollect_role(brain_id) IS NOT NULL AND actor_id=recollect_actor());
GRANT SELECT,INSERT,UPDATE ON mcp_calls TO recollect_app;
GRANT SELECT,INSERT ON mcp_call_payloads,mcp_call_resolutions TO recollect_app;
GRANT SELECT ON mcp_instances TO recollect_app;
GRANT SELECT,INSERT ON mcp_session_releases TO recollect_app;

-- Queue-wide counts expose only capacity, never another principal's payload.
CREATE FUNCTION recollect_mcp_capacity() RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(73241023);
    RETURN (SELECT count(*)<1000 AND count(*) FILTER(WHERE actor_id=recollect_actor())<32
            FROM mcp_calls WHERE state IN ('queued','starting','running'));
END $$;

CREATE FUNCTION recollect_mcp_runner_owned(ref text) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT (ref='central' AND recollect_actor() IS NULL AND recollect_device() IS NULL) OR
           EXISTS(SELECT 1 FROM devices d JOIN accounts a ON a.id=d.account_id
              WHERE d.id=recollect_device() AND d.account_id=recollect_actor() AND ref='device:'||d.id::text
                AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now() AND a.enabled
                AND (a.auth_kind='local' OR a.membership_until>now()))
$$;
CREATE FUNCTION recollect_mcp_runner_register(ref text, new_epoch uuid) RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE saved uuid;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) THEN RETURN false; END IF;
    INSERT INTO mcp_runners(reference,actor_id,device_id,epoch,lease_until)
       VALUES(ref,recollect_actor(),recollect_device(),new_epoch,clock_timestamp()+interval '30 seconds')
    ON CONFLICT(reference) DO UPDATE SET epoch=excluded.epoch,lease_until=excluded.lease_until
       WHERE mcp_runners.lease_until<=clock_timestamp()
    RETURNING epoch INTO saved;
    RETURN saved IS NOT NULL;
END $$;
CREATE FUNCTION recollect_mcp_runner_alive(ref text, target_epoch uuid) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT EXISTS(SELECT 1 FROM mcp_runners WHERE reference=ref AND epoch=target_epoch AND lease_until>clock_timestamp())
$$;
CREATE FUNCTION recollect_mcp_runner_touch(ref text, target_epoch uuid) RETURNS jsonb LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE expires timestamptz;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) THEN RETURN NULL; END IF;
    UPDATE mcp_runners SET lease_until=clock_timestamp()+interval '30 seconds'
      WHERE reference=ref AND epoch=target_epoch AND lease_until>clock_timestamp() RETURNING lease_until INTO expires;
    IF expires IS NULL THEN RETURN NULL; END IF;
    UPDATE mcp_calls SET lease_until=least(expires,deadline)
      WHERE runner_reference=ref AND runner_epoch=target_epoch AND state IN ('starting','running') AND lease_until>clock_timestamp();
    RETURN jsonb_build_object('runner_reference',ref,'epoch',target_epoch,'lease_until',expires,
      'cancel_calls',coalesce((SELECT jsonb_agg(c.id) FROM mcp_calls c WHERE c.runner_reference=ref AND c.runner_epoch=target_epoch
          AND c.state IN ('starting','running') AND (c.cancel_requested OR c.lease_until<=clock_timestamp()
            OR NOT recollect_mcp_effective(c.profile_id,c.actor_id,'use')
            OR (c.device_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM devices d WHERE d.id=c.device_id AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now())))), '[]'),
      'released_sessions',coalesce((SELECT jsonb_agg(DISTINCT jsonb_build_object('brain_id',s.brain_id,'actor_id',s.actor_id,'client_session_id',s.client_session_id)) FROM mcp_session_releases s JOIN mcp_instances i
          ON i.brain_id=s.brain_id AND i.actor_id=s.actor_id AND i.client_session_id=s.client_session_id
          WHERE i.runner_reference=ref AND i.runner_epoch=target_epoch AND i.state IN ('starting','ready','draining')), '[]'),
      'drain_instances',coalesce((SELECT jsonb_agg(i.id) FROM mcp_instances i JOIN mcp_connections c ON c.id=i.connection_id
          JOIN mcp_profiles p ON p.id=i.profile_id JOIN mcp_definitions d ON d.key=c.definition_key
          WHERE i.runner_reference=ref AND i.runner_epoch=target_epoch AND i.state IN ('starting','ready','draining')
            AND (NOT recollect_mcp_effective(i.profile_id,i.actor_id,'use') OR NOT p.enabled OR NOT c.enabled OR NOT d.enabled
              OR c.revision<>i.connection_revision OR d.updated_at<>i.definition_revision)), '[]'));
END $$;
CREATE FUNCTION recollect_mcp_next(ref text, target_epoch uuid) RETURNS TABLE(id uuid,brain_id uuid,actor_id uuid,device_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT c.id,c.brain_id,c.actor_id,c.device_id FROM mcp_calls c
    WHERE recollect_mcp_runner_owned(ref) AND recollect_mcp_runner_alive(ref,target_epoch)
      AND c.runner_reference=ref AND c.state='queued' AND c.queue_expires_at>clock_timestamp() AND c.not_before<=clock_timestamp()
      AND (ref='central' OR c.actor_id=recollect_actor())
      AND (SELECT count(*) FROM mcp_calls a WHERE a.runner_reference=ref AND a.state IN ('starting','running'))<16
    ORDER BY c.created_at,c.id LIMIT 1
$$;

CREATE FUNCTION recollect_mcp_attempt(ref text, target_epoch uuid, target_call uuid, target_token uuid)
RETURNS TABLE(id uuid,brain_id uuid,actor_id uuid,device_id uuid) LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT c.id,c.brain_id,c.actor_id,c.device_id FROM mcp_calls c
      WHERE recollect_mcp_runner_owned(ref) AND c.runner_reference=ref AND c.runner_epoch=target_epoch
        AND c.id=target_call AND c.attempt_token=target_token
$$;
CREATE FUNCTION recollect_mcp_reject_queued(ref text,target_epoch uuid,target_call uuid,reason text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c mcp_calls%ROWTYPE;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) OR NOT recollect_mcp_runner_alive(ref,target_epoch) OR reason !~ '^[a-z0-9_]{1,100}$' THEN RETURN; END IF;
    UPDATE mcp_calls SET state='failed',code=reason,completed_at=clock_timestamp(),payload_expires_at=clock_timestamp()+interval '1 hour'
      WHERE id=target_call AND runner_reference=ref AND state='queued' AND (ref='central' OR actor_id=recollect_actor()) RETURNING * INTO c;
    IF FOUND THEN INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
        VALUES(gen_random_uuid(),c.actor_id,c.device_id,c.brain_id,'mcp.admission_recheck',c.id,reason); END IF;
END $$;
CREATE FUNCTION recollect_mcp_defer(ref text,target_epoch uuid,target_call uuid,target_token uuid) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE changed uuid;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) OR NOT recollect_mcp_runner_alive(ref,target_epoch) THEN RETURN false; END IF;
    UPDATE mcp_calls SET state='queued',code='waiting_capacity',runner_epoch=NULL,attempt_token=NULL,lease_until=NULL,deadline=NULL,
      started_at=NULL,not_before=clock_timestamp()+interval '1 second'
      WHERE id=target_call AND runner_reference=ref AND runner_epoch=target_epoch AND attempt_token=target_token
        AND state='starting' AND NOT cancel_requested RETURNING id INTO changed;
    RETURN changed IS NOT NULL;
END $$;
CREATE FUNCTION recollect_mcp_instance(ref text,target_epoch uuid,target_call uuid,target_token uuid,
    instance uuid,generation uuid,phase text,active integer,idle bigint) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c mcp_calls%ROWTYPE; changed uuid;
BEGIN
    IF NOT recollect_mcp_runner_owned(ref) OR NOT recollect_mcp_runner_alive(ref,target_epoch)
      OR phase NOT IN ('starting','ready','draining','stopped') OR active NOT BETWEEN 0 AND 4 OR idle NOT BETWEEN 0 AND 86400 THEN RETURN false; END IF;
    SELECT * INTO c FROM mcp_calls WHERE id=target_call AND runner_reference=ref AND runner_epoch=target_epoch AND attempt_token=target_token;
    IF NOT FOUND THEN RETURN false; END IF;
    IF phase='starting' AND (c.state<>'starting' OR c.cancel_requested
        OR c.lease_until IS NULL OR c.lease_until<=clock_timestamp()
        OR c.deadline IS NULL OR c.deadline<=clock_timestamp()) THEN RETURN false; END IF;
    IF NOT EXISTS(SELECT 1 FROM mcp_instances WHERE id=instance) AND phase<>'starting' THEN RETURN false; END IF;
    INSERT INTO mcp_instances(id,brain_id,profile_id,connection_id,actor_id,device_id,client_session_id,runner_reference,runner_epoch,
      connection_revision,definition_revision,credential_generation,state,active_calls,idle_seconds)
    VALUES(instance,c.brain_id,c.profile_id,c.connection_id,c.actor_id,c.device_id,c.client_session_id,ref,target_epoch,
      c.connection_revision,c.definition_revision,generation,phase,active,idle)
    ON CONFLICT(id) DO UPDATE SET state=excluded.state,active_calls=excluded.active_calls,idle_seconds=excluded.idle_seconds,updated_at=clock_timestamp()
      WHERE mcp_instances.brain_id=c.brain_id AND mcp_instances.profile_id=c.profile_id AND mcp_instances.connection_id=c.connection_id
        AND mcp_instances.actor_id=c.actor_id AND mcp_instances.device_id IS NOT DISTINCT FROM c.device_id
        AND mcp_instances.client_session_id=c.client_session_id AND mcp_instances.runner_reference=ref AND mcp_instances.runner_epoch=target_epoch
        AND mcp_instances.connection_revision=c.connection_revision AND mcp_instances.definition_revision=c.definition_revision
        AND mcp_instances.credential_generation=generation AND mcp_instances.state NOT IN ('stopped','lost')
        AND NOT(mcp_instances.state<>'starting' AND phase='starting')
    RETURNING id INTO changed;
    RETURN changed IS NOT NULL;
END $$;
CREATE FUNCTION recollect_mcp_instance_matches(instance uuid,target_call uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT EXISTS(SELECT 1 FROM mcp_instances i JOIN mcp_calls c ON c.id=target_call
      WHERE i.id=instance AND i.brain_id=c.brain_id AND i.profile_id=c.profile_id AND i.connection_id=c.connection_id
        AND i.actor_id=c.actor_id AND i.device_id IS NOT DISTINCT FROM c.device_id AND i.client_session_id=c.client_session_id
        AND i.runner_reference=c.runner_reference AND i.runner_epoch=c.runner_epoch AND i.connection_revision=c.connection_revision
        AND i.definition_revision=c.definition_revision AND i.state IN ('starting','ready') AND recollect_mcp_can(c.profile_id,'use'))
$$;
CREATE FUNCTION recollect_mcp_runner_status(brain uuid,ref text) RETURNS jsonb
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT jsonb_build_object('runner_reference',ref,'available',coalesce(r.lease_until>clock_timestamp(),false),'lease_until',r.lease_until)
    FROM (SELECT 1) single LEFT JOIN mcp_runners r ON r.reference=ref
    WHERE recollect_role(brain) IS NOT NULL AND EXISTS(SELECT 1 FROM mcp_connections c WHERE c.brain_id=brain
      AND CASE WHEN c.placement='central' THEN ref='central' ELSE c.runner_reference=ref END)
$$;

-- Finishing an already-dispatched attempt is permitted after profile revocation;
-- returning its output is separately gated by current Use and Brain authority.
CREATE FUNCTION recollect_mcp_complete(ref text, target_epoch uuid, target_call uuid, target_token uuid,
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
       AND NOT (outcome='failed' AND body IS NULL AND reason IN ('provider_protocol_rejected','provider_instance_stale','tool_arguments_invalid','provider_tool_unavailable','cancelled_before_send','deadline_before_send')) THEN RETURN false; END IF;
    IF c.state NOT IN ('starting','running','unknown') THEN RETURN false; END IF;
    IF c.state='unknown' THEN
        -- Preserve the original unknown attempt; this is a separately attributed receipt.
        IF outcome IN ('succeeded','tool_error') OR (outcome='failed' AND body IS NULL
            AND reason IN ('provider_protocol_rejected','provider_instance_stale','tool_arguments_invalid','provider_tool_unavailable','cancelled_before_send','deadline_before_send')) THEN
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

CREATE FUNCTION recollect_mcp_maintain() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c mcp_calls%ROWTYPE; disposition text;
BEGIN
    -- This operator is called by the internal central executor, never a device route.
    IF recollect_actor() IS NOT NULL OR recollect_device() IS NOT NULL THEN RAISE insufficient_privilege; END IF;
    FOR c IN SELECT * FROM mcp_calls WHERE
       (state='queued' AND queue_expires_at<=clock_timestamp()) OR
       (state IN ('starting','running') AND (lease_until<=clock_timestamp() OR NOT recollect_mcp_runner_alive(runner_reference,runner_epoch)))
       ORDER BY created_at LIMIT 1000 FOR UPDATE SKIP LOCKED
    LOOP
        disposition=CASE WHEN c.state='running' THEN 'unknown' ELSE 'failed' END;
        UPDATE mcp_calls SET state=disposition,code=CASE WHEN c.state='queued' THEN 'queue_expired' ELSE 'lease_expired' END,
          completed_at=clock_timestamp(),payload_expires_at=clock_timestamp()+interval '1 hour' WHERE id=c.id;
        INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
          VALUES(gen_random_uuid(),c.actor_id,c.device_id,c.brain_id,'mcp.recovery',c.id,disposition);
    END LOOP;
    DELETE FROM mcp_call_payloads WHERE call_id IN (SELECT id FROM mcp_calls WHERE payload_expires_at<=clock_timestamp());
    UPDATE mcp_call_resolutions r SET explanation=NULL WHERE explanation IS NOT NULL AND
      (explanation_expires_at<=clock_timestamp() OR NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.id=r.source_version_id
        AND v.brain_id=r.brain_id AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'));
    UPDATE mcp_instances SET state='lost',active_calls=0,updated_at=clock_timestamp()
      WHERE state IN ('starting','ready','draining') AND NOT recollect_mcp_runner_alive(runner_reference,runner_epoch);
END $$;

-- The application has no unrestricted runner or instance mutation grant.
DO $$ DECLARE f record; BEGIN
  FOR f IN SELECT oid::regprocedure AS signature FROM pg_proc WHERE pronamespace='public'::regnamespace
    AND proname IN ('recollect_mcp_capacity','recollect_mcp_runner_owned','recollect_mcp_runner_register',
      'recollect_mcp_runner_alive','recollect_mcp_runner_touch','recollect_mcp_next','recollect_mcp_complete','recollect_mcp_maintain',
      'recollect_mcp_attempt','recollect_mcp_reject_queued','recollect_mcp_defer','recollect_mcp_instance','recollect_mcp_instance_matches','recollect_mcp_runner_status')
  LOOP
    EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.signature);
    IF f.signature::text NOT LIKE 'recollect_mcp_runner_owned(%' THEN
      EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO recollect_app',f.signature);
    END IF;
  END LOOP;
END $$;
