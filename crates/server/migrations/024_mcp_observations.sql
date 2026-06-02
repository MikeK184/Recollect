-- The managed runtime contributes through the existing capture/source engine.
-- Temporary execution receipts and published knowledge retain separate policy.
ALTER TABLE mcp_calls ADD COLUMN capture_policy jsonb;
ALTER TABLE mcp_calls ADD COLUMN capture_target jsonb;
ALTER TABLE mcp_calls ADD COLUMN capture_disposition text NOT NULL DEFAULT 'disabled';
ALTER TABLE mcp_calls ADD CHECK (capture_policy IS NULL OR octet_length(capture_policy::text)<=16384);
ALTER TABLE mcp_calls ADD CHECK (capture_target IS NULL OR octet_length(capture_target::text)<=65536);

ALTER TABLE capture_bindings ALTER COLUMN operation_id DROP NOT NULL;
ALTER TABLE capture_bindings ALTER COLUMN device_id DROP NOT NULL;
ALTER TABLE capture_bindings DROP CONSTRAINT capture_bindings_host_check;
ALTER TABLE capture_bindings ADD COLUMN managed_call_id uuid REFERENCES mcp_calls(id);
ALTER TABLE capture_bindings ADD COLUMN managed_target jsonb;
ALTER TABLE capture_bindings ADD CHECK (
  (host IN ('codex','claude_code') AND operation_id IS NOT NULL AND device_id IS NOT NULL AND managed_call_id IS NULL AND managed_target IS NULL)
  OR (host='managed_mcp' AND managed_call_id IS NOT NULL AND managed_call_id=id));
CREATE UNIQUE INDEX managed_capture_binding ON capture_bindings(managed_call_id) WHERE managed_call_id IS NOT NULL;

-- Public host inserts cannot forge the internal producer. Its binding is created
-- by the admitted call trigger from canonical identities, never supplied values.
DROP POLICY write_binding ON capture_bindings;
CREATE POLICY write_binding ON capture_bindings FOR INSERT WITH CHECK(
  host IN ('codex','claude_code') AND managed_call_id IS NULL
  AND recollect_role(brain_id) IN ('writer','admin')
  AND actor_id=recollect_actor() AND device_id=recollect_device());
DROP POLICY write_event ON capture_events;
DROP POLICY attach_source ON capture_events;
CREATE POLICY write_event ON capture_events FOR INSERT WITH CHECK(
  recollect_role(brain_id) IN ('writer','admin') AND EXISTS(
    SELECT 1 FROM capture_bindings b WHERE b.id=binding_id AND b.brain_id=capture_events.brain_id
      AND b.actor_id=recollect_actor() AND b.device_id IS NOT DISTINCT FROM recollect_device()));
CREATE POLICY attach_source ON capture_events FOR UPDATE USING(
  recollect_role(brain_id) IN ('writer','admin') AND EXISTS(
    SELECT 1 FROM capture_bindings b WHERE b.id=binding_id AND b.brain_id=capture_events.brain_id
      AND b.actor_id=recollect_actor() AND b.device_id IS NOT DISTINCT FROM recollect_device()));

CREATE TABLE mcp_observation_outbox (
  id uuid PRIMARY KEY,
  call_id uuid NOT NULL REFERENCES mcp_calls(id),
  brain_id uuid NOT NULL REFERENCES brains(id),
  actor_id uuid NOT NULL REFERENCES accounts(id),
  device_id uuid REFERENCES devices(id),
  stage text NOT NULL CHECK(stage IN ('terminal','late_receipt','connector_receipt','evidence')),
  outcome text NOT NULL CHECK(outcome IN ('succeeded','tool_error','failed','cancelled','unknown','not_executed')),
  code text CHECK(length(code)<=100),
  captured_at timestamptz NOT NULL,
  expires_at timestamptz NOT NULL,
  admission_policy jsonb,
  event jsonb CHECK(event IS NULL OR octet_length(event::text)<=524288),
  prepared boolean NOT NULL DEFAULT false,
  state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','error','published','filtered','skipped','expired','removed')),
  coverage jsonb NOT NULL DEFAULT '[]',
  attempts integer NOT NULL DEFAULT 0,
  next_attempt_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  error_code text CHECK(length(error_code)<=100),
  published_at timestamptz,
  lease_token uuid,
  lease_until timestamptz,
  UNIQUE(id,brain_id)
);
CREATE INDEX mcp_observation_pending ON mcp_observation_outbox(next_attempt_at,id) WHERE state IN ('pending','error');
CREATE INDEX mcp_observation_history ON mcp_observation_outbox(call_id,captured_at,id);
ALTER TABLE mcp_observation_outbox ENABLE ROW LEVEL SECURITY;
CREATE POLICY observation_read ON mcp_observation_outbox FOR SELECT USING(EXISTS(
  SELECT 1 FROM mcp_calls c WHERE c.id=call_id));
CREATE POLICY observation_publish ON mcp_observation_outbox FOR UPDATE USING(
  actor_id=recollect_actor() AND device_id IS NOT DISTINCT FROM recollect_device()
  AND recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,UPDATE ON mcp_observation_outbox TO recollect_app;

CREATE FUNCTION recollect_mcp_observation_fenced(b uuid,c uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT EXISTS(SELECT 1 FROM privacy_capture_fences f JOIN privacy_requests r ON r.id=f.request_id
    WHERE f.brain_id=b AND f.binding_id=c AND r.cause='erase');
$$;
REVOKE ALL ON FUNCTION recollect_mcp_observation_fenced(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_observation_fenced(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_mcp_capture_binding() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NEW.capture_policy IS NOT NULL THEN
    IF NEW.actor_id<>recollect_actor() OR recollect_role(NEW.brain_id) NOT IN ('writer','admin')
       OR NOT recollect_mcp_can(NEW.profile_id,'use')
       OR NOT coalesce((NEW.capture_policy->>'enabled')::boolean,false)
       OR NOT coalesce((NEW.capture_policy->>'managed_tools')::boolean,false) THEN
      RAISE insufficient_privilege;
    END IF;
    INSERT INTO capture_bindings(id,brain_id,operation_id,actor_id,device_id,host,host_version,selection,managed_call_id,managed_target)
      VALUES(NEW.id,NEW.brain_id,NEW.operation_id,NEW.actor_id,NEW.device_id,'managed_mcp','recollect',
        coalesce(NEW.scope->'selection',jsonb_build_object('repository_ids','[]'::jsonb,'area_ids','[]'::jsonb,'environment_id',NEW.environment_id)),
        NEW.id,NEW.capture_target);
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER mcp_capture_binding AFTER INSERT ON mcp_calls FOR EACH ROW EXECUTE FUNCTION recollect_mcp_capture_binding();

CREATE FUNCTION recollect_mcp_queue_observation() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE c mcp_calls; eid uuid; phase text; result_state text; happened timestamptz; disposition text;
BEGIN
  IF TG_TABLE_NAME='mcp_calls' THEN
    IF OLD.completed_at IS NOT NULL OR NEW.completed_at IS NULL THEN RETURN NEW; END IF;
    c:=NEW; eid:=c.id; phase:='terminal'; result_state:=c.state; happened:=c.completed_at;
  ELSE
    SELECT * INTO STRICT c FROM mcp_calls WHERE id=NEW.call_id;
    eid:=NEW.id; phase:=NEW.kind; result_state:=NEW.outcome; happened:=NEW.created_at;
  END IF;
  IF c.capture_policy IS NULL THEN RETURN NEW; END IF;
  -- Short capacity admission shared with payload attachment. It never spans
  -- provider calls, artifact I/O or publication, and uses no content hashes.
  PERFORM pg_advisory_xact_lock(73241024);
  disposition:=CASE WHEN recollect_mcp_observation_fenced(c.brain_id,c.id) THEN 'removed'
    WHEN (SELECT count(*) FROM mcp_observation_outbox WHERE brain_id=c.brain_id AND state IN ('pending','error'))>=1000 THEN 'skipped'
    ELSE 'pending' END;
  INSERT INTO mcp_observation_outbox(id,call_id,brain_id,actor_id,device_id,stage,outcome,code,captured_at,expires_at,admission_policy,state,coverage,error_code)
    VALUES(eid,c.id,c.brain_id,c.actor_id,c.device_id,phase,result_state,c.code,happened,
      recollect_retention_deadline(c.brain_id,'tool_output',happened),
      CASE WHEN disposition='pending' THEN c.capture_policy END,disposition,
      CASE WHEN disposition='skipped' THEN '["publication_capacity"]'::jsonb ELSE '[]'::jsonb END,
      CASE WHEN disposition='skipped' THEN 'publication_capacity' END)
    ON CONFLICT DO NOTHING;
  RETURN NEW;
END $$;
CREATE TRIGGER mcp_observation_terminal AFTER UPDATE OF completed_at ON mcp_calls FOR EACH ROW EXECUTE FUNCTION recollect_mcp_queue_observation();
CREATE TRIGGER mcp_observation_resolution AFTER INSERT ON mcp_call_resolutions FOR EACH ROW EXECUTE FUNCTION recollect_mcp_queue_observation();

-- Only the fenced producer sees the descriptor. Publication later rebinds to the
-- original actor; a private runner never acquires the caller's knowledge role.
CREATE FUNCTION recollect_mcp_observation_lock(ref text,ep uuid,cid uuid,tok uuid) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid;
BEGIN
  IF NOT recollect_mcp_runner_owned(ref) THEN RETURN false; END IF;
  SELECT brain_id INTO b FROM mcp_calls WHERE id=cid AND runner_reference=ref AND runner_epoch=ep AND attempt_token=tok;
  IF b IS NULL THEN RETURN false; END IF;
  PERFORM id FROM brains WHERE id=b FOR UPDATE;
  RETURN FOUND;
END $$;

CREATE FUNCTION recollect_mcp_observation_prepare(ref text,ep uuid,cid uuid,tok uuid) RETURNS SETOF jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NOT recollect_mcp_runner_owned(ref) OR NOT EXISTS(SELECT 1 FROM mcp_calls c WHERE
    c.id=cid AND c.runner_reference=ref AND c.runner_epoch=ep AND c.attempt_token=tok) THEN RETURN; END IF;
  RETURN QUERY SELECT to_jsonb(o)||jsonb_build_object('tool_name',c.tool_name,'client_session_id',c.client_session_id,
    'operation_id',c.operation_id,'target',b.managed_target,'selection',b.selection,
    'current_policy',coalesce(p.policy,'{}'::jsonb),'arguments',v.arguments,
    'source_version_id',r.source_version_id,'receipt_call_id',r.receipt_call_id,
    'result',CASE WHEN o.stage='connector_receipt' THEN receipt.result
      WHEN o.stage='late_receipt' OR (o.stage='terminal' AND o.outcome IN ('succeeded','tool_error')) THEN v.result END)
    FROM mcp_observation_outbox o JOIN mcp_calls c ON c.id=o.call_id
    JOIN capture_bindings b ON b.id=c.id LEFT JOIN capture_policies p ON p.brain_id=c.brain_id
    LEFT JOIN mcp_call_payloads v ON v.call_id=c.id
    LEFT JOIN mcp_call_resolutions r ON r.id=o.id
    LEFT JOIN mcp_call_payloads receipt ON receipt.call_id=r.receipt_call_id
    WHERE (o.call_id=cid OR r.receipt_call_id=cid) AND NOT o.prepared AND o.state IN ('pending','error') ORDER BY o.captured_at,o.id;
END $$;

CREATE FUNCTION recollect_mcp_observation_store(ref text,ep uuid,cid uuid,tok uuid,eid uuid,body jsonb) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; used bigint; pending bigint;
BEGIN
  IF NOT recollect_mcp_runner_owned(ref) OR NOT EXISTS(SELECT 1 FROM mcp_calls c WHERE
    c.id=cid AND c.runner_reference=ref AND c.runner_epoch=ep AND c.attempt_token=tok)
    OR body IS NULL OR octet_length(body::text)>524288 THEN RETURN false; END IF;
  SELECT o.brain_id INTO b FROM mcp_observation_outbox o WHERE o.id=eid AND
    (o.call_id=cid OR EXISTS(SELECT 1 FROM mcp_call_resolutions r WHERE r.id=o.id AND r.receipt_call_id=cid));
  IF b IS NULL THEN RETURN false; END IF;
  PERFORM id FROM brains WHERE id=b FOR UPDATE;
  PERFORM pg_advisory_xact_lock(73241024);
  IF EXISTS(SELECT 1 FROM mcp_observation_outbox o WHERE o.id=eid AND recollect_mcp_observation_fenced(b,o.call_id)) THEN
    UPDATE mcp_observation_outbox SET event=NULL,admission_policy=NULL,state='removed',prepared=true WHERE id=eid;
    RETURN true;
  END IF;
  SELECT coalesce(sum(octet_length(event::text)),0),count(*) INTO used,pending
    FROM mcp_observation_outbox WHERE brain_id=b AND state IN ('pending','error');
  IF used+octet_length(body::text)>67108864 OR pending>1000 THEN
    UPDATE mcp_observation_outbox SET event=NULL,admission_policy=NULL,state='skipped',prepared=true,
      coverage='["publication_capacity"]',error_code='publication_capacity' WHERE id=eid AND NOT prepared;
    RETURN true;
  END IF;
  UPDATE mcp_observation_outbox SET event=body,prepared=true,coverage=coalesce(body->'coverage','[]')
    WHERE id=eid AND NOT prepared AND state IN ('pending','error');
  RETURN true;
END $$;

CREATE FUNCTION recollect_mcp_observation_next(lease uuid)
RETURNS TABLE(id uuid,brain_id uuid,actor_id uuid,device_id uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_actor() IS NOT NULL OR recollect_device() IS NOT NULL THEN RAISE insufficient_privilege; END IF;
  RETURN QUERY WITH candidate AS (
    SELECT o.id FROM mcp_observation_outbox o WHERE o.state IN ('pending','error')
      AND o.next_attempt_at<=clock_timestamp() AND (o.lease_until IS NULL OR o.lease_until<=clock_timestamp())
      ORDER BY o.next_attempt_at,o.id LIMIT 1 FOR UPDATE SKIP LOCKED)
    UPDATE mcp_observation_outbox o SET lease_token=lease,lease_until=clock_timestamp()+interval '30 seconds',attempts=o.attempts+1
      FROM candidate c WHERE o.id=c.id RETURNING o.id,o.brain_id,o.actor_id,o.device_id;
END $$;

CREATE FUNCTION recollect_mcp_observation_finish(eid uuid,lease uuid,disposition text,reason text) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_actor() IS NOT NULL OR recollect_device() IS NOT NULL
    OR disposition NOT IN ('error','skipped','expired','removed')
    OR reason !~ '^[a-z0-9_]{1,100}$' THEN RAISE insufficient_privilege; END IF;
  UPDATE mcp_observation_outbox SET state=disposition,error_code=reason,lease_token=NULL,lease_until=NULL,
    next_attempt_at=clock_timestamp()+make_interval(secs=>least(60,power(2,least(attempts,6)))::double precision),
    event=CASE WHEN disposition='error' THEN event END,
    admission_policy=CASE WHEN disposition='error' THEN admission_policy END
    WHERE id=eid AND lease_token=lease AND state IN ('pending','error');
  RETURN FOUND;
END $$;

CREATE FUNCTION recollect_mcp_receipt_removals(ref text,ids uuid[]) RETURNS SETOF uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF cardinality(ids)>64 OR NOT recollect_mcp_runner_owned(ref) THEN RAISE insufficient_privilege; END IF;
  RETURN QUERY SELECT c.id FROM mcp_calls c WHERE c.id=ANY(ids) AND c.runner_reference=ref
    AND recollect_mcp_observation_fenced(c.brain_id,c.id)
    UNION SELECT f.binding_id FROM privacy_capture_fences f JOIN privacy_requests r ON r.id=f.request_id
    WHERE f.binding_id=ANY(ids) AND r.cause='erase' AND NOT EXISTS(SELECT 1 FROM mcp_calls c WHERE c.id=f.binding_id)
      AND recollect_mcp_runner_brain(ref,f.brain_id)
      AND (ref='central' OR ref LIKE 'private:%' OR recollect_role(f.brain_id) IS NOT NULL);
END $$;

-- The journal's existing opaque binding fences also cover queued/late receipts.
-- No FK to a missing managed call is needed when replaying an older database.
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_observation_privacy_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; extra jsonb; item record; version uuid; calls uuid[];
BEGIN
  m:=recollect_pre_observation_privacy_closure(b,k,t,c);
  IF c<>'erase' THEN RETURN m; END IF;
  SELECT array_agg(DISTINCT cb.id) INTO calls FROM capture_bindings cb
    WHERE cb.brain_id=b AND cb.host='managed_mcp' AND (
      EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(m->'capture_event_fences','[]')) f WHERE (f->>'binding_id')::uuid=cb.id)
      OR EXISTS(SELECT 1 FROM mcp_call_resolutions r WHERE r.call_id=cb.id
        AND r.source_version_id=ANY(recollect_privacy_ids(m,'source_versions'))));
  IF calls IS NULL THEN RETURN m; END IF;
  -- A later observation can contain the same reported output. Include all of
  -- this call's published derivatives, while preserving unrelated sources.
  FOR version IN SELECT source_version_id FROM capture_events WHERE brain_id=b AND binding_id=ANY(calls)
    AND source_version_id IS NOT NULL LOOP
    extra:=recollect_pre_observation_privacy_closure(b,'source_version',version,c);
    FOR item IN SELECT key,value FROM jsonb_each(extra) WHERE jsonb_typeof(value)='array' LOOP
      m:=jsonb_set(m,ARRAY[item.key],(SELECT coalesce(jsonb_agg(v ORDER BY v),'[]') FROM
        (SELECT DISTINCT value v FROM jsonb_array_elements(coalesce(m->item.key,'[]')||item.value)) merged));
    END LOOP;
  END LOOP;
  SELECT coalesce(jsonb_agg(v ORDER BY v),'[]') INTO extra FROM (
    SELECT DISTINCT value v FROM jsonb_array_elements(coalesce(m->'capture_event_fences','[]'))
    UNION SELECT jsonb_build_object('event_id',o.id,'binding_id',o.call_id,'native_key',NULL)
      FROM mcp_observation_outbox o WHERE o.brain_id=b AND o.call_id=ANY(calls)
    UNION SELECT jsonb_build_object('event_id',id,'binding_id',id,'native_key',NULL) FROM unnest(calls) id
  ) fences;
  m:=m||jsonb_build_object('capture_event_fences',extra);
  m:=m||jsonb_build_object('independent_claim_revisions',(SELECT count(*) FROM claim_revisions cr
    WHERE cr.brain_id=b AND cr.privacy_state='active' AND cr.claim_id=ANY(recollect_privacy_ids(m,'claim_ids'))
    AND NOT cr.id=ANY(recollect_privacy_ids(m,'claim_revisions'))));
  RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_observation_privacy_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
  -- Upgrade an older expiry fence when the same identity is subsequently erased.
  UPDATE privacy_capture_fences f SET request_id=rid WHERE f.brain_id=r.brain_id AND r.cause='erase'
    AND EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(r.manifest->'capture_event_fences','[]')) e
      WHERE (e->>'event_id')::uuid=f.event_id);
  PERFORM recollect_pre_observation_privacy_apply(rid);
  UPDATE mcp_observation_outbox o SET event=NULL,admission_policy=NULL,coverage='[]',
    state=CASE WHEN r.cause='erase' OR o.state='removed' THEN 'removed' ELSE 'expired' END,lease_token=NULL,lease_until=NULL
    WHERE o.brain_id=r.brain_id AND EXISTS(SELECT 1 FROM privacy_capture_fences f
      WHERE f.brain_id=o.brain_id AND f.request_id=rid AND (f.event_id=o.id OR (r.cause='erase' AND f.binding_id=o.call_id)));
  DELETE FROM mcp_call_payloads p WHERE p.brain_id=r.brain_id AND EXISTS(
    SELECT 1 FROM privacy_capture_fences f WHERE f.brain_id=p.brain_id AND f.request_id=rid AND f.binding_id=p.call_id);
  UPDATE mcp_calls c SET capture_policy=NULL,capture_target=NULL,capture_disposition='removed'
    WHERE c.brain_id=r.brain_id AND r.cause='erase' AND EXISTS(
    SELECT 1 FROM privacy_capture_fences f WHERE f.brain_id=c.brain_id AND f.request_id=rid AND f.binding_id=c.id);
  UPDATE capture_bindings b SET managed_target=NULL WHERE b.brain_id=r.brain_id AND b.host='managed_mcp' AND EXISTS(
    SELECT 1 FROM privacy_capture_fences f WHERE f.brain_id=b.brain_id AND f.request_id=rid AND f.binding_id=b.id)
    AND (r.cause='erase' OR NOT EXISTS(SELECT 1 FROM mcp_observation_outbox o WHERE o.call_id=b.id
      AND o.state NOT IN ('removed','expired')));
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;

ALTER FUNCTION recollect_mcp_maintain() RENAME TO recollect_pre_observation_mcp_maintain;
CREATE FUNCTION recollect_mcp_maintain() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  PERFORM recollect_pre_observation_mcp_maintain();
  UPDATE mcp_calls SET capture_policy=NULL,capture_target=NULL WHERE capture_policy IS NOT NULL
    AND recollect_retention_deadline(brain_id,'tool_output',coalesce(completed_at,created_at))<=clock_timestamp();
END $$;
REVOKE ALL ON FUNCTION recollect_mcp_maintain() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_maintain() TO recollect_app;

DO $$ DECLARE f record; BEGIN
  FOR f IN SELECT oid::regprocedure AS signature FROM pg_proc WHERE pronamespace='public'::regnamespace
    AND proname IN ('recollect_mcp_observation_lock','recollect_mcp_observation_prepare','recollect_mcp_observation_store',
      'recollect_mcp_observation_next','recollect_mcp_observation_finish','recollect_mcp_receipt_removals')
  LOOP
    EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f.signature);
    EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO recollect_app',f.signature);
  END LOOP;
END $$;
