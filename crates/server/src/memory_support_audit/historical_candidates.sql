-- Retain the complete root and exact/current-head target closure. Only missing
-- historical identities need metadata/support qualification, before capacity.
WITH RECURSIVE roots AS MATERIALIZED (
  SELECT r.id FROM claims c CROSS JOIN LATERAL (
    SELECT r.* FROM claim_revisions r
    WHERE r.brain_id=c.brain_id AND r.id=c.current_revision LIMIT 1
  ) r
  WHERE c.brain_id=$1
    AND recollect_content_state($1,'claim',r.privacy_state,r.recorded_at)='active'
    AND r.revision->>'review'<>'rejected'
    AND coalesce(r.revision->>'lifecycle','active')='active'
), edges AS MATERIALIZED (
  SELECT revision_id parent,input_revision_id child FROM claim_contributions WHERE brain_id=$1
  UNION SELECT d.revision_id,c.current_revision FROM claim_contributions d
  CROSS JOIN LATERAL (
    SELECT r.claim_id FROM claim_revisions r
    WHERE r.brain_id=d.brain_id AND r.id=d.input_revision_id LIMIT 1
  ) r CROSS JOIN LATERAL (
    SELECT c.current_revision FROM claims c
    WHERE c.brain_id=d.brain_id AND c.id=r.claim_id LIMIT 1
  ) c WHERE d.brain_id=$1
), targets(id) AS (
  SELECT id FROM roots UNION SELECT e.child FROM edges e JOIN targets t ON t.id=e.parent
), missing_targets AS MATERIALIZED (
  SELECT t.id FROM targets t WHERE t.id NOT IN (SELECT id FROM roots)
    AND NOT EXISTS (
      SELECT 1 FROM memory_support_assessments a WHERE a.brain_id=$1
        AND a.revision_id=t.id AND a.policy_id=$2 AND a.verifier_version=$3
    )
), ordered_candidates AS MATERIALIZED (
  SELECT r.* FROM missing_targets t CROSS JOIN LATERAL (
    SELECT r.* FROM claim_revisions r WHERE r.brain_id=$1 AND r.id=t.id LIMIT 1
  ) r
  WHERE recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
    AND r.revision->>'review'<>'rejected'
    AND coalesce(r.revision->>'lifecycle','active')='active'
    AND r.revision#>>'{content,freshness}'<>'superseded'
  ORDER BY r.recorded_at,r.id
) SELECT r.id FROM ordered_candidates r
WHERE NOT recollect_reviewed_revision($1,r.id)
  AND recollect_memory_exact_acyclic($1,r.id)
  AND NOT EXISTS (
    SELECT 1 FROM recollect_memory_exact_dependencies(r.brain_id,r.id) d
    WHERE d.revision_id<>r.id AND NOT recollect_revision_supported(r.brain_id,d.revision_id)
  ) LIMIT $4
