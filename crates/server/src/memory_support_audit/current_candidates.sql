-- Do not requalify revisions with an already recorded exact policy/verifier
-- attempt, regardless of its state. Missing identities remain provisional.
WITH missing_heads AS MATERIALIZED (
  SELECT c.current_revision FROM claims c
  WHERE c.brain_id=$1 AND NOT EXISTS (
    SELECT 1 FROM memory_support_assessments a WHERE a.brain_id=$1
      AND a.revision_id=c.current_revision AND a.policy_id=$2 AND a.verifier_version=$3
  )
), ordered_candidates AS MATERIALIZED (
  SELECT r.* FROM missing_heads h CROSS JOIN LATERAL (
    SELECT r.* FROM claim_revisions r
    WHERE r.brain_id=$1 AND r.id=h.current_revision LIMIT 1
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
