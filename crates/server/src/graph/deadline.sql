WITH selected AS (
 SELECT * FROM jsonb_to_recordset($2::jsonb) AS n(kind text,revision_id uuid)
), deadlines AS (
 SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) AS deadline
 FROM selected n JOIN source_versions v ON v.brain_id=$1 AND v.id=n.revision_id
 WHERE n.kind='source_version'
 UNION ALL
 SELECT recollect_retention_deadline(r.brain_id,'claim',r.recorded_at)
 FROM selected n JOIN claim_revisions r ON r.brain_id=$1 AND r.id=n.revision_id
 WHERE n.kind='claim'
 UNION ALL
 SELECT recollect_retention_deadline(s.brain_id,'repository',s.created_at)
 FROM selected n JOIN repository_facts f ON f.brain_id=$1 AND f.id=n.revision_id
 JOIN repository_snapshots s ON s.brain_id=f.brain_id AND s.id=f.snapshot_id
 WHERE n.kind='repository_fact'
 UNION ALL
 SELECT recollect_retention_deadline(s.brain_id,'repository',s.created_at)
 FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id=$3
)
SELECT min(deadline) FROM deadlines
