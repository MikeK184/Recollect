-- Set-based recall read path. Functions are pure; indexes serve the lexical
-- candidate window and exact-literal/coverage lookups. No data is rewritten and
-- no new authority is introduced: every read still runs in a Brain RLS transaction.

-- The single definition of the claim status predicate for a recall mode, shared by
-- candidate construction and the project brief (which evaluates strict modes).
CREATE FUNCTION recollect_claim_status_eligible(revision jsonb, mode text) RETURNS boolean
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
  SELECT mode='history' OR (
    revision->>'review'<>'rejected' AND coalesce(revision->>'lifecycle','active')='active'
    AND revision#>>'{content,freshness}'<>'superseded'
    AND (mode='investigation' OR (revision->>'review'='accepted'
      AND (revision->>'reviewer_id' IS NOT NULL OR length(btrim(coalesce(revision->>'acceptance_policy','')))>0)
      AND revision#>>'{content,freshness}'='current'
      AND (mode<>'strict_operational' OR (revision#>>'{content,operational}'='verified'
        AND revision#>>'{content,observed_at}' IS NOT NULL
        AND length(btrim(coalesce(revision#>>'{content,observation}','')))>0))
    ))
  )
$$;

-- A source candidate matches on the concatenation of its version (title) and
-- chunk vectors, so no single GIN index can answer it. This relaxes a parsed
-- query to the OR of its own lexemes: any vector that satisfies the query
-- contains at least one of them, so the OR over the two per-table indexes is a
-- superset window; the exact @@ recheck on the concatenation remains. NULL means
-- absence of every lexeme can satisfy the query (pure negation), which no index
-- can window, so callers fall back to the unwindowed scan.
CREATE FUNCTION recollect_lexical_prefilter(q tsquery) RETURNS tsquery
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
  SELECT CASE
    WHEN numnode(q)=0 THEN q
    WHEN ''::tsvector @@ q THEN NULL
    ELSE (SELECT string_agg(m.g[1],' | ') FROM regexp_matches(q::text,'(''(?:[^'']|'''')*'')','g') AS m(g))::tsquery
  END
$$;
REVOKE ALL ON FUNCTION recollect_claim_status_eligible(jsonb,text),recollect_lexical_prefilter(tsquery) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_claim_status_eligible(jsonb,text),recollect_lexical_prefilter(tsquery) TO recollect_app;

-- Row-level security forces the app role's scans through a non-leakproof
-- barrier: `tsvector @@ tsquery`, jsonb extraction and the like cannot be index
-- conditions there, so the GIN indexes above were never eligible. These windows
-- find candidate identities as the migration role, after the same
-- `recollect_role` membership test the table policies apply, and return only
-- identifiers. Candidate construction re-reads every row through RLS, so a window
-- can narrow the scan but never widen what a caller can see. A NULL argument
-- disables that branch.
CREATE FUNCTION recollect_window_claims(b uuid, q tsquery, literal text, exact uuid) RETURNS SETOF uuid
LANGUAGE plpgsql STABLE SECURITY DEFINER ROWS 10 SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(b) IS NULL THEN RETURN; END IF;
  RETURN QUERY
    SELECT claim_id FROM claim_revisions WHERE brain_id=b AND q IS NOT NULL AND recall_vector @@ q
    UNION SELECT claim_id FROM claim_revisions WHERE brain_id=b AND literal IS NOT NULL AND revision#>>'{content,subject}'=literal
    UNION SELECT exact WHERE exact IS NOT NULL;
END $$;
CREATE FUNCTION recollect_window_sources(b uuid, q tsquery, literal text, exact uuid) RETURNS SETOF uuid
LANGUAGE plpgsql STABLE SECURITY DEFINER ROWS 10 SET search_path=public,pg_temp AS $$
DECLARE rq tsquery := recollect_lexical_prefilter(q);
BEGIN
  IF recollect_role(b) IS NULL THEN RETURN; END IF;
  RETURN QUERY
    SELECT source_id FROM source_versions WHERE brain_id=b AND rq IS NOT NULL AND recall_vector @@ rq
    UNION SELECT v.source_id FROM source_chunks c JOIN source_versions v ON v.id=c.version_id AND v.brain_id=c.brain_id
      WHERE c.brain_id=b AND rq IS NOT NULL AND c.recall_vector @@ rq
    UNION SELECT source_id FROM source_versions WHERE brain_id=b AND q IS NOT NULL AND rq IS NULL
    UNION SELECT source_id FROM source_versions WHERE brain_id=b AND literal IS NOT NULL AND title=literal
    UNION SELECT source_id FROM source_versions WHERE brain_id=b AND exact IS NOT NULL AND id=exact
    UNION SELECT source_id FROM source_versions WHERE brain_id=b AND processing<>'ready';
END $$;
CREATE FUNCTION recollect_window_facts(b uuid, q tsquery, literal text, exact uuid) RETURNS SETOF uuid
LANGUAGE plpgsql STABLE SECURITY DEFINER ROWS 10 SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(b) IS NULL THEN RETURN; END IF;
  RETURN QUERY
    SELECT id FROM repository_facts WHERE brain_id=b AND q IS NOT NULL AND recall_vector @@ q
    UNION SELECT id FROM repository_facts WHERE brain_id=b AND literal IS NOT NULL
      AND (record->>'name'=literal OR coalesce(record->>'file',record#>>'{source,path}',record->>'path')=literal)
    UNION SELECT id FROM repository_facts WHERE brain_id=b AND exact IS NOT NULL AND id=exact
    UNION SELECT id FROM repository_facts WHERE brain_id=b AND length(record::text)>65536;
END $$;
CREATE FUNCTION recollect_window_manifests(b uuid, q tsquery, literal text, exact uuid) RETURNS SETOF uuid
LANGUAGE plpgsql STABLE SECURITY DEFINER ROWS 10 SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(b) IS NULL THEN RETURN; END IF;
  RETURN QUERY
    SELECT id FROM manifest_revisions WHERE brain_id=b AND q IS NOT NULL AND recall_vector @@ q
    UNION SELECT id FROM manifest_revisions WHERE brain_id=b AND literal IS NOT NULL AND revision->>'name'=literal
    UNION SELECT id FROM manifest_revisions WHERE brain_id=b AND exact IS NOT NULL AND id=exact
    UNION SELECT id FROM manifest_revisions WHERE brain_id=b AND length(revision::text)>65536;
END $$;
REVOKE ALL ON FUNCTION recollect_window_claims(uuid,tsquery,text,uuid),recollect_window_sources(uuid,tsquery,text,uuid),
  recollect_window_facts(uuid,tsquery,text,uuid),recollect_window_manifests(uuid,tsquery,text,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_window_claims(uuid,tsquery,text,uuid),recollect_window_sources(uuid,tsquery,text,uuid),
  recollect_window_facts(uuid,tsquery,text,uuid),recollect_window_manifests(uuid,tsquery,text,uuid) TO recollect_app;

-- Exact-literal identities (title/name/path/subject) and coverage rows are
-- found without scanning every Brain row.
CREATE INDEX claim_recall_subject ON claim_revisions USING HASH((revision#>>'{content,subject}'));
CREATE INDEX source_recall_title_literal ON source_versions USING HASH(title);
CREATE INDEX fact_recall_name ON repository_facts USING HASH((record->>'name'));
CREATE INDEX fact_recall_path ON repository_facts USING HASH((coalesce(record->>'file',record#>>'{source,path}',record->>'path')));
CREATE INDEX manifest_recall_name ON manifest_revisions USING HASH((revision->>'name'));
CREATE INDEX source_versions_unprocessed ON source_versions(brain_id) WHERE processing<>'ready';
CREATE INDEX fact_recall_oversize ON repository_facts(brain_id) WHERE length(record::text)>65536;
CREATE INDEX manifest_recall_oversize ON manifest_revisions(brain_id) WHERE length(revision::text)>65536;
