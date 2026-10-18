-- Model a journal manifest written before mapping/ancestry derivative fields.
-- This tests SQL replay compatibility; the separate actual older-database test
-- exercises the durable journal/restore path with a current manifest.
DO $$
DECLARE f jsonb:=current_setting('recollect.fixture')::jsonb;
 b uuid:=(f->>'brain')::uuid; actor uuid:=(f->>'actor')::uuid;
 target uuid; request uuid:=gen_random_uuid(); m jsonb;
BEGIN
 SELECT claim_id INTO STRICT target FROM claim_revisions WHERE brain_id=b AND id=(f->>'claim')::uuid;
 m:=recollect_privacy_preview(b,'claim',target);
 m:=m-'model_input_sources'-'model_input_claim_revisions'-'knowledge_mapping_inputs'
     -'semantic_entries'-'semantic_batches'-'learning_support_stages'
     -'handover_support_stages'-'memory_support_assessments'-'retrieval_feedback_contexts';
 INSERT INTO privacy_requests(id,brain_id,actor_id,target,cause,manifest)
  VALUES(request,b,actor,jsonb_build_object('kind','claim','id',target),'erase',m);
 PERFORM recollect_privacy_apply(request);
END $$;
