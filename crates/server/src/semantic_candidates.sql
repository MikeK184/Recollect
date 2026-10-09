-- Exact identity cohorts fence represented inputs before expensive current
-- qualification. Do not limit them before qualification: an unavailable prefix
-- must not hide later eligible inputs. Any existing entry state prevents replay.
WITH missing_chunks AS MATERIALIZED (
  SELECT c.id,c.version_id,c.brain_id FROM source_chunks c
  WHERE c.brain_id=$1 AND NOT EXISTS (
    SELECT 1 FROM semantic_entries e WHERE e.brain_id=$1 AND e.profile_id=$3::uuid
      AND e.kind='source_chunk' AND e.input_id=c.id
  )
), missing_claims AS MATERIALIZED (
  SELECT c.current_revision,c.brain_id FROM claims c
  WHERE c.brain_id=$1 AND 'claim'=ANY($2::text[]) AND NOT EXISTS (
    SELECT 1 FROM semantic_entries e WHERE e.brain_id=$1 AND e.profile_id=$3::uuid
      AND e.kind='claim_revision' AND e.input_id=c.current_revision
  )
), missing_facts AS MATERIALIZED (
  SELECT f.id,f.snapshot_id,f.brain_id FROM repository_facts f
  WHERE f.brain_id=$1 AND 'repository'=ANY($2::text[]) AND NOT EXISTS (
    SELECT 1 FROM semantic_entries e WHERE e.brain_id=$1 AND e.profile_id=$3::uuid
      AND e.kind='repository_fact' AND e.input_id=f.id
  )
), missing_manifests AS MATERIALIZED (
  SELECT m.current_revision,m.brain_id FROM revision_manifests m
  WHERE m.brain_id=$1 AND 'repository'=ANY($2::text[]) AND NOT EXISTS (
    SELECT 1 FROM semantic_entries e WHERE e.brain_id=$1 AND e.profile_id=$3::uuid
      AND e.kind='manifest_revision' AND e.input_id=m.current_revision
  )
), known_snapshots AS (
  SELECT DISTINCT ON(repository_id) id FROM repository_snapshots
  WHERE brain_id=$1 ORDER BY repository_id,created_at DESC,id DESC
), candidates AS (
  SELECT 'source_chunk'::text AS kind,c.id AS input_id,v.id AS source_version_id,
    NULL::uuid AS claim_revision_id,NULL::uuid AS fact_id,NULL::uuid AS manifest_revision_id,
    v.retention_class AS class,v.created_at
  FROM missing_chunks c CROSS JOIN LATERAL (
    SELECT v.* FROM source_versions v WHERE v.id=c.version_id AND v.brain_id=c.brain_id LIMIT 1
  ) v
  JOIN sources s ON s.current_version=v.id AND s.brain_id=v.brain_id
  WHERE v.retention_class=ANY($2::text[]) AND v.processing='ready' AND v.artifact_id IS NOT NULL
    AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
    AND NOT EXISTS(SELECT 1 FROM model_input_fences f WHERE f.brain_id=v.brain_id AND f.source_version_id=v.id)
  UNION ALL
  SELECT 'claim_revision',r.id,NULL,r.id,NULL,NULL,'claim',r.recorded_at
  FROM missing_claims c CROSS JOIN LATERAL (
    SELECT r.* FROM claim_revisions r WHERE r.id=c.current_revision AND r.brain_id=c.brain_id LIMIT 1
  ) r
  WHERE recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
    AND recollect_memory_supported(r.brain_id,r.id)
    AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
    AND r.revision#>>'{content,freshness}'<>'superseded'
    AND NOT EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=r.brain_id AND f.revision_id=r.id)
  UNION ALL
  SELECT 'repository_fact',f.id,NULL,NULL,f.id,NULL,'repository',s.created_at
  FROM missing_facts f CROSS JOIN LATERAL (
    SELECT s.* FROM repository_snapshots s WHERE s.id=f.snapshot_id AND s.brain_id=f.brain_id LIMIT 1
  ) s
  WHERE (s.id IN(SELECT id FROM known_snapshots) OR EXISTS(
    SELECT 1 FROM revision_manifests m JOIN manifest_revisions r ON r.id=m.current_revision AND r.brain_id=m.brain_id
    CROSS JOIN LATERAL jsonb_array_elements(r.revision->'entries') entry
    WHERE r.brain_id=$1 AND r.privacy_state='active' AND entry->>'snapshot_id'=s.id::text))
    AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'
  UNION ALL
  SELECT 'manifest_revision',r.id,NULL,NULL,NULL,r.id,'repository',r.created_at
  FROM missing_manifests m CROSS JOIN LATERAL (
    SELECT r.* FROM manifest_revisions r WHERE r.id=m.current_revision AND r.brain_id=m.brain_id LIMIT 1
  ) r
  WHERE r.privacy_state='active'
) SELECT * FROM candidates WHERE class=ANY($2::text[])
