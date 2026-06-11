-- Lexical representations derive directly from their canonical row. Clearing
-- canonical payloads also clears their postings; there is no independent writer.
ALTER TABLE claim_revisions ADD COLUMN recall_vector tsvector GENERATED ALWAYS AS (
  setweight(to_tsvector('simple'::regconfig,
    coalesce(revision#>>'{content,subject}','') || ' ' || coalesce(revision#>>'{content,predicate}','')), 'A') ||
  to_tsvector('simple'::regconfig,
    coalesce(revision#>>'{content,value}','') || ' ' || coalesce(revision#>>'{content,rationale}','') || ' ' ||
    coalesce((revision#>'{content,procedure}')::text,'') || ' ' || coalesce((revision#>'{content,handover}')::text,''))
) STORED;
CREATE INDEX claim_recall_lexical ON claim_revisions USING GIN(recall_vector);
CREATE INDEX claim_recall_time ON claim_revisions(brain_id,claim_id,recorded_at DESC);

ALTER TABLE source_versions ADD COLUMN recall_vector tsvector GENERATED ALWAYS AS (
  setweight(to_tsvector('simple'::regconfig,title),'A')
) STORED;
ALTER TABLE source_chunks ADD COLUMN recall_vector tsvector GENERATED ALWAYS AS (
  to_tsvector('simple'::regconfig,content)
) STORED;
CREATE INDEX source_recall_title ON source_versions USING GIN(recall_vector);
CREATE INDEX source_recall_lexical ON source_chunks USING GIN(recall_vector);
CREATE INDEX source_recall_time ON source_versions(brain_id,source_id,created_at DESC,id DESC);

ALTER TABLE repository_facts ADD COLUMN recall_vector tsvector GENERATED ALWAYS AS (
  setweight(to_tsvector('simple'::regconfig,coalesce(record->>'name','')),'A') ||
  to_tsvector('simple'::regconfig,left(record::text,65536))
) STORED;
CREATE INDEX fact_recall_lexical ON repository_facts USING GIN(recall_vector);
ALTER TABLE manifest_revisions ADD COLUMN recall_vector tsvector GENERATED ALWAYS AS (
  setweight(to_tsvector('simple'::regconfig,coalesce(revision->>'name','')),'A') ||
  to_tsvector('simple'::regconfig,left(revision::text,65536))
) STORED;
CREATE INDEX manifest_recall_lexical ON manifest_revisions USING GIN(recall_vector);

-- Scope is an applicability filter. This immutable helper reads no table and
-- confers no authority; all callers use authenticated Brain RLS transactions.
CREATE FUNCTION recollect_recall_scope(candidate jsonb, requested jsonb) RETURNS boolean
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
  SELECT (candidate->>'environment_id' IS NULL OR requested->>'environment_id' IS NULL
      OR candidate->>'environment_id'=requested->>'environment_id')
    AND (coalesce(candidate->'repository_ids','[]')='[]'::jsonb
      OR coalesce(requested->'repository_ids','[]')='[]'::jsonb
      OR (candidate->'repository_ids') ?| ARRAY(SELECT jsonb_array_elements_text(requested->'repository_ids')))
    AND (coalesce(candidate->'area_ids','[]')='[]'::jsonb
      OR coalesce(requested->'area_ids','[]')='[]'::jsonb
      OR (candidate->'area_ids') ?| ARRAY(SELECT jsonb_array_elements_text(requested->'area_ids')))
$$;
