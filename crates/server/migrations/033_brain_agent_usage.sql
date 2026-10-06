-- Share only an observed device identity and last-use timestamp within an
-- authorized Brain. This does not broaden private MCP call/payload RLS.
CREATE FUNCTION recollect_brain_agent_usage(target uuid)
RETURNS TABLE(device_id uuid,last_used timestamptz)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(target) IS NULL THEN RETURN; END IF;
  RETURN QUERY
    SELECT u.device_id,max(u.used_at) FROM (
      SELECT m.device_id,m.created_at used_at FROM mcp_calls m
      WHERE m.brain_id=target AND m.device_id IS NOT NULL
      UNION ALL
      SELECT b.device_id,e.received_at FROM capture_events e
      JOIN capture_bindings b ON b.id=e.binding_id AND b.brain_id=e.brain_id
      WHERE e.brain_id=target AND e.state='accepted'
        AND recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)>clock_timestamp()
    ) u GROUP BY u.device_id;
END $$;
REVOKE ALL ON FUNCTION recollect_brain_agent_usage(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_agent_usage(uuid) TO recollect_app;
