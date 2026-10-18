-- Explicit utility is separate from truth and from the production ranker.
CREATE TABLE retrieval_feedback_contexts (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 actor_id uuid NOT NULL REFERENCES accounts(id),
 device_id uuid REFERENCES devices(id),
 task_id uuid NOT NULL,
 operation_id uuid NOT NULL REFERENCES operation_bindings(id),
 scope_id uuid NOT NULL REFERENCES scope_snapshots(id),
 query_context_id uuid NOT NULL,
 scope jsonb NOT NULL CHECK(octet_length(scope::text)<=65536),
 input_kind text NOT NULL CHECK(input_kind IN ('source_version','claim')),
 revision_id uuid NOT NULL,
 source_version_id uuid GENERATED ALWAYS AS(CASE WHEN input_kind='source_version' THEN revision_id END) STORED,
 claim_revision_id uuid GENERATED ALWAYS AS(CASE WHEN input_kind='claim' THEN revision_id END) STORED,
 change_id uuid,
 expires_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id),
 UNIQUE(id,brain_id,actor_id),
 UNIQUE(actor_id,task_id,query_context_id,input_kind,revision_id),
 FOREIGN KEY(task_id,brain_id,actor_id) REFERENCES workspace_tasks(id,brain_id,account_id),
 FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(claim_revision_id,brain_id) REFERENCES claim_revisions(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX feedback_source_dependency ON retrieval_feedback_contexts(brain_id,source_version_id);
CREATE INDEX feedback_claim_dependency ON retrieval_feedback_contexts(brain_id,claim_revision_id);
CREATE TABLE retrieval_feedback_changes (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 context_id uuid NOT NULL,
 actor_id uuid NOT NULL REFERENCES accounts(id),
 device_id uuid REFERENCES devices(id),
 vote text NOT NULL CHECK(vote IN ('helpful','unhelpful','reused')),
 previous_change_id uuid,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id,context_id),
 FOREIGN KEY(context_id,brain_id,actor_id) REFERENCES retrieval_feedback_contexts(id,brain_id,actor_id) ON DELETE CASCADE,
 FOREIGN KEY(previous_change_id,brain_id,context_id) REFERENCES retrieval_feedback_changes(id,brain_id,context_id)
);
ALTER TABLE retrieval_feedback_contexts ADD CONSTRAINT feedback_current_change
 FOREIGN KEY(change_id,brain_id,id) REFERENCES retrieval_feedback_changes(id,brain_id,context_id)
 DEFERRABLE INITIALLY DEFERRED;
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['retrieval_feedback_contexts','retrieval_feedback_changes'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY feedback_read ON %I FOR SELECT USING(actor_id=recollect_actor() AND brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY feedback_write ON %I FOR ALL USING(actor_id=recollect_actor() AND brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(actor_id=recollect_actor() AND brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
  EXECUTE format('GRANT SELECT,INSERT,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
GRANT UPDATE(change_id) ON retrieval_feedback_contexts TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (588,'retrieval_feedback_changes'),(589,'retrieval_feedback_contexts');

CREATE FUNCTION recollect_feedback_affected(b uuid,sources uuid[],claims uuid[],descendants boolean) RETURNS uuid[]
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 WITH RECURSIVE versions AS MATERIALIZED (
  SELECT id FROM recollect_source_derivative_versions(b,sources) WHERE descendants
  UNION SELECT unnest(sources) WHERE NOT descendants
 ), affected(id) AS (
  SELECT unnest(claims)
  UNION SELECT revision_id FROM claim_supports WHERE brain_id=b AND source_version_id IN(SELECT id FROM versions)
  UNION SELECT c.revision_id FROM claim_contributions c JOIN affected a ON a.id=c.input_revision_id WHERE c.brain_id=b
 ) SELECT coalesce(array_agg(id ORDER BY id),'{}') FROM retrieval_feedback_contexts
 WHERE brain_id=b AND (source_version_id IN(SELECT id FROM versions) OR claim_revision_id IN(SELECT id FROM affected))
$$;
REVOKE ALL ON FUNCTION recollect_feedback_affected(uuid,uuid[],uuid[],boolean) FROM PUBLIC;

CREATE FUNCTION recollect_feedback_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 UPDATE command_receipts SET invalidated=true,input='{}',response=NULL
 WHERE brain_id IN(SELECT DISTINCT brain_id FROM feedback_removed_contexts)
 AND operation IN ('retrieval.feedback.context','retrieval.feedback.vote');
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_feedback_removed() FROM PUBLIC;
CREATE TRIGGER feedback_context_removed AFTER DELETE ON retrieval_feedback_contexts
 REFERENCING OLD TABLE AS feedback_removed_contexts
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_feedback_removed();

CREATE FUNCTION recollect_feedback_fenced() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; seeds uuid[]; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM feedback_new_fences LOOP
  SELECT array_agg(source_version_id) INTO seeds FROM feedback_new_fences WHERE brain_id=b;
  DELETE FROM retrieval_feedback_contexts WHERE brain_id=b AND id=ANY(recollect_feedback_affected(b,seeds,'{}',true));
 END LOOP;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_feedback_fenced() FROM PUBLIC;
CREATE TRIGGER feedback_source_fenced AFTER INSERT ON model_input_fences
 REFERENCING NEW TABLE AS feedback_new_fences
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_feedback_fenced();

CREATE FUNCTION recollect_feedback_claim_fenced() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; seeds uuid[]; BEGIN
 FOR b IN SELECT DISTINCT brain_id FROM feedback_new_claim_fences LOOP
  SELECT array_agg(revision_id) INTO seeds FROM feedback_new_claim_fences WHERE brain_id=b;
  DELETE FROM retrieval_feedback_contexts WHERE brain_id=b AND id=ANY(recollect_feedback_affected(b,'{}',seeds,true));
 END LOOP;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_feedback_claim_fenced() FROM PUBLIC;
CREATE TRIGGER feedback_claim_fenced AFTER INSERT ON model_claim_fences
 REFERENCING NEW TABLE AS feedback_new_claim_fences
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_feedback_claim_fenced();

CREATE FUNCTION recollect_feedback_scope_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF OLD.current_scope IS DISTINCT FROM NEW.current_scope THEN
  DELETE FROM retrieval_feedback_contexts WHERE brain_id=NEW.brain_id AND task_id=NEW.id;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_feedback_scope_changed() FROM PUBLIC;
CREATE TRIGGER feedback_task_scope_changed AFTER UPDATE OF current_scope ON workspace_tasks
 FOR EACH ROW EXECUTE FUNCTION recollect_feedback_scope_changed();

CREATE FUNCTION recollect_feedback_head_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_TABLE_NAME='sources' THEN
  IF OLD.current_version IS DISTINCT FROM NEW.current_version AND OLD.current_version IS NOT NULL THEN
   DELETE FROM retrieval_feedback_contexts WHERE brain_id=NEW.brain_id AND id=ANY(recollect_feedback_affected(NEW.brain_id,ARRAY[OLD.current_version],'{}',false));
  END IF;
 ELSE
  IF OLD.current_revision IS DISTINCT FROM NEW.current_revision AND OLD.current_revision IS NOT NULL THEN
   DELETE FROM retrieval_feedback_contexts WHERE brain_id=NEW.brain_id AND id=ANY(recollect_feedback_affected(NEW.brain_id,'{}',ARRAY[OLD.current_revision],false));
  END IF;
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_feedback_head_changed() FROM PUBLIC;
CREATE TRIGGER feedback_source_head_changed AFTER UPDATE OF current_version ON sources
 FOR EACH ROW EXECUTE FUNCTION recollect_feedback_head_changed();
CREATE TRIGGER feedback_claim_head_changed AFTER UPDATE OF current_revision ON claims
 FOR EACH ROW EXECUTE FUNCTION recollect_feedback_head_changed();

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_feedback_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; ids uuid[]; BEGIN
 m:=recollect_pre_feedback_closure(b,k,t,c);
 IF k='brain' THEN SELECT coalesce(array_agg(id ORDER BY id),'{}') INTO ids FROM retrieval_feedback_contexts WHERE brain_id=b;
 ELSE ids:=recollect_feedback_affected(b,recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources'),recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions'),c='erase');
 END IF;
 RETURN m||jsonb_build_object('retrieval_feedback_contexts',ids);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_feedback_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_feedback_normalize(m)||jsonb_build_object(
  'retrieval_feedback_contexts',to_jsonb(recollect_privacy_ids(m,'retrieval_feedback_contexts')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_feedback_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; sources uuid[]; contexts uuid[]; BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 sources:=recollect_privacy_ids(r.manifest,'source_versions')||recollect_privacy_ids(r.manifest,'model_input_sources');
 IF r.cause='erase' THEN
  SELECT coalesce(array_agg(DISTINCT id),'{}') INTO sources FROM (
   SELECT unnest(sources) id UNION
   SELECT source_version_id FROM claim_supports WHERE brain_id=r.brain_id AND revision_id=ANY(recollect_privacy_ids(r.manifest,'claim_revisions')) AND source_version_id IS NOT NULL UNION
   SELECT source_version_id FROM model_input_fences WHERE brain_id=r.brain_id AND request_id=rid
  ) seed;
 END IF;
 contexts:=recollect_privacy_ids(r.manifest,'retrieval_feedback_contexts')||recollect_feedback_affected(r.brain_id,sources,recollect_privacy_ids(r.manifest,'claim_revisions')||recollect_privacy_ids(r.manifest,'model_input_claim_revisions'),r.cause='erase');
 DELETE FROM retrieval_feedback_contexts WHERE brain_id=r.brain_id AND id=ANY(contexts);
 PERFORM recollect_pre_feedback_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
