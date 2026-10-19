-- Human decisions link original identities; no automatic identity merge.
ALTER TABLE knowledge_relation_candidates ADD CONSTRAINT alias_candidate_brain UNIQUE(id,brain_id);
CREATE TABLE knowledge_alias_changes (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 candidate_id uuid NOT NULL,
 actor_id uuid NOT NULL REFERENCES accounts(id),
 action text NOT NULL CHECK(action IN ('accept','reject','revoke')),
 previous_change_id uuid,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id,candidate_id),
 FOREIGN KEY(candidate_id,brain_id) REFERENCES knowledge_relation_candidates(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(previous_change_id,brain_id,candidate_id) REFERENCES knowledge_alias_changes(id,brain_id,candidate_id)
);
CREATE TABLE knowledge_alias_heads (
 brain_id uuid NOT NULL,
 candidate_id uuid PRIMARY KEY,
 change_id uuid NOT NULL,
 FOREIGN KEY(candidate_id,brain_id) REFERENCES knowledge_relation_candidates(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(change_id,brain_id,candidate_id) REFERENCES knowledge_alias_changes(id,brain_id,candidate_id)
);
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_alias_changes','knowledge_alias_heads'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY alias_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY alias_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (586,'knowledge_alias_heads'),(587,'knowledge_alias_changes');
CREATE FUNCTION recollect_alias_history_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN RAISE EXCEPTION 'alias decision history is immutable'; END $$;
REVOKE ALL ON FUNCTION recollect_alias_history_immutable() FROM PUBLIC;
CREATE TRIGGER alias_history_immutable BEFORE UPDATE ON knowledge_alias_changes
 FOR EACH ROW EXECUTE FUNCTION recollect_alias_history_immutable();

-- Preview exact decision dependencies as well as their parent mapping inputs.
-- Physical removal and old-journal replay use the source->input->candidate FK
-- chain, so older journals do not need these additive preview fields to close.
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_alias_decisions_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; candidates uuid[]; changes uuid[];
BEGIN
 m:=recollect_pre_alias_decisions_closure(b,k,t,c);
 SELECT coalesce(array_agg(r.id ORDER BY r.id),'{}') INTO candidates
 FROM knowledge_relation_candidates r WHERE r.brain_id=b AND r.relation='alias'
  AND (k='brain' OR r.input_id=ANY(recollect_privacy_ids(m,'knowledge_mapping_inputs')));
 SELECT coalesce(array_agg(id ORDER BY id),'{}') INTO changes FROM knowledge_alias_changes
 WHERE brain_id=b AND candidate_id=ANY(candidates);
 RETURN m||jsonb_build_object('knowledge_alias_candidates',candidates,'knowledge_alias_changes',changes);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_alias_decisions_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_alias_decisions_normalize(m)||jsonb_build_object(
  'knowledge_alias_candidates',to_jsonb(recollect_privacy_ids(m,'knowledge_alias_candidates')),
  'knowledge_alias_changes',to_jsonb(recollect_privacy_ids(m,'knowledge_alias_changes')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
