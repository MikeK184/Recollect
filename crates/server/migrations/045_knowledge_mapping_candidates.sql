-- Canonical, source-bound candidate staging. No extractor is selected and no
-- unchecked candidate becomes a claim, alias equivalence or visible topic.
CREATE TABLE knowledge_mapping_contexts (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 source_id uuid NOT NULL,
 context jsonb NOT NULL,
 UNIQUE(id,brain_id),
 FOREIGN KEY(source_id,brain_id) REFERENCES sources(id,brain_id)
);
CREATE INDEX knowledge_mapping_context_source ON knowledge_mapping_contexts(brain_id,source_id);
CREATE TABLE knowledge_mapping_inputs (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 source_version_id uuid NOT NULL,
 schema_revision text NOT NULL,
 adapter_revision text NOT NULL,
 preparation_epoch bigint NOT NULL,
 context_id uuid NOT NULL,
 payload jsonb NOT NULL,
 state text NOT NULL DEFAULT 'staged' CHECK(state IN ('staged','published')),
 created_by uuid NOT NULL REFERENCES accounts(id),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id),
 UNIQUE(brain_id,source_version_id,schema_revision,adapter_revision,context_id),
 FOREIGN KEY(context_id,brain_id) REFERENCES knowledge_mapping_contexts(id,brain_id),
 FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id),
 CHECK(octet_length(payload::text)<=262144)
);
CREATE TABLE knowledge_entities (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 kind text NOT NULL CHECK(kind IN ('person','technology','service','environment','repository','concept','setting')),
 realm jsonb NOT NULL,
 normalized_label text NOT NULL CHECK(octet_length(normalized_label) BETWEEN 1 AND 1024),
 UNIQUE(id,brain_id),
 UNIQUE(brain_id,kind,realm,normalized_label)
);
CREATE TABLE knowledge_mentions (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 input_id uuid NOT NULL,
 entity_id uuid NOT NULL,
 local_key text NOT NULL,
 byte_start integer NOT NULL CHECK(byte_start>=0),
 byte_end integer NOT NULL CHECK(byte_end>byte_start),
 quote text NOT NULL CHECK(octet_length(quote) BETWEEN 1 AND 256),
 confidence double precision NOT NULL CHECK(confidence>=0 AND confidence<=1),
 UNIQUE(id,input_id,brain_id),
 UNIQUE(input_id,local_key),
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(entity_id,brain_id) REFERENCES knowledge_entities(id,brain_id)
);
CREATE INDEX knowledge_mentions_entity ON knowledge_mentions(brain_id,entity_id);
CREATE TABLE knowledge_relation_candidates (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 input_id uuid NOT NULL,
 from_mention uuid NOT NULL,
 to_mention uuid NOT NULL,
 relation text NOT NULL CHECK(relation IN ('uses','configured_by','depends_on','located_in','implements','related_to','alias')),
 byte_start integer NOT NULL CHECK(byte_start>=0),
 byte_end integer NOT NULL CHECK(byte_end>byte_start),
 quote text NOT NULL,
 disposition text NOT NULL DEFAULT 'pending' CHECK(disposition IN ('pending','rejected','accepted')),
 CHECK(from_mention<>to_mention),
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(from_mention,input_id,brain_id) REFERENCES knowledge_mentions(id,input_id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(to_mention,input_id,brain_id) REFERENCES knowledge_mentions(id,input_id,brain_id) ON DELETE CASCADE
);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_mapping_contexts','knowledge_mapping_inputs','knowledge_entities','knowledge_mentions','knowledge_relation_candidates'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY mapping_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
  EXECUTE format('CREATE POLICY mapping_write ON %I FOR ALL USING(recollect_role(brain_id) IN (''writer'',''admin'')) WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
-- Deepest first; before sources, accounts, or the Brain are removed.
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (610,'knowledge_relation_candidates'),(620,'knowledge_mentions'),
 (630,'knowledge_mapping_inputs'),(640,'knowledge_entities'),(650,'knowledge_mapping_contexts');
-- Migration 035's cascading Brain children also belong in the explicit,
-- catalog-verified deletion inventory. Delete before their operation parent.
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (660,'agent_brain_usage'),(670,'source_import_scopes');

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_mapping_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; inputs uuid[];
BEGIN
 m:=recollect_pre_mapping_closure(b,k,t,c);
 SELECT coalesce(array_agg(id ORDER BY id),'{}') INTO inputs FROM knowledge_mapping_inputs i
 WHERE i.brain_id=b AND (k='brain' OR i.source_version_id=ANY(
  recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')));
 RETURN m||jsonb_build_object('knowledge_mapping_inputs',inputs);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_mapping_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 -- Include exact dependencies when replaying an older journal without the new
 -- derivative field. Cascade removes raw payloads, quotations and endpoints.
 DELETE FROM knowledge_mapping_inputs i WHERE i.brain_id=r.brain_id AND (
  r.target->>'kind'='brain' OR i.id=ANY(recollect_privacy_ids(r.manifest,'knowledge_mapping_inputs'))
  OR i.source_version_id=ANY(recollect_privacy_ids(r.manifest,'source_versions')||recollect_privacy_ids(r.manifest,'model_input_sources')));
 DELETE FROM knowledge_entities e WHERE e.brain_id=r.brain_id
  AND NOT EXISTS(SELECT 1 FROM knowledge_mentions m WHERE m.brain_id=e.brain_id AND m.entity_id=e.id);
 DELETE FROM knowledge_mapping_contexts c WHERE c.brain_id=r.brain_id
  AND NOT EXISTS(SELECT 1 FROM knowledge_mapping_inputs i WHERE i.brain_id=c.brain_id AND i.context_id=c.id);
 PERFORM recollect_pre_mapping_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_mapping_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_mapping_normalize(m)||jsonb_build_object('knowledge_mapping_inputs',to_jsonb(recollect_privacy_ids(m,'knowledge_mapping_inputs')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
