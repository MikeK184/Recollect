-- Realistically shaped large Brain for the Brain deletion scale proof
-- (crates/server/tests/brain_deletion.rs). The test replaces __BRAIN__,
-- __OWNER__, __NAME__, __SOURCES__ and __CLAIMS__ before execution. It writes
-- no content markers: the scale proof measures closure time and fingerprint
-- equality, not retained text.
--
-- Shape, per source: one captured session event, an older version superseded
-- by a current one, a chunk, an automatic support excerpt for every fifth
-- source, a completed learning run, a mapping context and input and a pending
-- semantic entry. Per claim: one or two revisions (the second derived from the
-- first), support from a source version, a learning input, a support
-- assessment, and a review decision with a rejected-value rule for every tenth
-- claim. A few jobs stay queued so pending work is fenced. Shares of sources
-- and claims also carry every later derivative class (mapping stages, source
-- inputs, shadow candidates, topic intents, alias decisions, feedback contexts
-- and field inputs). Rows that no Brain member target reaches are seeded too,
-- so an equivalence check proves the closure leaves them out exactly as before.
DO $$
DECLARE
 b uuid:=__BRAIN__; o uuid:=__OWNER__; n integer:=__SOURCES__; m integer:=__CLAIMS__;
 policy uuid:=gen_random_uuid(); audit uuid:=gen_random_uuid(); task uuid:=gen_random_uuid();
 scope uuid:=gen_random_uuid(); op uuid:=gen_random_uuid(); binding uuid:=gen_random_uuid();
 managed uuid:=gen_random_uuid(); profile uuid:=gen_random_uuid(); conn uuid:=gen_random_uuid();
 mprof uuid:=gen_random_uuid(); call uuid:=gen_random_uuid(); dev uuid:=gen_random_uuid();
 report uuid:=gen_random_uuid(); report_job uuid:=gen_random_uuid(); topic uuid:=gen_random_uuid();
BEGIN
 INSERT INTO brains(id,owner_id,name) VALUES(b,o,__NAME__);
 INSERT INTO devices(id,account_id,name,token,claimed,expires_at) VALUES(dev,o,'scale-device',dev,true,now()+interval '1 day');
 INSERT INTO model_policies(id,brain_id,policy,created_by) VALUES(policy,b,'{}',o);
 INSERT INTO model_policy_heads(brain_id,policy_id) VALUES(b,policy);
 INSERT INTO mutation_audit(id,brain_id,actor_id,action,target_id,disposition) VALUES(audit,b,o,'fixture.scale',b,'created');
 INSERT INTO workspace_tasks(id,brain_id,account_id,label) VALUES(task,b,o,'scale-task');
 INSERT INTO scope_snapshots(id,brain_id,task_id,account_id,snapshot) VALUES(scope,b,task,o,'{"repository_ids":[]}');
 UPDATE workspace_tasks SET current_scope=scope WHERE id=task;
 INSERT INTO operation_bindings(id,brain_id,task_id,scope_id,account_id,device_id,kind) VALUES(op,b,task,scope,o,dev,'context');
 INSERT INTO capture_bindings(id,brain_id,operation_id,actor_id,device_id,host,host_version,selection)
  VALUES(binding,b,op,o,dev,'codex','scale','{}');
 INSERT INTO semantic_profiles(id,brain_id,provider,model,dimensions,representation,created_by,policy_id)
  VALUES(profile,b,'fixture-provider','fixture-model',3072,'dense',o,policy);
 INSERT INTO semantic_heads(brain_id,profile_id) VALUES(b,profile);

 CREATE TEMP TABLE scale_sources ON COMMIT DROP AS
  SELECT g,gen_random_uuid() s,gen_random_uuid() v_old,gen_random_uuid() v,gen_random_uuid() chunk,
   gen_random_uuid() event,gen_random_uuid() run,gen_random_uuid() job,gen_random_uuid() ctx,gen_random_uuid() input,
   CASE WHEN g%5=0 THEN gen_random_uuid() END excerpt
  FROM generate_series(1,n) g;
 INSERT INTO sources(id,brain_id,created_by) SELECT s,b,o FROM scale_sources;
 INSERT INTO source_versions(id,source_id,brain_id,title,media_type,byte_length,created_by,processing,retention_class)
  SELECT v_old,s,b,'scale old '||g,'text/plain',8,o,'ready','document' FROM scale_sources
  UNION ALL SELECT v,s,b,'scale '||g,'text/plain',10,o,'ready','document' FROM scale_sources;
 UPDATE sources x SET current_version=y.v FROM scale_sources y WHERE x.id=y.s;
 INSERT INTO source_chunks(id,brain_id,version_id,ordinal,byte_start,byte_end,line_start,line_end,content)
  SELECT chunk,b,v,0,0,10,1,1,'scale chunk '||g FROM scale_sources;
 -- Every fifth source has an automatic support excerpt of its current version.
 INSERT INTO sources(id,brain_id,created_by) SELECT excerpt,b,o FROM scale_sources WHERE excerpt IS NOT NULL;
 INSERT INTO source_versions(id,source_id,brain_id,title,media_type,byte_length,created_by,processing,retention_class)
  SELECT excerpt,excerpt,b,'scale excerpt '||g,'text/plain',4,o,'ready','support_excerpt' FROM scale_sources WHERE excerpt IS NOT NULL;
 UPDATE sources x SET current_version=x.id FROM scale_sources y WHERE x.id=y.excerpt;
 INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line)
  SELECT b,excerpt,s,v,1,1 FROM scale_sources WHERE excerpt IS NOT NULL;
 INSERT INTO capture_events(id,brain_id,binding_id,retention_class,captured_at,expires_at,state,metadata,source_id,source_version_id)
  SELECT event,b,binding,'raw_session',now(),now()+interval '30 days','accepted',jsonb_build_object('host_session_id','scale-'||(g%7)),s,v
  FROM scale_sources;
 INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,state)
  SELECT job,b,o,audit,run,'source.learn','model',CASE WHEN g%37=0 THEN 'queued' ELSE 'succeeded' END FROM scale_sources;
 INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) SELECT ctx,b,s,'{}' FROM scale_sources;
 INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,created_by)
  SELECT input,b,v,'source-mentions-1','fixture',0,ctx,'{}',o FROM scale_sources;
 INSERT INTO semantic_entries(id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state)
  SELECT gen_random_uuid(),b,profile,'source_chunk',chunk,chunk,v,'pending' FROM scale_sources;

 CREATE TEMP TABLE scale_claims ON COMMIT DROP AS
  SELECT g,gen_random_uuid() cl,gen_random_uuid() r1,CASE WHEN g%2=0 THEN gen_random_uuid() END r2,
   1+(g%n) source_g,CASE WHEN g%10=0 THEN gen_random_uuid() END decision
  FROM generate_series(1,m) g;
 INSERT INTO claims(id,brain_id,created_by) SELECT cl,b,o FROM scale_claims;
 INSERT INTO claim_revisions(id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key)
  SELECT r1,cl,b,now()-interval '1 day',jsonb_build_object('content',jsonb_build_object('value','scale '||g)),'sub'||g,'pred','scale '||g FROM scale_claims
  UNION ALL SELECT r2,cl,b,now(),jsonb_build_object('content',jsonb_build_object('value','scale new '||g)),'sub'||g,'pred','scale new '||g FROM scale_claims WHERE r2 IS NOT NULL;
 UPDATE claims x SET current_revision=coalesce(y.r2,y.r1) FROM scale_claims y WHERE x.id=y.cl;
 INSERT INTO claim_supports(revision_id,brain_id,ordinal,source_version_id)
  SELECT c.r1,b,0,s.v FROM scale_claims c JOIN scale_sources s ON s.g=c.source_g
  UNION ALL SELECT c.r2,b,0,coalesce(s.excerpt,s.v) FROM scale_claims c JOIN scale_sources s ON s.g=c.source_g WHERE c.r2 IS NOT NULL;
 INSERT INTO claim_contributions(revision_id,brain_id,input_revision_id) SELECT r2,b,r1 FROM scale_claims WHERE r2 IS NOT NULL;
 INSERT INTO learning_runs(id,brain_id,source_version_id,policy_id,actor_id,selection,job_id,state,claim_ids)
  SELECT s.run,b,s.v,policy,o,'{}',s.job,'succeeded',coalesce((SELECT array_agg(c.cl) FROM scale_claims c WHERE c.source_g=s.g),'{}')
  FROM scale_sources s;
 INSERT INTO learning_run_inputs(run_id,brain_id,revision_id)
  SELECT s.run,b,c.r1 FROM scale_claims c JOIN scale_sources s ON s.g=c.source_g;
 INSERT INTO memory_support_assessments(id,brain_id,revision_id,policy_id,verifier_version,actor_id)
  SELECT gen_random_uuid(),b,r1,policy,'source-support-3',o FROM scale_claims;
 INSERT INTO memory_decisions(id,brain_id,decision)
  SELECT decision,b,jsonb_build_object('reason','scale','transitions',jsonb_build_array(jsonb_build_object('before_revision',r1,'after_revision',r1)))
  FROM scale_claims WHERE decision IS NOT NULL;
 INSERT INTO assertion_rules(id,brain_id,decision_id,subject_key,predicate_key,value_key,rule)
  SELECT gen_random_uuid(),b,decision,'sub'||g,'pred','rejected '||g,jsonb_build_object('revision_id',r1,'text','scale')
  FROM scale_claims WHERE decision IS NOT NULL;

 -- Rows no member target reaches: an unreferenced decision and rule, an
 -- unrelated queued job, a capture event without a source and a managed MCP
 -- call whose receipt reports an existing version. The closure must keep
 -- excluding (or including) them exactly as the per-target union does.
 INSERT INTO memory_decisions(id,brain_id,decision) VALUES(gen_random_uuid(),b,'{"reason":"orphan","transitions":[]}');
 INSERT INTO assertion_rules(id,brain_id,decision_id,subject_key,predicate_key,value_key,rule)
  VALUES(gen_random_uuid(),b,(SELECT id FROM memory_decisions WHERE brain_id=b AND decision->>'reason'='orphan'),'orphan','pred','orphan','{"text":"orphan"}');
 INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,state)
  VALUES(gen_random_uuid(),b,o,audit,gen_random_uuid(),'graph','heavy','queued');
 INSERT INTO capture_events(id,brain_id,binding_id,retention_class,captured_at,expires_at,state,metadata)
  VALUES(gen_random_uuid(),b,binding,'raw_session',now(),now()+interval '30 days','accepted','{"host_session_id":"scale-orphan"}');
 INSERT INTO mcp_definitions(key,id,manifest,approved_by) VALUES('scale.definition.'||b,gen_random_uuid(),'{}',o)
  ON CONFLICT DO NOTHING;
 INSERT INTO mcp_connections(id,brain_id,name,definition_key,target,placement,configuration,revision,created_by)
  VALUES(conn,b,'scale-conn','scale.definition.'||b,'http://127.0.0.1:9','central','{}',gen_random_uuid(),o);
 INSERT INTO mcp_profiles(id,brain_id,name,revision,created_by) VALUES(mprof,b,'scale-profile',gen_random_uuid(),o);
 INSERT INTO mcp_calls(id,brain_id,actor_id,request_id,profile_id,connection_id,profile_revision,connection_revision,definition_revision,tool_name,client_session_id,runner_reference,state,timeout_seconds)
  VALUES(call,b,o,gen_random_uuid(),mprof,conn,gen_random_uuid(),gen_random_uuid(),now(),'scale_tool',gen_random_uuid(),'central','queued',60);
 INSERT INTO capture_bindings(id,brain_id,actor_id,host,host_version,selection,managed_call_id)
  VALUES(call,b,o,'managed_mcp','scale','{}',call);
 INSERT INTO mcp_call_resolutions(id,call_id,brain_id,actor_id,kind,outcome,source_version_id)
  SELECT gen_random_uuid(),call,b,o,'evidence','succeeded',v FROM scale_sources WHERE g=1;
 INSERT INTO mcp_observation_outbox(id,call_id,brain_id,actor_id,stage,outcome,captured_at,expires_at,state)
  VALUES(gen_random_uuid(),call,b,o,'terminal','succeeded',now(),now()+interval '30 days','pending');

 -- Later derivative classes for a share of sources and claims: extraction
 -- requests with mapping stages, organization source inputs, shadow
 -- candidates and report sources, manual topic intents, alias decisions,
 -- retrieval feedback contexts and structured field inputs. One structured
 -- field input names an identity outside the Brain and stays unreached.
 CREATE TEMP TABLE scale_requests ON COMMIT DROP AS
  SELECT s.g,gen_random_uuid() req,s.s,s.v,s.run FROM scale_sources s WHERE s.g%3=0;
 INSERT INTO model_requests(id,brain_id,actor_id,operation_id,policy_id,purpose,model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens)
  SELECT req,b,o,gen_random_uuid(),policy,'extraction','fixture-model','p','s','running',gen_random_uuid(),100,0 FROM scale_requests;
 INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id) SELECT req,b,'source_version',v FROM scale_requests;
 INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id)
  SELECT r.req,b,'claim_revision',c.r1 FROM scale_requests r JOIN scale_claims c ON c.g=r.g WHERE r.g%6=0;
 INSERT INTO knowledge_mapping_stages(run_id,brain_id,request_id,source_version_id,context,payload,expires_at)
  SELECT run,b,req,v,'{}','{}',now()+interval '1 day' FROM scale_requests;
 CREATE TEMP TABLE scale_source_inputs ON COMMIT DROP AS
  SELECT g,gen_random_uuid() id,s,v FROM scale_sources WHERE g%2=0;
 INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by)
  SELECT id,b,s,v,'{}',o FROM scale_source_inputs;
 INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,state)
  VALUES(report_job,b,o,audit,report_job,'analytics','heavy','succeeded');
 INSERT INTO analytics_reports(id,brain_id,actor_id,job_id,algorithm,direction,parameters,analytics_epoch,privacy_sequence,node_count,edge_count)
  VALUES(report,b,o,report_job,'pagerank','both','{}',0,0,1,0);
 CREATE TEMP TABLE scale_shadow ON COMMIT DROP AS SELECT gen_random_uuid() id,r.* FROM scale_requests r WHERE r.g%4=0;
 INSERT INTO knowledge_shadow_candidates(id,brain_id,source_id,source_version_id,request_id,policy_id,actor_id,context,schema_revision,adapter_revision,output_sha256,offered_inputs,payload,expires_at)
  SELECT id,b,s,v,req,policy,o,'{}','source-mentions-1','learning-quote-anchors-3',md5(id::text)||md5(v::text),'[]','{}',now()+interval '1 day' FROM scale_shadow;
 INSERT INTO knowledge_shadow_sources(id,brain_id,report_id,source_id,source_version_id,candidate_id,context)
  SELECT gen_random_uuid(),b,report,s,v,id,'{}'::jsonb FROM scale_shadow
  UNION ALL SELECT gen_random_uuid(),b,report,s,v_old,NULL,'{}' FROM scale_sources WHERE g%5=1;
 CREATE TEMP TABLE scale_entities ON COMMIT DROP AS
  SELECT s.g,s.s,s.ctx,s.input,gen_random_uuid() e1,gen_random_uuid() e2,gen_random_uuid() m1,gen_random_uuid() m2,gen_random_uuid() cand
  FROM scale_sources s WHERE s.g%10=0;
 INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label)
  SELECT e1,b,'service',jsonb_build_object('source_id',s,'context_id',ctx),'scale entity '||g FROM scale_entities
  UNION ALL SELECT e2,b,'service',jsonb_build_object('source_id',s,'context_id',ctx),'scale alias '||g FROM scale_entities;
 INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence)
  SELECT m1,b,input,e1,'one',0,1,'scale',0.9 FROM scale_entities
  UNION ALL SELECT m2,b,input,e2,'two',1,2,'alias',0.9 FROM scale_entities;
 INSERT INTO knowledge_relation_candidates(id,brain_id,input_id,from_mention,to_mention,relation,byte_start,byte_end,quote)
  SELECT cand,b,input,m1,m2,'alias',0,2,'scale alias' FROM scale_entities;
 INSERT INTO knowledge_alias_changes(id,brain_id,candidate_id,actor_id,action)
  SELECT gen_random_uuid(),b,cand,o,'accept' FROM scale_entities WHERE g%20=0;
 INSERT INTO knowledge_topics(id,brain_id,kind,change_id,recipe) VALUES(topic,b,'manual_overlay',gen_random_uuid(),'manual');
 INSERT INTO knowledge_topic_intent_sources(id,brain_id,topic_id,input_id)
  SELECT gen_random_uuid(),b,topic,id FROM scale_source_inputs WHERE g%8=0;
 INSERT INTO knowledge_topic_intent_mappings(id,brain_id,topic_id,input_id,entity_id)
  SELECT gen_random_uuid(),b,topic,input,e1 FROM scale_entities;
 INSERT INTO retrieval_feedback_contexts(id,brain_id,actor_id,task_id,operation_id,scope_id,query_context_id,scope,input_kind,revision_id)
  SELECT gen_random_uuid(),b,o,task,op,scope,gen_random_uuid(),'{}'::jsonb,'claim',r1 FROM scale_claims WHERE g%5=0
  UNION ALL SELECT gen_random_uuid(),b,o,task,op,scope,gen_random_uuid(),'{}','source_version',v FROM scale_sources WHERE g%7=0;
 INSERT INTO knowledge_field_inputs(id,brain_id,origin_kind,logical_id,revision_id,context,evidence_kind,evidence_id,evidence_revision,created_by)
  SELECT gen_random_uuid(),b,'claim_revision',cl,r1,'{}'::jsonb,'claim',cl,r1,o FROM scale_claims WHERE g%5=0
  UNION ALL SELECT gen_random_uuid(),b,'source_context',s,v,'{}','source_version',s,v,o FROM scale_sources WHERE g%5=0
  UNION ALL SELECT gen_random_uuid(),b,'repository_snapshot',gen_random_uuid(),gen_random_uuid(),'{}','repository_fact',gen_random_uuid(),gen_random_uuid(),o;
END $$;
