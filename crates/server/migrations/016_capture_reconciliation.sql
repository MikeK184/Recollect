-- Receipt is server knowledge time after the Brain write lock. Replays preserve
-- the first receipt. Historical transaction-start receipts cannot be reconstructed.
ALTER TABLE capture_events ALTER COLUMN received_at SET DEFAULT clock_timestamp();

-- Shared exact-original-version lookup; retention still consumes created_at.
-- Invoker permissions and RLS apply instead of the migration owner's authority.
CREATE VIEW recollect_source_knowledge WITH (security_invoker=true) AS
  SELECT v.*,coalesce(e.received_at,v.created_at) AS recorded_at
  FROM source_versions v LEFT JOIN capture_events e
    ON e.brain_id=v.brain_id AND e.source_version_id=v.id;
GRANT SELECT ON recollect_source_knowledge TO recollect_app;

-- Select supported revisions through source lineage instead of repeatedly
-- evaluating every claim's unrelated evidence at the Brain capacity boundary.
CREATE INDEX claim_source_lineage ON claim_supports(brain_id,source_version_id,revision_id)
  WHERE source_version_id IS NOT NULL;
CREATE INDEX claim_current_identity ON claims(brain_id,current_revision);
