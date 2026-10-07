-- Exact retained copies are storage aliases, never a second source of truth.
CREATE TABLE automatic_support_excerpts (
 version_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 parent_version_id uuid NOT NULL,
 first_line integer NOT NULL CHECK(first_line>0),
 last_line integer NOT NULL CHECK(last_line>=first_line),
 selection jsonb NOT NULL,
 manifest_revision_id uuid,
 identity_key text NOT NULL,
 original_class text NOT NULL CHECK(original_class IN ('raw_session','tool_output','document')),
 provenance jsonb NOT NULL CHECK(octet_length(provenance::text)<=32768),
 privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased')),
 UNIQUE(brain_id,identity_key),
 FOREIGN KEY(version_id,brain_id) REFERENCES source_versions(id,brain_id),
 FOREIGN KEY(parent_version_id,brain_id) REFERENCES source_versions(id,brain_id),
 FOREIGN KEY(manifest_revision_id,brain_id) REFERENCES manifest_revisions(id,brain_id)
);
ALTER TABLE automatic_support_excerpts ENABLE ROW LEVEL SECURITY;
ALTER TABLE automatic_support_excerpts FORCE ROW LEVEL SECURITY;
CREATE POLICY automatic_excerpt_read ON automatic_support_excerpts FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY automatic_excerpt_write ON automatic_support_excerpts FOR INSERT WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT ON automatic_support_excerpts TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(999,'automatic_support_excerpts');
CREATE FUNCTION recollect_automatic_excerpt_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.privacy_state<>'active' AND NEW.privacy_state IS DISTINCT FROM OLD.privacy_state THEN
   UPDATE automatic_support_excerpts SET provenance='{}',selection='{}',privacy_state=CASE WHEN privacy_state='erased' THEN 'erased' ELSE NEW.privacy_state END
    WHERE version_id=NEW.id AND brain_id=NEW.brain_id;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_automatic_excerpt_removed() FROM PUBLIC;
CREATE TRIGGER automatic_excerpt_removed AFTER UPDATE OF privacy_state ON source_versions FOR EACH ROW EXECUTE FUNCTION recollect_automatic_excerpt_removed();

CREATE FUNCTION recollect_automatic_excerpt_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP='INSERT' THEN
   IF NEW.identity_key<>md5(jsonb_build_array(NEW.brain_id,NEW.parent_version_id,NEW.first_line,NEW.last_line,NEW.selection,NEW.manifest_revision_id)::text) THEN RAISE EXCEPTION 'invalid excerpt identity'; END IF;
   IF NEW.privacy_state<>'active' OR NOT EXISTS(
     SELECT 1 FROM source_excerpts e
     JOIN source_versions child ON child.id=e.version_id AND child.brain_id=e.brain_id
     JOIN source_versions parent ON parent.id=e.parent_version_id AND parent.brain_id=e.brain_id
     WHERE e.brain_id=NEW.brain_id AND e.version_id=NEW.version_id
       AND e.parent_version_id=NEW.parent_version_id AND e.parent_source_id=parent.source_id
       AND e.first_line=NEW.first_line AND e.last_line=NEW.last_line
       AND child.retention_class='support_excerpt' AND parent.retention_class=NEW.original_class
       AND child.privacy_state='active' AND parent.privacy_state='active'
   ) THEN RAISE EXCEPTION 'invalid excerpt provenance'; END IF;
 ELSE
   IF (to_jsonb(NEW)-ARRAY['provenance','selection','privacy_state']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['provenance','selection','privacy_state'])
    OR NEW.privacy_state='active' OR (OLD.privacy_state='erased' AND NEW.privacy_state<>'erased')
    OR NEW.provenance<>'{}'::jsonb OR NEW.selection<>'{}'::jsonb
    OR NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.id=NEW.version_id AND v.brain_id=NEW.brain_id AND v.privacy_state=NEW.privacy_state)
   THEN RAISE EXCEPTION 'immutable excerpt identity'; END IF;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_automatic_excerpt_immutable() FROM PUBLIC;
CREATE TRIGGER automatic_excerpt_immutable BEFORE INSERT OR UPDATE ON automatic_support_excerpts FOR EACH ROW EXECUTE FUNCTION recollect_automatic_excerpt_immutable();

-- This ledger deliberately has no parent FK: it commits independently while
-- canonical publication holds its Brain/input locks. RLS validates the exact
-- native run/stage identity; it stores only opaque IDs, never retained content.
CREATE TABLE excerpt_artifact_intents (
 artifact_id uuid PRIMARY KEY,brain_id uuid NOT NULL,run_id uuid NOT NULL,
 parent_version_id uuid NOT NULL,created_at timestamptz NOT NULL DEFAULT clock_timestamp(),removed_at timestamptz
);
ALTER TABLE excerpt_artifact_intents ENABLE ROW LEVEL SECURITY;
ALTER TABLE excerpt_artifact_intents FORCE ROW LEVEL SECURITY;
CREATE POLICY excerpt_intent_read ON excerpt_artifact_intents FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY excerpt_intent_write ON excerpt_artifact_intents FOR INSERT WITH CHECK(
 recollect_role(brain_id) IN ('writer','admin') AND excerpt_artifact_intents.removed_at IS NULL AND EXISTS(
  SELECT 1 FROM learning_runs r JOIN learning_support_stages stage ON stage.run_id=r.id AND stage.brain_id=r.brain_id
  JOIN jobs j ON j.id=r.job_id AND j.brain_id=r.brain_id AND j.target_id=r.id AND j.kind='source.learn'
  WHERE r.id=excerpt_artifact_intents.run_id AND r.brain_id=excerpt_artifact_intents.brain_id AND r.source_version_id=parent_version_id
    AND r.actor_id=recollect_actor() AND r.device_id IS NOT DISTINCT FROM recollect_device()
    AND r.automatic AND r.state IN ('queued','running') AND stage.privacy_state='active' AND stage.verdicts IS NOT NULL
    AND j.state='running' AND j.lease_until>clock_timestamp() AND j.actor_id=r.actor_id AND j.device_id IS NOT DISTINCT FROM r.device_id
 ) AND NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=excerpt_artifact_intents.brain_id AND v.artifact_id=excerpt_artifact_intents.artifact_id)
);
CREATE POLICY excerpt_intent_delete ON excerpt_artifact_intents FOR DELETE USING(
 recollect_role(brain_id) IN ('writer','admin') AND EXISTS(
 SELECT 1 FROM source_versions v WHERE v.brain_id=excerpt_artifact_intents.brain_id AND v.artifact_id=excerpt_artifact_intents.artifact_id));
GRANT SELECT,INSERT,DELETE ON excerpt_artifact_intents TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(993,'excerpt_artifact_intents');
CREATE FUNCTION recollect_pending_excerpt_artifacts(selected_run uuid DEFAULT NULL) RETURNS TABLE(brain_id uuid,artifact_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT i.brain_id,i.artifact_id FROM excerpt_artifact_intents i
 LEFT JOIN learning_runs r ON r.id=i.run_id AND r.brain_id=i.brain_id
 LEFT JOIN jobs j ON j.id=r.job_id AND j.brain_id=i.brain_id
 WHERE recollect_actor() IS NULL AND recollect_device() IS NULL AND i.removed_at IS NULL
  AND (selected_run IS NULL OR i.run_id=selected_run)
  AND NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=i.brain_id AND v.artifact_id=i.artifact_id)
  AND (j.id IS NULL OR j.state<>'running' OR j.lease_until<=clock_timestamp())
 ORDER BY i.created_at,i.artifact_id LIMIT 100
$$;
CREATE FUNCTION recollect_forget_excerpt_artifact(b uuid,a uuid) RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 UPDATE excerpt_artifact_intents i SET removed_at=clock_timestamp() WHERE i.brain_id=b AND i.artifact_id=a
  AND recollect_actor() IS NULL AND recollect_device() IS NULL AND i.removed_at IS NULL
  AND NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=b AND v.artifact_id=a)
  AND NOT EXISTS(SELECT 1 FROM learning_runs r JOIN jobs j ON j.id=r.job_id AND j.brain_id=r.brain_id
   WHERE r.id=i.run_id AND r.brain_id=i.brain_id AND j.state='running' AND j.lease_until>clock_timestamp())
$$;
REVOKE ALL ON FUNCTION recollect_pending_excerpt_artifacts(uuid),recollect_forget_excerpt_artifact(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_pending_excerpt_artifacts(uuid),recollect_forget_excerpt_artifact(uuid,uuid) TO recollect_app;

-- Arrival order is server-owned. Legacy rows get a stable ordinal, not a
-- reconstructed assertion about host chronology.
CREATE SEQUENCE capture_receipt_sequence;
ALTER TABLE capture_events ADD COLUMN receipt_sequence bigint;
WITH numbered AS (SELECT id,row_number() OVER(ORDER BY received_at,id) n FROM capture_events)
 UPDATE capture_events e SET receipt_sequence=n.n FROM numbered n WHERE n.id=e.id;
SELECT setval('capture_receipt_sequence',greatest(coalesce((SELECT max(receipt_sequence) FROM capture_events),0)+1,1),false);
ALTER TABLE capture_events ALTER COLUMN receipt_sequence SET DEFAULT nextval('capture_receipt_sequence');
ALTER TABLE capture_events ALTER COLUMN receipt_sequence SET NOT NULL;
CREATE UNIQUE INDEX capture_receipt_order ON capture_events(receipt_sequence);
GRANT USAGE ON SEQUENCE capture_receipt_sequence TO recollect_app;
CREATE TABLE session_digest_partitions (
 id uuid PRIMARY KEY,brain_id uuid NOT NULL REFERENCES brains(id),binding_id uuid NOT NULL,
 task_id uuid NOT NULL,actor_id uuid NOT NULL REFERENCES accounts(id),device_id uuid NOT NULL REFERENCES devices(id),
 identity_key text NOT NULL,host_session_id text NOT NULL,agent_id text,selection jsonb NOT NULL,manifest_revision_id uuid,
 observed_receipt bigint NOT NULL DEFAULT 0,covered_receipt bigint NOT NULL DEFAULT 0,
 coverage jsonb NOT NULL DEFAULT '{}',last_checked_at timestamptz NOT NULL DEFAULT '-infinity',
 privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased')),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id),
 UNIQUE(brain_id,identity_key),
 FOREIGN KEY(binding_id,brain_id) REFERENCES capture_bindings(id,brain_id),
 FOREIGN KEY(task_id,brain_id,actor_id) REFERENCES workspace_tasks(id,brain_id,account_id),
 FOREIGN KEY(manifest_revision_id,brain_id) REFERENCES manifest_revisions(id,brain_id)
);
CREATE TABLE session_digest_events (
 event_id uuid PRIMARY KEY REFERENCES capture_events(id),partition_id uuid NOT NULL,brain_id uuid NOT NULL,
 FOREIGN KEY(partition_id,brain_id) REFERENCES session_digest_partitions(id,brain_id)
);
CREATE TABLE session_digest_generations (
 id uuid PRIMARY KEY,brain_id uuid NOT NULL,partition_id uuid NOT NULL,page integer NOT NULL CHECK(page>=0),
 contributors uuid[] NOT NULL CHECK(cardinality(contributors) BETWEEN 1 AND 12),policy_id uuid NOT NULL,
 version text NOT NULL DEFAULT 'session-digest-1',receipt bigint NOT NULL,run_id uuid NOT NULL,candidate_bytes integer NOT NULL CHECK(candidate_bytes>=256),candidate_overhead integer NOT NULL CHECK(candidate_overhead>=0 AND candidate_overhead<candidate_bytes),base_revision_id uuid,
 privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased')),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE NULLS NOT DISTINCT(partition_id,page,contributors,policy_id,version,base_revision_id),
 UNIQUE(run_id),UNIQUE(id,brain_id),
 FOREIGN KEY(partition_id,brain_id) REFERENCES session_digest_partitions(id,brain_id),
 FOREIGN KEY(base_revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
 FOREIGN KEY(run_id,brain_id) REFERENCES handover_runs(id,brain_id)
);
CREATE TABLE session_digest_claims (
 brain_id uuid NOT NULL,partition_id uuid NOT NULL,page integer NOT NULL,claim_id uuid NOT NULL,
 generation_id uuid NOT NULL,revision_id uuid NOT NULL,covered_receipt bigint NOT NULL,
 PRIMARY KEY(partition_id,page),UNIQUE(claim_id),
 FOREIGN KEY(partition_id,brain_id) REFERENCES session_digest_partitions(id,brain_id),
 FOREIGN KEY(generation_id,brain_id) REFERENCES session_digest_generations(id,brain_id),
 FOREIGN KEY(claim_id,brain_id) REFERENCES claims(id,brain_id),
 FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id)
);
CREATE TABLE session_digest_pages (
 brain_id uuid NOT NULL,partition_id uuid NOT NULL,page integer NOT NULL,generation_id uuid NOT NULL,active boolean NOT NULL DEFAULT true,
 PRIMARY KEY(partition_id,page),
 FOREIGN KEY(partition_id,brain_id) REFERENCES session_digest_partitions(id,brain_id),
 FOREIGN KEY(generation_id,brain_id) REFERENCES session_digest_generations(id,brain_id)
);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['session_digest_partitions','session_digest_events','session_digest_generations','session_digest_claims','session_digest_pages'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY digest_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
  EXECUTE format('CREATE POLICY digest_write ON %I FOR INSERT WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
  EXECUTE format('CREATE POLICY digest_update ON %I FOR UPDATE USING(recollect_role(brain_id) IN (''writer'',''admin'')) WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(994,'session_digest_pages'),(995,'session_digest_claims'),(996,'session_digest_generations'),(997,'session_digest_events'),(998,'session_digest_partitions');

CREATE FUNCTION recollect_digest_admission() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_TABLE_NAME='session_digest_partitions' THEN
   IF NEW.privacy_state<>'active' OR NOT EXISTS(SELECT 1 FROM capture_bindings b JOIN operation_bindings o ON o.id=b.operation_id AND o.brain_id=b.brain_id
     WHERE b.id=NEW.binding_id AND b.brain_id=NEW.brain_id AND b.actor_id=NEW.actor_id AND b.device_id=NEW.device_id
      AND b.selection=NEW.selection AND o.task_id=NEW.task_id AND o.account_id=NEW.actor_id AND o.device_id=NEW.device_id)
     OR NOT EXISTS(SELECT 1 FROM capture_events e WHERE e.brain_id=NEW.brain_id AND e.binding_id=NEW.binding_id AND e.state='accepted' AND e.metadata->>'host_session_id'=NEW.host_session_id AND e.metadata->>'agent_id' IS NOT DISTINCT FROM NEW.agent_id)
     OR NEW.manifest_revision_id IS NOT NULL
     OR NEW.identity_key<>md5(jsonb_build_array(NEW.binding_id,NEW.host_session_id,NEW.agent_id,NEW.selection,NULL)::text)
   THEN RAISE EXCEPTION 'invalid digest partition authority'; END IF;
 ELSIF TG_TABLE_NAME='session_digest_events' THEN
   IF NOT EXISTS(SELECT 1 FROM capture_events e JOIN session_digest_partitions p ON p.id=NEW.partition_id AND p.brain_id=NEW.brain_id
     WHERE e.id=NEW.event_id AND e.brain_id=p.brain_id AND e.binding_id=p.binding_id AND e.metadata->>'host_session_id'=p.host_session_id
      AND e.metadata->>'agent_id' IS NOT DISTINCT FROM p.agent_id AND p.privacy_state='active')
   THEN RAISE EXCEPTION 'invalid digest event authority'; END IF;
 ELSIF TG_TABLE_NAME='session_digest_generations' THEN
   IF NEW.privacy_state<>'active' OR NEW.version<>'session-digest-1' OR NOT EXISTS(SELECT 1 FROM handover_runs r JOIN session_digest_partitions p ON p.id=NEW.partition_id AND p.brain_id=NEW.brain_id
     WHERE r.id=NEW.run_id AND r.brain_id=NEW.brain_id AND r.automatic AND r.contributions=NEW.contributors
      AND r.selection=p.selection AND r.policy_id=NEW.policy_id AND r.base_revision_id IS NOT DISTINCT FROM NEW.base_revision_id AND NEW.receipt=p.observed_receipt AND p.privacy_state='active')
     OR EXISTS(SELECT 1 FROM unnest(NEW.contributors) i LEFT JOIN claim_revisions r ON r.id=i AND r.brain_id=NEW.brain_id
       JOIN session_digest_partitions p ON p.id=NEW.partition_id WHERE r.id IS NULL OR r.revision#>'{content,selection}'<>p.selection
        OR r.revision#>>'{content,manifest_revision_id}' IS DISTINCT FROM p.manifest_revision_id::text
        OR NOT EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
          JOIN learning_runs lr ON lr.brain_id=e.brain_id AND lr.source_version_id=e.source_version_id
          WHERE d.partition_id=p.id AND r.claim_id=ANY(lr.claim_ids)))
   THEN RAISE EXCEPTION 'invalid digest generation authority'; END IF;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_digest_admission() FROM PUBLIC;
CREATE TRIGGER digest_partition_admission BEFORE INSERT ON session_digest_partitions FOR EACH ROW EXECUTE FUNCTION recollect_digest_admission();
CREATE TRIGGER digest_event_admission BEFORE INSERT ON session_digest_events FOR EACH ROW EXECUTE FUNCTION recollect_digest_admission();
CREATE TRIGGER digest_generation_admission BEFORE INSERT ON session_digest_generations FOR EACH ROW EXECUTE FUNCTION recollect_digest_admission();

-- This bounded enumerator reads only authenticated capture identity. Private
-- operation/task RLS must not exclude another contributor under Brain policy.
CREATE FUNCTION recollect_digest_discover(b uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF recollect_role(b) NOT IN ('writer','admin') OR recollect_role(b) IS NULL THEN RAISE EXCEPTION 'digest writer required'; END IF;
 WITH candidates AS (SELECT DISTINCT cb.brain_id,cb.id binding_id,o.task_id,cb.actor_id,cb.device_id,cb.selection,e.metadata->>'host_session_id' session,e.metadata->>'agent_id' agent,
  md5(jsonb_build_array(cb.id,e.metadata->>'host_session_id',e.metadata->>'agent_id',cb.selection,NULL)::text) identity
  FROM capture_events e JOIN capture_bindings cb ON cb.id=e.binding_id AND cb.brain_id=e.brain_id JOIN operation_bindings o ON o.id=cb.operation_id AND o.brain_id=cb.brain_id
  WHERE e.brain_id=b AND e.state='accepted' AND length(e.metadata->>'host_session_id')>0
   AND NOT EXISTS(SELECT 1 FROM session_digest_partitions p WHERE p.brain_id=b AND p.identity_key=md5(jsonb_build_array(cb.id,e.metadata->>'host_session_id',e.metadata->>'agent_id',cb.selection,NULL)::text)) LIMIT 100)
 INSERT INTO session_digest_partitions(id,brain_id,binding_id,task_id,actor_id,device_id,selection,host_session_id,agent_id,identity_key)
 SELECT gen_random_uuid(),brain_id,binding_id,task_id,actor_id,device_id,selection,session,agent,identity FROM candidates ON CONFLICT DO NOTHING;
 INSERT INTO session_digest_events(event_id,partition_id,brain_id) SELECT e.id,p.id,p.brain_id FROM session_digest_partitions p JOIN capture_events e ON e.brain_id=p.brain_id AND e.binding_id=p.binding_id
  WHERE p.brain_id=b AND p.privacy_state='active' AND e.metadata->>'host_session_id'=p.host_session_id AND e.metadata->>'agent_id' IS NOT DISTINCT FROM p.agent_id ON CONFLICT DO NOTHING;
 UPDATE session_digest_partitions p SET observed_receipt=coalesce((SELECT max(e.receipt_sequence) FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id WHERE d.partition_id=p.id),0) WHERE p.brain_id=b AND p.privacy_state='active';
END $$;
CREATE FUNCTION recollect_digest_task_closed(b uuid,partition uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT recollect_role(b) IS NOT NULL AND EXISTS(SELECT 1 FROM session_digest_partitions p JOIN workspace_tasks t ON t.id=p.task_id AND t.brain_id=p.brain_id AND t.account_id=p.actor_id AND t.device_id=p.device_id WHERE p.brain_id=b AND p.id=partition AND t.closed)
$$;
REVOKE ALL ON FUNCTION recollect_digest_discover(uuid),recollect_digest_task_closed(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_digest_discover(uuid),recollect_digest_task_closed(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_digest_arrival() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE p session_digest_partitions;
BEGIN
 FOR p IN SELECT d.* FROM session_digest_partitions d JOIN capture_bindings b ON b.id=d.binding_id AND b.brain_id=d.brain_id
  WHERE d.brain_id=NEW.brain_id AND d.binding_id=NEW.binding_id AND d.privacy_state='active'
   AND d.host_session_id=NEW.metadata->>'host_session_id' AND d.agent_id IS NOT DISTINCT FROM NEW.metadata->>'agent_id'
 LOOP
  INSERT INTO session_digest_events(event_id,partition_id,brain_id) VALUES(NEW.id,p.id,p.brain_id) ON CONFLICT DO NOTHING;
  UPDATE session_digest_partitions SET observed_receipt=greatest(observed_receipt,NEW.receipt_sequence) WHERE id=p.id;
  INSERT INTO memory_epochs(brain_id,epoch) VALUES(p.brain_id,1) ON CONFLICT(brain_id) DO UPDATE SET epoch=memory_epochs.epoch+1;
 END LOOP;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_digest_arrival() FROM PUBLIC;
CREATE TRIGGER digest_arrival AFTER INSERT ON capture_events FOR EACH ROW EXECUTE FUNCTION recollect_digest_arrival();

CREATE FUNCTION recollect_digest_current(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT NOT EXISTS(SELECT 1 FROM session_digest_claims d JOIN claim_revisions r ON r.claim_id=d.claim_id AND r.brain_id=d.brain_id
  JOIN session_digest_partitions p ON p.id=d.partition_id JOIN session_digest_generations g ON g.id=d.generation_id
  WHERE d.brain_id=b AND r.id=target AND r.revision->>'origin'='model_synthesized' AND
   (r.id<>d.revision_id OR p.privacy_state<>'active' OR g.privacy_state<>'active'
   OR r.revision#>>'{content,kind}'<>'handover' OR r.revision#>'{content,selection}'<>p.selection
   OR r.revision#>>'{content,manifest_revision_id}' IS DISTINCT FROM p.manifest_revision_id::text
   OR d.covered_receipt<p.observed_receipt OR NOT EXISTS(SELECT 1 FROM session_digest_pages desired WHERE desired.partition_id=d.partition_id AND desired.page=d.page AND desired.generation_id=g.id AND desired.active) OR EXISTS(SELECT 1 FROM unnest(g.contributors) input
    LEFT JOIN claim_revisions i ON i.id=input AND i.brain_id=b LEFT JOIN claims c ON c.id=i.claim_id AND c.brain_id=b
    WHERE c.current_revision IS DISTINCT FROM input OR NOT recollect_revision_supported(b,input))))
$$;
ALTER FUNCTION recollect_memory_supported(uuid,uuid) RENAME TO recollect_pre_digest_supported;
CREATE FUNCTION recollect_memory_supported(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_digest_supported(b,target) AND NOT EXISTS(
  SELECT 1 FROM recollect_memory_dependencies(b,target) d WHERE NOT recollect_digest_current(b,d.revision_id))
$$;
REVOKE ALL ON FUNCTION recollect_digest_current(uuid,uuid),recollect_memory_supported(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_digest_current(uuid,uuid),recollect_memory_supported(uuid,uuid),recollect_pre_digest_supported(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_digest_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
DECLARE oldjson jsonb; newjson jsonb;
BEGIN
 oldjson:=to_jsonb(OLD);newjson:=to_jsonb(NEW);
 IF TG_TABLE_NAME='session_digest_partitions' THEN
   IF NEW.observed_receipt<OLD.observed_receipt OR NEW.covered_receipt<OLD.covered_receipt THEN RAISE EXCEPTION 'monotonic digest coverage'; END IF;
   oldjson:=oldjson-ARRAY['observed_receipt','covered_receipt','coverage','last_checked_at','privacy_state','host_session_id','agent_id','selection'];
   newjson:=newjson-ARRAY['observed_receipt','covered_receipt','coverage','last_checked_at','privacy_state','host_session_id','agent_id','selection'];
   IF NEW.privacy_state='active' AND (OLD.privacy_state<>'active' OR (NEW.host_session_id,NEW.agent_id,NEW.selection) IS DISTINCT FROM (OLD.host_session_id,OLD.agent_id,OLD.selection)) THEN RAISE EXCEPTION 'immutable digest scope'; END IF;
 ELSIF TG_TABLE_NAME='session_digest_generations' THEN
   IF NEW.receipt<OLD.receipt OR (OLD.privacy_state<>'active' AND NEW.privacy_state='active') THEN RAISE EXCEPTION 'monotonic digest generation'; END IF;
   oldjson:=oldjson-ARRAY['receipt','privacy_state'];newjson:=newjson-ARRAY['receipt','privacy_state'];
 ELSIF TG_TABLE_NAME='session_digest_claims' THEN
   IF NEW.covered_receipt<OLD.covered_receipt THEN RAISE EXCEPTION 'monotonic digest claim coverage'; END IF;
   oldjson:=oldjson-ARRAY['generation_id','covered_receipt'];newjson:=newjson-ARRAY['generation_id','covered_receipt'];
 ELSIF TG_TABLE_NAME='session_digest_pages' THEN
   oldjson:=oldjson-ARRAY['generation_id','active'];newjson:=newjson-ARRAY['generation_id','active'];
 END IF;
 IF newjson IS DISTINCT FROM oldjson THEN RAISE EXCEPTION 'immutable digest identity'; END IF;
 IF OLD.privacy_state='erased' AND NEW.privacy_state<>'erased' THEN RAISE EXCEPTION 'digest erasure is final'; END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_digest_immutable() FROM PUBLIC;
CREATE TRIGGER digest_partition_immutable BEFORE UPDATE ON session_digest_partitions FOR EACH ROW EXECUTE FUNCTION recollect_digest_immutable();
CREATE TRIGGER digest_generation_immutable BEFORE UPDATE ON session_digest_generations FOR EACH ROW EXECUTE FUNCTION recollect_digest_immutable();
-- Mappings contain opaque identity only, and still require matching page ownership.
CREATE FUNCTION recollect_digest_mapping() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.brain_id,NEW.partition_id,NEW.page) IS DISTINCT FROM (OLD.brain_id,OLD.partition_id,OLD.page) THEN RAISE EXCEPTION 'immutable digest mapping'; END IF;
 IF NOT EXISTS(SELECT 1 FROM session_digest_generations g WHERE g.id=NEW.generation_id AND g.brain_id=NEW.brain_id AND g.partition_id=NEW.partition_id AND g.page=NEW.page) THEN RAISE EXCEPTION 'digest page mismatch'; END IF;
 IF TG_TABLE_NAME='session_digest_claims' THEN
   IF NOT EXISTS(SELECT 1 FROM claim_revisions r JOIN session_digest_partitions p ON p.id=NEW.partition_id
      WHERE r.id=NEW.revision_id AND r.brain_id=NEW.brain_id AND r.claim_id=NEW.claim_id AND r.revision->>'origin'='model_synthesized'
       AND r.revision#>>'{content,kind}'='handover' AND r.revision#>'{content,selection}'=p.selection)
     OR NOT EXISTS(WITH RECURSIVE lineage AS (
       SELECT hr.id,hr.retry_of,0 depth FROM claim_revisions r JOIN handover_runs hr ON hr.id=(r.revision#>>'{derivation,run_id}')::uuid AND hr.brain_id=r.brain_id WHERE r.id=NEW.revision_id AND r.brain_id=NEW.brain_id
       UNION ALL SELECT hr.id,hr.retry_of,l.depth+1 FROM handover_runs hr JOIN lineage l ON hr.id=l.retry_of WHERE hr.brain_id=NEW.brain_id AND l.depth<8)
       SELECT 1 FROM lineage l JOIN session_digest_generations g ON g.run_id=l.id AND g.id=NEW.generation_id AND g.brain_id=NEW.brain_id)
   THEN RAISE EXCEPTION 'invalid digest publication'; END IF;
   IF TG_OP='UPDATE' THEN
     IF NEW.claim_id IS DISTINCT FROM OLD.claim_id OR NEW.covered_receipt<OLD.covered_receipt THEN RAISE EXCEPTION 'immutable digest claim identity'; END IF;
   END IF;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_digest_mapping() FROM PUBLIC;
CREATE TRIGGER digest_claim_mapping BEFORE INSERT OR UPDATE ON session_digest_claims FOR EACH ROW EXECUTE FUNCTION recollect_digest_mapping();
CREATE TRIGGER digest_page_mapping BEFORE INSERT OR UPDATE ON session_digest_pages FOR EACH ROW EXECUTE FUNCTION recollect_digest_mapping();
REVOKE UPDATE ON session_digest_events FROM recollect_app;
REVOKE UPDATE ON session_digest_partitions,session_digest_generations FROM recollect_app;
GRANT UPDATE(observed_receipt,covered_receipt,coverage,last_checked_at) ON session_digest_partitions TO recollect_app;
GRANT UPDATE(receipt) ON session_digest_generations TO recollect_app;

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_digest_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; partitions uuid[]; generations uuid[];
BEGIN
 m:=recollect_pre_digest_closure(b,k,t,c);
 m:=m||jsonb_build_object('artifacts',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM (
  SELECT unnest(recollect_privacy_ids(m,'artifacts')) id
  UNION SELECT i.artifact_id FROM excerpt_artifact_intents i WHERE i.brain_id=b
   AND (k='brain' OR i.parent_version_id=ANY(recollect_privacy_ids(m,'source_versions')))
 ) pending),'[]'::jsonb));
 SELECT coalesce(array_agg(DISTINCT p.id),'{}') INTO partitions FROM session_digest_partitions p
 WHERE p.brain_id=b AND (
  EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
   WHERE d.partition_id=p.id AND (e.source_version_id=ANY(recollect_privacy_ids(m,'source_versions'))
    OR e.id IN(SELECT (f->>'event_id')::uuid FROM jsonb_array_elements(coalesce(m->'capture_event_fences','[]')) f)))
  OR EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
    JOIN learning_runs l ON l.brain_id=e.brain_id AND l.source_version_id=e.source_version_id
    JOIN claims current_claim ON current_claim.brain_id=l.brain_id AND current_claim.id=ANY(l.claim_ids)
    WHERE d.partition_id=p.id AND current_claim.current_revision=ANY(recollect_privacy_ids(m,'claim_revisions')))
  OR EXISTS(SELECT 1 FROM session_digest_generations g WHERE g.partition_id=p.id AND g.contributors && recollect_privacy_ids(m,'claim_revisions'))
  OR EXISTS(SELECT 1 FROM session_digest_generations g JOIN claim_supports cs ON cs.brain_id=g.brain_id AND cs.revision_id=ANY(g.contributors)
    WHERE g.partition_id=p.id AND cs.source_version_id=ANY(recollect_privacy_ids(m,'source_versions')))
 ) AND (c='erase' OR (
  NOT EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
    JOIN source_versions v ON v.id=e.source_version_id AND v.brain_id=e.brain_id
    WHERE d.partition_id=p.id AND e.state='accepted'
     AND NOT v.id=ANY(recollect_privacy_ids(m,'source_versions'))
     AND recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at)='active')
  AND NOT EXISTS(SELECT 1 FROM session_digest_events d JOIN capture_events e ON e.id=d.event_id
    JOIN learning_runs l ON l.brain_id=e.brain_id AND l.source_version_id=e.source_version_id AND l.state IN ('succeeded','removed')
    JOIN claims current_claim ON current_claim.brain_id=l.brain_id AND current_claim.id=ANY(l.claim_ids)
    JOIN claim_revisions cr ON cr.id=current_claim.current_revision AND cr.brain_id=l.brain_id
    WHERE d.partition_id=p.id AND cr.privacy_state='active' AND cr.revision->>'review'='accepted'
     AND NOT cr.id=ANY(recollect_privacy_ids(m,'claim_revisions')) AND recollect_memory_supported(b,cr.id))));
 SELECT coalesce(array_agg(DISTINCT g.id),'{}') INTO generations FROM session_digest_generations g
 WHERE g.brain_id=b AND (g.partition_id=ANY(partitions) OR g.contributors && recollect_privacy_ids(m,'claim_revisions')
  OR EXISTS(SELECT 1 FROM session_digest_claims d JOIN claim_revisions r ON r.claim_id=d.claim_id AND r.brain_id=b
    WHERE d.generation_id=g.id AND r.id=ANY(recollect_privacy_ids(m,'claim_revisions')) AND r.id=(SELECT current_revision FROM claims WHERE id=r.claim_id AND brain_id=b)));
 RETURN m||jsonb_build_object('session_digest_partitions',partitions,'session_digest_generations',generations);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_digest_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; partitions uuid[]; generations uuid[];
BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 partitions:=recollect_privacy_ids(r.manifest,'session_digest_partitions');generations:=recollect_privacy_ids(r.manifest,'session_digest_generations');
 UPDATE session_digest_partitions SET selection='{}',host_session_id='',agent_id=NULL,coverage='{}',privacy_state=CASE WHEN privacy_state='erased' OR r.cause='erase' THEN 'erased' ELSE 'expired' END WHERE brain_id=r.brain_id AND id=ANY(partitions);
 UPDATE session_digest_generations SET privacy_state=CASE WHEN privacy_state='erased' OR r.cause='erase' THEN 'erased' ELSE 'expired' END WHERE brain_id=r.brain_id AND (id=ANY(generations) OR partition_id=ANY(partitions));
 PERFORM recollect_pre_digest_apply(rid);
 DELETE FROM excerpt_artifact_intents WHERE brain_id=r.brain_id AND artifact_id=ANY(recollect_privacy_ids(r.manifest,'artifacts'));
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=r.brain_id AND kind='handover.generate' AND target_id IN(SELECT run_id FROM session_digest_generations WHERE brain_id=r.brain_id AND privacy_state<>'active') AND state IN ('queued','running');
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_digest_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_digest_normalize(m)||jsonb_build_object('session_digest_partitions',to_jsonb(recollect_privacy_ids(m,'session_digest_partitions')),'session_digest_generations',to_jsonb(recollect_privacy_ids(m,'session_digest_generations')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;

CREATE OR REPLACE FUNCTION recollect_defer_handover_job(jid uuid,token uuid,reason text) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE target uuid;
BEGIN
 UPDATE jobs SET state='queued',error_code=reason,attempts=greatest(attempts-1,0),lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp(),
   not_before=CASE WHEN reason='model_budget_exhausted' THEN (date_trunc('day',clock_timestamp() AT TIME ZONE 'UTC')+interval '1 day') AT TIME ZONE 'UTC' ELSE clock_timestamp()+interval '10 seconds' END
 WHERE id=jid AND kind='handover.generate' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
   AND reason IN ('model_budget_exhausted','model_concurrency_full','session_coverage_pending') RETURNING target_id INTO target;
 IF target IS NULL THEN RETURN false; END IF;
 UPDATE handover_runs SET state='queued',error_code=reason WHERE id=target AND state='running'; RETURN true;
END $$;
REVOKE ALL ON FUNCTION recollect_defer_handover_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_defer_handover_job(uuid,uuid,text) TO recollect_app;
