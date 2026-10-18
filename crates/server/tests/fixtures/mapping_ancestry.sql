-- Run only in an owned disposable schema after all product migrations.
-- This exercises the production SQL gates/triggers with no model or fixture
-- oracle: exact graph identities, a deep chain, natural expiry and a late copy.
CREATE TEMP TABLE mapping_ancestry_proof(result jsonb);
DO $$
DECLARE actor uuid:=gen_random_uuid(); b uuid:=gen_random_uuid(); src uuid:=gen_random_uuid();
 root uuid:=gen_random_uuid(); parent uuid; child uuid; independent uuid:=gen_random_uuid();
 request uuid:=gen_random_uuid(); started timestamptz; elapsed double precision; n integer;
BEGIN
 INSERT INTO accounts(id,username) VALUES(actor,'isolated-ancestry-'||actor::text);
 PERFORM set_config('recollect.actor',actor::text,true);
 INSERT INTO brains(id,owner_id,name) VALUES(b,actor,'Isolated ancestry SQL proof');
 INSERT INTO sources(id,brain_id,created_by) VALUES(src,b,actor);
 INSERT INTO source_versions(id,source_id,brain_id,title,media_type,created_by,processing,byte_length)
  VALUES(root,src,b,'Synthetic ancestry root','text/plain',actor,'reference_only',0),
        (independent,src,b,'Independent synthetic version','text/plain',actor,'reference_only',0);
 UPDATE sources SET current_version=root WHERE id=src;
 parent:=root;
 FOR n IN 1..256 LOOP
  child:=gen_random_uuid();
  INSERT INTO source_versions(id,source_id,brain_id,title,media_type,created_by,processing,byte_length,retention_class)
   VALUES(child,src,b,'Synthetic nested excerpt','text/plain',actor,'reference_only',0,'support_excerpt');
  INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line)
   VALUES(b,child,src,parent,1,1);
  parent:=child;
 END LOOP;
 IF recollect_source_model_fenced(b,parent) THEN RAISE EXCEPTION 'Unfenced lineage must remain eligible'; END IF;
 UPDATE source_versions SET privacy_state='expired' WHERE id=root;
 IF recollect_source_model_fenced(b,parent) THEN RAISE EXCEPTION 'Natural parent expiry must not fence an independent excerpt'; END IF;
 INSERT INTO privacy_requests(id,brain_id,actor_id,target,cause,manifest)
  VALUES(request,b,actor,jsonb_build_object('kind','claim','id',gen_random_uuid()),'erase','{}');
 started:=clock_timestamp();
 PERFORM set_config('statement_timeout','2s',true);
 INSERT INTO model_input_fences(brain_id,source_version_id,request_id) VALUES(b,root,request);
 elapsed:=extract(epoch FROM clock_timestamp()-started)*1000;
 SELECT count(*) INTO n FROM model_input_fences WHERE brain_id=b;
 IF n<>257 THEN RAISE EXCEPTION 'Exact descendant fence count mismatch: %',n; END IF;
 IF NOT recollect_source_model_fenced(b,parent) THEN RAISE EXCEPTION 'Deep fenced lineage reopened'; END IF;
 IF recollect_source_model_fenced(b,independent) THEN RAISE EXCEPTION 'Independent evidence was fenced'; END IF;
 -- A later copy must inherit an existing fence without any model transmission.
 child:=gen_random_uuid();
 INSERT INTO source_versions(id,source_id,brain_id,title,media_type,created_by,processing,byte_length,retention_class)
  VALUES(child,src,b,'Synthetic late retained copy','text/plain',actor,'reference_only',0,'support_excerpt');
 INSERT INTO source_excerpts(brain_id,version_id,parent_source_id,parent_version_id,first_line,last_line)
  VALUES(b,child,src,parent,1,1);
 IF NOT EXISTS(SELECT 1 FROM model_input_fences WHERE brain_id=b AND source_version_id=child)
  THEN RAISE EXCEPTION 'Late excerpt bypassed ancestor fence'; END IF;
 -- Empty conflict transition tables must terminate rather than recurse.
 INSERT INTO model_input_fences(brain_id,source_version_id,request_id) VALUES(b,root,request) ON CONFLICT DO NOTHING;
 INSERT INTO mapping_ancestry_proof VALUES(jsonb_build_object(
  'depth',256,'descendant_fences',257,'propagation_ms',elapsed,
  'natural_expiry_preserved',true,'independent_version_preserved',true,
  'late_excerpt_fenced',true,'empty_transition_terminated',true,'model_requests',0));
END $$;
SELECT result FROM mapping_ancestry_proof;
