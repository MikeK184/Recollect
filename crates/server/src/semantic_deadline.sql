WITH selected AS (
  SELECT * FROM semantic_entries WHERE brain_id=$1 AND id=ANY($2)
), deadlines AS (
  SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) AS deadline
    FROM selected e JOIN source_versions v ON v.brain_id=e.brain_id AND v.id=e.source_version_id
  UNION ALL
  SELECT recollect_memory_deadline(r.brain_id,r.id)
    FROM selected e JOIN claim_revisions r ON r.brain_id=e.brain_id AND r.id=e.claim_revision_id
  UNION ALL
  SELECT recollect_retention_deadline(s.brain_id,'repository',s.created_at)
    FROM selected e JOIN repository_facts f ON f.brain_id=e.brain_id AND f.id=e.fact_id
    JOIN repository_snapshots s ON s.brain_id=f.brain_id AND s.id=f.snapshot_id
)
SELECT min(deadline) FROM deadlines
