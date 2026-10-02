-- Admit the native OpenCode adapter without changing managed-producer authority.
DO $$
DECLARE producer_constraint text;
BEGIN
  SELECT conname INTO STRICT producer_constraint
  FROM pg_constraint
  WHERE conrelid='capture_bindings'::regclass AND contype='c'
    AND pg_get_constraintdef(oid) LIKE '%managed_mcp%';
  EXECUTE format('ALTER TABLE capture_bindings DROP CONSTRAINT %I', producer_constraint);
END $$;
ALTER TABLE capture_bindings ADD CONSTRAINT capture_bindings_producer_check CHECK (
  (host IN ('codex','claude_code','opencode') AND operation_id IS NOT NULL
    AND device_id IS NOT NULL AND managed_call_id IS NULL AND managed_target IS NULL)
  OR (host='managed_mcp' AND managed_call_id IS NOT NULL AND managed_call_id=id)
);
DROP POLICY write_binding ON capture_bindings;
CREATE POLICY write_binding ON capture_bindings FOR INSERT WITH CHECK (
  host IN ('codex','claude_code','opencode') AND managed_call_id IS NULL
  AND recollect_role(brain_id) IN ('writer','admin')
  AND actor_id=recollect_actor() AND device_id=recollect_device()
);
