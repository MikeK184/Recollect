-- The root was already checked by CASE. Keep its cycle gate, but avoid a second
-- complete positive support assessment and a Brain-wide revision join. Exact
-- and current-head descendants still pass every original canonical gate.
CREATE OR REPLACE FUNCTION recollect_pre_digest_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_revision_supported(b,target) THEN NOT EXISTS(
   SELECT 1 FROM recollect_memory_dependencies(b,target) d
   LEFT JOIN LATERAL (
     SELECT r.revision FROM claim_revisions r WHERE r.brain_id=b AND r.id=d.revision_id LIMIT 1
   ) r ON true
   WHERE CASE WHEN d.revision_id=target THEN false ELSE NOT recollect_revision_supported(b,d.revision_id) END
     OR NOT recollect_memory_exact_acyclic(b,d.revision_id)
     OR (d.revision_id<>target AND r.revision#>>'{content,kind}'='handover')) ELSE false END
$$;

-- Resolve each member of the exact/current-head closure before evaluating its
-- RLS-protected metadata. LIMIT 1 is justified only by an immutable primary
-- key; support sets and manifest entries retain every row. Invoker RLS remains.
CREATE OR REPLACE FUNCTION recollect_memory_deadline(b uuid,target uuid) RETURNS timestamptz
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH dependencies(id) AS MATERIALIZED (
   SELECT revision_id FROM recollect_memory_dependencies(b,target)
 ), revisions AS MATERIALIZED (
   SELECT r.* FROM dependencies d CROSS JOIN LATERAL (
     SELECT r.id,r.brain_id,r.recorded_at,r.revision FROM claim_revisions r
     WHERE r.brain_id=b AND r.id=d.id LIMIT 1
   ) r
 ), supports AS MATERIALIZED (
   SELECT s.* FROM dependencies d CROSS JOIN LATERAL (
     SELECT s.* FROM claim_supports s
     WHERE s.brain_id=b AND s.revision_id=d.id OFFSET 0
   ) s
 ), manifests(id,selected) AS (
   SELECT (r.revision#>>'{content,manifest_revision_id}')::uuid,
     ARRAY(SELECT jsonb_array_elements_text(r.revision#>'{content,selection,repository_ids}')::uuid)
   FROM revisions r
   UNION SELECT s.manifest_revision_id,NULL::uuid[] FROM supports s
 ), deadlines(deadline) AS (
   SELECT recollect_retention_deadline(r.brain_id,'claim',r.recorded_at) FROM revisions r
   UNION ALL
   SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)
   FROM supports s CROSS JOIN LATERAL (
     SELECT v.* FROM source_versions v WHERE v.brain_id=b AND v.id=s.source_version_id LIMIT 1
   ) v WHERE NOT recollect_reviewed_revision(b,s.revision_id)
     OR recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)='active'
   UNION ALL
   SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at)
   FROM supports s CROSS JOIN LATERAL (
     SELECT f.snapshot_id FROM repository_facts f WHERE f.brain_id=b AND f.id=s.fact_id LIMIT 1
   ) f CROSS JOIN LATERAL (
     SELECT p.* FROM repository_snapshots p WHERE p.brain_id=b AND p.id=f.snapshot_id LIMIT 1
   ) p WHERE NOT recollect_reviewed_revision(b,s.revision_id)
     OR recollect_content_state(b,'repository',p.privacy_state,p.created_at)='active'
   UNION ALL
   SELECT recollect_retention_deadline(p.brain_id,'repository',p.created_at)
   FROM manifests x CROSS JOIN LATERAL (
     SELECT m.revision FROM manifest_revisions m WHERE m.brain_id=b AND m.id=x.id LIMIT 1
   ) m CROSS JOIN LATERAL jsonb_array_elements(coalesce(m.revision->'entries','[]')) e
   CROSS JOIN LATERAL (
     SELECT p.* FROM repository_snapshots p WHERE p.brain_id=b AND p.id=(e->>'snapshot_id')::uuid LIMIT 1
   ) p WHERE x.selected IS NULL OR (e->>'repository_id')::uuid=ANY(x.selected)
 ) SELECT min(deadline) FROM deadlines
$$;
