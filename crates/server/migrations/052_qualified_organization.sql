-- Semantic navigation is canonical PostgreSQL data with exact contributors.
-- Browser/test staging is deliberately not production publication authority.
CREATE INDEX knowledge_mentions_representative ON knowledge_mentions(brain_id,input_id,entity_id,id);
ALTER TABLE knowledge_mapping_inputs ADD COLUMN producer_request_id uuid;
ALTER TABLE knowledge_mapping_inputs ADD COLUMN producer_policy_id uuid;
ALTER TABLE knowledge_mapping_inputs ADD COLUMN producer_change_id uuid;
ALTER TABLE knowledge_mapping_inputs ADD CONSTRAINT mapping_producer_request
 FOREIGN KEY(producer_request_id,brain_id) REFERENCES model_requests(id,brain_id);
ALTER TABLE knowledge_mapping_inputs ADD CONSTRAINT mapping_producer_policy
 FOREIGN KEY(producer_policy_id,brain_id) REFERENCES model_policies(id,brain_id);
ALTER TABLE knowledge_mapping_inputs ADD CONSTRAINT mapping_published_provenance
 CHECK(state<>'published' OR num_nonnulls(producer_request_id,producer_policy_id,producer_change_id)=3);

CREATE TABLE knowledge_topics (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 kind text NOT NULL CHECK(kind IN ('facet','community','split')),
 anchor_entity_id uuid,
 change_id uuid NOT NULL,
 pinned boolean NOT NULL DEFAULT false,
 recipe text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id),
 FOREIGN KEY(anchor_entity_id,brain_id) REFERENCES knowledge_entities(id,brain_id) ON DELETE SET NULL(anchor_entity_id)
);
CREATE UNIQUE INDEX knowledge_facet_identity ON knowledge_topics(brain_id,anchor_entity_id) WHERE kind='facet';
CREATE TABLE knowledge_topic_memberships (
 brain_id uuid NOT NULL,
 topic_id uuid NOT NULL,
 entity_id uuid NOT NULL,
 input_id uuid NOT NULL,
 reason text NOT NULL CHECK(reason IN ('entity_facet','weighted_leiden','explicit_split')),
 weight double precision NOT NULL CHECK(weight>0 AND weight<='Infinity'::float8 AND weight<>'Infinity'::float8),
 PRIMARY KEY(topic_id,entity_id,input_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(entity_id,brain_id) REFERENCES knowledge_entities(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX knowledge_topic_input ON knowledge_topic_memberships(brain_id,input_id,topic_id);
CREATE TABLE knowledge_topic_labels (
 brain_id uuid NOT NULL,
 topic_id uuid PRIMARY KEY,
 basis_input uuid NOT NULL,
 label text NOT NULL CHECK(octet_length(label) BETWEEN 1 AND 256),
 actor_id uuid NOT NULL REFERENCES accounts(id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(basis_input,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_topic_exclusions (
 brain_id uuid NOT NULL,
 topic_id uuid NOT NULL,
 input_id uuid NOT NULL,
 actor_id uuid NOT NULL REFERENCES accounts(id),
 PRIMARY KEY(topic_id,input_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE knowledge_topic_splits (
 brain_id uuid NOT NULL,
 original_topic_id uuid NOT NULL,
 split_topic_id uuid NOT NULL,
 input_id uuid NOT NULL,
 actor_id uuid NOT NULL REFERENCES accounts(id),
 CHECK(original_topic_id<>split_topic_id),
 PRIMARY KEY(original_topic_id,input_id),
 FOREIGN KEY(original_topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(split_topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE
);
-- Changes contain IDs/actions, never an independently retained copy of names.
CREATE TABLE knowledge_topic_changes (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 topic_id uuid NOT NULL,
 actor_id uuid NOT NULL REFERENCES accounts(id),
 action text NOT NULL CHECK(action IN ('create','rebuild','rename','pin','exclude','split')),
 previous_change_id uuid,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id,topic_id),
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(previous_change_id,brain_id,topic_id) REFERENCES knowledge_topic_changes(id,brain_id,topic_id)
);
CREATE FUNCTION recollect_organization_entity_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 DELETE FROM knowledge_topics WHERE brain_id=OLD.brain_id AND kind='facet' AND anchor_entity_id=OLD.id;
 RETURN OLD;
END $$;
REVOKE ALL ON FUNCTION recollect_organization_entity_removed() FROM PUBLIC;
CREATE TRIGGER organization_entity_removed BEFORE DELETE ON knowledge_entities
 FOR EACH ROW EXECUTE FUNCTION recollect_organization_entity_removed();
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_topics','knowledge_topic_memberships','knowledge_topic_labels','knowledge_topic_exclusions','knowledge_topic_splits','knowledge_topic_changes'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY organization_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY organization_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (580,'knowledge_topic_changes'),(581,'knowledge_topic_splits'),(582,'knowledge_topic_exclusions'),
 (583,'knowledge_topic_labels'),(584,'knowledge_topic_memberships'),(585,'knowledge_topics');

CREATE FUNCTION recollect_organization_prune() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM organization_removed_inputs LOOP
  DELETE FROM knowledge_topics t WHERE brain_id=b AND NOT EXISTS(
   SELECT 1 FROM knowledge_topic_memberships m WHERE m.brain_id=b AND m.topic_id=t.id);
  DELETE FROM knowledge_entities e WHERE e.brain_id=b AND NOT EXISTS(
   SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=b AND m.entity_id=e.id);
  DELETE FROM knowledge_mapping_contexts c WHERE c.brain_id=b AND NOT EXISTS(
   SELECT 1 FROM knowledge_mapping_inputs i WHERE i.brain_id=b AND i.context_id=c.id);
  -- Receipt payloads can contain a human rename. Invalidate the whole command,
  -- so an older backup or idempotency retry cannot return that removed name.
  UPDATE command_receipts SET invalidated=true,input='{}',response=NULL
   WHERE brain_id=b AND operation IN ('organization.topic','organization.alias');
 END LOOP;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_organization_prune() FROM PUBLIC;
CREATE TRIGGER organization_input_removed AFTER DELETE ON knowledge_mapping_inputs
 REFERENCING OLD TABLE AS organization_removed_inputs
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_prune();

-- Evaluate the same current accessible/writable Brain sets once per statement.
-- The old correlated role check was repeated for every mention and nested
-- producer-origin join. Both SELECT and ALL policies need this shape: ALL also
-- participates in SELECT policy composition. No authorization is cached beyond
-- this statement or widened to another Brain/actor/device.
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_mapping_contexts','knowledge_mapping_inputs','knowledge_entities','knowledge_mentions','knowledge_relation_candidates'] LOOP
  EXECUTE format('DROP POLICY mapping_read ON %I',tab);
  EXECUTE format('DROP POLICY mapping_write ON %I',tab);
  EXECUTE format('CREATE POLICY mapping_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY mapping_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
 END LOOP;
END $$;
