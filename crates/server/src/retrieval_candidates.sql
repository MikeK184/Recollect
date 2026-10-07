WITH captured_sources AS MATERIALIZED (
  -- Read each authorized capture association once. Expanding these joins for
  -- every candidate repeatedly evaluates event/binding RLS under mixed load.
  SELECT e.source_id,e.source_version_id,e.received_at,b.selection
  FROM capture_events e
  LEFT JOIN capture_bindings b ON b.id=e.binding_id AND b.brain_id=e.brain_id
  WHERE e.brain_id=$1
), source_knowledge AS MATERIALIZED (
  -- Same exact-version knowledge time as recollect_source_knowledge. The
  -- separate source-id join below preserves capture scope for later versions.
  SELECT v.*,coalesce(e.received_at,v.created_at) AS recorded_at,
    coalesce(a.selection,i.selection) AS import_selection
  FROM source_versions v
  LEFT JOIN captured_sources e ON e.source_version_id=v.id
  -- Resolve version-specific import applicability once, before the chunk
  -- fan-out, so scoped RLS is not re-evaluated for every search fragment.
  LEFT JOIN source_import_scopes i ON i.version_id=v.id AND i.brain_id=v.brain_id
  LEFT JOIN automatic_support_excerpts a ON a.version_id=v.id AND a.brain_id=v.brain_id
  WHERE v.brain_id=$1
), known_claims AS (
  SELECT DISTINCT ON(claim_id) * FROM claim_revisions
  WHERE brain_id=$1 AND recorded_at<=$2 ORDER BY claim_id,recorded_at DESC
), selected_claims AS (
  -- Apply the same exact manifest boundary as claim_manifest before any
  -- channel ranks or counts claim representations. Keep the canonical view
  -- recheck as well; similarity and projection coverage cannot supply scope.
  SELECT r.* FROM known_claims r WHERE recollect_memory_supported(r.brain_id,r.id) AND ($6::jsonb IS NULL OR (
    NOT EXISTS(SELECT 1 FROM jsonb_array_elements_text(r.revision#>'{content,selection,repository_ids}') repo
      WHERE NOT EXISTS(SELECT 1 FROM jsonb_array_elements($6->'entries') entry
        WHERE entry->>'repository_id'=repo))
    AND NOT EXISTS(SELECT 1 FROM claim_supports cs
      JOIN repository_facts f ON f.brain_id=cs.brain_id AND f.id=cs.fact_id
      JOIN repository_snapshots s ON s.brain_id=f.brain_id AND s.id=f.snapshot_id
      WHERE cs.brain_id=$1 AND cs.revision_id=r.id
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements($6->'entries') entry
          WHERE entry->>'repository_id'=s.repository_id::text
            AND entry->>'snapshot_id'=s.id::text AND entry->>'revision'=s.revision))
    AND (r.revision#>>'{content,manifest_revision_id}' IS NULL OR EXISTS(
      SELECT 1 FROM manifest_revisions original
      WHERE original.brain_id=$1 AND original.id=(r.revision#>>'{content,manifest_revision_id}')::uuid
        AND original.privacy_state='active'
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements_text(r.revision#>'{content,selection,repository_ids}') repo
          WHERE NOT EXISTS(SELECT 1 FROM jsonb_array_elements(original.revision->'entries') old_entry
            JOIN jsonb_array_elements($6->'entries') selected_entry
              ON old_entry->>'repository_id'=selected_entry->>'repository_id'
            WHERE old_entry->>'repository_id'=repo
              AND jsonb_build_object('repository_id',old_entry->'repository_id','revision',old_entry->'revision',
                'snapshot_id',old_entry->'snapshot_id','config_paths',coalesce(old_entry->'config_paths','[]'::jsonb))=selected_entry))))
  ))
), known_sources AS (
  SELECT DISTINCT ON(source_id) id FROM source_knowledge
  WHERE brain_id=$1 AND recorded_at<=$2 ORDER BY source_id,recorded_at DESC,id DESC
), known_snapshots AS (
  SELECT DISTINCT ON(repository_id) id FROM repository_snapshots
  WHERE brain_id=$1 AND created_at<=$2 ORDER BY repository_id,created_at DESC,id DESC
), known_manifests AS (
  SELECT DISTINCT ON(manifest_id) id FROM manifest_revisions
  WHERE brain_id=$1 AND created_at<=$2 ORDER BY manifest_id,created_at DESC,id DESC
), candidates AS (
  SELECT 'claim'::text kind,r.claim_id id,r.id revision_id,NULL::uuid chunk_id,
    r.revision#>>'{content,subject}' label,
    'Subject: ' || (r.revision#>>'{content,subject}') || E'\nProperty: ' || (r.revision#>>'{content,predicate}') || E'\nValue: ' ||
      (r.revision#>>'{content,value}') || E'\nRationale: ' || coalesce(r.revision#>>'{content,rationale}','') || E'\n' ||
      coalesce((r.revision#>'{content,procedure}')::text,'') || E'\n' ||
      coalesce((r.revision#>'{content,handover}')::text,'') AS text,
    r.recorded_at,r.revision#>'{content,selection}' selection,
    NULL::uuid source_id,NULL::uuid repository_id,NULL::uuid snapshot_id,NULL::text revision,
    NULL::text path,NULL::integer line_from,NULL::integer line_to,
    NULL::integer byte_from,NULL::integer byte_to,NULL::uuid artifact_id,0 byte_length,
    'ready'::text processing,r.revision data,r.recall_vector vector,
    recollect_retention_deadline(r.brain_id,'claim',r.recorded_at) expires_at,
    ($3<>'' AND r.revision#>>'{content,subject}'=$3) literal,
    ($7='history' OR (
      r.revision->>'review'<>'rejected' AND coalesce(r.revision->>'lifecycle','active')='active'
      AND r.revision#>>'{content,freshness}'<>'superseded'
      AND ($7='investigation' OR (r.revision->>'review'='accepted'
        AND (r.revision->>'reviewer_id' IS NOT NULL OR length(btrim(coalesce(r.revision->>'acceptance_policy','')))>0)
        AND r.revision#>>'{content,freshness}'='current'
        AND ($7<>'strict_operational' OR (r.revision#>>'{content,operational}'='verified'
          AND r.revision#>>'{content,observed_at}' IS NOT NULL
          AND length(btrim(coalesce(r.revision#>>'{content,observation}','')))>0))
      ))
    )) status_eligible
  FROM selected_claims r
  WHERE recollect_content_state(r.brain_id,'claim',r.privacy_state,r.recorded_at)='active'
    AND recollect_recall_scope(r.revision#>'{content,selection}',$4)
    AND ($5::uuid IS NULL OR EXISTS(SELECT 1 FROM claim_supports s
      JOIN source_versions v ON v.id=s.source_version_id AND v.brain_id=s.brain_id
      JOIN evidence_memberships m ON m.source_id=v.source_id AND m.brain_id=v.brain_id
      WHERE s.brain_id=$1 AND s.revision_id=r.id AND m.group_id=$5))

  UNION ALL

  SELECT 'source_version',v.id,v.id,c.id,v.title,coalesce(c.content,''),v.recorded_at,
    coalesce(v.import_selection,e.selection,'{}'::jsonb),v.source_id,NULL,NULL,NULL,NULL,
    c.line_start,c.line_end,c.byte_start,c.byte_end,v.artifact_id,v.byte_length,v.processing,
    jsonb_build_object('retention_class',v.retention_class),
    v.recall_vector || coalesce(c.recall_vector,''::tsvector),
    recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at),
    ($3<>'' AND v.title=$3),true
  FROM source_knowledge v
  LEFT JOIN source_chunks c ON c.version_id=v.id AND c.brain_id=v.brain_id
  LEFT JOIN captured_sources e ON e.source_id=v.source_id
  WHERE v.brain_id=$1 AND v.recorded_at<=$2 AND $7 IN ('investigation','history')
    AND (v.id IN(SELECT id FROM known_sources) OR ($7='history' AND $8='source_version' AND v.id=$9))
    AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
    AND recollect_recall_scope(coalesce(v.import_selection,e.selection,'{}'::jsonb),$4)
    AND ($5::uuid IS NULL OR EXISTS(SELECT 1 FROM evidence_memberships m
      WHERE m.brain_id=$1 AND m.source_id=v.source_id AND m.group_id=$5))
    AND ($4->>'environment_id' IS NULL OR NOT EXISTS(SELECT 1 FROM evidence_memberships m
      JOIN evidence_groups g ON g.id=m.group_id WHERE m.brain_id=$1 AND m.source_id=v.source_id AND g.kind='environment')
      OR EXISTS(SELECT 1 FROM evidence_memberships m WHERE m.brain_id=$1 AND m.source_id=v.source_id
        AND m.group_id=($4->>'environment_id')::uuid))
    AND (coalesce($4->'area_ids','[]')='[]'::jsonb OR NOT EXISTS(SELECT 1 FROM evidence_memberships m
      JOIN evidence_groups g ON g.id=m.group_id WHERE m.brain_id=$1 AND m.source_id=v.source_id AND g.kind='area')
      OR EXISTS(SELECT 1 FROM evidence_memberships m WHERE m.brain_id=$1 AND m.source_id=v.source_id
        AND ($4->'area_ids') ? m.group_id::text))

  UNION ALL

  SELECT 'repository_fact',f.id,f.id,NULL,coalesce(f.record->>'name',f.record->>'kind','Repository fact'),
    f.record::text,s.created_at,jsonb_build_object('repository_ids',jsonb_build_array(s.repository_id)),
    NULL,s.repository_id,s.id,s.revision,
    coalesce(f.record->>'file',f.record#>>'{source,path}',f.record->>'path'),
    NULL,NULL,NULL,NULL,NULL,0,'ready',f.record,f.recall_vector,
    recollect_retention_deadline(s.brain_id,'repository',s.created_at),
    ($3<>'' AND ($3=f.record->>'name' OR $3=coalesce(f.record->>'file',f.record#>>'{source,path}',f.record->>'path'))),true
  FROM repository_facts f JOIN repository_snapshots s ON s.id=f.snapshot_id AND s.brain_id=f.brain_id
  WHERE f.brain_id=$1 AND s.created_at<=$2
    AND $5::uuid IS NULL AND $7 IN ('investigation','history')
    AND recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at)='active'
    AND recollect_recall_scope(jsonb_build_object('repository_ids',jsonb_build_array(s.repository_id)),$4)
    AND (($6::jsonb IS NOT NULL AND EXISTS(SELECT 1 FROM jsonb_array_elements($6->'entries') entry
      WHERE entry->>'repository_id'=s.repository_id::text AND entry->>'snapshot_id'=s.id::text
        AND (coalesce(entry->'config_paths','[]'::jsonb)='[]'::jsonb OR EXISTS(
          SELECT 1 FROM jsonb_array_elements_text(entry->'config_paths') selected_path
          WHERE coalesce(f.record->>'file',f.record#>>'{source,path}',f.record->>'path')=selected_path
            OR starts_with(coalesce(f.record->>'file',f.record#>>'{source,path}',f.record->>'path'),selected_path||'/')))))
      OR ($6::jsonb IS NULL AND $4->>'environment_id' IS NULL AND
        (s.id IN(SELECT id FROM known_snapshots) OR ($7='history' AND $8='repository_fact' AND f.id=$9))))

  UNION ALL

  SELECT 'manifest_revision',r.id,r.id,NULL,r.revision->>'name',r.revision::text,r.created_at,
    jsonb_build_object('environment_id',r.revision->'environment_id','repository_ids',
      coalesce((SELECT jsonb_agg(e->'repository_id') FROM jsonb_array_elements(r.revision->'entries') e),'[]'::jsonb)),
    NULL,NULL,NULL,r.id::text,NULL,NULL,NULL,NULL,NULL,NULL,0,'ready',r.revision,r.recall_vector,NULL,
    ($3<>'' AND r.revision->>'name'=$3),true
  FROM manifest_revisions r
  WHERE r.brain_id=$1 AND r.created_at<=$2 AND r.privacy_state='active'
    AND $5::uuid IS NULL AND $7 IN ('investigation','history')
    AND ($6::jsonb IS NULL OR $6->>'id'=r.id::text)
    AND (r.id IN(SELECT id FROM known_manifests) OR $6->>'id'=r.id::text
      OR ($7='history' AND $8='manifest_revision' AND r.id=$9))
    AND ($4->>'environment_id' IS NULL OR r.revision->>'environment_id'=$4->>'environment_id')
    AND recollect_recall_scope(jsonb_build_object('repository_ids',
      coalesce((SELECT jsonb_agg(e->'repository_id') FROM jsonb_array_elements(r.revision->'entries') e),'[]'::jsonb)),$4)
), matched AS (
  SELECT c.*,
    ('exact'=ANY($10::text[]) AND (coalesce(literal,false) OR coalesce($8=kind AND $9=id,false))) exact_match,
    ('lexical'=ANY($10::text[]) AND $3<>'' AND vector @@ websearch_to_tsquery('simple',$3)) lexical_match,
    ts_rank_cd(vector,websearch_to_tsquery('simple',$3),32) rank
  FROM candidates c
)
