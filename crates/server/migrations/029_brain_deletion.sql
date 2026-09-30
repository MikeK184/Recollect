-- Brain deletion is a Brain-wide erasure target (ADR 0016). It composes the
-- existing per-class closure rules, the durable journal and the existing
-- cleanup queue; only minimal tombstone metadata is new. The rows below carry
-- identity, actor, time, disposition, the honored closure counter and derived
-- cleanup state only. No name, description, subject, value, title, URI, path,
-- excerpt or reason column exists anywhere in this migration.
CREATE TABLE brain_deletions (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    -- Nullable because the retained journal format carries no actor identity,
    -- so a tombstone rebuilt by restore replay cannot know the initiator.
    actor_id uuid REFERENCES accounts(id),
    closure bigint NOT NULL,
    disposition text NOT NULL CHECK (disposition IN ('deleted')),
    created_at timestamptz NOT NULL
);
CREATE TABLE brain_tombstones (
    brain_id uuid PRIMARY KEY,
    deletion_id uuid NOT NULL UNIQUE REFERENCES brain_deletions(id),
    deleted_at timestamptz NOT NULL
);
-- The Brain row is gone, so Brain role authority cannot apply. Only the
-- initiating actor or the installation owner may read either tombstone; the
-- API layer turns "no visible row" into unknown rather than forbidden.
ALTER TABLE brain_deletions ENABLE ROW LEVEL SECURITY;
CREATE POLICY deletion_read ON brain_deletions FOR SELECT
    USING (actor_id = recollect_actor() OR EXISTS(
        SELECT 1 FROM accounts a WHERE a.id = recollect_actor() AND a.enabled AND a.installation_owner));
ALTER TABLE brain_tombstones ENABLE ROW LEVEL SECURITY;
CREATE POLICY tombstone_read ON brain_tombstones FOR SELECT
    USING (EXISTS(SELECT 1 FROM brain_deletions d WHERE d.id = deletion_id AND
        (d.actor_id = recollect_actor() OR EXISTS(
            SELECT 1 FROM accounts a WHERE a.id = recollect_actor() AND a.enabled AND a.installation_owner))));
GRANT SELECT ON brain_deletions, brain_tombstones TO recollect_app;

-- The journal and its content-free device acknowledgement positions must
-- outlive the Brain they describe, or a restored older database could not be
-- held against its retained deletion entries and could not distinguish central
-- completion from offline companion copies. Command receipts survive too: the
-- existing apply chain invalidates them (key kept, input and response emptied)
-- before the Brain row is removed. All three tables keep their existing
-- content-free columns; only the Brain existence constraint is released.
DO $$ DECLARE c record; BEGIN
  FOR c IN SELECT child.relname AS table_name,con.conname AS constraint_name
    FROM pg_constraint con
    JOIN pg_class child ON child.oid=con.conrelid
    JOIN pg_class parent ON parent.oid=con.confrelid
    JOIN pg_namespace n ON n.oid=child.relnamespace
    WHERE n.nspname='public' AND con.contype='f' AND parent.relname='brains'
      AND child.relname IN ('privacy_requests','privacy_device_positions','command_receipts')
  LOOP
    EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I',c.table_name,c.constraint_name);
  END LOOP;
END $$;

-- Static, content-free removal order over every relation that holds a brain_id
-- column, deepest dependent first. Excluded on purpose: brains (removed last by
-- identity), privacy_requests and privacy_artifacts (the retained journal and
-- its cleanup state), privacy_publication_fences and privacy_device_positions
-- (retained Brain-free fence and acknowledgement records) and command_receipts
-- (invalidated, not deleted, by the existing apply chain: the command key and a
-- content-free invalidated result are retained for the original window). Every
-- non-deferrable
-- foreign key among these relations is satisfied by this order; the deferred
-- "current pointer" pairs (claims/claim_revisions, sources/source_versions,
-- revision_manifests/manifest_revisions, workspace_tasks/scope_snapshots) are
-- checked at commit, when both sides of the pair are already gone.
-- A later migration that adds a Brain-dependent relation must extend this list,
-- or the catalog test in crates/server/tests/brain_deletion.rs fails.
CREATE TABLE brain_deletion_dependents (
    ordinal integer PRIMARY KEY,
    relation text NOT NULL UNIQUE
);
INSERT INTO brain_deletion_dependents (ordinal,relation) VALUES
  -- tier 0: nothing else in this list references them
  (1010,'analytics_attempts'),
  (1020,'answer_requests'),
  (1030,'brain_directory'),
  (1040,'brain_grants'),
  (1050,'brain_group_grants'),
  (1060,'capture_device_reports'),
  (1070,'capture_events'),
  (1080,'capture_policies'),
  (1090,'checkout_registrations'),
  (1100,'claim_contributions'),
  (1110,'claim_model_derivations'),
  (1120,'claim_supports'),
  (1140,'evidence_memberships'),
  (1150,'graph_generations'),
  (1160,'handover_run_inputs'),
  (1170,'learning_run_inputs'),
  (1180,'mcp_call_payloads'),
  (1190,'mcp_call_resolutions'),
  (1200,'mcp_instances'),
  (1210,'mcp_observation_outbox'),
  (1220,'mcp_private_runners'),
  (1230,'mcp_profile_connections'),
  (1240,'mcp_profile_grants'),
  (1250,'mcp_session_releases'),
  (1260,'memory_decision_claims'),
  (1270,'memory_decisions'),
  (1280,'memory_epochs'),
  (1290,'memory_rule_exceptions'),
  (1300,'model_claim_fences'),
  (1310,'model_input_fences'),
  (1320,'model_policy_heads'),
  (1330,'model_request_inputs'),
  (1340,'privacy_capture_fences'),
  (1350,'repository_artifacts'),
  (1360,'repository_contributions'),
  (1370,'repository_files'),
  (1380,'repository_jobs'),
  (1390,'repository_origins'),
  (1400,'retention_policies'),
  (1410,'semantic_batch_inputs'),
  (1420,'semantic_heads'),
  (1430,'source_excerpts'),
  -- tier 1
  (2010,'analytics_reports'),
  (2020,'capture_bindings'),
  (2030,'handover_runs'),
  (2040,'learning_runs'),
  (2050,'assertion_rules'),
  (2060,'semantic_entries'),
  -- tier 2
  (3010,'mcp_calls'),
  (3020,'operation_bindings'),
  (3030,'claim_revisions'),
  (3040,'manifest_revisions'),
  (3050,'repository_facts'),
  (3060,'semantic_batches'),
  (3070,'source_chunks'),
  -- tier 3
  (4010,'mcp_connections'),
  (4020,'mcp_profiles'),
  (4030,'scope_snapshots'),
  (4040,'claims'),
  (4050,'repository_snapshots'),
  (4060,'revision_manifests'),
  (4070,'jobs'),
  (4080,'model_requests'),
  (4090,'semantic_profiles'),
  (4100,'source_versions'),
  -- tier 4
  (5010,'repositories'),
  (5020,'evidence_groups'),
  (5030,'mutation_audit'),
  (5040,'model_policies'),
  (5050,'sources'),
  (5060,'workspace_tasks'),
  -- tier 5
  (6010,'workspace_registrations');
GRANT SELECT ON brain_deletion_dependents TO recollect_app;

-- Content-free fingerprint of a closure document. The preview counter and the
-- stored tombstone counter both come from this one function, so they agree by
-- construction for the same pre-deletion state.
CREATE FUNCTION recollect_brain_hash(document jsonb) RETURNS bigint
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
  SELECT (('x'||left(md5(coalesce(document::text,'null')),15))::bit(60))::bigint
$$;

-- Union of two closure documents. Arrays merge by identity; the Brain-wide
-- scope has no independently retained remainder and shares no outside source.
CREATE FUNCTION recollect_brain_merge(target jsonb, addition jsonb) RETURNS jsonb
LANGUAGE plpgsql IMMUTABLE SET search_path=public,pg_temp AS $$
DECLARE item record;
BEGIN
  FOR item IN SELECT key,value FROM jsonb_each(addition) LOOP
    IF jsonb_typeof(item.value)='array' THEN
      target:=jsonb_set(target,ARRAY[item.key],(SELECT coalesce(jsonb_agg(v ORDER BY v),'[]') FROM
        (SELECT DISTINCT value v FROM jsonb_array_elements(coalesce(target->item.key,'[]')||item.value)) merged));
    END IF;
  END LOOP;
  RETURN target;
END $$;

-- Brain-wide closure is the union of each domain owner's existing per-class
-- erasure closure. It writes no bespoke dependent set of its own: whole-Brain
-- targets own every member identity, so iterating the durable target classes
-- covers each record exactly through its owner's rule. The result is always a
-- Manifest-shaped document, because every contribution comes from
-- recollect_privacy_closure.
CREATE FUNCTION recollect_brain_closure(b uuid) RETURNS jsonb
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; t record;
BEGIN
  IF recollect_role(b) IS NULL THEN RETURN NULL; END IF;
  m:='{"source_versions":[],"source_ids":[],"claim_revisions":[],"claim_ids":[],"snapshots":[],"facts":[],"manifest_revisions":[],"rules":[],"decisions":[],"artifacts":[],"jobs":[],"model_input_sources":[],"model_input_claim_revisions":[],"capture_event_fences":[],"publication_fences":[],"independent_claim_revisions":0,"shared_sources":false}'::jsonb;
  FOR t IN SELECT 'source'::text AS k,id FROM sources WHERE brain_id=b
           UNION ALL SELECT 'claim',id FROM claims WHERE brain_id=b
           UNION ALL SELECT 'snapshot',id FROM repository_snapshots WHERE brain_id=b
           UNION ALL SELECT 'manifest',id FROM revision_manifests WHERE brain_id=b
  LOOP
    m:=recollect_brain_merge(m,recollect_privacy_closure(b,t.k,t.id,'erase'));
  END LOOP;
  -- Every revision is covered by its own target, so nothing is independently
  -- retained, and no shared source reaches another Brain.
  m:=jsonb_set(m,'{independent_claim_revisions}','0');
  m:=jsonb_set(m,'{shared_sources}','false');
  RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_brain_hash(jsonb),recollect_brain_merge(jsonb,jsonb),recollect_brain_closure(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_closure(uuid) TO recollect_app;

-- The closure counter a DELETE must honor. It changes whenever any dependent
-- identity changes, and preserves no controlled value.
CREATE FUNCTION recollect_brain_counter(b uuid) RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT recollect_brain_hash(recollect_brain_closure(b))
  WHERE recollect_role(b) IS NOT NULL
$$;
REVOKE ALL ON FUNCTION recollect_brain_counter(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_counter(uuid) TO recollect_app;

-- Preview identity, name, counter, per-class dependent counts, work that will
-- be fenced, retained physical copies and the product-managed backup window.
-- Administrator-only and browser-only; NULL for anything else, so the API can
-- answer unknown rather than forbidden. No controlled content, title, path or
-- excerpt is returned.
CREATE FUNCTION recollect_brain_preview(b uuid) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER STABLE SET search_path=public,pg_temp AS $$
DECLARE m jsonb;
BEGIN
  IF recollect_role(b) IS DISTINCT FROM 'admin' OR recollect_device() IS NOT NULL THEN RETURN NULL; END IF;
  m:=recollect_brain_closure(b);
  RETURN jsonb_build_object(
    'brain_id',b,
    'name',(SELECT name FROM brains WHERE id=b),
    'archived',(SELECT archived FROM brains WHERE id=b),
    'closure',recollect_brain_hash(m),
    'counts',jsonb_build_object(
      'sources',(SELECT count(*) FROM sources WHERE brain_id=b),
      'source_versions',jsonb_array_length(m->'source_versions'),
      'excerpts',(SELECT count(*) FROM source_excerpts WHERE brain_id=b),
      'chunks',(SELECT count(*) FROM source_chunks WHERE brain_id=b),
      'claims',(SELECT count(*) FROM claims WHERE brain_id=b),
      'claim_revisions',jsonb_array_length(m->'claim_revisions'),
      'review_decisions',jsonb_array_length(m->'decisions'),
      'rejected_rules',jsonb_array_length(m->'rules'),
      'snapshots',jsonb_array_length(m->'snapshots'),
      'facts',jsonb_array_length(m->'facts'),
      'manifests',(SELECT count(*) FROM revision_manifests WHERE brain_id=b),
      'manifest_revisions',jsonb_array_length(m->'manifest_revisions'),
      'environments',(SELECT count(*) FROM evidence_groups WHERE brain_id=b AND kind='environment'),
      'collections',(SELECT count(*) FROM evidence_groups WHERE brain_id=b AND kind='collection'),
      'areas',(SELECT count(*) FROM evidence_groups WHERE brain_id=b AND kind='area'),
      'memberships',(SELECT count(*) FROM evidence_memberships WHERE brain_id=b),
      'repositories',(SELECT count(*) FROM repositories WHERE brain_id=b),
      'workspaces',(SELECT count(*) FROM workspace_registrations WHERE brain_id=b),
      'checkouts',(SELECT count(*) FROM checkout_registrations WHERE brain_id=b),
      'tasks',(SELECT count(*) FROM workspace_tasks WHERE brain_id=b),
      'capture_bindings',(SELECT count(*) FROM capture_bindings WHERE brain_id=b),
      'capture_events',(SELECT count(*) FROM capture_events WHERE brain_id=b),
      'capture_reports',(SELECT count(*) FROM capture_device_reports WHERE brain_id=b),
      'connections',(SELECT count(*) FROM mcp_connections WHERE brain_id=b),
      'profiles',(SELECT count(*) FROM mcp_profiles WHERE brain_id=b),
      'profile_grants',(SELECT count(*) FROM mcp_profile_grants WHERE brain_id=b),
      'calls',(SELECT count(*) FROM mcp_calls WHERE brain_id=b),
      'private_runners',(SELECT count(*) FROM mcp_private_runners WHERE brain_id=b),
      'semantic_entries',(SELECT count(*) FROM semantic_entries WHERE brain_id=b AND state<>'removed'),
      'semantic_profiles',(SELECT count(*) FROM semantic_profiles WHERE brain_id=b),
      'learning_runs',(SELECT count(*) FROM learning_runs WHERE brain_id=b),
      'handover_runs',(SELECT count(*) FROM handover_runs WHERE brain_id=b),
      'graph_generations',(SELECT count(*) FROM graph_generations WHERE brain_id=b),
      'analytics_reports',(SELECT count(*) FROM analytics_reports WHERE brain_id=b),
      'members',(SELECT count(*) FROM brain_grants WHERE brain_id=b),
      'group_members',(SELECT count(*) FROM brain_group_grants WHERE brain_id=b),
      -- Model, capture and retention policies are separate rows; the repository
      -- policy is a Brain column, so it is reported as a flag, not a count.
      'policies',(SELECT count(*) FROM (
          SELECT 1 FROM retention_policies WHERE brain_id=b
          UNION ALL SELECT 1 FROM capture_policies WHERE brain_id=b
          UNION ALL SELECT 1 FROM model_policies WHERE brain_id=b) p),
      'artifacts',jsonb_array_length(m->'artifacts')),
    'repository_content_allowed',(SELECT allow_repository_content FROM brains WHERE id=b),
    'pending_work',(SELECT count(*) FROM jobs WHERE brain_id=b AND state IN('queued','running')),
    'backup_days',coalesce((SELECT (policy->>'backup_days')::integer FROM retention_policies WHERE brain_id=b),7));
END $$;
REVOKE ALL ON FUNCTION recollect_brain_preview(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_preview(uuid) TO recollect_app;

-- Brain-wide removal runs inside the existing privacy apply boundary, so the
-- live deletion and the journal replay share one canonical path. It removes
-- every dependent relation by recorded Brain identity, never by a caller
-- supplied path or payload, and then the Brain row itself.
CREATE FUNCTION recollect_brain_remove(b uuid, rid uuid, actor uuid, at timestamptz) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE d record;
BEGIN
  PERFORM id FROM brains WHERE id=b FOR UPDATE;
  FOR d IN SELECT relation FROM brain_deletion_dependents ORDER BY ordinal LOOP
    IF to_regclass(format('public.%I',d.relation)) IS NULL THEN RAISE invalid_parameter_value; END IF;
    EXECUTE format('DELETE FROM %I WHERE brain_id=$1',d.relation) USING b;
  END LOOP;
  DELETE FROM brains WHERE id=b;
  -- The counter is the fingerprint of the retained journal manifest, which is
  -- the same document the accepted preview counter was computed from.
  INSERT INTO brain_deletions(id,brain_id,actor_id,closure,disposition,created_at)
    SELECT rid,b,actor,recollect_brain_hash(manifest),'deleted',at
    FROM privacy_requests WHERE id=rid
    ON CONFLICT(id) DO NOTHING;
  INSERT INTO brain_tombstones(brain_id,deletion_id,deleted_at) VALUES(b,rid,at)
    ON CONFLICT(brain_id) DO NOTHING;
END $$;

-- The existing apply chain redacts and cancels through each domain owner's own
-- per-class rules first; only a Brain-wide target then removes the Brain.
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_deletion_privacy_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  PERFORM recollect_pre_deletion_privacy_apply(rid);
  IF r.cause='erase' AND r.target->>'kind'='brain' THEN
    PERFORM recollect_brain_remove(r.brain_id,r.id,r.actor_id,r.created_at);
  END IF;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid),recollect_brain_remove(uuid,uuid,uuid,timestamptz) FROM PUBLIC;

-- True when this principal may see the Brain's tombstone. A principal that
-- lost access cannot distinguish "deleted" from "never existed".
CREATE FUNCTION recollect_brain_deleted(b uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT EXISTS(SELECT 1 FROM brain_deletions d WHERE d.brain_id=b
    AND (d.actor_id=recollect_actor() OR EXISTS(
      SELECT 1 FROM accounts a WHERE a.id=recollect_actor() AND a.enabled AND a.installation_owner)))
$$;
REVOKE ALL ON FUNCTION recollect_brain_deleted(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_deleted(uuid) TO recollect_app;

-- One accepted Brain-wide request. Administrator-only, browser-only, fenced by
-- the preview counter and the exact Brain-name confirmation, and never by a
-- caller supplied closure. Archived Brains are deletable. Distinct SQLstates:
-- 22023 confirmation mismatch, 40001 stale counter, insufficient_privilege for
-- no authority, NULL return for an already-absent Brain.
CREATE FUNCTION recollect_brain_delete(b uuid,expected bigint,confirmation text,rid uuid) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE name text; m jsonb; closure bigint;
BEGIN
  IF recollect_device() IS NOT NULL THEN RAISE insufficient_privilege; END IF;
  IF recollect_brain_deleted(b) THEN RETURN NULL; END IF;
  IF recollect_role(b) IS DISTINCT FROM 'admin' THEN RAISE insufficient_privilege; END IF;
  PERFORM id FROM brains WHERE id=b FOR UPDATE;
  SELECT br.name INTO name FROM brains br WHERE br.id=b;
  IF confirmation IS DISTINCT FROM name THEN
    RAISE EXCEPTION 'brain deletion confirmation mismatch' USING ERRCODE='22023';
  END IF;
  m:=recollect_brain_closure(b);
  closure:=recollect_brain_hash(m);
  IF closure IS DISTINCT FROM expected THEN
    RAISE EXCEPTION 'brain deletion preview changed' USING ERRCODE='40001';
  END IF;
  INSERT INTO privacy_requests(id,brain_id,actor_id,target,cause,manifest)
    VALUES(rid,b,recollect_actor(),jsonb_build_object('kind','brain','id',b),'erase',m);
  PERFORM recollect_privacy_apply(rid);
  -- Minimal mutation audit: actor, time, Brain identity and disposition. The
  -- Brain id rides in target_id, because a Brain-scoped audit row is itself a
  -- dependent the closure removes.
  INSERT INTO mutation_audit(id,actor_id,action,target_id,disposition)
    VALUES(gen_random_uuid(),recollect_actor(),'brain.delete',b,'deleted');
  RETURN rid;
END $$;
REVOKE ALL ON FUNCTION recollect_brain_delete(uuid,bigint,text,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_delete(uuid,bigint,text,uuid) TO recollect_app;

-- Bounded, content-free deletion status for an authorized caller. Cleanup state
-- is derived from the retained journal row, so pending physical work stays
-- visible after the Brain is gone and an error never reports success.
CREATE FUNCTION recollect_brain_deletion(b uuid,rid uuid) RETURNS jsonb
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT jsonb_build_object(
    'id',d.id,'brain_id',d.brain_id,'actor_id',d.actor_id,'closure',d.closure,
    'disposition',d.disposition,'created_at',d.created_at,'deleted_at',t.deleted_at,
    'sequence',p.sequence,'state',p.state,'journaled',p.journaled,
    'graph_pending',p.graph_pending,'error_code',p.error_code,
    'pending_artifacts',(SELECT count(*) FROM privacy_artifacts a WHERE a.request_id=p.id AND a.removed_at IS NULL),
    'acknowledged_devices',(SELECT count(*) FROM privacy_device_positions q
       WHERE q.brain_id=p.brain_id AND q.sequence>=p.sequence))
  FROM brain_deletions d
  JOIN brain_tombstones t ON t.deletion_id=d.id
  JOIN privacy_requests p ON p.id=d.id
  WHERE d.brain_id=b AND d.id=rid AND recollect_brain_deleted(b)
$$;
REVOKE ALL ON FUNCTION recollect_brain_deletion(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_deletion(uuid,uuid) TO recollect_app;
