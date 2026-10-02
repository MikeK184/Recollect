-- Devices record how they were connected: which coding host (when the issuer
-- knows it) and which of the two integration kinds minted the credential.
ALTER TABLE devices ADD COLUMN host_kind text CHECK (host_kind IN ('codex','claude_code','opencode'));
ALTER TABLE devices ADD COLUMN integration text NOT NULL DEFAULT 'mcp' CHECK (integration IN ('mcp','plugin'));
-- Nullable on the pairing request: NULL means the issuer did not supply the
-- marker, so approval must not overwrite what the device record already holds.
ALTER TABLE device_pairings ADD COLUMN host_kind text CHECK (host_kind IN ('codex','claude_code','opencode'));
ALTER TABLE device_pairings ADD COLUMN integration text CHECK (integration IN ('mcp','plugin'));

-- Backfill from the only structured host source: capture bindings.
UPDATE devices d SET host_kind = b.host
FROM (
  SELECT DISTINCT ON (device_id) device_id, host
  FROM capture_bindings
  WHERE device_id IS NOT NULL AND host IN ('codex','claude_code','opencode')
  ORDER BY device_id
) b
WHERE b.device_id = d.id AND d.host_kind IS NULL;

UPDATE devices d SET integration = 'plugin'
FROM capture_bindings b
WHERE b.device_id = d.id AND d.integration = 'mcp';
