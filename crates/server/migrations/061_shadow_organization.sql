-- Internal experimental evaluation only. No public admission, production
-- producer/head, learning success, entity or topic is created here.
ALTER TABLE model_requests ADD COLUMN mapping_output_sha256 text CHECK(mapping_output_sha256 ~ '^[a-f0-9]{64}$');
ALTER TABLE model_requests ADD COLUMN mapping_source_version_id uuid;
ALTER TABLE model_requests ADD FOREIGN KEY(mapping_source_version_id,brain_id) REFERENCES source_versions(id,brain_id);
ALTER TABLE model_requests ADD CHECK(mapping_output_sha256 IS NULL OR mapping_source_version_id IS NOT NULL);
CREATE FUNCTION recollect_finish_model_request(rid uuid,token uuid,outcome text,reason text,returned text,it bigint,ot bigint,tt bigint,dims integer,cost double precision,mapping jsonb,mapping_source_version uuid)
RETURNS boolean LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE finished boolean; BEGIN
 finished:=recollect_finish_model_request(rid,token,outcome,reason,returned,it,ot,tt,dims,cost);
 IF finished AND outcome='succeeded' AND jsonb_typeof(mapping)='object' AND mapping_source_version IS NOT NULL THEN
  UPDATE model_requests q SET mapping_output_sha256=encode(sha256(convert_to(mapping::text,'UTF8')),'hex'),mapping_source_version_id=mapping_source_version
  WHERE q.id=rid AND q.call_token=token AND q.purpose='extraction' AND NOT q.suppressed AND NOT q.detail_expired
   AND EXISTS(SELECT 1 FROM model_request_inputs i WHERE i.brain_id=q.brain_id AND i.request_id=q.id AND i.kind='source_version' AND i.input_id=mapping_source_version)
   AND EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=q.brain_id AND v.id=mapping_source_version);
 END IF;
 RETURN finished; END $$;
REVOKE ALL ON FUNCTION recollect_finish_model_request(uuid,uuid,text,text,text,bigint,bigint,bigint,integer,double precision,jsonb,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_finish_model_request(uuid,uuid,text,text,text,bigint,bigint,bigint,integer,double precision,jsonb,uuid) TO recollect_app;
ALTER TABLE analytics_reports DROP CONSTRAINT analytics_reports_purpose_check;
ALTER TABLE analytics_reports ADD CHECK(purpose IN('topology','organization','organization_shadow'));
ALTER TABLE analytics_reports DROP CONSTRAINT analytics_reports_node_count_check;
ALTER TABLE analytics_reports ADD CHECK(node_count BETWEEN 0 AND 10000 AND(node_count>0 OR purpose='organization_shadow'));
CREATE TABLE knowledge_shadow_candidates(
 id uuid PRIMARY KEY,brain_id uuid NOT NULL,source_id uuid NOT NULL,source_version_id uuid NOT NULL,
 request_id uuid NOT NULL UNIQUE,policy_id uuid NOT NULL,actor_id uuid NOT NULL REFERENCES accounts(id),device_id uuid REFERENCES devices(id),
 context jsonb NOT NULL CHECK(octet_length(context::text)<=262144),
 schema_revision text NOT NULL CHECK(schema_revision='source-mentions-1'),adapter_revision text NOT NULL CHECK(adapter_revision='learning-quote-anchors-1'),
 output_sha256 text NOT NULL CHECK(output_sha256 ~ '^[a-f0-9]{64}$'),offered_inputs jsonb NOT NULL CHECK(octet_length(offered_inputs::text)<=262144),
 payload jsonb NOT NULL CHECK(octet_length(payload::text)<=262144),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),expires_at timestamptz NOT NULL CHECK(expires_at<=created_at+interval '7 days'),
 UNIQUE(id,brain_id),FOREIGN KEY(source_id,brain_id) REFERENCES sources(id,brain_id),
 FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id),
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id)
);
CREATE INDEX shadow_candidate_source ON knowledge_shadow_candidates(brain_id,source_version_id);
CREATE TRIGGER shadow_candidate_immutable BEFORE UPDATE ON knowledge_shadow_candidates FOR EACH ROW EXECUTE FUNCTION recollect_source_input_immutable();
CREATE TABLE knowledge_shadow_calculations(
 report_id uuid PRIMARY KEY,brain_id uuid NOT NULL,feature_parameters jsonb NOT NULL,
 expected_gds_version text NOT NULL CHECK(octet_length(expected_gds_version) BETWEEN 1 AND 64),
 projection jsonb NOT NULL CHECK(octet_length(projection::text)<=8388608),
 preparation jsonb NOT NULL CHECK(octet_length(preparation::text)<=8388608),
 result jsonb CHECK(octet_length(result::text)<=8388608),
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_shadow_sources(
 id uuid PRIMARY KEY,brain_id uuid NOT NULL,report_id uuid NOT NULL,source_id uuid NOT NULL,source_version_id uuid NOT NULL,
 candidate_id uuid,context jsonb NOT NULL CHECK(octet_length(context::text)<=262144),
 UNIQUE(report_id,source_version_id),UNIQUE(id,brain_id),
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(source_id,brain_id) REFERENCES sources(id,brain_id),FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id),
 FOREIGN KEY(candidate_id,brain_id) REFERENCES knowledge_shadow_candidates(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX shadow_source_version ON knowledge_shadow_sources(brain_id,source_version_id);
CREATE TRIGGER shadow_source_immutable BEFORE UPDATE ON knowledge_shadow_sources FOR EACH ROW EXECUTE FUNCTION recollect_source_input_immutable();
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_shadow_candidates','knowledge_shadow_calculations','knowledge_shadow_sources'] LOOP
 EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
 EXECUTE format('CREATE POLICY shadow_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
 EXECUTE format('CREATE POLICY shadow_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin'')))',tab);
 EXECUTE format('GRANT SELECT,INSERT,DELETE ON %I TO recollect_app',tab);
 END LOOP; END $$;
GRANT UPDATE ON knowledge_shadow_calculations TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(527,'knowledge_shadow_sources'),(528,'knowledge_shadow_calculations'),(529,'knowledge_shadow_candidates');
CREATE FUNCTION recollect_shadow_dependency_removed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; BEGIN
 SELECT array_agg(DISTINCT report_id) INTO reports FROM shadow_removed_sources;
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE kind='graph.analyze' AND state IN('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='content_removed' WHERE id=ANY(reports) AND state<>'removed';
 DELETE FROM knowledge_shadow_calculations WHERE report_id=ANY(reports);RETURN NULL;END $$;
REVOKE ALL ON FUNCTION recollect_shadow_dependency_removed() FROM PUBLIC;
CREATE TRIGGER shadow_source_removed AFTER DELETE ON knowledge_shadow_sources REFERENCING OLD TABLE AS shadow_removed_sources FOR EACH STATEMENT EXECUTE FUNCTION recollect_shadow_dependency_removed();
CREATE OR REPLACE FUNCTION recollect_organization_calculation_removed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_organization_calculations WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_inputs WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_sources WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_vectors WHERE report_id=NEW.id;
 DELETE FROM knowledge_shadow_calculations WHERE report_id=NEW.id;
 DELETE FROM knowledge_shadow_sources WHERE report_id=NEW.id;
 DELETE FROM knowledge_topic_sources m USING knowledge_topics t WHERE m.topic_id=t.id AND t.organization_report_id=NEW.id AND m.reason IN('hybrid_entity_affinity','hybrid_retained_vector','hybrid_entity_and_vector');
 RETURN NEW; END $$;
-- Any canonical change invalidates experimental results immediately; it does
-- not retry/admit an experiment or select a production configuration.
ALTER FUNCTION recollect_analytics_invalidate(uuid[]) RENAME TO recollect_pre_shadow_analytics_invalidate;
CREATE FUNCTION recollect_analytics_invalidate(affected uuid[]) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 PERFORM recollect_pre_shadow_analytics_invalidate(affected);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='analytics_inputs_changed'
 WHERE brain_id=ANY(affected) AND purpose='organization_shadow' AND state<>'removed';
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_invalidate(uuid[]) FROM PUBLIC;
CREATE FUNCTION recollect_shadow_request_clear() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 IF NEW.suppressed OR NEW.detail_expired THEN NEW.mapping_output_sha256:=NULL;NEW.mapping_source_version_id:=NULL; END IF;
 RETURN NEW; END $$;
CREATE TRIGGER shadow_request_clear BEFORE UPDATE ON model_requests FOR EACH ROW EXECUTE FUNCTION recollect_shadow_request_clear();
CREATE FUNCTION recollect_shadow_request_removed() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_shadow_candidates c USING shadow_new_requests n WHERE c.brain_id=n.brain_id AND c.request_id=n.id
 AND (n.suppressed OR n.detail_expired OR n.state<>'succeeded' OR n.mapping_output_sha256 IS DISTINCT FROM c.output_sha256 OR n.mapping_source_version_id IS DISTINCT FROM c.source_version_id);
 RETURN NULL; END $$;
CREATE TRIGGER shadow_request_removed AFTER UPDATE ON model_requests REFERENCING NEW TABLE AS shadow_new_requests FOR EACH STATEMENT EXECUTE FUNCTION recollect_shadow_request_removed();
CREATE FUNCTION recollect_shadow_source_fenced() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_shadow_candidates c USING shadow_new_fences f WHERE c.brain_id=f.brain_id AND c.source_version_id=f.source_version_id;
 DELETE FROM knowledge_shadow_sources s USING shadow_new_fences f WHERE s.brain_id=f.brain_id AND s.source_version_id=f.source_version_id;
 RETURN NULL; END $$;
CREATE TRIGGER shadow_source_fenced AFTER INSERT ON model_input_fences REFERENCING NEW TABLE AS shadow_new_fences FOR EACH STATEMENT EXECUTE FUNCTION recollect_shadow_source_fenced();
CREATE FUNCTION recollect_shadow_maintain() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; BEGIN
 DELETE FROM knowledge_shadow_candidates WHERE id IN(SELECT id FROM knowledge_shadow_candidates WHERE expires_at<=clock_timestamp() ORDER BY expires_at,id LIMIT 100);
 SELECT array_agg(id) INTO reports FROM(SELECT id FROM analytics_reports WHERE purpose='organization_shadow' AND state<>'removed' AND expires_at<=clock_timestamp() ORDER BY expires_at,id LIMIT 100)r;
 UPDATE jobs SET state='cancelled',error_code='analytics_inputs_expired',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE kind='graph.analyze' AND state IN('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='analytics_inputs_expired' WHERE id=ANY(reports) AND state<>'removed';
END $$;
REVOKE ALL ON FUNCTION recollect_shadow_request_clear(),recollect_shadow_request_removed(),recollect_shadow_source_fenced(),recollect_shadow_maintain() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_shadow_maintain() TO recollect_app;

CREATE FUNCTION recollect_shadow_candidates_affected(b uuid,m jsonb,erasing boolean) RETURNS uuid[] LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH versions AS MATERIALIZED(SELECT unnest(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')) id WHERE NOT erasing UNION SELECT id FROM recollect_source_derivative_versions(b,recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')) WHERE erasing),
 claims AS(SELECT unnest(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions')) id)
 SELECT coalesce(array_agg(c.id ORDER BY c.id),'{}') FROM knowledge_shadow_candidates c WHERE c.brain_id=b AND(c.source_version_id IN(SELECT id FROM versions) OR(erasing AND EXISTS(SELECT 1 FROM model_request_inputs i WHERE i.brain_id=b AND i.request_id=c.request_id AND((i.kind='source_version' AND i.input_id IN(SELECT id FROM versions))OR(i.kind='claim_revision' AND i.input_id IN(SELECT id FROM claims)))))) $$;
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_shadow_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; ids uuid[]; reports uuid[]; BEGIN
 m:=recollect_pre_shadow_closure(b,k,t,c);ids:=recollect_shadow_candidates_affected(b,m,c='erase');
 SELECT coalesce(array_agg(DISTINCT s.report_id),'{}') INTO reports FROM knowledge_shadow_sources s WHERE s.brain_id=b AND(s.source_version_id=ANY(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')) OR s.candidate_id=ANY(ids));
 RETURN m||jsonb_build_object('knowledge_shadow_candidates',ids,'knowledge_shadow_reports',reports);END $$;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_shadow_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$ SELECT recollect_pre_shadow_normalize(m)||jsonb_build_object('knowledge_shadow_candidates',recollect_privacy_ids(m,'knowledge_shadow_candidates'),'knowledge_shadow_reports',recollect_privacy_ids(m,'knowledge_shadow_reports')) $$;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_shadow_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; ids uuid[]; versions uuid[]; BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 ids:=recollect_privacy_ids(r.manifest,'knowledge_shadow_candidates')||recollect_shadow_candidates_affected(r.brain_id,r.manifest,r.cause='erase');
 DELETE FROM knowledge_shadow_candidates WHERE brain_id=r.brain_id AND(r.target->>'kind'='brain' OR id=ANY(ids));
 versions:=recollect_privacy_ids(r.manifest,'source_versions')||recollect_privacy_ids(r.manifest,'model_input_sources');
 IF r.cause='erase' THEN SELECT coalesce(array_agg(id),'{}') INTO versions FROM recollect_source_derivative_versions(r.brain_id,versions); END IF;
 DELETE FROM knowledge_shadow_sources WHERE brain_id=r.brain_id AND(r.target->>'kind'='brain' OR source_version_id=ANY(versions) OR report_id=ANY(recollect_privacy_ids(r.manifest,'knowledge_shadow_reports')));
 PERFORM recollect_pre_shadow_apply(rid);END $$;
REVOKE ALL ON FUNCTION recollect_shadow_candidates_affected(uuid,jsonb,boolean),recollect_privacy_closure(uuid,text,uuid,text),recollect_privacy_normalize(jsonb),recollect_privacy_apply(uuid) FROM PUBLIC;
