-- Guaranteed false-only early exits; all positive canonical gates remain.
-- Walk the same exact/current-head edges from each reachable parent instead
-- of materializing every contributor edge in the Brain for a leaf lookup.
CREATE OR REPLACE FUNCTION recollect_memory_dependencies(b uuid,target uuid) RETURNS TABLE(revision_id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE dependencies(id) AS (
   SELECT target UNION
   SELECT e.child FROM dependencies d CROSS JOIN LATERAL (
     SELECT c.input_revision_id child FROM claim_contributions c WHERE c.brain_id=b AND c.revision_id=d.id
     UNION
     SELECT h.current_revision FROM claim_contributions c
       JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.input_revision_id
       JOIN claims h ON h.brain_id=r.brain_id AND h.id=r.claim_id
       WHERE c.brain_id=b AND c.revision_id=d.id
   ) e
 ) SELECT id FROM dependencies
$$;

CREATE OR REPLACE FUNCTION recollect_revision_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_reviewed_revision(b,target) OR EXISTS(
   SELECT 1 FROM memory_support_assessments a JOIN model_policy_heads p ON p.brain_id=a.brain_id AND p.policy_id=a.policy_id
   WHERE a.brain_id=b AND a.revision_id=target AND a.verifier_version='source-support-3'
     AND a.state='succeeded' AND a.disposition='supported' AND a.privacy_state='active') THEN (
 SELECT EXISTS(SELECT 1 FROM claim_revisions WHERE brain_id=b AND id=target)
 AND NOT EXISTS(SELECT 1 FROM claim_revisions r WHERE r.id=target AND r.brain_id=b
   AND (recollect_content_state(b,'claim',r.privacy_state,r.recorded_at)<>'active'
   OR r.revision->>'review'='rejected' OR coalesce(r.revision->>'lifecycle','active')<>'active'
   OR r.revision#>>'{content,freshness}'='superseded'
   OR NOT recollect_selection_valid(b,r.revision#>'{content,selection}')
   OR recollect_rule_blocks(b,r.id)
   OR EXISTS(SELECT 1 FROM model_claim_fences f WHERE f.brain_id=b AND f.revision_id=r.id)
   OR (r.revision#>>'{content,manifest_revision_id}' IS NOT NULL AND NOT recollect_manifest_supported(b,(r.revision#>>'{content,manifest_revision_id}')::uuid,
       ARRAY(SELECT jsonb_array_elements_text(r.revision#>'{content,selection,repository_ids}')::uuid)))
   OR (r.revision#>>'{content,manifest_revision_id}' IS NOT NULL AND NOT EXISTS(SELECT 1 FROM manifest_revisions m
     WHERE m.brain_id=b AND m.id=(r.revision#>>'{content,manifest_revision_id}')::uuid
       AND m.revision->>'environment_id' IS NOT DISTINCT FROM r.revision#>>'{content,selection,environment_id}'))
   OR NOT (recollect_reviewed_revision(b,r.id) OR EXISTS(
     SELECT 1 FROM memory_support_assessments a JOIN model_policy_heads p ON p.brain_id=a.brain_id AND p.policy_id=a.policy_id
     WHERE a.brain_id=b AND a.revision_id=r.id AND a.verifier_version='source-support-3'
       AND a.state='succeeded' AND a.disposition='supported' AND a.privacy_state='active'))
   OR EXISTS(SELECT 1 FROM claim_supports s LEFT JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=b
     LEFT JOIN repository_facts f ON f.id=s.fact_id AND f.brain_id=b
     LEFT JOIN repository_snapshots p ON p.id=f.snapshot_id AND p.brain_id=b
     LEFT JOIN manifest_revisions m ON m.id=s.manifest_revision_id AND m.brain_id=b
     WHERE s.brain_id=b AND s.revision_id=r.id AND (
       (s.source_version_id IS NOT NULL AND (v.id IS NULL OR (v.artifact_id IS NULL AND NOT recollect_reviewed_revision(b,r.id))
        OR (recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)<>'active'
          AND NOT (recollect_reviewed_revision(b,r.id)
            AND recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)='expired'))
        OR EXISTS(SELECT 1 FROM model_input_fences x WHERE x.brain_id=b AND x.source_version_id=v.id)))
       OR (s.fact_id IS NOT NULL AND (p.id IS NULL
         OR (recollect_content_state(b,'repository',p.privacy_state,p.created_at)<>'active'
           AND NOT (recollect_reviewed_revision(b,r.id)
             AND recollect_content_state(b,'repository',p.privacy_state,p.created_at)='expired'))))
       OR (s.manifest_revision_id IS NOT NULL AND NOT recollect_manifest_supported(b,s.manifest_revision_id))))))
 ) ELSE false END
$$;

CREATE OR REPLACE FUNCTION recollect_pre_digest_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_revision_supported(b,target) THEN NOT EXISTS(
   SELECT 1 FROM recollect_memory_dependencies(b,target) d
   LEFT JOIN claim_revisions r ON r.brain_id=b AND r.id=d.revision_id
   WHERE NOT recollect_revision_supported(b,d.revision_id) OR NOT recollect_memory_exact_acyclic(b,d.revision_id)
     OR (d.revision_id<>target AND r.revision#>>'{content,kind}'='handover')) ELSE false END
$$;

CREATE OR REPLACE FUNCTION recollect_memory_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_pre_digest_supported(b,target) THEN NOT EXISTS(
  SELECT 1 FROM recollect_memory_dependencies(b,target) d WHERE NOT recollect_digest_current(b,d.revision_id)) ELSE false END
$$;
