WITH selected AS (
  SELECT * FROM semantic_entries WHERE brain_id=$1 AND id=ANY($2)
), deadlines AS (
  SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) AS deadline
    FROM selected e JOIN source_versions v ON v.brain_id=e.brain_id AND v.id=e.source_version_id
  UNION ALL
  SELECT recollect_retention_deadline(r.brain_id,'claim',r.recorded_at)
    FROM selected e JOIN claim_revisions r ON r.brain_id=e.brain_id AND r.id=e.claim_revision_id
  UNION ALL
  -- Only the handover's required contributors gate this representation.
  -- Historical reasoning edges and ordinary raw supports have their own TTLs.
  SELECT recollect_retention_deadline(c.brain_id,'claim',c.recorded_at)
    FROM selected e JOIN claim_revisions r ON r.brain_id=e.brain_id AND r.id=e.claim_revision_id
    CROSS JOIN LATERAL jsonb_array_elements_text(coalesce(r.revision#>'{content,handover,contributions}','[]'::jsonb)) input
    JOIN claim_revisions c ON c.brain_id=r.brain_id AND c.id=input::uuid
  UNION ALL
  SELECT recollect_retention_deadline(s.brain_id,'repository',s.created_at)
    FROM selected e JOIN repository_facts f ON f.brain_id=e.brain_id AND f.id=e.fact_id
    JOIN repository_snapshots s ON s.brain_id=f.brain_id AND s.id=f.snapshot_id
)
SELECT min(deadline) FROM deadlines
