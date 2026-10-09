-- Preserve the exact predicate from 037 while avoiding one dependency scan per
-- current root. Current revisions are the common case during legacy cutover;
-- historical contributors retain the full exact/current-head closure.
CREATE OR REPLACE FUNCTION recollect_support_audit_target(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN EXISTS (
   SELECT 1 FROM claim_revisions r JOIN claims c ON c.brain_id=r.brain_id AND c.current_revision=r.id
   WHERE r.brain_id=b AND r.id=target
     AND recollect_content_state(b,'claim',r.privacy_state,r.recorded_at)='active'
     AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
 ) THEN true ELSE (
   WITH RECURSIVE roots AS MATERIALIZED (
     SELECT r.id FROM claims c JOIN claim_revisions r ON r.brain_id=c.brain_id AND r.id=c.current_revision
     WHERE c.brain_id=b AND recollect_content_state(b,'claim',r.privacy_state,r.recorded_at)='active'
       AND r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
   ), edges AS MATERIALIZED (
     SELECT revision_id parent,input_revision_id child FROM claim_contributions WHERE brain_id=b
     UNION SELECT d.revision_id,c.current_revision FROM claim_contributions d
       JOIN claim_revisions r ON r.brain_id=d.brain_id AND r.id=d.input_revision_id
       JOIN claims c ON c.brain_id=r.brain_id AND c.id=r.claim_id WHERE d.brain_id=b
   ), targets(id) AS (
     SELECT id FROM roots UNION SELECT e.child FROM edges e JOIN targets t ON t.id=e.parent
   ) SELECT EXISTS(SELECT 1 FROM targets WHERE id=target)
 ) END
$$;
