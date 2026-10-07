-- Version-specific applicability for canonical native document imports.
CREATE TABLE source_import_scopes (
  version_id uuid PRIMARY KEY REFERENCES source_versions(id) ON DELETE CASCADE,
  brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
  operation_id uuid REFERENCES operation_bindings(id),
  selection jsonb NOT NULL
);
ALTER TABLE source_import_scopes ENABLE ROW LEVEL SECURITY;
CREATE POLICY import_scope_read ON source_import_scopes FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY import_scope_write ON source_import_scopes FOR INSERT WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT ON source_import_scopes TO recollect_app;

-- Successful authorized Brain operations, payload-free and throttled per minute.
CREATE TABLE agent_brain_usage (
  brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
  device_id uuid NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  host_kind text NOT NULL CHECK(host_kind IN ('unknown','codex','claude_code','opencode')),
  used_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY(brain_id,device_id,host_kind)
);
ALTER TABLE agent_brain_usage ENABLE ROW LEVEL SECURITY;
CREATE POLICY agent_usage_read ON agent_brain_usage FOR SELECT USING(recollect_role(brain_id) IS NOT NULL AND EXISTS(SELECT 1 FROM devices d WHERE d.id=device_id AND d.account_id=recollect_actor()));
CREATE POLICY agent_usage_insert ON agent_brain_usage FOR INSERT WITH CHECK(recollect_role(brain_id) IS NOT NULL AND device_id=recollect_device());
CREATE POLICY agent_usage_update ON agent_brain_usage FOR UPDATE USING(recollect_role(brain_id) IS NOT NULL AND device_id=recollect_device());
GRANT SELECT,INSERT,UPDATE ON agent_brain_usage TO recollect_app;

CREATE OR REPLACE FUNCTION recollect_brain_agent_usage(target uuid)
RETURNS TABLE(device_id uuid,last_used timestamptz)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(target) IS NULL THEN RETURN; END IF;
  RETURN QUERY SELECT u.device_id,max(u.used_at) FROM (
    SELECT a.device_id,a.used_at FROM agent_brain_usage a WHERE a.brain_id=target
    UNION ALL SELECT m.device_id,m.created_at FROM mcp_calls m WHERE m.brain_id=target AND m.device_id IS NOT NULL
    UNION ALL SELECT b.device_id,e.received_at FROM capture_events e JOIN capture_bindings b ON b.id=e.binding_id AND b.brain_id=e.brain_id
      WHERE e.brain_id=target AND e.state='accepted' AND recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)>clock_timestamp()
  ) u GROUP BY u.device_id;
END $$;
CREATE FUNCTION recollect_agent_hosts(target uuid, target_brain uuid DEFAULT NULL)
RETURNS text[] LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT coalesce(array_agg(DISTINCT h.host ORDER BY h.host),'{}'::text[]) FROM (
   SELECT a.host_kind host FROM agent_brain_usage a WHERE a.device_id=target AND a.host_kind<>'unknown' AND (target_brain IS NULL OR a.brain_id=target_brain) AND recollect_role(a.brain_id) IS NOT NULL
   UNION ALL SELECT b.host FROM capture_bindings b JOIN capture_events e ON e.binding_id=b.id AND e.brain_id=b.brain_id WHERE b.device_id=target AND b.host IN ('codex','claude_code','opencode') AND e.state='accepted' AND (target_brain IS NULL OR b.brain_id=target_brain) AND recollect_role(b.brain_id) IS NOT NULL
 ) h WHERE EXISTS(SELECT 1 FROM devices d WHERE d.id=target AND (d.account_id=recollect_actor() OR (target_brain IS NOT NULL AND recollect_role(target_brain) IS NOT NULL)));
$$;
REVOKE ALL ON FUNCTION recollect_agent_hosts(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_agent_hosts(uuid,uuid) TO recollect_app;
