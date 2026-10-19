-- An unqualified candidate only: this migration installs no recipe/head and
-- never requests embeddings. Source contributors are independent of NER.
ALTER TABLE knowledge_organization_recipes DROP CONSTRAINT knowledge_organization_recipes_recipe_check;
ALTER TABLE knowledge_organization_recipes ADD CHECK(recipe IN('entity-idf-bipartite-leiden-1','source-idf-retained-vector-leiden-1'));
ALTER TABLE knowledge_organization_recipes ADD COLUMN embedding_provider text;
ALTER TABLE knowledge_organization_recipes ADD COLUMN embedding_model text;
ALTER TABLE knowledge_organization_recipes ADD COLUMN embedding_dimensions integer;
ALTER TABLE knowledge_organization_recipes ADD COLUMN representation text;
ALTER TABLE knowledge_organization_recipes ADD COLUMN feature_parameters jsonb NOT NULL DEFAULT '{"neighbors":5,"minimum_cosine":0.75,"vector_scale":1.0}'::jsonb
 CHECK(jsonb_typeof(feature_parameters)='object' AND feature_parameters ?& ARRAY['neighbors','minimum_cosine','vector_scale'] AND feature_parameters-'neighbors'-'minimum_cosine'-'vector_scale'='{}'::jsonb
 AND jsonb_typeof(feature_parameters->'neighbors')='number' AND jsonb_typeof(feature_parameters->'minimum_cosine')='number' AND jsonb_typeof(feature_parameters->'vector_scale')='number'
 AND (feature_parameters->>'neighbors')::integer IN(3,5,8) AND (feature_parameters->>'minimum_cosine')::float8 IN(0.65,0.75,0.85) AND (feature_parameters->>'vector_scale')::float8 IN(0.25,0.5,1.0));
ALTER TABLE knowledge_organization_recipes ADD CHECK(recipe<>'source-idf-retained-vector-leiden-1' OR
 (embedding_provider IS NOT NULL AND embedding_model IS NOT NULL AND embedding_dimensions IS NOT NULL AND representation IS NOT NULL AND embedding_dimensions BETWEEN 1 AND 4096 AND representation='engineering-text-1'));

CREATE TABLE knowledge_source_inputs(
 id uuid PRIMARY KEY, brain_id uuid NOT NULL, source_id uuid NOT NULL,
 source_version_id uuid NOT NULL, context jsonb NOT NULL CHECK(octet_length(context::text)<=262144),
 created_by uuid NOT NULL REFERENCES accounts(id), created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id),
 FOREIGN KEY(source_id,brain_id) REFERENCES sources(id,brain_id),
 FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id)
);
CREATE INDEX organization_source_input_identity ON knowledge_source_inputs(brain_id,source_version_id);
CREATE FUNCTION recollect_organization_source_context(b uuid,v uuid,s uuid) RETURNS jsonb LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT jsonb_build_object('selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=b AND version_id=v),'{}'::jsonb),
 'areas',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=b AND m.source_id=s AND g.kind='area' ORDER BY g.id),
 'environments',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=b AND m.source_id=s AND g.kind='environment' ORDER BY g.id)) $$;
CREATE FUNCTION recollect_organization_current_source_ids(b uuid,targets uuid[]) RETURNS TABLE(input_id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp SET recollect.bulk_qualification='on' AS $$
 WITH RECURSIVE requested AS MATERIALIZED(SELECT * FROM knowledge_source_inputs WHERE brain_id=b AND id=ANY(targets) AND cardinality(targets)<=250),
 lineage(root,id) AS(SELECT source_version_id,source_version_id FROM requested UNION SELECT l.root,e.parent_version_id FROM lineage l JOIN source_excerpts e ON e.brain_id=b AND e.version_id=l.id),
 lineage_status AS MATERIALIZED(
  SELECT l.root,bool_or(e.version_id IS NULL) has_root,bool_or(v.id IS NULL OR v.privacy_state='erased' OR f.source_version_id IS NOT NULL OR(e.version_id IS NOT NULL AND p.id IS NULL)) blocked
  FROM lineage l LEFT JOIN source_versions v ON v.brain_id=b AND v.id=l.id
  LEFT JOIN source_excerpts e ON e.brain_id=b AND e.version_id=l.id
  LEFT JOIN source_versions p ON p.brain_id=b AND p.id=e.parent_version_id AND p.source_id=e.parent_source_id
  LEFT JOIN model_input_fences f ON f.brain_id=b AND f.source_version_id=l.id GROUP BY l.root)
 SELECT i.id FROM requested i JOIN sources s ON s.brain_id=i.brain_id AND s.id=i.source_id AND s.current_version=i.source_version_id
 JOIN recollect_source_knowledge v ON v.brain_id=i.brain_id AND v.id=i.source_version_id
 JOIN lineage_status ancestry ON ancestry.root=v.id AND ancestry.has_root AND NOT ancestry.blocked
 WHERE v.processing='ready' AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
 AND i.context=jsonb_build_object('selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=b AND version_id=i.source_version_id),'{}'::jsonb),
 'areas',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=b AND m.source_id=i.source_id AND g.kind='area' ORDER BY g.id),
 'environments',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=b AND m.source_id=i.source_id AND g.kind='environment' ORDER BY g.id)) $$;
CREATE FUNCTION recollect_source_input_immutable() RETURNS trigger LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN RAISE EXCEPTION 'Source feature input identities are immutable'; END $$;
CREATE TRIGGER source_input_immutable BEFORE UPDATE ON knowledge_source_inputs FOR EACH ROW EXECUTE FUNCTION recollect_source_input_immutable();
CREATE TABLE knowledge_organization_sources(
 brain_id uuid NOT NULL, report_id uuid NOT NULL, input_id uuid NOT NULL,
 PRIMARY KEY(report_id,input_id),
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_organization_vectors(
 brain_id uuid NOT NULL, report_id uuid NOT NULL, entry_id uuid NOT NULL, profile_id uuid NOT NULL, request_id uuid NOT NULL,
 PRIMARY KEY(report_id,entry_id),
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(entry_id,brain_id) REFERENCES semantic_entries(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(profile_id,brain_id) REFERENCES semantic_profiles(id,brain_id),
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
CREATE INDEX organization_vector_request ON knowledge_organization_vectors(brain_id,request_id);
ALTER TABLE knowledge_organization_calculations ADD COLUMN source_features jsonb;
ALTER TABLE knowledge_organization_outbox ADD COLUMN vector_coverage text;
CREATE TABLE knowledge_topic_sources(
 brain_id uuid NOT NULL, topic_id uuid NOT NULL, input_id uuid NOT NULL,
 reason text NOT NULL CHECK(reason IN('hybrid_entity_affinity','hybrid_retained_vector','hybrid_entity_and_vector','explicit_source_split')),
 weight double precision NOT NULL CHECK(weight>0 AND weight<'Infinity'::float8),
 PRIMARY KEY(topic_id,input_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX organization_sources_input ON knowledge_topic_sources(brain_id,input_id,topic_id);
CREATE TABLE knowledge_topic_source_labels(
 brain_id uuid NOT NULL, topic_id uuid PRIMARY KEY, basis_input uuid NOT NULL,
 label text NOT NULL CHECK(octet_length(label) BETWEEN 1 AND 240),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(basis_input,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_topic_source_exclusions(
 brain_id uuid NOT NULL, topic_id uuid NOT NULL, input_id uuid NOT NULL, PRIMARY KEY(topic_id,input_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_topic_source_splits(
 brain_id uuid NOT NULL, original_topic_id uuid NOT NULL, split_topic_id uuid NOT NULL, input_id uuid NOT NULL,
 PRIMARY KEY(original_topic_id,input_id),
 FOREIGN KEY(original_topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(split_topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_organization_source_lineage(
 id uuid PRIMARY KEY, brain_id uuid NOT NULL, topic_id uuid NOT NULL, previous_topic_id uuid NOT NULL,
 basis_input uuid NOT NULL, report_id uuid NOT NULL,
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(basis_input,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE
);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_source_inputs','knowledge_organization_sources','knowledge_organization_vectors',
 'knowledge_topic_sources','knowledge_topic_source_labels','knowledge_topic_source_exclusions','knowledge_topic_source_splits','knowledge_organization_source_lineage'] LOOP
 EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
 EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
 EXECUTE format('CREATE POLICY organization_source_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
 EXECUTE format('CREATE POLICY organization_source_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin'')))',tab);
 EXECUTE format('GRANT SELECT,INSERT,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
GRANT UPDATE ON knowledge_topic_sources,knowledge_topic_source_labels TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (530,'knowledge_organization_source_lineage'),(531,'knowledge_topic_source_splits'),(532,'knowledge_topic_source_labels'),
 (533,'knowledge_topic_source_exclusions'),(534,'knowledge_topic_sources'),(535,'knowledge_organization_vectors'),
 (536,'knowledge_organization_sources'),(553,'knowledge_source_inputs');

-- Source context/version fences are requalified before every view/publication.
-- Exact dependency removal also physically clears the complete aggregate.
CREATE FUNCTION recollect_organization_source_dependency_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; affected uuid[]; changed_reports boolean; BEGIN
 SELECT array_agg(DISTINCT report_id),array_agg(DISTINCT brain_id) INTO reports,affected FROM organization_removed_source_dependencies;
 IF reports IS NULL THEN RETURN NULL; END IF;
 SELECT EXISTS(SELECT 1 FROM analytics_reports WHERE id=ANY(reports) AND state<>'removed') INTO changed_reports;
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE kind='graph.analyze' AND state IN('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='content_removed'
 WHERE id=ANY(reports) AND state<>'removed';
 DELETE FROM knowledge_organization_calculations WHERE report_id=ANY(reports);
 IF changed_reports THEN PERFORM recollect_analytics_invalidate(affected); END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER organization_source_dependency_removed AFTER DELETE ON knowledge_organization_sources REFERENCING OLD TABLE AS organization_removed_source_dependencies FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_source_dependency_removed();
CREATE TRIGGER organization_vector_dependency_removed AFTER DELETE ON knowledge_organization_vectors REFERENCING OLD TABLE AS organization_removed_source_dependencies FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_source_dependency_removed();
CREATE OR REPLACE FUNCTION recollect_organization_calculation_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_organization_calculations WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_inputs WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_sources WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_vectors WHERE report_id=NEW.id;
 DELETE FROM knowledge_topic_sources m USING knowledge_topics t WHERE m.topic_id=t.id AND t.organization_report_id=NEW.id AND m.reason IN('hybrid_entity_affinity','hybrid_retained_vector','hybrid_entity_and_vector');
 RETURN NEW; END $$;
CREATE FUNCTION recollect_organization_vectors_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; affected uuid[]; BEGIN
 SELECT array_agg(DISTINCT d.report_id) INTO reports FROM knowledge_organization_vectors d JOIN organization_old_vectors o ON o.id=d.entry_id
 JOIN organization_new_vectors n ON n.id=o.id WHERE
 (o.embedding IS DISTINCT FROM n.embedding OR o.state IS DISTINCT FROM n.state OR o.request_id IS DISTINCT FROM n.request_id OR o.truncated IS DISTINCT FROM n.truncated OR o.profile_id IS DISTINCT FROM n.profile_id OR o.source_version_id IS DISTINCT FROM n.source_version_id OR o.input_id IS DISTINCT FROM n.input_id);
 SELECT array_agg(DISTINCT n.brain_id) INTO affected FROM organization_new_vectors n JOIN organization_old_vectors o ON o.id=n.id WHERE
 (o.embedding IS DISTINCT FROM n.embedding OR o.state IS DISTINCT FROM n.state OR o.request_id IS DISTINCT FROM n.request_id OR o.truncated IS DISTINCT FROM n.truncated OR o.profile_id IS DISTINCT FROM n.profile_id OR o.source_version_id IS DISTINCT FROM n.source_version_id OR o.input_id IS DISTINCT FROM n.input_id);
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE kind='graph.analyze' AND state IN('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='content_removed' WHERE id=ANY(reports) AND state<>'removed';
 IF affected IS NOT NULL THEN PERFORM recollect_analytics_invalidate(affected); END IF;
 RETURN NULL; END $$;
CREATE TRIGGER organization_vectors_changed AFTER UPDATE ON semantic_entries REFERENCING OLD TABLE AS organization_old_vectors NEW TABLE AS organization_new_vectors FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_vectors_changed();
CREATE TRIGGER organization_vector_inserted AFTER INSERT ON semantic_entries REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed();
CREATE TRIGGER organization_vector_deleted AFTER DELETE ON semantic_entries REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed();
CREATE FUNCTION recollect_organization_vector_head_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; affected uuid[]; BEGIN
 IF TG_OP='DELETE' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM old_rows;
 ELSE SELECT array_agg(DISTINCT brain_id) INTO affected FROM new_rows; END IF;
 SELECT array_agg(DISTINCT report_id) INTO reports FROM knowledge_organization_vectors WHERE brain_id=ANY(affected);
 UPDATE jobs SET state='cancelled',error_code='analytics_inputs_changed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp() WHERE kind='graph.analyze' AND state IN('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='analytics_inputs_changed' WHERE id=ANY(reports) AND state<>'removed';
 IF affected IS NOT NULL THEN PERFORM recollect_analytics_invalidate(affected); END IF;
 RETURN NULL; END $$;
CREATE TRIGGER organization_semantic_head_insert AFTER INSERT ON semantic_heads REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_vector_head_changed();
CREATE TRIGGER organization_semantic_head_update AFTER UPDATE ON semantic_heads REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_vector_head_changed();
CREATE TRIGGER organization_semantic_head_delete AFTER DELETE ON semantic_heads REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_vector_head_changed();
CREATE FUNCTION recollect_organization_vector_suppressed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_organization_vectors d USING organization_new_requests n,organization_old_requests o
 WHERE n.id=o.id AND n.suppressed AND NOT o.suppressed AND d.brain_id=n.brain_id AND d.request_id=n.id;
 RETURN NULL; END $$;
CREATE TRIGGER organization_vector_suppressed AFTER UPDATE ON model_requests REFERENCING OLD TABLE AS organization_old_requests NEW TABLE AS organization_new_requests FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_vector_suppressed();
CREATE FUNCTION recollect_organization_source_fenced() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_source_inputs i USING organization_source_new_fences f WHERE i.brain_id=f.brain_id AND i.source_version_id=f.source_version_id;
 RETURN NULL; END $$;
CREATE TRIGGER organization_source_fenced AFTER INSERT ON model_input_fences REFERENCING NEW TABLE AS organization_source_new_fences FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_source_fenced();

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_source_features_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; versions uuid[]; ids uuid[]; BEGIN
 m:=recollect_pre_source_features_closure(b,k,t,c);
 versions:=recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources');
 IF c='erase' THEN SELECT coalesce(array_agg(id),'{}') INTO versions FROM recollect_source_derivative_versions(b,versions); END IF;
 SELECT coalesce(array_agg(id ORDER BY id),'{}') INTO ids FROM knowledge_source_inputs WHERE brain_id=b AND (k='brain' OR source_version_id=ANY(versions));
 RETURN m||jsonb_build_object('knowledge_source_inputs',ids); END $$;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_source_features_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_source_features_normalize(m)||jsonb_build_object('knowledge_source_inputs',recollect_privacy_ids(m,'knowledge_source_inputs')) $$;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_source_features_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; versions uuid[]; BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 versions:=recollect_privacy_ids(r.manifest,'source_versions')||recollect_privacy_ids(r.manifest,'model_input_sources');
 IF r.cause='erase' THEN SELECT coalesce(array_agg(id),'{}') INTO versions FROM recollect_source_derivative_versions(r.brain_id,versions); END IF;
 DELETE FROM knowledge_source_inputs WHERE brain_id=r.brain_id AND (r.target->>'kind'='brain' OR source_version_id=ANY(versions) OR id=ANY(recollect_privacy_ids(r.manifest,'knowledge_source_inputs')));
 PERFORM recollect_pre_source_features_apply(rid); END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text),recollect_privacy_normalize(jsonb),recollect_privacy_apply(uuid),recollect_organization_source_dependency_removed(),recollect_organization_vectors_changed(),recollect_organization_vector_suppressed(),recollect_organization_source_fenced(),recollect_source_input_immutable(),recollect_organization_vector_head_changed() FROM PUBLIC;
CREATE OR REPLACE FUNCTION recollect_organization_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT b.id,p.created_by FROM brains b JOIN knowledge_organization_outbox o ON o.brain_id=b.id
 JOIN model_policy_heads h ON h.brain_id=b.id JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id
 JOIN accounts a ON a.id=p.created_by WHERE NOT b.archived AND a.enabled AND p.policy->>'enabled'='true'
 AND (p.policy->'purposes' ? 'extraction' OR p.policy->'purposes' ? 'embedding')
 AND recollect_effective_role(b.id,p.created_by) IN('writer','admin') ORDER BY b.id $$;
