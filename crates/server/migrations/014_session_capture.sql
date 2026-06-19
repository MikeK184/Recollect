-- Capture is an evidence producer under independent standing Brain permission.
CREATE TABLE capture_policies (
  brain_id uuid PRIMARY KEY REFERENCES brains(id),
  change_id uuid NOT NULL,
  policy jsonb NOT NULL,
  updated_by uuid NOT NULL REFERENCES accounts(id),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE capture_bindings (
  id uuid PRIMARY KEY,
  brain_id uuid NOT NULL REFERENCES brains(id),
  operation_id uuid NOT NULL REFERENCES operation_bindings(id),
  actor_id uuid NOT NULL REFERENCES accounts(id),
  device_id uuid NOT NULL REFERENCES devices(id),
  host text NOT NULL CHECK(host IN ('codex','claude_code')),
  host_version text NOT NULL CHECK(length(host_version) BETWEEN 1 AND 120),
  selection jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE(id,brain_id)
);
CREATE TABLE capture_events (
  id uuid PRIMARY KEY,
  brain_id uuid NOT NULL REFERENCES brains(id),
  binding_id uuid NOT NULL,
  native_key text CHECK(length(native_key)<=1200),
  metadata jsonb CHECK(metadata IS NULL OR metadata->'content'='null'::jsonb),
  admission_policy jsonb,
  retention_class text NOT NULL CHECK(retention_class IN ('raw_session','tool_output')),
  captured_at timestamptz NOT NULL,
  received_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  state text NOT NULL DEFAULT 'accepted' CHECK(state IN ('accepted','expired','removed')),
  source_id uuid,
  source_version_id uuid,
  FOREIGN KEY(binding_id,brain_id) REFERENCES capture_bindings(id,brain_id),
  FOREIGN KEY(source_version_id,source_id,brain_id) REFERENCES source_versions(id,source_id,brain_id),
  UNIQUE(binding_id,native_key),
  UNIQUE(source_version_id)
);
CREATE INDEX capture_activity ON capture_events(brain_id,received_at DESC,id DESC);
CREATE INDEX capture_owner ON capture_bindings(brain_id,actor_id,device_id,created_at DESC);
CREATE TABLE capture_device_reports (
  brain_id uuid NOT NULL REFERENCES brains(id),
  device_id uuid NOT NULL REFERENCES devices(id),
  report jsonb NOT NULL,
  reported_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY(brain_id,device_id)
);
ALTER TABLE capture_device_reports ENABLE ROW LEVEL SECURITY;
CREATE POLICY read_capture_report ON capture_device_reports FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY write_capture_report ON capture_device_reports FOR ALL
  USING(recollect_role(brain_id) IS NOT NULL AND device_id=recollect_device())
  WITH CHECK(recollect_role(brain_id) IS NOT NULL AND device_id=recollect_device() AND EXISTS(
    SELECT 1 FROM capture_bindings b WHERE b.brain_id=capture_device_reports.brain_id
      AND b.device_id=recollect_device() AND b.actor_id=recollect_actor()));
GRANT SELECT,INSERT,UPDATE ON capture_device_reports TO recollect_app;
-- These references deliberately have no event/binding FK: a journal can precede
-- those identities in a restored database. No captured content is kept here.
CREATE TABLE privacy_capture_fences (
  brain_id uuid NOT NULL REFERENCES brains(id),
  event_id uuid NOT NULL,
  binding_id uuid NOT NULL,
  native_key text,
  request_id uuid NOT NULL REFERENCES privacy_requests(id),
  PRIMARY KEY(brain_id,event_id),
  UNIQUE(brain_id,binding_id,native_key)
);
ALTER TABLE capture_policies ENABLE ROW LEVEL SECURITY;
CREATE POLICY read_policy ON capture_policies FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY write_policy ON capture_policies FOR ALL USING(recollect_role(brain_id)='admin' AND recollect_device() IS NULL)
  WITH CHECK(recollect_role(brain_id)='admin' AND recollect_device() IS NULL);
GRANT SELECT,INSERT,UPDATE ON capture_policies TO recollect_app;
ALTER TABLE capture_bindings ENABLE ROW LEVEL SECURITY;
CREATE POLICY read_binding ON capture_bindings FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY write_binding ON capture_bindings FOR INSERT WITH CHECK(recollect_role(brain_id) IN ('writer','admin') AND actor_id=recollect_actor() AND device_id=recollect_device());
GRANT SELECT,INSERT ON capture_bindings TO recollect_app;
ALTER TABLE capture_events ENABLE ROW LEVEL SECURITY;
CREATE POLICY read_event ON capture_events FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY write_event ON capture_events FOR INSERT WITH CHECK(recollect_role(brain_id) IN ('writer','admin') AND EXISTS(
  SELECT 1 FROM capture_bindings b WHERE b.id=binding_id AND b.brain_id=capture_events.brain_id AND b.actor_id=recollect_actor() AND b.device_id=recollect_device()));
CREATE POLICY attach_source ON capture_events FOR UPDATE USING(recollect_role(brain_id) IN ('writer','admin') AND EXISTS(
  SELECT 1 FROM capture_bindings b WHERE b.id=binding_id AND b.brain_id=capture_events.brain_id AND b.actor_id=recollect_actor() AND b.device_id=recollect_device()));
GRANT SELECT,INSERT ON capture_events TO recollect_app;
GRANT UPDATE(source_id,source_version_id) ON capture_events TO recollect_app;
ALTER TABLE privacy_capture_fences ENABLE ROW LEVEL SECURITY;
CREATE POLICY read_fence ON privacy_capture_fences FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
GRANT SELECT ON privacy_capture_fences TO recollect_app;

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_autonomous_privacy_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; fences jsonb;
BEGIN
  m:=recollect_autonomous_privacy_closure(b,k,t,c);
  SELECT coalesce(jsonb_agg(jsonb_build_object('event_id',id,'binding_id',binding_id,'native_key',native_key) ORDER BY id),'[]')
    INTO fences FROM capture_events WHERE brain_id=b AND
      (source_version_id=ANY(recollect_privacy_ids(m,'source_versions')) OR (k='capture_event' AND id=t));
  IF jsonb_array_length(fences)>0 THEN m:=m||jsonb_build_object('capture_event_fences',fences); END IF;
  RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_autonomous_privacy_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
  INSERT INTO privacy_capture_fences(brain_id,event_id,binding_id,native_key,request_id)
    SELECT r.brain_id,(f->>'event_id')::uuid,(f->>'binding_id')::uuid,f->>'native_key',r.id
    FROM jsonb_array_elements(coalesce(r.manifest->'capture_event_fences','[]')) f ON CONFLICT DO NOTHING;
  UPDATE capture_events e SET metadata=NULL,admission_policy=NULL,
    expires_at=least(e.expires_at,recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)),
    state=CASE WHEN r.cause='erase' OR e.state='removed' THEN 'removed' ELSE 'expired' END
    WHERE e.brain_id=r.brain_id AND EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(r.manifest->'capture_event_fences','[]')) f
      WHERE (f->>'event_id')::uuid=e.id OR ((f->>'binding_id')::uuid=e.binding_id AND f->>'native_key'=e.native_key));
  PERFORM recollect_autonomous_privacy_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;

-- Like the existing system expiry entry point, this can remove only canonically
-- due content. It never accepts caller-supplied policy, payload or closure.
CREATE FUNCTION recollect_expire_capture_event(e uuid) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE candidate capture_events; rid uuid;
BEGIN
  SELECT * INTO candidate FROM capture_events WHERE id=e;
  IF NOT FOUND THEN RETURN NULL; END IF;
  PERFORM id FROM brains WHERE id=candidate.brain_id FOR UPDATE;
  SELECT * INTO candidate FROM capture_events WHERE id=e;
  IF candidate.state<>'accepted' OR recollect_retention_deadline(candidate.brain_id,candidate.retention_class,candidate.captured_at)>clock_timestamp() THEN RETURN NULL; END IF;
  -- Source expiry owns the full evidence closure; marker expiry owns only itself.
  rid:=gen_random_uuid();
  INSERT INTO privacy_requests(id,brain_id,target,cause,manifest)
    VALUES(rid,candidate.brain_id,jsonb_build_object('kind','capture_event','id',e),'expire',
      recollect_privacy_closure(candidate.brain_id,
        CASE WHEN candidate.source_version_id IS NULL THEN 'capture_event' ELSE 'source_version' END,
        coalesce(candidate.source_version_id,e),'expire'));
  PERFORM recollect_privacy_apply(rid);
  RETURN rid;
END $$;
REVOKE ALL ON FUNCTION recollect_expire_capture_event(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_capture_event(uuid) TO recollect_app;

ALTER FUNCTION recollect_expire_one() RENAME TO recollect_evidence_expire_one;
CREATE FUNCTION recollect_expire_one() RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE rid uuid; event_id uuid;
BEGIN
  rid:=recollect_evidence_expire_one();
  IF rid IS NOT NULL THEN RETURN rid; END IF;
  SELECT id INTO event_id FROM capture_events WHERE state='accepted'
    AND recollect_retention_deadline(brain_id,retention_class,captured_at)<=clock_timestamp()
    ORDER BY captured_at,id LIMIT 1;
  IF event_id IS NULL THEN RETURN NULL; END IF;
  RETURN recollect_expire_capture_event(event_id);
END $$;
REVOKE ALL ON FUNCTION recollect_expire_one() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_one() TO recollect_app;

-- Optional closure arrays are omitted by the typed journal when empty. Compare
-- their semantic form so a preexisting committed request is replay-idempotent.
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE plpgsql IMMUTABLE SET search_path=public,pg_temp AS $$
DECLARE k text;
BEGIN
  FOREACH k IN ARRAY ARRAY['model_input_sources','model_input_claim_revisions','capture_event_fences'] LOOP
    IF m->k='[]'::jsonb THEN m:=m-k; END IF;
  END LOOP;
  RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
CREATE OR REPLACE FUNCTION recollect_privacy_replay(entry jsonb) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE rid uuid; b uuid; position bigint; found_entry privacy_requests;
BEGIN
  IF (entry->>'installation_id')::uuid IS DISTINCT FROM (SELECT id FROM privacy_installation) THEN RAISE invalid_parameter_value; END IF;
  rid:=(entry->>'id')::uuid; b:=(entry->>'brain_id')::uuid; position:=(entry->>'sequence')::bigint;
  SELECT * INTO found_entry FROM privacy_requests WHERE id=rid;
  IF FOUND THEN
    IF found_entry.brain_id<>b OR found_entry.sequence<>position OR
      recollect_privacy_normalize(found_entry.manifest)<>recollect_privacy_normalize(entry->'manifest') THEN RAISE invalid_parameter_value; END IF;
    RETURN;
  END IF;
  IF NOT EXISTS(SELECT 1 FROM brains WHERE id=b) THEN
    INSERT INTO privacy_replayed_entries(id,sequence) VALUES(rid,position) ON CONFLICT(id) DO NOTHING;
    PERFORM setval(pg_get_serial_sequence('privacy_requests','sequence'),greatest(position,(SELECT coalesce(max(sequence),1) FROM privacy_requests),(SELECT coalesce(max(sequence),1) FROM privacy_replayed_entries)),true);
    RETURN;
  END IF;
  PERFORM id FROM brains WHERE id=b FOR UPDATE;
  INSERT INTO privacy_requests(id,brain_id,sequence,target,cause,manifest,created_at,journaled) OVERRIDING SYSTEM VALUE
    VALUES(rid,b,position,entry->'target',entry->>'cause',entry->'manifest',(entry->>'created_at')::timestamptz,true);
  PERFORM recollect_privacy_apply(rid);
  PERFORM setval(pg_get_serial_sequence('privacy_requests','sequence'),greatest(position,(SELECT coalesce(max(sequence),1) FROM privacy_requests)),true);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_replay(jsonb) FROM PUBLIC;
