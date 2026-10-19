-- Deterministic canonical UUID references. Never source spans or extractor output.
CREATE TABLE knowledge_field_entities (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 kind text NOT NULL CHECK(kind IN ('repository','environment')),
 repository_id uuid,
 environment_id uuid,
 UNIQUE(id,brain_id),
 CHECK((kind='repository' AND repository_id IS NOT NULL AND environment_id IS NULL)
    OR (kind='environment' AND environment_id IS NOT NULL AND repository_id IS NULL)),
 FOREIGN KEY(repository_id,brain_id) REFERENCES repositories(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(environment_id,brain_id) REFERENCES evidence_groups(id,brain_id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX field_repository_identity ON knowledge_field_entities(brain_id,repository_id) WHERE repository_id IS NOT NULL;
CREATE UNIQUE INDEX field_environment_identity ON knowledge_field_entities(brain_id,environment_id) WHERE environment_id IS NOT NULL;
CREATE TABLE knowledge_field_inputs (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 origin_kind text NOT NULL CHECK(origin_kind IN ('source_context','claim_revision','repository_snapshot','manifest_revision')),
 logical_id uuid NOT NULL,
 revision_id uuid NOT NULL,
 context jsonb NOT NULL CHECK(octet_length(context::text)<=32768),
 evidence_kind text NOT NULL CHECK(evidence_kind IN ('source_version','claim','repository_fact','manifest_revision')),
 evidence_id uuid NOT NULL,
 evidence_revision uuid NOT NULL,
 created_by uuid NOT NULL REFERENCES accounts(id),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(id,brain_id),
 UNIQUE(brain_id,origin_kind,logical_id)
);
CREATE TABLE knowledge_field_references (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 input_id uuid NOT NULL,
 entity_id uuid NOT NULL,
 field_pointer text NOT NULL CHECK(octet_length(field_pointer) BETWEEN 1 AND 256 AND left(field_pointer,1)='/'),
 value uuid NOT NULL,
 UNIQUE(input_id,field_pointer),
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_field_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(entity_id,brain_id) REFERENCES knowledge_field_entities(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX field_reference_entity ON knowledge_field_references(brain_id,entity_id,input_id);
CREATE TABLE knowledge_field_outbox (
 brain_id uuid PRIMARY KEY REFERENCES brains(id),
 actor_id uuid NOT NULL REFERENCES accounts(id),
 revision bigint NOT NULL DEFAULT 1,
 handled_revision bigint NOT NULL DEFAULT 0,
 phase integer NOT NULL DEFAULT 0 CHECK(phase BETWEEN 0 AND 4),
 after_id uuid,
 state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','queued','ready','blocked','failed')),
 job_id uuid REFERENCES jobs(id) ON DELETE SET NULL,
 deadline timestamptz,
 updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE FUNCTION recollect_field_immutable() RETURNS trigger LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN RAISE EXCEPTION 'Canonical field inputs and references are immutable'; END $$;
CREATE TRIGGER field_input_immutable BEFORE UPDATE ON knowledge_field_inputs FOR EACH ROW EXECUTE FUNCTION recollect_field_immutable();
CREATE TRIGGER field_reference_immutable BEFORE UPDATE ON knowledge_field_references FOR EACH ROW EXECUTE FUNCTION recollect_field_immutable();
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_field_entities','knowledge_field_inputs','knowledge_field_references','knowledge_field_outbox'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY field_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('CREATE POLICY field_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (600,'knowledge_field_references'),(601,'knowledge_field_inputs'),
 (602,'knowledge_field_entities'),(9991,'knowledge_field_outbox');

-- A server-owned immutable representation of the exact source metadata fields.
-- Names are deliberately absent; readers hydrate only current qualified names.
CREATE FUNCTION recollect_field_source_context(b uuid,v uuid) RETURNS jsonb
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT jsonb_build_object('selection',coalesce((SELECT selection FROM source_import_scopes WHERE brain_id=b AND version_id=v),'{}'::jsonb),
 'environments',coalesce((SELECT jsonb_agg(m.group_id ORDER BY m.group_id)
  FROM source_versions sv JOIN evidence_memberships m ON m.brain_id=sv.brain_id AND m.source_id=sv.source_id
  JOIN evidence_groups g ON g.brain_id=m.brain_id AND g.id=m.group_id AND g.kind='environment'
  WHERE sv.brain_id=b AND sv.id=v),'[]'::jsonb))
$$;
REVOKE ALL ON FUNCTION recollect_field_source_context(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_field_source_context(uuid,uuid) TO recollect_app;
CREATE FUNCTION recollect_field_context(b uuid,k text,v uuid) RETURNS jsonb
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT CASE k WHEN 'source_context' THEN recollect_field_source_context(b,v)
 WHEN 'claim_revision' THEN (SELECT jsonb_build_object('content',jsonb_build_object('selection',revision#>'{content,selection}')) FROM claim_revisions WHERE brain_id=b AND id=v)
 WHEN 'repository_snapshot' THEN (SELECT jsonb_build_object('repository_id',repository_id) FROM repository_snapshots WHERE brain_id=b AND id=v)
 WHEN 'manifest_revision' THEN (SELECT jsonb_build_object('environment_id',revision->'environment_id','entries',
  coalesce((SELECT jsonb_agg(jsonb_build_object('repository_id',e->'repository_id') ORDER BY n) FROM jsonb_array_elements(revision->'entries') WITH ORDINALITY x(e,n)),'[]'::jsonb)) FROM manifest_revisions WHERE brain_id=b AND id=v) ELSE NULL END
$$;
REVOKE ALL ON FUNCTION recollect_field_context(uuid,text,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_field_context(uuid,text,uuid) TO recollect_app;
CREATE FUNCTION recollect_field_input_current(b uuid,target uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT EXISTS(SELECT 1 FROM knowledge_field_inputs i WHERE i.brain_id=b AND i.id=target AND CASE i.origin_kind
 WHEN 'source_context' THEN EXISTS(SELECT 1 FROM source_versions v JOIN sources s ON s.brain_id=v.brain_id AND s.id=v.source_id AND s.current_version=v.id
  WHERE v.brain_id=b AND v.id=i.revision_id AND s.id=i.logical_id AND i.evidence_kind='source_version' AND i.evidence_id=s.id AND i.evidence_revision=v.id
   AND v.processing='ready' AND NOT recollect_source_model_fenced(b,v.id)
   AND recollect_content_state(b,v.retention_class,v.privacy_state,v.created_at)='active'
   AND i.context=recollect_field_source_context(b,v.id))
 WHEN 'claim_revision' THEN EXISTS(SELECT 1 FROM claim_revisions r JOIN claims c ON c.brain_id=r.brain_id AND c.id=r.claim_id AND c.current_revision=r.id
  WHERE r.brain_id=b AND r.id=i.revision_id AND c.id=i.logical_id AND i.evidence_kind='claim' AND i.evidence_id=c.id AND i.evidence_revision=r.id
   AND i.context=recollect_field_context(b,i.origin_kind,r.id) AND recollect_content_state(b,'claim',r.privacy_state,r.recorded_at)='active'
   AND recollect_memory_supported(b,r.id))
 WHEN 'repository_snapshot' THEN EXISTS(SELECT 1 FROM repository_snapshots s JOIN repository_facts f ON f.brain_id=s.brain_id AND f.snapshot_id=s.id
  WHERE s.brain_id=b AND s.id=i.revision_id AND s.repository_id=i.logical_id AND i.evidence_kind='repository_fact' AND i.evidence_id=f.id AND i.evidence_revision=f.id
   AND i.context=jsonb_build_object('repository_id',s.repository_id)
   AND recollect_content_state(b,'repository',s.privacy_state,s.created_at)='active')
 WHEN 'manifest_revision' THEN EXISTS(SELECT 1 FROM manifest_revisions r JOIN revision_manifests m ON m.brain_id=r.brain_id AND m.id=r.manifest_id AND m.current_revision=r.id
  WHERE r.brain_id=b AND r.id=i.revision_id AND m.id=i.logical_id AND i.evidence_kind='manifest_revision' AND i.evidence_id=r.id AND i.evidence_revision=r.id
   AND i.context=recollect_field_context(b,i.origin_kind,r.id) AND r.privacy_state='active') ELSE false END)
$$;
REVOKE ALL ON FUNCTION recollect_field_input_current(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_field_input_current(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_field_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE affected uuid[]; BEGIN
 IF TG_OP='INSERT' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM new_rows;
 ELSIF TG_OP='DELETE' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM old_rows;
 ELSE SELECT array_agg(DISTINCT brain_id) INTO affected FROM (SELECT brain_id FROM new_rows UNION SELECT brain_id FROM old_rows) x; END IF;
 INSERT INTO knowledge_field_outbox(brain_id,actor_id)
 SELECT b.id,CASE WHEN recollect_effective_role(b.id,recollect_actor()) IN ('writer','admin') THEN recollect_actor() ELSE b.owner_id END FROM brains b WHERE b.id=ANY(affected)
 ON CONFLICT(brain_id) DO UPDATE SET revision=knowledge_field_outbox.revision+1,
  phase=0,after_id=NULL,state='pending',actor_id=CASE WHEN recollect_effective_role(excluded.brain_id,recollect_actor()) IN ('writer','admin') THEN recollect_actor() ELSE knowledge_field_outbox.actor_id END,updated_at=clock_timestamp();
 RETURN NULL; END $$;
REVOKE ALL ON FUNCTION recollect_field_changed() FROM PUBLIC;
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['sources','source_versions','source_chunks','source_import_scopes','evidence_memberships','evidence_groups',
 'claims','claim_revisions','claim_supports','claim_contributions','memory_epochs','assertion_rules','memory_rule_exceptions',
 'repositories','repository_snapshots','repository_facts','revision_manifests','manifest_revisions','retention_policies','brain_grants','brain_group_grants'] LOOP
  EXECUTE format('CREATE TRIGGER field_insert AFTER INSERT ON %I REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_field_changed()',tab);
  EXECUTE format('CREATE TRIGGER field_update AFTER UPDATE ON %I REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_field_changed()',tab);
  EXECUTE format('CREATE TRIGGER field_delete AFTER DELETE ON %I REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_field_changed()',tab);
 END LOOP;
END $$;
-- Account and OIDC authority changes invalidate the originating writer.
CREATE FUNCTION recollect_field_account_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 UPDATE knowledge_field_outbox o SET revision=o.revision+1,phase=0,after_id=NULL,state='pending',updated_at=clock_timestamp()
 WHERE o.actor_id IN (SELECT n.id FROM new_rows n JOIN old_rows previous_row USING(id)
  WHERE ROW(n.enabled,n.auth_kind,n.oidc_issuer,n.oidc_groups,n.membership_until)
   IS DISTINCT FROM ROW(previous_row.enabled,previous_row.auth_kind,previous_row.oidc_issuer,previous_row.oidc_groups,previous_row.membership_until));
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_field_account_changed() FROM PUBLIC;
CREATE TRIGGER field_account_update AFTER UPDATE ON accounts
 REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_field_account_changed();
INSERT INTO knowledge_field_outbox(brain_id,actor_id) SELECT id,owner_id FROM brains;
-- Reader-visible readiness contains no actor or cross-Brain payload.
CREATE FUNCTION recollect_field_actor_current(b uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT coalesce(recollect_role(b) IS NOT NULL AND EXISTS(
  SELECT 1 FROM knowledge_field_outbox o JOIN brains br ON br.id=o.brain_id
  WHERE o.brain_id=b AND NOT br.archived
   AND recollect_effective_role(b,o.actor_id) IN ('writer','admin')),false)
$$;
REVOKE ALL ON FUNCTION recollect_field_actor_current(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_field_actor_current(uuid) TO recollect_app;
CREATE FUNCTION recollect_field_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT o.brain_id,o.actor_id FROM knowledge_field_outbox o JOIN brains b ON b.id=o.brain_id JOIN accounts a ON a.id=o.actor_id
 WHERE NOT b.archived AND a.enabled AND recollect_effective_role(b.id,a.id) IN ('writer','admin')
  AND (o.revision>o.handled_revision OR o.deadline<=clock_timestamp())
  AND NOT EXISTS(SELECT 1 FROM jobs j WHERE j.brain_id=b.id AND j.kind='knowledge.fields' AND j.state IN ('queued','running'))
 ORDER BY o.updated_at,b.id LIMIT 8
$$;
REVOKE ALL ON FUNCTION recollect_field_brains() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_field_brains() TO recollect_app;

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_fields_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; ids uuid[]; BEGIN
 m:=recollect_pre_fields_closure(b,k,t,c);
 SELECT coalesce(array_agg(i.id ORDER BY i.id),'{}') INTO ids FROM knowledge_field_inputs i WHERE i.brain_id=b AND (k='brain'
  OR (i.origin_kind='source_context' AND i.revision_id=ANY(recollect_privacy_ids(m,'source_versions')||recollect_privacy_ids(m,'model_input_sources')))
  OR (i.origin_kind='claim_revision' AND i.revision_id=ANY(recollect_privacy_ids(m,'claim_revisions')||recollect_privacy_ids(m,'model_input_claim_revisions')))
  OR (i.origin_kind='repository_snapshot' AND i.revision_id=ANY(recollect_privacy_ids(m,'snapshots')))
  OR (i.origin_kind='manifest_revision' AND i.revision_id=ANY(recollect_privacy_ids(m,'manifest_revisions'))));
 RETURN m||jsonb_build_object('knowledge_field_inputs',ids); END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_fields_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 DELETE FROM knowledge_field_inputs i WHERE i.brain_id=r.brain_id AND (r.target->>'kind'='brain' OR i.id=ANY(recollect_privacy_ids(r.manifest,'knowledge_field_inputs'))
  OR (i.origin_kind='source_context' AND i.revision_id=ANY(recollect_privacy_ids(r.manifest,'source_versions')||recollect_privacy_ids(r.manifest,'model_input_sources')))
  OR (i.origin_kind='claim_revision' AND i.revision_id=ANY(recollect_privacy_ids(r.manifest,'claim_revisions')||recollect_privacy_ids(r.manifest,'model_input_claim_revisions')))
  OR (i.origin_kind='repository_snapshot' AND i.revision_id=ANY(recollect_privacy_ids(r.manifest,'snapshots')))
  OR (i.origin_kind='manifest_revision' AND i.revision_id=ANY(recollect_privacy_ids(r.manifest,'manifest_revisions'))));
 DELETE FROM knowledge_field_entities e WHERE e.brain_id=r.brain_id AND NOT EXISTS(SELECT 1 FROM knowledge_field_references f WHERE f.brain_id=e.brain_id AND f.entity_id=e.id);
 PERFORM recollect_pre_fields_apply(rid); END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_normalize(jsonb) RENAME TO recollect_pre_fields_normalize;
CREATE FUNCTION recollect_privacy_normalize(m jsonb) RETURNS jsonb LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT recollect_pre_fields_normalize(m)||jsonb_build_object('knowledge_field_inputs',to_jsonb(recollect_privacy_ids(m,'knowledge_field_inputs')))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_normalize(jsonb) FROM PUBLIC;
