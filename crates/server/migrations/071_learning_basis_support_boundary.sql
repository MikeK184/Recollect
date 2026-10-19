-- A revision published by learning reconciliation is linked by a
-- claim_contributions edge (autonomous::basis) to every reconciliation input the
-- run offered: the revision it replaces or retires, which belongs to its own
-- claim, and the other offered revisions, which belong to other claims of the
-- same session. Those edges are provenance. They keep every offered input an
-- exact contributor for erasure, so erasing an offered input's source still
-- erases the published revision, and they supply the support audit's
-- exact-lineage prerequisites and cycle gate. They are not evidence the
-- published revision needs: a published claim's support is verified on its own
-- through its supports and verdicts, and the supersession contract says a
-- replacement must not inherit the replaced revision's source expiry. Until now
-- the support and deadline closure followed every contribution edge, so a
-- published revision became unsupported (and its deadline moved into the past)
-- as soon as the raw session that supported any offered input expired.
--
-- Edge classification. claim_contributions has no kind column and two writers:
-- procedures::append writes the contributions of a handover (including a
-- digest), only when the revision's content carries a handover section, which
-- procedures::validate ties one-to-one to content.kind='handover', and it
-- rejects an input of the same claim or of kind handover; autonomous::basis
-- writes the offered inputs to each published replacement, retirement and new
-- claim, whose kind the extraction schema restricts to claim, decision and
-- procedure and whose replaced or retired target must keep that kind. An edge
-- is therefore a procedure contribution exactly when its parent revision is a
-- handover, and a learning basis edge exactly when it is not. Handover and
-- digest contributions, supports, review decisions, rules, manifests, digest
-- currency and the cycle gate are unchanged dependencies. A contributor's own
-- basis edges are provenance too, so they are not followed from a handover.
--
-- The full graph stays in recollect_memory_dependencies,
-- recollect_memory_exact_dependencies, recollect_memory_exact_acyclic,
-- recollect_support_audit_target and the privacy closures. A cycle through a
-- basis edge still fails the root's exact acyclic gate, which walks the full
-- exact graph.

-- The exact-contribution and current-head closure of
-- recollect_memory_dependencies restricted to procedure contributions: only a
-- handover (or digest) revision's contributions, and each contributor's current
-- head, are followed. A learning basis edge is provenance and is not followed.
-- Invoker rights.
CREATE FUNCTION recollect_memory_support_dependencies(b uuid,target uuid) RETURNS TABLE(revision_id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE dependencies(id) AS (
   SELECT target UNION
   SELECT e.child FROM dependencies d
   JOIN claim_revisions p ON p.brain_id=b AND p.id=d.id AND p.revision#>>'{content,kind}'='handover'
   CROSS JOIN LATERAL (
     SELECT c.input_revision_id child FROM claim_contributions c
       WHERE c.brain_id=b AND c.revision_id=d.id
     UNION
     SELECT h.current_revision FROM claim_contributions c
       JOIN claim_revisions i ON i.brain_id=c.brain_id AND i.id=c.input_revision_id
       JOIN claims h ON h.brain_id=i.brain_id AND h.id=i.claim_id
       WHERE c.brain_id=b AND c.revision_id=d.id
   ) e
 ) SELECT id FROM dependencies
$$;
REVOKE ALL ON FUNCTION recollect_memory_support_dependencies(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_memory_support_dependencies(uuid,uuid) TO recollect_app;

-- The 046 body over the support closure.
CREATE OR REPLACE FUNCTION recollect_pre_digest_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_revision_supported(b,target) THEN NOT EXISTS(
   SELECT 1 FROM recollect_memory_support_dependencies(b,target) d
   LEFT JOIN LATERAL (
     SELECT r.revision FROM claim_revisions r WHERE r.brain_id=b AND r.id=d.revision_id LIMIT 1
   ) r ON true
   WHERE CASE WHEN d.revision_id=target THEN false ELSE NOT recollect_revision_supported(b,d.revision_id) END
     OR NOT recollect_memory_exact_acyclic(b,d.revision_id)
     OR (d.revision_id<>target AND r.revision#>>'{content,kind}'='handover')) ELSE false END
$$;

-- The 068 body with digest currency over the support closure.
CREATE OR REPLACE FUNCTION recollect_memory_supported_core(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE WHEN recollect_pre_digest_supported(b,target) THEN NOT EXISTS(
  SELECT 1 FROM recollect_memory_support_dependencies(b,target) d WHERE NOT recollect_digest_current(b,d.revision_id)) ELSE false END
$$;

-- The 046 body over the support closure: the revision's own claim lifetime and
-- evidence, and those of every genuine dependency, but not of a learning
-- reconciliation input offered to it.
CREATE OR REPLACE FUNCTION recollect_memory_deadline(b uuid,target uuid) RETURNS timestamptz
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH dependencies(id) AS MATERIALIZED (
   SELECT revision_id FROM recollect_memory_support_dependencies(b,target)
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

-- The 068 batch function with the same closure edges as the scalar function.
CREATE OR REPLACE FUNCTION recollect_memory_supported_many(b uuid,targets uuid[]) RETURNS TABLE(revision_id uuid,supported boolean)
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
  SELECT c.root,e.child FROM closure c
  JOIN claim_revisions p ON p.brain_id=b AND p.id=c.id AND p.revision#>>'{content,kind}'='handover'
  CROSS JOIN LATERAL (
   SELECT x.input_revision_id child FROM claim_contributions x
    WHERE x.brain_id=b AND x.revision_id=c.id
   UNION
   SELECT h.current_revision FROM claim_contributions x
    JOIN claim_revisions i ON i.brain_id=x.brain_id AND i.id=x.input_revision_id
    JOIN claims h ON h.brain_id=i.brain_id AND h.id=i.claim_id
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
