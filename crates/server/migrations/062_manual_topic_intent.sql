-- Human-adopted contributors are independent of invalidatable machine results.
-- No model call, recipe qualification or automatic topic selection occurs here.
ALTER TABLE knowledge_topics DROP CONSTRAINT knowledge_topics_kind_check;
ALTER TABLE knowledge_topics ADD CHECK(kind IN('facet','community','split','manual_overlay'));
ALTER TABLE knowledge_topic_memberships DROP CONSTRAINT knowledge_topic_memberships_reason_check;
ALTER TABLE knowledge_topic_memberships ADD CHECK(reason IN('entity_facet','weighted_leiden','explicit_split','explicit_overlay'));
ALTER TABLE knowledge_topic_sources DROP CONSTRAINT knowledge_topic_sources_reason_check;
ALTER TABLE knowledge_topic_sources ADD CHECK(reason IN('hybrid_entity_affinity','hybrid_retained_vector','hybrid_entity_and_vector','explicit_source_split','explicit_source_overlay'));
-- Historical entity-topic names allowed 256 bytes. Preserve those exact names
-- when bridging their canonical source basis; current command limits stay put.
ALTER TABLE knowledge_topic_source_labels DROP CONSTRAINT knowledge_topic_source_labels_label_check;
ALTER TABLE knowledge_topic_source_labels ADD CHECK(octet_length(label) BETWEEN 1 AND 256);

CREATE TABLE knowledge_topic_intent_sources(
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 topic_id uuid NOT NULL,
 input_id uuid NOT NULL,
 UNIQUE(id,brain_id),
 UNIQUE(topic_id,input_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_source_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX organization_intent_source_input ON knowledge_topic_intent_sources(brain_id,input_id);
CREATE TABLE knowledge_topic_intent_mappings(
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 topic_id uuid NOT NULL,
 input_id uuid NOT NULL,
 entity_id uuid NOT NULL,
 UNIQUE(id,brain_id),
 UNIQUE(topic_id,input_id,entity_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(entity_id,brain_id) REFERENCES knowledge_entities(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX organization_intent_mapping_input ON knowledge_topic_intent_mappings(brain_id,input_id);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_topic_intent_sources','knowledge_topic_intent_mappings'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY organization_intent_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY organization_intent_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin'')))',tab);
  EXECUTE format('GRANT SELECT,INSERT,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (525,'knowledge_topic_intent_mappings'),(526,'knowledge_topic_intent_sources');

-- Upgrade seeds are provisional dependencies, not visible accepted membership.
-- Rust requalifies the entire cohort before refreshing or converting a group.
WITH edited AS MATERIALIZED(
 SELECT t.id,t.brain_id FROM knowledge_topics t WHERE t.kind IN('community','split') AND
 (t.pinned OR EXISTS(SELECT 1 FROM knowledge_topic_labels l WHERE l.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_labels l WHERE l.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_exclusions e WHERE e.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions e WHERE e.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_splits s WHERE s.original_topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_splits s WHERE s.original_topic_id=t.id))),
 legacy AS MATERIALIZED(
 SELECT DISTINCT ON(i.brain_id,i.source_version_id,c.context)
 i.brain_id,v.source_id,i.source_version_id,c.context,i.created_by
 FROM edited t JOIN knowledge_topic_memberships m ON m.brain_id=t.brain_id AND m.topic_id=t.id
 JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id
 JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id
 JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id
 ORDER BY i.brain_id,i.source_version_id,c.context,i.id)
INSERT INTO knowledge_source_inputs(id,brain_id,source_id,source_version_id,context,created_by)
 SELECT gen_random_uuid(),l.brain_id,l.source_id,l.source_version_id,l.context,l.created_by FROM legacy l
 WHERE NOT EXISTS(SELECT 1 FROM knowledge_source_inputs s WHERE s.brain_id=l.brain_id AND s.source_version_id=l.source_version_id AND s.context=l.context);
WITH edited AS MATERIALIZED(
 SELECT t.id,t.brain_id FROM knowledge_topics t WHERE t.kind IN('community','split') AND
 (t.pinned OR EXISTS(SELECT 1 FROM knowledge_topic_labels l WHERE l.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_labels l WHERE l.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_exclusions e WHERE e.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions e WHERE e.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_splits s WHERE s.original_topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_splits s WHERE s.original_topic_id=t.id)))
INSERT INTO knowledge_topic_intent_sources(id,brain_id,topic_id,input_id)
 SELECT gen_random_uuid(),x.brain_id,x.topic_id,x.input_id FROM(
 SELECT m.brain_id,m.topic_id,m.input_id FROM knowledge_topic_sources m
 JOIN edited t ON t.brain_id=m.brain_id AND t.id=m.topic_id
 UNION
 SELECT m.brain_id,m.topic_id,s.id FROM knowledge_topic_memberships m
 JOIN edited t ON t.brain_id=m.brain_id AND t.id=m.topic_id
 JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id
 JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id
 JOIN knowledge_source_inputs s ON s.brain_id=i.brain_id AND s.source_version_id=i.source_version_id AND s.context=c.context
 )x;
WITH edited AS MATERIALIZED(
 SELECT t.id,t.brain_id FROM knowledge_topics t WHERE t.kind IN('community','split') AND
 (t.pinned OR EXISTS(SELECT 1 FROM knowledge_topic_labels l WHERE l.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_labels l WHERE l.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_exclusions e WHERE e.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions e WHERE e.topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_splits s WHERE s.original_topic_id=t.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_splits s WHERE s.original_topic_id=t.id)))
INSERT INTO knowledge_topic_intent_mappings(id,brain_id,topic_id,input_id,entity_id)
 SELECT gen_random_uuid(),m.brain_id,m.topic_id,m.input_id,m.entity_id FROM knowledge_topic_memberships m
 JOIN edited t ON t.brain_id=m.brain_id AND t.id=m.topic_id;
-- Bridge an adopted legacy name during upgrade, before an old producer can be
-- replaced and cascade its mapping-input label. Exact version/context only.
-- Durable controls precede the name bridge, so excluded/split bases stay hidden
-- even when their old mapping inputs are later replaced.
INSERT INTO knowledge_topic_source_exclusions(brain_id,topic_id,input_id)
 SELECT DISTINCT e.brain_id,e.topic_id,s.id FROM knowledge_topic_exclusions e
 JOIN knowledge_mapping_inputs i ON i.brain_id=e.brain_id AND i.id=e.input_id
 JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id
 JOIN knowledge_topic_intent_sources m ON m.brain_id=e.brain_id AND m.topic_id=e.topic_id
 JOIN knowledge_source_inputs s ON s.brain_id=m.brain_id AND s.id=m.input_id
 AND s.source_version_id=i.source_version_id AND s.context=c.context
 ON CONFLICT(topic_id,input_id) DO NOTHING;
WITH mapped AS MATERIALIZED(
 SELECT DISTINCT x.brain_id,x.original_topic_id,x.split_topic_id,s.id input_id
 FROM knowledge_topic_splits x
 JOIN knowledge_mapping_inputs i ON i.brain_id=x.brain_id AND i.id=x.input_id
 JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id
 JOIN knowledge_topic_intent_sources m ON m.brain_id=x.brain_id AND m.topic_id=x.original_topic_id
 JOIN knowledge_source_inputs s ON s.brain_id=m.brain_id AND s.id=m.input_id
 AND s.source_version_id=i.source_version_id AND s.context=c.context),
 resolved AS(SELECT brain_id,original_topic_id,input_id FROM mapped
 GROUP BY brain_id,original_topic_id,input_id HAVING count(DISTINCT split_topic_id)=1)
INSERT INTO knowledge_topic_source_splits(brain_id,original_topic_id,split_topic_id,input_id)
 SELECT m.brain_id,m.original_topic_id,m.split_topic_id,m.input_id FROM mapped m
 JOIN resolved r USING(brain_id,original_topic_id,input_id)
 ON CONFLICT(original_topic_id,input_id) DO NOTHING;
INSERT INTO knowledge_topic_sources(brain_id,topic_id,input_id,reason,weight)
 SELECT x.brain_id,x.split_topic_id,x.input_id,'explicit_source_split',1
 FROM knowledge_topic_source_splits x
 JOIN knowledge_topic_intent_sources m ON m.brain_id=x.brain_id AND m.topic_id=x.original_topic_id AND m.input_id=x.input_id
 ON CONFLICT(topic_id,input_id) DO NOTHING;
INSERT INTO knowledge_topic_source_labels(brain_id,topic_id,basis_input,label)
 SELECT DISTINCT ON(l.topic_id) l.brain_id,l.topic_id,s.id,l.label
 FROM knowledge_topic_labels l
 JOIN knowledge_mapping_inputs i ON i.brain_id=l.brain_id AND i.id=l.basis_input
 JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id
 JOIN knowledge_topic_intent_sources m ON m.brain_id=l.brain_id AND m.topic_id=l.topic_id
 JOIN knowledge_source_inputs s ON s.brain_id=m.brain_id AND s.id=m.input_id
 AND s.source_version_id=i.source_version_id AND s.context=c.context
 WHERE (NOT EXISTS(SELECT 1 FROM knowledge_topic_exclusions e WHERE e.brain_id=l.brain_id AND e.topic_id=l.topic_id AND e.input_id=l.basis_input)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_exclusions e WHERE e.brain_id=l.brain_id AND e.topic_id=l.topic_id AND e.input_id=s.id))
 AND (NOT EXISTS(SELECT 1 FROM knowledge_topic_splits x WHERE x.brain_id=l.brain_id AND x.original_topic_id=l.topic_id AND x.input_id=l.basis_input)
 OR EXISTS(SELECT 1 FROM knowledge_topic_source_splits x WHERE x.brain_id=l.brain_id AND x.original_topic_id=l.topic_id AND x.input_id=s.id))
 ORDER BY l.topic_id,s.id ON CONFLICT(topic_id) DO NOTHING;

CREATE FUNCTION recollect_organization_prune_brain(b uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$ BEGIN
 DELETE FROM knowledge_topics t WHERE t.brain_id=b
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_memberships m WHERE m.brain_id=b AND m.topic_id=t.id)
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_sources m WHERE m.brain_id=b AND m.topic_id=t.id)
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_sources m WHERE m.brain_id=b AND m.topic_id=t.id)
 AND NOT EXISTS(SELECT 1 FROM knowledge_topic_intent_mappings m WHERE m.brain_id=b AND m.topic_id=t.id);
 DELETE FROM knowledge_entities e WHERE e.brain_id=b
 AND NOT EXISTS(SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=b AND m.entity_id=e.id);
 DELETE FROM knowledge_mapping_contexts c WHERE c.brain_id=b
 AND NOT EXISTS(SELECT 1 FROM knowledge_mapping_inputs i WHERE i.brain_id=b AND i.context_id=c.id);
 UPDATE command_receipts SET invalidated=true,input='{}',response=NULL
 WHERE brain_id=b AND operation IN('organization.topic','organization.alias');
END $$;
CREATE OR REPLACE FUNCTION recollect_organization_prune() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM organization_removed_inputs LOOP
  PERFORM recollect_organization_prune_brain(b);
 END LOOP;
 RETURN NULL;
END $$;
CREATE FUNCTION recollect_organization_source_prune() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM organization_removed_source_inputs LOOP
  PERFORM recollect_organization_prune_brain(b);
 END LOOP;
 RETURN NULL;
END $$;
CREATE TRIGGER organization_source_input_removed AFTER DELETE ON knowledge_source_inputs
 REFERENCING OLD TABLE AS organization_removed_source_inputs FOR EACH STATEMENT
 EXECUTE FUNCTION recollect_organization_source_prune();

CREATE FUNCTION recollect_topic_intent_affected(b uuid,m jsonb,erasing boolean,whole_brain boolean)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE versions uuid[]; source_ids uuid[]; mapping_ids uuid[]; topic_ids uuid[]; entity_ids uuid[];
 sources uuid[]; mappings uuid[]; BEGIN
 versions:=recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources');
 IF erasing THEN SELECT coalesce(array_agg(id),'{}') INTO versions FROM recollect_source_derivative_versions(b,versions); END IF;
 source_ids:=recollect_privacy_ids(m,'knowledge_source_inputs');
 mapping_ids:=recollect_privacy_ids(m,'knowledge_mapping_inputs');
 topic_ids:=recollect_privacy_ids(m,'knowledge_topics');
 entity_ids:=recollect_privacy_ids(m,'knowledge_entities');
 SELECT coalesce(array_agg(s.id ORDER BY s.id),'{}') INTO sources
 FROM knowledge_topic_intent_sources s JOIN knowledge_source_inputs i ON i.brain_id=s.brain_id AND i.id=s.input_id
 WHERE s.brain_id=b AND(whole_brain OR s.input_id=ANY(source_ids) OR s.topic_id=ANY(topic_ids) OR i.source_version_id=ANY(versions));
 SELECT coalesce(array_agg(s.id ORDER BY s.id),'{}') INTO mappings
 FROM knowledge_topic_intent_mappings s JOIN knowledge_mapping_inputs i ON i.brain_id=s.brain_id AND i.id=s.input_id
 WHERE s.brain_id=b AND(whole_brain OR s.input_id=ANY(mapping_ids) OR s.topic_id=ANY(topic_ids) OR s.entity_id=ANY(entity_ids) OR i.source_version_id=ANY(versions));
 RETURN jsonb_build_object('knowledge_topic_intent_sources',sources,'knowledge_topic_intent_mappings',mappings);
END $$;
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_topic_intent_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; BEGIN
 m:=recollect_pre_topic_intent_closure(b,k,t,c);
 RETURN m||recollect_topic_intent_affected(b,m,c='erase',k='brain');
END $$;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_topic_intent_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_topic_intent_normalize(m)||jsonb_build_object(
 'knowledge_topic_intent_sources',recollect_privacy_ids(m,'knowledge_topic_intent_sources'),
 'knowledge_topic_intent_mappings',recollect_privacy_ids(m,'knowledge_topic_intent_mappings'))
$$;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_topic_intent_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; affected jsonb; source_ids uuid[]; mapping_ids uuid[]; BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 affected:=recollect_topic_intent_affected(r.brain_id,r.manifest,r.cause='erase',r.target->>'kind'='brain');
 source_ids:=recollect_privacy_ids(r.manifest,'knowledge_topic_intent_sources')||recollect_privacy_ids(affected,'knowledge_topic_intent_sources');
 mapping_ids:=recollect_privacy_ids(r.manifest,'knowledge_topic_intent_mappings')||recollect_privacy_ids(affected,'knowledge_topic_intent_mappings');
 DELETE FROM knowledge_topic_intent_sources WHERE brain_id=r.brain_id AND(r.target->>'kind'='brain' OR id=ANY(source_ids));
 DELETE FROM knowledge_topic_intent_mappings WHERE brain_id=r.brain_id AND(r.target->>'kind'='brain' OR id=ANY(mapping_ids));
 PERFORM recollect_pre_topic_intent_apply(rid);
 PERFORM recollect_organization_prune_brain(r.brain_id);
END $$;
REVOKE ALL ON FUNCTION recollect_organization_prune_brain(uuid),recollect_organization_prune(),recollect_organization_source_prune(),recollect_topic_intent_affected(uuid,jsonb,boolean,boolean),recollect_privacy_closure(uuid,text,uuid,text),recollect_privacy_normalize(jsonb),recollect_privacy_apply(uuid) FROM PUBLIC;

-- Human-only upkeep does not need an enabled model policy or provider key.
CREATE OR REPLACE FUNCTION recollect_organization_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH candidates AS(
 SELECT b.id brain_id,p.created_by actor_id,0 priority FROM brains b JOIN knowledge_organization_outbox o ON o.brain_id=b.id
 JOIN model_policy_heads h ON h.brain_id=b.id JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id
 JOIN accounts a ON a.id=p.created_by WHERE NOT b.archived AND a.enabled
 AND(a.auth_kind='local' OR a.membership_until>clock_timestamp()) AND p.policy->>'enabled'='true'
 AND(p.policy->'purposes' ? 'extraction' OR p.policy->'purposes' ? 'embedding')
 AND recollect_effective_role(b.id,p.created_by) IN('writer','admin')
 UNION ALL
 SELECT b.id,b.owner_id,1 FROM brains b JOIN knowledge_organization_outbox o ON o.brain_id=b.id
 JOIN accounts a ON a.id=b.owner_id WHERE NOT b.archived AND a.enabled
 AND(a.auth_kind='local' OR a.membership_until>clock_timestamp())
 AND recollect_effective_role(b.id,b.owner_id) IN('writer','admin')
 AND(EXISTS(SELECT 1 FROM knowledge_topic_intent_sources i WHERE i.brain_id=b.id)
 OR EXISTS(SELECT 1 FROM knowledge_topic_intent_mappings i WHERE i.brain_id=b.id)))
 SELECT DISTINCT ON(brain_id) brain_id,actor_id FROM candidates ORDER BY brain_id,priority,actor_id
$$;
