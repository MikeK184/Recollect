-- No production extractor is selected by this migration. A canonical profile
-- must carry an independent quality receipt before it can authorize visibility.
-- Profiles contain only immutable provider/schema identity and a receipt hash.
CREATE TABLE knowledge_mapping_producer_profiles (
 change_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 provider text NOT NULL CHECK(provider IN ('openai','openrouter')),
 requested_model text NOT NULL CHECK(length(requested_model) BETWEEN 1 AND 256),
 returned_model text NOT NULL CHECK(length(returned_model) BETWEEN 1 AND 256),
 prompt_label text NOT NULL CHECK(length(prompt_label) BETWEEN 1 AND 128),
 schema_label text NOT NULL CHECK(length(schema_label) BETWEEN 1 AND 128),
 adapter_revision text NOT NULL CHECK(length(adapter_revision) BETWEEN 1 AND 128),
 mapping_schema text NOT NULL CHECK(mapping_schema='source-mentions-1'),
 quality_state text NOT NULL DEFAULT 'blocked' CHECK(quality_state IN ('blocked','qualified')),
 quality_receipt_sha256 text CHECK(quality_receipt_sha256 ~ '^[a-f0-9]{64}$'),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(change_id,brain_id),
 CHECK(quality_state<>'qualified' OR quality_receipt_sha256 IS NOT NULL)
);
CREATE TABLE knowledge_mapping_producer_heads (
 brain_id uuid PRIMARY KEY REFERENCES brains(id),
 change_id uuid NOT NULL,
 FOREIGN KEY(change_id,brain_id) REFERENCES knowledge_mapping_producer_profiles(change_id,brain_id)
);
ALTER TABLE knowledge_mapping_inputs ADD CONSTRAINT mapping_producer_profile
 FOREIGN KEY(producer_change_id,brain_id) REFERENCES knowledge_mapping_producer_profiles(change_id,brain_id);
-- Legacy/browser identity stays separate from exact trusted request identity.
DO $$ DECLARE key_name text; BEGIN
 SELECT conname INTO STRICT key_name FROM pg_constraint
 WHERE conrelid='knowledge_mapping_inputs'::regclass AND contype='u'
 AND pg_get_constraintdef(oid)='UNIQUE (brain_id, source_version_id, schema_revision, adapter_revision, context_id)';
 EXECUTE format('ALTER TABLE knowledge_mapping_inputs DROP CONSTRAINT %I',key_name);
END $$;
CREATE UNIQUE INDEX mapping_legacy_identity ON knowledge_mapping_inputs
 (brain_id,source_version_id,schema_revision,adapter_revision,context_id)
 WHERE producer_request_id IS NULL;
CREATE UNIQUE INDEX mapping_production_identity ON knowledge_mapping_inputs
 (brain_id,producer_request_id,source_version_id,schema_revision,adapter_revision,context_id)
 WHERE producer_request_id IS NOT NULL;
CREATE TABLE knowledge_mapping_origins (
 input_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 run_id uuid NOT NULL,
 request_id uuid NOT NULL,
 policy_id uuid NOT NULL,
 producer_change_id uuid NOT NULL,
 -- Exact immutable offered references, never model text or a response cache.
 offered_inputs jsonb NOT NULL CHECK(jsonb_typeof(offered_inputs)='array' AND octet_length(offered_inputs::text)<=32768),
 published_at timestamptz,
 UNIQUE(input_id,brain_id),
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(run_id,brain_id) REFERENCES learning_runs(id,brain_id),
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),
 FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id),
 FOREIGN KEY(producer_change_id,brain_id) REFERENCES knowledge_mapping_producer_profiles(change_id,brain_id)
);
CREATE FUNCTION recollect_mapping_origin_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_TABLE_NAME='knowledge_mapping_producer_profiles' THEN
  RAISE EXCEPTION 'Immutable mapping producer profile required';
 END IF;
 IF (to_jsonb(NEW)-'published_at') IS DISTINCT FROM (to_jsonb(OLD)-'published_at')
  OR (OLD.published_at IS NOT NULL AND NEW.published_at IS DISTINCT FROM OLD.published_at)
  OR NEW.published_at IS NULL THEN
  RAISE EXCEPTION 'Immutable mapping production identity required';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER mapping_profile_immutable BEFORE UPDATE ON knowledge_mapping_producer_profiles
 FOR EACH ROW EXECUTE FUNCTION recollect_mapping_origin_immutable();
CREATE TRIGGER mapping_origin_immutable BEFORE UPDATE ON knowledge_mapping_origins
 FOR EACH ROW EXECUTE FUNCTION recollect_mapping_origin_immutable();
CREATE TABLE knowledge_mapping_coverage (
 run_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 request_id uuid,
 state text NOT NULL CHECK(state IN ('missing','invalid','staged','blocked_profile','published')),
 input_id uuid,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 FOREIGN KEY(run_id,brain_id) REFERENCES learning_runs(id,brain_id),
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id),
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE
);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_mapping_producer_profiles','knowledge_mapping_producer_heads','knowledge_mapping_origins','knowledge_mapping_coverage'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY mapping_origin_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('GRANT SELECT ON %I TO recollect_app',tab);
 END LOOP;
 FOREACH tab IN ARRAY ARRAY['knowledge_mapping_origins','knowledge_mapping_coverage'] LOOP
  EXECUTE format('CREATE POLICY mapping_origin_write ON %I FOR ALL USING(brain_id IN (SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN (SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
  EXECUTE format('GRANT INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
 FOREACH tab IN ARRAY ARRAY['knowledge_mapping_producer_heads','knowledge_mapping_origins'] LOOP
  EXECUTE format('CREATE TRIGGER analytics_insert AFTER INSERT ON %I REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
  EXECUTE format('CREATE TRIGGER analytics_update AFTER UPDATE ON %I REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
  EXECUTE format('CREATE TRIGGER analytics_delete AFTER DELETE ON %I REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (608,'knowledge_mapping_coverage'),(609,'knowledge_mapping_origins'),
 (658,'knowledge_mapping_producer_heads'),(659,'knowledge_mapping_producer_profiles');

-- This is an invoker-RLS predicate. First publication needs live extraction
-- audit detail; subsequent reads/rebuilds use the immutable minimal origin.
-- Ordinary audit expiry is not a semantic erasure, while source ancestry fences
-- and changes to the exact canonical scope always remove visibility.
CREATE FUNCTION recollect_mapping_current_input_ids(b uuid,targets uuid[]) RETURNS TABLE(input_id uuid)
LANGUAGE sql STABLE SET search_path=public,pg_temp SET recollect.bulk_qualification='on' AS $$
 WITH RECURSIVE requested AS MATERIALIZED (
  SELECT i.* FROM knowledge_mapping_inputs i WHERE i.brain_id=b AND i.id=ANY(targets) AND cardinality(targets)<=10000
 ), current_policy AS MATERIALIZED (
  SELECT p.* FROM model_policy_heads ph JOIN model_policies p ON p.brain_id=ph.brain_id AND p.id=ph.policy_id WHERE ph.brain_id=b
 ), current_profile AS MATERIALIZED (
  SELECT f.* FROM knowledge_mapping_producer_heads h JOIN knowledge_mapping_producer_profiles f ON f.brain_id=h.brain_id AND f.change_id=h.change_id WHERE h.brain_id=b
 ), offered AS MATERIALIZED (
  SELECT mi.request_id,jsonb_agg(jsonb_build_object('kind',mi.kind,'id',mi.input_id) ORDER BY mi.kind,mi.input_id) AS refs
  FROM model_request_inputs mi WHERE mi.brain_id=b AND mi.request_id IN(SELECT producer_request_id FROM requested) GROUP BY mi.request_id
 ), lineage(root,id) AS (
  SELECT v.id,v.id FROM requested i JOIN source_versions v ON v.brain_id=b AND v.id=i.source_version_id
  UNION
  SELECT l.root,e.parent_version_id FROM lineage l JOIN source_excerpts e ON e.brain_id=b AND e.version_id=l.id
 ), lineage_status AS MATERIALIZED (
  -- Same complete ancestry predicate as049, shared by this exact cohort.
  -- UNION terminates cycles; absence of an actual root rejects those cycles.
  -- Parent expiry alone does not invalidate an independently retained copy.
  SELECT l.root,bool_or(e.version_id IS NULL) AS has_root,
   bool_or(v.id IS NULL OR v.privacy_state='erased' OR f.source_version_id IS NOT NULL
    OR (e.version_id IS NOT NULL AND p.id IS NULL)) AS blocked
  FROM lineage l LEFT JOIN source_versions v ON v.brain_id=b AND v.id=l.id
  LEFT JOIN source_excerpts e ON e.brain_id=b AND e.version_id=l.id
  LEFT JOIN source_versions p ON p.brain_id=b AND p.id=e.parent_version_id AND p.source_id=e.parent_source_id
  LEFT JOIN model_input_fences f ON f.brain_id=b AND f.source_version_id=l.id
  GROUP BY l.root
 )
 SELECT i.id FROM requested i
 JOIN knowledge_mapping_origins o ON o.input_id=i.id AND o.brain_id=i.brain_id
 JOIN knowledge_mapping_contexts c ON c.id=i.context_id AND c.brain_id=i.brain_id
 JOIN current_profile f ON f.brain_id=i.brain_id AND f.change_id=i.producer_change_id
 JOIN learning_runs r ON r.brain_id=o.brain_id AND r.id=o.run_id
 JOIN model_requests q ON q.brain_id=o.brain_id AND q.id=o.request_id
 JOIN source_versions v ON v.brain_id=i.brain_id AND v.id=i.source_version_id
 JOIN lineage_status ancestry ON ancestry.root=v.id AND ancestry.has_root AND NOT ancestry.blocked
 JOIN sources s ON s.brain_id=v.brain_id AND s.id=v.source_id AND s.current_version=v.id
 JOIN current_policy p ON p.brain_id=i.brain_id
 JOIN offered offered_refs ON offered_refs.request_id=q.id
 WHERE i.brain_id=b AND i.id=ANY(targets) AND cardinality(targets)<=10000
  AND f.quality_state='qualified' AND f.quality_receipt_sha256 IS NOT NULL
  AND f.mapping_schema=i.schema_revision AND f.adapter_revision=i.adapter_revision
  AND i.producer_request_id=o.request_id AND i.producer_policy_id=o.policy_id AND i.producer_change_id=o.producer_change_id
  AND r.state='succeeded' AND r.request_id=q.id AND r.source_version_id=v.id
  AND r.policy_id=o.policy_id AND q.policy_id=o.policy_id AND q.operation_id=r.id
  AND q.actor_id=r.actor_id AND q.actor_id=i.created_by AND q.device_id IS NOT DISTINCT FROM r.device_id
  AND q.purpose='extraction' AND q.state='succeeded' AND NOT q.suppressed
  AND (o.published_at IS NOT NULL OR (
   NOT q.detail_expired
   AND EXISTS(SELECT 1 FROM accounts a WHERE a.id=q.actor_id AND a.enabled)
   AND q.actor_id=recollect_actor() AND q.device_id IS NOT DISTINCT FROM recollect_device()
   AND recollect_role(b) IN ('writer','admin')
   AND (recollect_retention_deadline(q.brain_id,'audit',q.created_at) IS NULL OR recollect_retention_deadline(q.brain_id,'audit',q.created_at)>clock_timestamp())
   AND q.provider=f.provider AND q.model=f.requested_model AND q.returned_model=f.returned_model
   AND q.prompt_label=f.prompt_label AND q.schema_label=f.schema_label
   AND EXISTS(SELECT 1 FROM learning_support_stages ls WHERE ls.brain_id=b AND ls.run_id=r.id
    AND ls.extraction_request_id=q.id AND ls.privacy_state='active')))
  AND v.processing='ready' AND v.artifact_id IS NOT NULL
  AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active'
  AND p.policy->>'enabled'='true' AND p.policy->'purposes' ? 'extraction'
  AND p.policy->>'provider'=f.provider AND p.policy->>'text_model'=f.requested_model
  AND (o.published_at IS NOT NULL OR p.id=o.policy_id)
  AND p.policy->'content_classes' ? v.retention_class
  AND c.source_id=v.source_id AND c.context=jsonb_build_object(
   'selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=b AND version_id=v.id),'{}'::jsonb),
   'areas',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=b AND m.source_id=v.source_id AND g.kind='area' ORDER BY g.id),
   'environments',ARRAY(SELECT g.id FROM evidence_memberships m JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id WHERE m.brain_id=b AND m.source_id=v.source_id AND g.kind='environment' ORDER BY g.id))
  AND offered_refs.refs @> jsonb_build_array(jsonb_build_object('kind','source_version','id',v.id))
  AND o.offered_inputs=offered_refs.refs
$$;
REVOKE ALL ON FUNCTION recollect_mapping_current_input_ids(uuid,uuid[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_current_input_ids(uuid,uuid[]) TO recollect_app;

-- Compatibility for a single publication/override target. Readers should
-- validate their entire bounded input cohort once using the bulk function.
CREATE FUNCTION recollect_mapping_origin_current(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM recollect_mapping_current_input_ids(b,ARRAY[target]))
$$;
REVOKE ALL ON FUNCTION recollect_mapping_origin_current(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_origin_current(uuid,uuid) TO recollect_app;

-- Reconciliation may offer erased claims beside an otherwise independent
-- source. Suppression of the exact producing request physically removes its
-- whole mapping derivative; ordinary detail expiry does not trigger deletion.
CREATE FUNCTION recollect_mapping_producer_suppressed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 DELETE FROM knowledge_mapping_inputs i USING (
  SELECT n.id,n.brain_id FROM mapping_new_requests n JOIN mapping_old_requests o
   ON o.id=n.id AND o.brain_id=n.brain_id WHERE n.suppressed AND NOT o.suppressed
 ) affected WHERE i.brain_id=affected.brain_id AND (
  i.producer_request_id=affected.id OR EXISTS(SELECT 1 FROM knowledge_mapping_origins origin
   WHERE origin.brain_id=i.brain_id AND origin.input_id=i.id AND origin.request_id=affected.id));
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_producer_suppressed() FROM PUBLIC;
CREATE TRIGGER mapping_producer_suppressed AFTER UPDATE ON model_requests
 REFERENCING OLD TABLE AS mapping_old_requests NEW TABLE AS mapping_new_requests
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_mapping_producer_suppressed();
DELETE FROM knowledge_mapping_inputs i WHERE EXISTS(
 SELECT 1 FROM model_requests q WHERE q.brain_id=i.brain_id AND q.id=i.producer_request_id AND q.suppressed);
