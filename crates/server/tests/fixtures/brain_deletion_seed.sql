-- Brain deletion fixture (crates/server/tests/brain_deletion.rs). Seeds a rich
-- Brain A ("Alpha") holding at least one row in every dependent class of the
-- 029 removal list, plus a small control Brain B ("Beta"). The double
-- underscore tokens are replaced by the test with fresh UUIDs and content
-- markers before execution.

-- Extended below through support/digest and native mapping candidate storage.

INSERT INTO accounts (id,username) VALUES (__MEMBER__,'fixture-member');
INSERT INTO devices (id,account_id,name,token,claimed,expires_at)
  VALUES (__DEV__,__MEMBER__,'fixture-device',__DEV__,true,now()+interval '1 day');

INSERT INTO brains (id,owner_id,name) VALUES (__A__,__OWNER__,'Alpha');
INSERT INTO brains (id,owner_id,name) VALUES (__B__,__OWNER__,'Beta');

INSERT INTO brain_grants (brain_id,account_id,role) VALUES (__A__,__MEMBER__,'reader');
INSERT INTO brain_group_grants (id,brain_id,issuer,group_name,role)
  VALUES (gen_random_uuid(),__A__,'fixture-issuer','fixture-team','reader');

-- Dormant privacy request for Brain A: the fence tables reference
-- privacy_requests(id, brain_id), so it must exist before any of them.
-- Brain deletion retains (tombstones) it like every other privacy row.
INSERT INTO privacy_requests (id,brain_id,actor_id,target,cause,manifest)
  VALUES (__SEEDREQ__,__A__,__OWNER__,'{"kind":"source_version"}','expire','{}');

-- Audit rows must exist before the jobs that reference them.
INSERT INTO mutation_audit (id,brain_id,actor_id,action,target_id,disposition) VALUES
  (gen_random_uuid(),__A__,__OWNER__,'fixture.seed',__JH__,'created'),
  (gen_random_uuid(),__A__,__OWNER__,'fixture.seed',__JL__,'created'),
  (gen_random_uuid(),__A__,__OWNER__,'fixture.seed',__JA__,'created'),
  (gen_random_uuid(),__A__,__OWNER__,'fixture.seed',__JG__,'created'),
  (gen_random_uuid(),__A__,__OWNER__,'fixture.seed',__JW__,'created');
INSERT INTO jobs (id,brain_id,actor_id,audit_id,target_id,kind,lane,state) VALUES
  (__JH__,__A__,__OWNER__,(SELECT id FROM mutation_audit WHERE brain_id=__A__ AND target_id=__JH__),__JH__,'handover','heavy','queued'),
  (__JL__,__A__,__OWNER__,(SELECT id FROM mutation_audit WHERE brain_id=__A__ AND target_id=__JL__),__JL__,'learning','model','queued'),
  (__JA__,__A__,__OWNER__,(SELECT id FROM mutation_audit WHERE brain_id=__A__ AND target_id=__JA__),__JA__,'analytics','heavy','queued'),
  (__JG__,__A__,__OWNER__,(SELECT id FROM mutation_audit WHERE brain_id=__A__ AND target_id=__JG__),__JG__,'graph','heavy','queued'),
  (__JW__,__A__,__OWNER__,(SELECT id FROM mutation_audit WHERE brain_id=__A__ AND target_id=__JW__),__V2__,'capture','capture','queued');

INSERT INTO model_policies (id,brain_id,policy,created_by) VALUES (__MP__,__A__,'{}',__OWNER__);
INSERT INTO model_policy_heads (brain_id,policy_id) VALUES (__A__,__MP__);
INSERT INTO memory_support_discovery(brain_id,policy_id,verifier_version,last_revision_id,version)
 VALUES(__A__,__MP__,'source-support-3',__R1__,1);
INSERT INTO retention_policies (brain_id,change_id,policy,updated_by)
  VALUES (__A__,gen_random_uuid(),'{"document_days":30,"backup_days":7}',__OWNER__);

INSERT INTO sources (id,brain_id,created_by,current_version) VALUES (__S1__,__A__,__OWNER__,__V2__);
INSERT INTO source_versions (id,source_id,brain_id,title,media_type,source_uri,byte_length,created_by,processing,artifact_id,retention_class)
  VALUES (__V1__,__S1__,__A__,'__MT_TITLE__ old','text/plain','__MT_URI__',8,__OWNER__,'ready',NULL,'document'),
         (__V2__,__S1__,__A__,'__MT_TITLE__','text/plain','__MT_URI__',10,__OWNER__,'ready',__ART1__,'support_excerpt');
INSERT INTO source_chunks (id,brain_id,version_id,ordinal,byte_start,byte_end,line_start,line_end,content)
  VALUES (gen_random_uuid(),__A__,__V1__,0,0,8,1,1,'__MT_CHUNK__ old'),
         (__C2__,__A__,__V2__,0,0,10,1,1,'__MT_CHUNK__');
INSERT INTO source_excerpts (brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line)
  VALUES (__A__,__V2__,__S1__,__V1__,1,1);

INSERT INTO claims (id,brain_id,created_by,current_revision) VALUES (__CL1__,__A__,__OWNER__,__R2__);
INSERT INTO claim_revisions (id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key)
  VALUES (__R1__,__CL1__,__A__,now()-interval '1 day','{"content":{"value":"__MT_CLAIM__ old"}}','sub','pred','__MT_CLAIM__ old'),
         (__R2__,__CL1__,__A__,now(),jsonb_build_object('origin','model_synthesized','content',jsonb_build_object('kind','handover','selection','{}'::jsonb,'value','__MT_CLAIM__'),'derivation',jsonb_build_object('run_id',__JH__::uuid)),'sub','pred','__MT_CLAIM__');
INSERT INTO claim_supports (revision_id,brain_id,ordinal,source_version_id) VALUES (__R2__,__A__,0,__V2__);
INSERT INTO claim_contributions (revision_id,brain_id,input_revision_id) VALUES (__R2__,__A__,__R1__);

INSERT INTO memory_decisions (id,brain_id,decision)
  VALUES (__D1__,__A__,('{"reason":"__MT_RULE__","transitions":[{"before_revision":"' || __R1Q__ || '","after_revision":"' || __R2Q__ || '"}]}')::jsonb);
INSERT INTO assertion_rules (id,brain_id,decision_id,subject_key,predicate_key,value_key,rule)
  VALUES (__AR1__,__A__,__D1__,'sub','pred','__MT_CLAIM__',('{"revision_id":"' || __R2Q__ || '","text":"__MT_RULE__"}')::jsonb);
INSERT INTO memory_rule_exceptions (rule_id,revision_id,decision_id,brain_id) VALUES (__AR1__,__R2__,__D1__,__A__);
INSERT INTO memory_decision_claims (decision_id,claim_id,brain_id) VALUES (__D1__,__CL1__,__A__);
INSERT INTO memory_epochs (brain_id,epoch) VALUES (__A__,1);

-- Workspace scope rows must exist before repository contributions reference
-- the operation binding.
INSERT INTO workspace_tasks (id,brain_id,account_id,label)
  VALUES (__T1__,__A__,__OWNER__,'fixture-task');
INSERT INTO scope_snapshots (id,brain_id,task_id,account_id,snapshot)
  VALUES (__SS1__,__A__,__T1__,__OWNER__,'{"repository_ids":[]}');
UPDATE workspace_tasks SET current_scope=__SS1__ WHERE id=__T1__;
INSERT INTO operation_bindings (id,brain_id,task_id,scope_id,account_id,device_id,kind)
  VALUES (gen_random_uuid(),__A__,__T1__,__SS1__,__OWNER__,__DEV__,'context');

INSERT INTO repositories (id,brain_id,canonical_origin,created_by) VALUES (__REP1__,__A__,'origin-alpha',__OWNER__);
INSERT INTO repository_origins (brain_id,origin,repository_id,created_by)
  VALUES (__A__,'__MT_PATH__',__REP1__,__OWNER__);
INSERT INTO repository_snapshots (id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by)
  VALUES (__SNAP1__,__A__,__REP1__,'abc123def','test-adapter','1','1','{}','{}',1,1,0,__OWNER__);
INSERT INTO repository_files (id,brain_id,snapshot_id,path,object_id,mode,status,extraction,artifact_id,byte_length)
  VALUES (gen_random_uuid(),__A__,__SNAP1__,'__MT_PATH__','oid-1','100644','captured','none',NULL,5);
INSERT INTO repository_facts (id,brain_id,snapshot_id,ordinal,record)
  VALUES (gen_random_uuid(),__A__,__SNAP1__,0,'{"value":"__MT_CLAIM__ fact"}');
INSERT INTO repository_artifacts (id,brain_id,snapshot_id,kind,byte_length) VALUES (__ART1__,__A__,__SNAP1__,'facts',3);
INSERT INTO repository_contributions (id,brain_id,snapshot_id,actor_id,device_id,operation_id,scope,origin,dirty,captured_at)
  VALUES (gen_random_uuid(),__A__,__SNAP1__,__OWNER__,__DEV__,
          (SELECT id FROM operation_bindings WHERE brain_id=__A__),
          '{}','__MT_PATH__',false,now());
INSERT INTO repository_jobs (job_id,brain_id,snapshot_id,selection) VALUES (__JW__,__A__,__SNAP1__,'{}');

INSERT INTO evidence_groups (id,brain_id,kind,name,created_by) VALUES
  (__ENV1__,__A__,'environment','env-alpha',__OWNER__),
  (__COL1__,__A__,'collection','col-alpha',__OWNER__),
  (__AREA1__,__A__,'area','area-alpha',__OWNER__);
INSERT INTO evidence_memberships (brain_id,source_id,group_id) VALUES (__A__,__S1__,__COL1__);
INSERT INTO revision_manifests (id,brain_id,environment_id,name,current_revision)
  VALUES (__MAN1__,__A__,__ENV1__,'__MT_MANIFEST__',__MREV1__);
INSERT INTO manifest_revisions (id,manifest_id,brain_id,revision)
  VALUES (__MREV1__,__MAN1__,__A__,('{"entries":[{"snapshot_id":"' || __SNAP1Q__ || '"}]}')::jsonb);

INSERT INTO workspace_registrations (id,brain_id,account_id,device_id,root,complete,notes)
  VALUES (__WS1__,__A__,__OWNER__,__DEV__,'__MT_PATH__/ws',true,'{}');
INSERT INTO checkout_registrations (id,workspace_id,brain_id,account_id,device_id,local_path,observation)
  VALUES (gen_random_uuid(),__WS1__,__A__,__OWNER__,__DEV__,'__MT_PATH__/co','{}');

INSERT INTO capture_policies (brain_id,change_id,policy,updated_by)
  VALUES (__A__,gen_random_uuid(),'{"enabled":true}',__OWNER__);
INSERT INTO capture_bindings (id,brain_id,operation_id,actor_id,device_id,host,host_version,selection)
  VALUES (__CB1__,__A__,(SELECT id FROM operation_bindings WHERE brain_id=__A__),__OWNER__,__DEV__,'codex','fixture','{}');
INSERT INTO capture_events (id,brain_id,binding_id,retention_class,captured_at,expires_at,state,metadata,source_id,source_version_id)
  VALUES (__CE1__,__A__,__CB1__,'raw_session',now(),now()+interval '30 days','accepted','{"host_session_id":"fixture-session"}',__S1__,__V2__);
INSERT INTO capture_device_reports (brain_id,device_id,report) VALUES (__A__,__DEV__,'{"note":"fixture"}');
INSERT INTO privacy_capture_fences (brain_id,event_id,binding_id,request_id)
  VALUES (__A__,__CE1__,__CB1__,__SEEDREQ__);

INSERT INTO mcp_definitions (key,id,manifest,approved_by) VALUES ('fixture.definition',gen_random_uuid(),'{}',__OWNER__);
INSERT INTO mcp_connections (id,brain_id,name,definition_key,target,placement,configuration,revision,created_by)
  VALUES (__CONN1__,__A__,'conn-alpha','fixture.definition','http://127.0.0.1:9','central','{}',gen_random_uuid(),__OWNER__);
INSERT INTO mcp_profiles (id,brain_id,name,revision,created_by) VALUES (__PROF1__,__A__,'prof-alpha',gen_random_uuid(),__OWNER__);
INSERT INTO mcp_profile_connections (brain_id,profile_id,connection_id) VALUES (__A__,__PROF1__,__CONN1__);
INSERT INTO mcp_profile_grants (id,brain_id,profile_id,account_id,can_use,can_manage,can_share)
  VALUES (gen_random_uuid(),__A__,__PROF1__,__MEMBER__,true,false,false);
INSERT INTO mcp_calls (id,brain_id,actor_id,request_id,profile_id,connection_id,profile_revision,connection_revision,definition_revision,tool_name,client_session_id,runner_reference,state,timeout_seconds)
  VALUES (__MC1__,__A__,__OWNER__,gen_random_uuid(),__PROF1__,__CONN1__,gen_random_uuid(),gen_random_uuid(),now(),'fixture_tool',gen_random_uuid(),'central','queued',60);
INSERT INTO mcp_call_payloads (call_id,brain_id,request,arguments)
  VALUES (__MC1__,__A__,'{"note":"__MT_RECEIPT__"}','{}');
INSERT INTO mcp_call_resolutions (id,call_id,brain_id,actor_id,kind,outcome,source_version_id)
  VALUES (gen_random_uuid(),__MC1__,__A__,__OWNER__,'evidence','succeeded',__V2__);
INSERT INTO mcp_runners (reference,epoch,lease_until) VALUES ('central',gen_random_uuid(),now()+interval '1 hour');
INSERT INTO mcp_instances (id,brain_id,profile_id,connection_id,actor_id,client_session_id,runner_reference,runner_epoch,connection_revision,definition_revision,credential_generation,state,active_calls,idle_seconds)
  VALUES (gen_random_uuid(),__A__,__PROF1__,__CONN1__,__OWNER__,
          (SELECT client_session_id FROM mcp_calls WHERE id=__MC1__),
          'central',gen_random_uuid(),gen_random_uuid(),now(),gen_random_uuid(),'ready',0,5);
INSERT INTO mcp_observation_outbox (id,call_id,brain_id,actor_id,stage,outcome,captured_at,expires_at,state)
  VALUES (gen_random_uuid(),__MC1__,__A__,__OWNER__,'terminal','succeeded',now(),now()+interval '30 days','pending');
INSERT INTO mcp_private_runners (id,brain_id,device_id,name,revision,created_by)
  VALUES (gen_random_uuid(),__A__,__DEV__,'runner-alpha',gen_random_uuid(),__OWNER__);
INSERT INTO mcp_session_releases (brain_id,actor_id,client_session_id)
  VALUES (__A__,__OWNER__,(SELECT client_session_id FROM mcp_calls WHERE id=__MC1__));

INSERT INTO semantic_profiles (id,brain_id,provider,model,dimensions,representation,created_by,policy_id)
  VALUES (__SP1__,__A__,'fixture-provider','fixture-model',3072,'dense',__OWNER__,__MP__);
INSERT INTO semantic_entries (id,brain_id,profile_id,kind,input_id,chunk_id,source_version_id,state)
  VALUES (gen_random_uuid(),__A__,__SP1__,'source_chunk',__C2__,__C2__,__V2__,'pending');
INSERT INTO semantic_batches (id,brain_id,profile_id,actor_id,policy_id,job_id,state,input_count)
  VALUES (gen_random_uuid(),__A__,__SP1__,__OWNER__,__MP__,__JW__,'queued',1);
INSERT INTO semantic_batch_inputs (brain_id,batch_id,entry_id,ordinal)
  VALUES (__A__,(SELECT id FROM semantic_batches WHERE brain_id=__A__),
          (SELECT id FROM semantic_entries WHERE brain_id=__A__ AND input_id=__C2__),0);
INSERT INTO semantic_heads (brain_id,profile_id) VALUES (__A__,__SP1__);

-- The fence tables below reference the dormant privacy_requests row seeded
-- with the Brain rows above, not model_requests.
INSERT INTO model_requests (id,brain_id,actor_id,operation_id,policy_id,purpose,model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens)
  VALUES (__MREQ1__,__A__,__OWNER__,gen_random_uuid(),__MP__,'extraction','fixture-model','p','s','running',gen_random_uuid(),100,0);
INSERT INTO model_request_inputs (request_id,brain_id,kind,input_id) VALUES (__MREQ1__,__A__,'source_version',__V2__);
-- Fence the superseded version, not __V2__: fencing a version with live
-- semantic entries cascades to their batches and cancels the batch's job.
INSERT INTO model_input_fences (brain_id,source_version_id,request_id) VALUES (__A__,__V1__,__SEEDREQ__);
INSERT INTO model_claim_fences (brain_id,revision_id,request_id) VALUES (__A__,__R2__,__SEEDREQ__);

INSERT INTO learning_runs (id,brain_id,source_version_id,policy_id,actor_id,selection,job_id,state,claim_ids)
  VALUES (gen_random_uuid(),__A__,__V2__,__MP__,__OWNER__,'{}',__JL__,'queued',ARRAY[__CL1__]::uuid[]);
INSERT INTO claim_model_derivations (revision_id,brain_id,run_id,request_id)
  VALUES (__R2__,__A__,(SELECT id FROM learning_runs WHERE brain_id=__A__),__MREQ1__);
INSERT INTO learning_run_inputs (run_id,brain_id,revision_id)
  VALUES ((SELECT id FROM learning_runs WHERE brain_id=__A__),__A__,__R2__);
INSERT INTO handover_runs (id,brain_id,title,contributions,selection,policy_id,actor_id,job_id,state,automatic)
  VALUES (__JH__,__A__,'handover-alpha',ARRAY[__R1Q__]::uuid[],'{}',__MP__,__OWNER__,__JH__,'queued',true);
INSERT INTO handover_run_inputs (run_id,brain_id,revision_id)
  VALUES ((SELECT id FROM handover_runs WHERE brain_id=__A__),__A__,__R1__);

INSERT INTO analytics_reports (id,brain_id,actor_id,job_id,algorithm,direction,parameters,analytics_epoch,privacy_sequence,node_count,edge_count)
  VALUES (gen_random_uuid(),__A__,__OWNER__,__JA__,'pagerank','both','{}',0,0,1,0);
INSERT INTO analytics_attempts (id,brain_id,report_id,job_id,lease_token,privacy_sequence,created_at,deadline)
  VALUES (gen_random_uuid(),__A__,(SELECT id FROM analytics_reports WHERE brain_id=__A__),__JA__,gen_random_uuid(),0,now(),now()+interval '1 hour');
INSERT INTO graph_generations (id,brain_id,kind,input_epoch,actor_id,job_id,state)
  VALUES (gen_random_uuid(),__A__,'knowledge',0,__OWNER__,__JG__,'queued');
INSERT INTO answer_requests (id,brain_id,actor_id,call_token,state)
  VALUES (gen_random_uuid(),__A__,__OWNER__,gen_random_uuid(),'retrieving');
INSERT INTO brain_directory (brain_id,name,description,archived,source_change)
  VALUES (__A__,'Alpha','fixture alpha description',false,gen_random_uuid());

INSERT INTO command_receipts (actor_id,key,operation,input,brain_id,response)
  VALUES (__OWNER__,'fixture-receipt-key','fixture-op','{"payload":"__MT_RECEIPT__"}',__A__,'{"result":"__MT_RECEIPT__"}');
INSERT INTO privacy_device_positions (brain_id,device_id,sequence) VALUES (__A__,__DEV__,0);

-- Control Brain B: enough content to prove per-record erasure still works.
INSERT INTO sources (id,brain_id,created_by,current_version) VALUES (__SB__,__B__,__OWNER__,__VB__);
INSERT INTO source_versions (id,source_id,brain_id,title,media_type,byte_length,created_by,processing)
  VALUES (__VB__,__SB__,__B__,'Beta title','text/plain',8,__OWNER__,'ready');
INSERT INTO source_chunks (id,brain_id,version_id,ordinal,byte_start,byte_end,line_start,line_end,content)
  VALUES (gen_random_uuid(),__B__,__VB__,0,0,8,1,1,'beta chunk content');
INSERT INTO claims (id,brain_id,created_by,current_revision) VALUES (__CLB__,__B__,__OWNER__,__RB__);
INSERT INTO claim_revisions (id,claim_id,brain_id,recorded_at,revision,subject_key,predicate_key,value_key)
  VALUES (__RB__,__CLB__,__B__,now(),'{"content":{"value":"beta value"}}','sub','pred','beta value');

-- Retained support/digest derivatives added after the original 029 fixture.
-- These are canonical schema rows for deletion proof, not publication proof.
INSERT INTO source_import_scopes(version_id,brain_id,operation_id,selection)
 VALUES(__V2__,__A__,(SELECT id FROM operation_bindings WHERE brain_id=__A__),'{}');
INSERT INTO agent_brain_usage(brain_id,device_id,host_kind) VALUES(__A__,__DEV__,'codex');
INSERT INTO learning_support_stages(run_id,brain_id,extraction_request_id,assessment_operation_id,verifier_version,payload)
 SELECT id,__A__,__MREQ1__,gen_random_uuid(),'source-support-3','{"marker":"__MT_CLAIM__"}' FROM learning_runs WHERE brain_id=__A__;
INSERT INTO memory_support_assessments(id,brain_id,revision_id,policy_id,verifier_version,actor_id,reason)
 VALUES(gen_random_uuid(),__A__,__R2__,__MP__,'source-support-3',__OWNER__,'__MT_CLAIM__');
INSERT INTO handover_support_stages(run_id,brain_id,synthesis_request_id,assessment_operation_id,verifier_version,payload)
 SELECT id,__A__,__MREQ1__,gen_random_uuid(),'source-support-3','{"marker":"__MT_CLAIM__"}' FROM handover_runs WHERE brain_id=__A__;
INSERT INTO automatic_support_excerpts(version_id,brain_id,parent_version_id,first_line,last_line,selection,identity_key,original_class,provenance)
 VALUES(__V2__,__A__,__V1__,1,1,'{}',md5(jsonb_build_array(__A__::uuid,__V1__::uuid,1,1,'{}'::jsonb,NULL)::text),'document','{"marker":"__MT_CLAIM__"}');
INSERT INTO excerpt_artifact_intents(artifact_id,brain_id,run_id,parent_version_id)
 SELECT gen_random_uuid(),__A__,id,__V2__ FROM learning_runs WHERE brain_id=__A__;
DO $$ DECLARE partition uuid:=gen_random_uuid(); generation uuid:=gen_random_uuid(); BEGIN
 INSERT INTO session_digest_partitions(id,brain_id,binding_id,task_id,actor_id,device_id,identity_key,host_session_id,selection)
 VALUES(partition,__A__,__CB1__,__T1__,__OWNER__,__DEV__,md5(jsonb_build_array(__CB1__::uuid,'fixture-session',NULL,'{}'::jsonb,NULL)::text),'fixture-session','{}');
 INSERT INTO session_digest_events(event_id,partition_id,brain_id) VALUES(__CE1__,partition,__A__);
 INSERT INTO session_digest_generations(id,brain_id,partition_id,page,contributors,policy_id,receipt,run_id,candidate_bytes,candidate_overhead)
 SELECT generation,__A__,partition,0,ARRAY[__R1__]::uuid[],__MP__,0,id,1024,32 FROM handover_runs WHERE brain_id=__A__;
 INSERT INTO session_digest_claims(brain_id,partition_id,page,claim_id,generation_id,revision_id,covered_receipt)
 VALUES(__A__,partition,0,__CL1__,generation,__R2__,0);
 INSERT INTO session_digest_pages(brain_id,partition_id,page,generation_id) VALUES(__A__,partition,0,generation);
END $$;
DO $$ DECLARE context uuid:=gen_random_uuid(); input uuid:=gen_random_uuid();
 entity uuid:=gen_random_uuid(); other_entity uuid:=gen_random_uuid();
 mention uuid:=gen_random_uuid(); other_mention uuid:=gen_random_uuid(); BEGIN
 INSERT INTO knowledge_mapping_contexts(id,brain_id,source_id,context) VALUES(context,__A__,__S1__,'{}');
 INSERT INTO knowledge_mapping_inputs(id,brain_id,source_version_id,schema_revision,adapter_revision,preparation_epoch,context_id,payload,created_by)
 VALUES(input,__A__,__V2__,'source-mentions-1','fixture',0,context,'{"marker":"__MT_CLAIM__"}',__OWNER__);
 INSERT INTO knowledge_entities(id,brain_id,kind,realm,normalized_label) VALUES
 (entity,__A__,'service',jsonb_build_object('source_id',__S1__::uuid,'context_id',context),'__MT_CLAIM__'),
 (other_entity,__A__,'service',jsonb_build_object('source_id',__S1__::uuid,'context_id',context),'__MT_TITLE__');
 INSERT INTO knowledge_mentions(id,brain_id,input_id,entity_id,local_key,byte_start,byte_end,quote,confidence) VALUES
 (mention,__A__,input,entity,'one',0,1,'__MT_CLAIM__',0.9),
 (other_mention,__A__,input,other_entity,'two',1,2,'__MT_TITLE__',0.9);
 INSERT INTO knowledge_relation_candidates(id,brain_id,input_id,from_mention,to_mention,relation,byte_start,byte_end,quote)
 VALUES(gen_random_uuid(),__A__,input,mention,other_mention,'alias',0,2,'__MT_CLAIM__');
END $$;
