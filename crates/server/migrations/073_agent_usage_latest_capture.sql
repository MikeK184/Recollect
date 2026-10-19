-- Agent rosters need only the newest unexpired accepted capture of each
-- binding. Reading every accepted event ran the Brain role and retention checks
-- once per event, so My agents and the Brain roster slowed linearly with
-- capture history (about 5 s at 10k events).
CREATE INDEX capture_binding_latest ON capture_events(binding_id,received_at DESC,id DESC) WHERE state='accepted';

CREATE OR REPLACE FUNCTION recollect_brain_agent_usage(target uuid)
RETURNS TABLE(device_id uuid,last_used timestamptz)
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(target) IS NULL THEN RETURN; END IF;
  RETURN QUERY SELECT u.device_id,max(u.used_at) FROM (
    SELECT a.device_id,a.used_at FROM agent_brain_usage a WHERE a.brain_id=target
    UNION ALL SELECT m.device_id,m.created_at FROM mcp_calls m WHERE m.brain_id=target AND m.device_id IS NOT NULL
    UNION ALL SELECT b.device_id,e.received_at FROM capture_bindings b CROSS JOIN LATERAL (
      SELECT c.received_at FROM capture_events c
      WHERE c.binding_id=b.id AND c.brain_id=b.brain_id AND c.state='accepted'
        AND recollect_retention_deadline(c.brain_id,c.retention_class,c.captured_at)>clock_timestamp()
      ORDER BY c.received_at DESC,c.id DESC LIMIT 1) e
    WHERE b.brain_id=target
  ) u GROUP BY u.device_id;
END $$;
