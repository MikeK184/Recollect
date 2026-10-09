-- A single identity still receives the whole immutable target/prerequisite
-- closure. Used both provisionally and under fresh writer authority.
SELECT r.id FROM (
 SELECT r.* FROM claim_revisions r WHERE r.brain_id=$1 AND r.id=$4 LIMIT 1
) r WHERE
 recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
 AND r.revision->>'review'<>'rejected'
 AND coalesce(r.revision->>'lifecycle','active')='active'
 AND r.revision#>>'{content,freshness}'<>'superseded'
 AND NOT EXISTS (
  SELECT 1 FROM memory_support_assessments a WHERE a.brain_id=$1
   AND a.revision_id=r.id AND a.policy_id=$2 AND a.verifier_version=$3
 )
 AND recollect_support_audit_target($1,r.id)
 AND NOT recollect_reviewed_revision($1,r.id)
 AND recollect_memory_exact_acyclic($1,r.id)
 AND NOT EXISTS (
  SELECT 1 FROM recollect_memory_exact_dependencies(r.brain_id,r.id) d
  WHERE d.revision_id<>r.id AND NOT recollect_revision_supported(r.brain_id,d.revision_id)
 )
