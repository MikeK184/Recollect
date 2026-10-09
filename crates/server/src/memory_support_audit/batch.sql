-- Bounds examination, not a candidate's closure. Unavailable rows do not pin
-- the next pass to the same prefix. No metadata/support functions run here.
SELECT r.id FROM claim_revisions r
WHERE r.brain_id=$1 AND ($4::uuid IS NULL OR r.id>$4)
 AND ($6::uuid IS NULL OR r.id<=$6)
 AND NOT EXISTS (
  SELECT 1 FROM memory_support_assessments a WHERE a.brain_id=$1
   AND a.revision_id=r.id AND a.policy_id=$2 AND a.verifier_version=$3
 )
ORDER BY r.id LIMIT $5
