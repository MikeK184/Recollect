-- The canonical support predicate is evaluated as the migration role after one
-- Brain membership test when the caller is subject to row-level security. Every
-- table the predicate reads has a per-row read policy that reduces to Brain
-- membership (`recollect_role(brain_id) IS NOT NULL`, or its bulk form), and
-- `recollect_role` is itself a SECURITY DEFINER function with SET clauses, so
-- evaluating the policy for each visited row cost an application-role call
-- 25-66 ms per revision against under 1 ms without row security. A member sees
-- exactly the rows the definer reads and a non-member sees none, so the result
-- is unchanged; the platform policy-guard test fails if a restrictive or
-- non-membership read policy ever appears on a table the predicate reads. As for
-- the migration 065 window functions, the definer must bypass row security (the
-- migration role does).
--
-- A caller that row-level security does not apply to (the database owner, and
-- the owner-rights maintenance functions such as the privacy closure that call
-- this predicate without a Brain actor) previously evaluated the predicate
-- unfiltered, and still does: the membership test applies exactly when the old
-- body would have been filtered by policies.

-- The unchanged predicate body, with the caller's rights.
CREATE FUNCTION recollect_memory_supported_core(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_pre_digest_supported(b,target) THEN NOT EXISTS(
  SELECT 1 FROM recollect_memory_dependencies(b,target) d WHERE NOT recollect_digest_current(b,d.revision_id)) ELSE false END
$$;

-- The same verdict for a caller subject to row-level security, evaluated with
-- the owner's rights after the membership test.
CREATE FUNCTION recollect_memory_supported_member(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_role(b) IS NULL THEN false ELSE recollect_memory_supported_core(b,target) END
$$;

CREATE OR REPLACE FUNCTION recollect_memory_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN row_security_active('claim_revisions'::regclass)
  THEN recollect_memory_supported_member(b,target) ELSE recollect_memory_supported_core(b,target) END
$$;
REVOKE ALL ON FUNCTION recollect_memory_supported_core(uuid,uuid),recollect_memory_supported_member(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_memory_supported_core(uuid,uuid),recollect_memory_supported_member(uuid,uuid),recollect_memory_supported(uuid,uuid) TO recollect_app;

-- Support for many revisions of one Brain in one statement, for callers acting
-- as a Brain member (every call site here has an actor; a caller without a role
-- on the Brain gets false, the safe answer). It returns one row
-- per distinct non-null target with exactly the scalar function's verdict: the
-- root's own gates first, then one shared exact/current-head closure whose
-- members are each evaluated once (own gates unless the member is the root,
-- exact-contribution acyclicity, no handover other than the root), then digest
-- currency for the members of the roots that passed. A non-member gets false
-- for every target. A closure that contains a NULL member (a claim without a
-- current revision) is unsupported, as in the scalar function.
CREATE FUNCTION recollect_memory_supported_many(b uuid,targets uuid[]) RETURNS TABLE(revision_id uuid,supported boolean)
LANGUAGE sql STABLE SECURITY DEFINER ROWS 100 SET search_path=public,pg_temp AS $$
 WITH RECURSIVE roots AS MATERIALIZED (
  SELECT DISTINCT t.id FROM unnest(targets) t(id) WHERE t.id IS NOT NULL
 ), member AS MATERIALIZED (
  SELECT recollect_role(b) IS NOT NULL AS ok
 ), qualified AS MATERIALIZED (
  SELECT r.id FROM roots r
  WHERE CASE WHEN (SELECT ok FROM member) THEN recollect_revision_supported(b,r.id) ELSE false END
 ), closure(root,id) AS (
  SELECT q.id,q.id FROM qualified q
  UNION
  SELECT c.root,e.child FROM closure c CROSS JOIN LATERAL (
   SELECT x.input_revision_id child FROM claim_contributions x WHERE x.brain_id=b AND x.revision_id=c.id
   UNION
   SELECT h.current_revision FROM claim_contributions x
    JOIN claim_revisions r ON r.brain_id=x.brain_id AND r.id=x.input_revision_id
    JOIN claims h ON h.brain_id=r.brain_id AND h.id=r.claim_id
    WHERE x.brain_id=b AND x.revision_id=c.id
  ) e
 ), members AS MATERIALIZED (
  SELECT DISTINCT c.id FROM closure c WHERE c.id IS NOT NULL
 ), shape AS MATERIALIZED (
  SELECT m.id,recollect_revision_supported(b,m.id) AS own,recollect_memory_exact_acyclic(b,m.id) AS acyclic,
   (SELECT r.revision#>>'{content,kind}' FROM claim_revisions r WHERE r.brain_id=b AND r.id=m.id LIMIT 1) AS kind
  FROM members m
 ), structural AS MATERIALIZED (
  SELECT q.id FROM qualified q WHERE NOT EXISTS(
   SELECT 1 FROM closure c LEFT JOIN shape s ON s.id=c.id
   WHERE c.root=q.id AND (c.id IS NULL OR (CASE WHEN c.id=c.root THEN false ELSE NOT s.own END)
    OR NOT s.acyclic OR (c.id<>c.root AND s.kind='handover')) IS TRUE)
 ), current_digest AS MATERIALIZED (
  SELECT m.id,recollect_digest_current(b,m.id) AS fresh
  FROM (SELECT DISTINCT c.id FROM closure c JOIN structural s ON s.id=c.root WHERE c.id IS NOT NULL) m
 )
 SELECT r.id,EXISTS(SELECT 1 FROM structural s WHERE s.id=r.id)
  AND NOT EXISTS(SELECT 1 FROM closure c JOIN current_digest d ON d.id=c.id WHERE c.root=r.id AND NOT d.fresh)
 FROM roots r
$$;
REVOKE ALL ON FUNCTION recollect_memory_supported_many(uuid,uuid[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_memory_supported_many(uuid,uuid[]) TO recollect_app;

-- The organization field view pages knowledge_field_inputs by (brain, id). The
-- primary key alone reads every other Brain's inputs below the cursor and
-- filters them out; this keyset index reads exactly the page (60,000 inputs in
-- 20 Brains: 4,500 rows read and 1.7 ms before, 251 rows and 0.27 ms after).
CREATE INDEX knowledge_field_inputs_page ON knowledge_field_inputs(brain_id,id);
