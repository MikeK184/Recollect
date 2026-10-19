-- Synthetic populated derivative storage, never extractor quality evidence.
-- The test supplies exact canonical identities through a transaction-local JSON
-- setting. Production privacy functions/triggers perform every removal.
CREATE TEMP TABLE mapping_populated_derivatives(result jsonb) ON COMMIT DROP;
DO $$
DECLARE f jsonb:=current_setting('recollect.fixture')::jsonb;
 b uuid:=(f->>'brain')::uuid; actor uuid:=(f->>'actor')::uuid;
 parent uuid:=(f->>'parent')::uuid; excerpt uuid:=(f->>'excerpt')::uuid;
 claim uuid:=(f->>'claim')::uuid; independent uuid:=(f->>'independent')::uuid;
 policy uuid; profile uuid:=gen_random_uuid(); audit uuid:=gen_random_uuid();
 learn uuid:=gen_random_uuid(); handover uuid:=gen_random_uuid(); batch uuid:=gen_random_uuid();
 learn_job uuid:=gen_random_uuid(); handover_job uuid:=gen_random_uuid(); vector_job uuid:=gen_random_uuid();
 extract_request uuid:=gen_random_uuid(); synthesis_request uuid:=gen_random_uuid();
 assessment_request uuid:=gen_random_uuid(); embedding_request uuid:=gen_random_uuid();
 assessment_operation uuid:=gen_random_uuid(); learning_assessment uuid:=gen_random_uuid();
 source_vector uuid:=gen_random_uuid(); claim_vector uuid:=gen_random_uuid();
 queued_vector uuid:=gen_random_uuid(); independent_vector uuid:=gen_random_uuid();
BEGIN
 SELECT policy_id INTO policy FROM model_policy_heads WHERE brain_id=b;
 IF policy IS NULL THEN
  policy:=gen_random_uuid();
  INSERT INTO model_policies(id,brain_id,policy,created_by)
   VALUES(policy,b,'{"enabled":false,"fixture":"physical-cleanup-only"}',actor);
 END IF;
 INSERT INTO mutation_audit(id,brain_id,actor_id,action,target_id,disposition)
  VALUES(audit,b,actor,'fixture.derivatives',parent,'synthetic_physical_cleanup');
 INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,state,lease_token,lease_until)
  VALUES(learn_job,b,actor,audit,learn,'source.learn','model','running',gen_random_uuid(),clock_timestamp()+interval '20 seconds'),
        (handover_job,b,actor,audit,handover,'handover.generate','model','succeeded',NULL,NULL),
        (vector_job,b,actor,audit,batch,'semantic.generate','model','queued',NULL,NULL);
 INSERT INTO model_requests(id,brain_id,actor_id,operation_id,policy_id,purpose,model,returned_model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens)
  VALUES(extract_request,b,actor,learn,policy,'extraction','synthetic-only','synthetic-only','fixture-1','fixture-1','succeeded',gen_random_uuid(),10,1),
        (synthesis_request,b,actor,handover,policy,'synthesis','synthetic-only','synthetic-only','fixture-1','fixture-1','succeeded',gen_random_uuid(),10,1),
        (assessment_request,b,actor,assessment_operation,policy,'synthesis','synthetic-only','synthetic-only','fixture-1','fixture-1','succeeded',gen_random_uuid(),10,1),
        (embedding_request,b,actor,gen_random_uuid(),policy,'embedding','synthetic-only','synthetic-only','fixture-1','fixture-1','succeeded',gen_random_uuid(),10,1);
 INSERT INTO learning_runs(id,brain_id,source_version_id,policy_id,actor_id,selection,job_id,state,request_id)
  VALUES(learn,b,excerpt,policy,actor,'{}',learn_job,'running',extract_request);
 INSERT INTO learning_support_stages(run_id,brain_id,extraction_request_id,assessment_operation_id,verifier_version,payload,verdicts)
  VALUES(learn,b,extract_request,learning_assessment,'source-support-3','{"synthetic_sensitive_text":"ancestral learning payload"}','[{"reason":"ancestral learning verdict"}]');
 INSERT INTO handover_runs(id,brain_id,title,contributions,selection,policy_id,actor_id,job_id,state)
  VALUES(handover,b,'Ancestral succeeded handover title',ARRAY[claim],'{"area_ids":[]}',policy,actor,handover_job,'queued');
 INSERT INTO handover_run_inputs(run_id,brain_id,revision_id) VALUES(handover,b,claim);
 UPDATE handover_runs SET state='succeeded',request_id=synthesis_request WHERE id=handover;
 INSERT INTO handover_support_stages(run_id,brain_id,synthesis_request_id,assessment_operation_id,assessment_request_id,verifier_version,payload,verdict)
  VALUES(handover,b,synthesis_request,assessment_operation,assessment_request,'source-support-3','{"synthetic_sensitive_text":"ancestral handover payload"}','{"reason":"ancestral handover verdict"}');
 INSERT INTO semantic_profiles(id,brain_id,provider,model,dimensions,representation,created_by,policy_id)
  VALUES(profile,b,'openai','text-embedding-3-large',3072,'engineering-text-1',actor,policy);
 INSERT INTO semantic_batches(id,brain_id,profile_id,actor_id,policy_id,job_id,state,input_count)
  VALUES(batch,b,profile,actor,policy,vector_job,'queued',1);
 INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state,embedding,request_id)
  SELECT source_vector,b,profile,'source_chunk',c.id,c.id,c.version_id,'ready',array_fill(1::real,ARRAY[3072])::vector,embedding_request
  FROM source_chunks c WHERE c.brain_id=b AND c.version_id=parent ORDER BY c.ordinal LIMIT 1;
 INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,claim_revision_id,state,embedding,request_id)
  VALUES(claim_vector,b,profile,'claim_revision',claim,claim,'ready',array_fill(1::real,ARRAY[3072])::vector,embedding_request);
 INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state,batch_id)
  SELECT queued_vector,b,profile,'source_chunk',c.id,c.id,c.version_id,'queued',batch
  FROM source_chunks c WHERE c.brain_id=b AND c.version_id=excerpt ORDER BY c.ordinal LIMIT 1;
 INSERT INTO semantic_batch_inputs(brain_id,batch_id,entry_id,ordinal) VALUES(b,batch,queued_vector,0);
 INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state,embedding)
  SELECT independent_vector,b,profile,'source_chunk',c.id,c.id,c.version_id,'ready',array_fill(1::real,ARRAY[3072])::vector
  FROM source_chunks c WHERE c.brain_id=b AND c.version_id=independent ORDER BY c.ordinal LIMIT 1;
 INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id)
  VALUES(extract_request,b,'source_version',excerpt),
        (synthesis_request,b,'claim_revision',claim),
        (assessment_request,b,'handover_support_stage',handover),
        (embedding_request,b,'semantic_entry',source_vector),
        (embedding_request,b,'semantic_entry',claim_vector);
 INSERT INTO mapping_populated_derivatives VALUES(jsonb_build_object(
  'learning',learn,'handover',handover,'batch',batch,'jobs',ARRAY[learn_job,vector_job],
  'requests',ARRAY[extract_request,synthesis_request,assessment_request,embedding_request],
  'vectors',ARRAY[source_vector,claim_vector,queued_vector],'independent_vector',independent_vector));
END $$;
