a.brain_id=$1 AND NOT a.payload_erased
AND recollect_recall_scope((a.rule#>'{content,selection}')-'area_ids',$2::jsonb-'area_ids')
AND recollect_recall_scope((a.rule#>'{content,selection}')-'area_ids',r.selection-'area_ids')
AND ((strpos(lower(r.text),a.subject_key)>0 AND strpos(lower(r.text),a.predicate_key)>0 AND strpos(r.text,a.value_key)>0)
 OR EXISTS(SELECT 1 FROM jsonb_array_elements(a.rule#>'{content,supports}') s
  WHERE s->>'kind'=r.kind AND s->>'id'=r.id::text
   AND (s->>'line_from' IS NULL OR r.line_from IS NULL OR
    ((s->>'line_from')::integer<=r.line_to AND (s->>'line_to')::integer>=r.line_from))))
