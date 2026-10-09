WITH known_snapshots AS (
  SELECT DISTINCT ON(repository_id) id FROM repository_snapshots
  WHERE brain_id=$1 ORDER BY repository_id,created_at DESC,id DESC
), candidates AS (
  SELECT 'source_chunk'::text AS kind,c.id AS input_id,v.id AS source_version_id,
    NULL::uuid AS claim_revision_id,NULL::uuid AS fact_id,NULL::uuid AS manifest_revision_id,
    v.retention_class AS class,v.created_at
  FROM source_chunks c JOIN source_versions v ON v.id=c.version_id AND v.brain_id=c.brain_id
  JOIN sources s ON s.current_version=v.id AND s.brain_id=v.brain_id
  WHERE c.brain_id=$1 AND v.processing='ready' AND v.artifact_id IS NOT NULL
    AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
    AND NOT EXISTS(SELECT 1 FROM model_input_fences f WHERE f.brain_id=v.brain_id AND f.source_version_id=v.id)
  UNION ALL
  SELECT 'claim_revision',r.id,NULL,r.id,NULL,NULL,'claim',r.recorded_at
  FROM claims c JOIN claim_revisions r ON r.id=c.current_revision AND r.brain_id=c.brain_id
  WHERE c.brain_id=$1 AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
    AND recollect_memory_supported(r.brain_id,r.id)
    AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
    AND r.revision#>>'{content,freshness}'<>'superseded'
    AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)
  UNION ALL
  SELECT 'repository_fact',f.id,NULL,NULL,f.id,NULL,'repository',s.created_at
  FROM repository_facts f JOIN repository_snapshots s ON s.id=f.snapshot_id AND s.brain_id=f.brain_id
  WHERE f.brain_id=$1 AND (s.id IN(SELECT id FROM known_snapshots) OR EXISTS(
    SELECT 1 FROM revision_manifests m JOIN manifest_revisions r ON r.id=m.current_revision AND r.brain_id=m.brain_id
    CROSS JOIN LATERAL jsonb_array_elements(r.revision->'entries') entry
    WHERE r.brain_id=$1 AND r.privacy_state='active' AND entry->>'snapshot_id'=s.id::text))
    AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'
  UNION ALL
  SELECT 'manifest_revision',r.id,NULL,NULL,NULL,r.id,'repository',r.created_at
  FROM manifest_revisions r JOIN revision_manifests m ON m.current_revision=r.id AND m.brain_id=r.brain_id
  WHERE r.brain_id=$1 AND r.privacy_state='active'
) SELECT * FROM candidates WHERE class=ANY($2::text[])
