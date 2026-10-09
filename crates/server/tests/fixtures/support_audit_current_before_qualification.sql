WITH ordered_candidates AS MATERIALIZED (
       SELECT r.* FROM claim_revisions r JOIN claims c ON c.brain_id=r.brain_id AND c.current_revision=r.id
       WHERE r.brain_id=$1 AND recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
       AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active' AND r.revision#>>'{content,freshness}'<>'superseded'
       AND NOT EXISTS(SELECT 1 FROM memory_support_assessments a WHERE a.brain_id=$1 AND a.revision_id=r.id AND a.policy_id=$2 AND a.verifier_version=$3)
       ORDER BY r.recorded_at,r.id
       ) SELECT r.id FROM ordered_candidates r
       WHERE NOT recollect_reviewed_revision($1,r.id)
       AND recollect_memory_exact_acyclic($1,r.id)
       AND NOT EXISTS(SELECT 1 FROM recollect_memory_exact_dependencies(r.brain_id,r.id) d
         WHERE d.revision_id<>r.id AND NOT recollect_revision_supported(r.brain_id,d.revision_id))
       LIMIT $4
