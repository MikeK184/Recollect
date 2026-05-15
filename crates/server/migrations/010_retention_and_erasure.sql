CREATE TABLE retention_policies (
    brain_id uuid PRIMARY KEY REFERENCES brains(id),
    change_id uuid NOT NULL,
    policy jsonb NOT NULL,
    updated_by uuid NOT NULL REFERENCES accounts(id),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
ALTER TABLE retention_policies ENABLE ROW LEVEL SECURITY;
CREATE POLICY retention_read ON retention_policies FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY retention_write ON retention_policies FOR ALL USING(recollect_role(brain_id)='admin') WITH CHECK(recollect_role(brain_id)='admin');
GRANT SELECT,INSERT,UPDATE ON retention_policies TO recollect_app;

ALTER TABLE sources ADD COLUMN retention_class text NOT NULL DEFAULT 'document' CHECK(retention_class IN ('document','raw_session','tool_output','support_excerpt'));
ALTER TABLE source_versions ADD COLUMN retention_class text NOT NULL DEFAULT 'document' CHECK(retention_class IN ('document','raw_session','tool_output','support_excerpt'));
ALTER TABLE source_versions ADD COLUMN privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased'));
ALTER TABLE source_versions ADD COLUMN removed_at timestamptz;
ALTER TABLE claim_revisions ADD COLUMN privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased'));
ALTER TABLE claim_revisions ADD COLUMN removed_at timestamptz;
ALTER TABLE repository_snapshots ADD COLUMN privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased'));
ALTER TABLE repository_snapshots ADD COLUMN removed_at timestamptz;
ALTER TABLE manifest_revisions ADD COLUMN privacy_state text NOT NULL DEFAULT 'active' CHECK(privacy_state IN ('active','expired','erased'));
ALTER TABLE manifest_revisions ADD COLUMN removed_at timestamptz;
ALTER TABLE memory_decisions ADD COLUMN payload_erased boolean NOT NULL DEFAULT false;
ALTER TABLE assertion_rules ADD COLUMN payload_erased boolean NOT NULL DEFAULT false;
ALTER TABLE command_receipts ADD COLUMN invalidated boolean NOT NULL DEFAULT false;

CREATE TABLE source_excerpts (
    brain_id uuid NOT NULL,
    version_id uuid PRIMARY KEY,
    parent_source_id uuid NOT NULL,
    parent_version_id uuid NOT NULL,
    first_line integer NOT NULL CHECK(first_line>0),
    last_line integer NOT NULL CHECK(last_line>=first_line),
    captured_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(version_id,brain_id) REFERENCES source_versions(id,brain_id),
    FOREIGN KEY(parent_source_id,brain_id) REFERENCES sources(id,brain_id),
    FOREIGN KEY(parent_version_id,brain_id) REFERENCES source_versions(id,brain_id)
);
ALTER TABLE source_excerpts ENABLE ROW LEVEL SECURITY;
CREATE POLICY excerpt_read ON source_excerpts FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY excerpt_write ON source_excerpts FOR INSERT WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT ON source_excerpts TO recollect_app;

CREATE TABLE privacy_installation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
    id uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    initialized boolean NOT NULL DEFAULT false
);
INSERT INTO privacy_installation(singleton) VALUES(true);
GRANT SELECT ON privacy_installation TO recollect_app;

CREATE TABLE privacy_requests (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    sequence bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    actor_id uuid REFERENCES accounts(id),
    target jsonb NOT NULL,
    cause text NOT NULL CHECK(cause IN ('erase','expire')),
    manifest jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','error','complete')),
    journaled boolean NOT NULL DEFAULT false,
    error_code text,
    UNIQUE(id,brain_id)
);
CREATE TABLE privacy_artifacts (
    request_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    artifact_id uuid NOT NULL,
    removed_at timestamptz,
    PRIMARY KEY(request_id,artifact_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES privacy_requests(id,brain_id)
);
CREATE TABLE privacy_publication_fences (
    brain_id uuid NOT NULL,
    repository_id uuid NOT NULL,
    revision text NOT NULL,
    request_id uuid NOT NULL,
    PRIMARY KEY(repository_id,revision),
    FOREIGN KEY(request_id,brain_id) REFERENCES privacy_requests(id,brain_id)
);
CREATE TABLE privacy_device_positions (
    brain_id uuid NOT NULL REFERENCES brains(id),
    device_id uuid NOT NULL REFERENCES devices(id),
    sequence bigint NOT NULL CHECK(sequence>=0),
    checked_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(brain_id,device_id)
);
DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['privacy_requests','privacy_artifacts','privacy_publication_fences','privacy_device_positions'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY privacy_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('GRANT SELECT ON %I TO recollect_app',tab);
  END LOOP;
END $$;

CREATE FUNCTION recollect_retention_days(target uuid, content_class text) RETURNS integer
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
  SELECT CASE content_class
    WHEN 'raw_session' THEN coalesce((p.policy->>'raw_session_days')::integer,30)
    WHEN 'tool_output' THEN coalesce((p.policy->>'tool_output_days')::integer,30)
    WHEN 'document' THEN (p.policy->>'document_days')::integer
    WHEN 'support_excerpt' THEN (p.policy->>'support_excerpt_days')::integer
    WHEN 'repository' THEN (p.policy->>'repository_days')::integer
    WHEN 'claim' THEN (p.policy->>'claim_days')::integer
    WHEN 'audit' THEN coalesce((p.policy->>'audit_days')::integer,365)
    ELSE NULL END
  FROM (SELECT 1) seed LEFT JOIN retention_policies p ON p.brain_id=target
$$;
CREATE FUNCTION recollect_retention_deadline(target uuid, content_class text, captured timestamptz) RETURNS timestamptz
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
  SELECT captured + recollect_retention_days(target,content_class) * interval '24 hours'
$$;
REVOKE ALL ON FUNCTION recollect_retention_days(uuid,text),recollect_retention_deadline(uuid,text,timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_retention_days(uuid,text),recollect_retention_deadline(uuid,text,timestamptz) TO recollect_app;

CREATE FUNCTION recollect_content_state(target uuid,content_class text,recorded_state text,captured timestamptz) RETURNS text
LANGUAGE sql SET search_path=public,pg_temp AS $$
  SELECT CASE WHEN recorded_state='active' AND recollect_retention_deadline(target,content_class,captured)<=clock_timestamp() THEN 'expired' ELSE recorded_state END
$$;
REVOKE ALL ON FUNCTION recollect_content_state(uuid,text,text,timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_content_state(uuid,text,text,timestamptz) TO recollect_app;

CREATE FUNCTION recollect_invalidate_receipts(target uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(target) IS DISTINCT FROM 'admin' THEN RAISE insufficient_privilege; END IF;
  UPDATE command_receipts SET input='{}',response=NULL,invalidated=true WHERE brain_id=target;
END $$;
REVOKE ALL ON FUNCTION recollect_invalidate_receipts(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_invalidate_receipts(uuid) TO recollect_app;

-- Closure and redaction helpers are private to their authenticated/admin callers.
-- The journal stores only these exact identities, so replay does not need old payloads.
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
WITH RECURSIVE
versions(id) AS (
  SELECT v.id FROM source_versions v WHERE v.brain_id=b AND
    ((k='source' AND v.source_id=t) OR (k='source_version' AND v.id=t) OR
     (k='collection' AND EXISTS(SELECT 1 FROM evidence_memberships m WHERE m.brain_id=b AND m.group_id=t AND m.source_id=v.source_id)))
  UNION
  SELECT e.version_id FROM source_excerpts e JOIN versions p ON p.id=e.parent_version_id WHERE e.brain_id=b AND c='erase'
),
snapshots AS (SELECT id FROM repository_snapshots WHERE brain_id=b AND k='snapshot' AND id=t),
facts AS (SELECT id FROM repository_facts WHERE brain_id=b AND snapshot_id IN(SELECT id FROM snapshots)),
manifests AS (
  SELECT id FROM manifest_revisions r WHERE r.brain_id=b AND
    ((k='manifest' AND r.manifest_id=t) OR (c='erase' AND EXISTS(
      SELECT 1 FROM jsonb_array_elements(coalesce(r.revision->'entries','[]')) e WHERE (e->>'snapshot_id')::uuid IN(SELECT id FROM snapshots))))
),
revisions AS (
  SELECT id FROM claim_revisions r WHERE r.brain_id=b AND
    ((k='claim' AND r.claim_id=t) OR (k='claim_revision' AND r.id=t) OR (c='erase' AND (
      EXISTS(SELECT 1 FROM claim_supports s WHERE s.brain_id=b AND s.revision_id=r.id AND
        (s.source_version_id IN(SELECT id FROM versions) OR s.fact_id IN(SELECT id FROM facts) OR s.manifest_revision_id IN(SELECT id FROM manifests)))
      OR (r.revision#>>'{content,manifest_revision_id}')::uuid IN(SELECT id FROM manifests))))
),
rules AS (
  SELECT id,decision_id FROM assertion_rules a WHERE a.brain_id=b AND c='erase' AND (
    (a.rule->>'revision_id')::uuid IN(SELECT id FROM revisions)
    OR (a.rule#>>'{content,manifest_revision_id}')::uuid IN(SELECT id FROM manifests)
    OR EXISTS(SELECT 1 FROM jsonb_array_elements(coalesce(a.rule#>'{content,supports}','[]')) s WHERE
      ((s->>'kind'='source_version' AND (s->>'id')::uuid IN(SELECT id FROM versions))
       OR (s->>'kind'='repository_fact' AND (s->>'id')::uuid IN(SELECT id FROM facts))
       OR (s->>'kind'='manifest_revision' AND (s->>'id')::uuid IN(SELECT id FROM manifests)))))
),
decisions AS (
  SELECT id FROM memory_decisions d WHERE d.brain_id=b AND c='erase' AND
    (d.id IN(SELECT decision_id FROM rules) OR EXISTS(
      SELECT 1 FROM jsonb_array_elements(coalesce(d.decision->'transitions','[]')) tr
      WHERE (tr->>'before_revision')::uuid IN(SELECT id FROM revisions) OR (tr->>'after_revision')::uuid IN(SELECT id FROM revisions)))
),
artifacts AS (
  SELECT artifact_id AS id FROM source_versions WHERE brain_id=b AND id IN(SELECT id FROM versions) AND artifact_id IS NOT NULL
  UNION SELECT id FROM repository_artifacts WHERE brain_id=b AND snapshot_id IN(SELECT id FROM snapshots)
  UNION SELECT artifact_id FROM repository_files WHERE brain_id=b AND snapshot_id IN(SELECT id FROM snapshots) AND artifact_id IS NOT NULL
),
work AS (SELECT id FROM jobs WHERE brain_id=b AND target_id IN(SELECT id FROM versions UNION SELECT id FROM snapshots) AND state IN('queued','running')),
independent AS (SELECT count(*) AS n FROM claim_revisions r WHERE brain_id=b AND privacy_state='active' AND id NOT IN(SELECT id FROM revisions)
  AND claim_id IN(SELECT claim_id FROM claim_revisions WHERE id IN(SELECT id FROM revisions)))
SELECT jsonb_build_object(
  'source_versions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM versions),'[]'),
  'source_ids',coalesce((SELECT jsonb_agg(DISTINCT source_id) FROM source_versions WHERE brain_id=b AND id IN(SELECT id FROM versions)),'[]'),
  'claim_revisions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM revisions),'[]'),
  'claim_ids',coalesce((SELECT jsonb_agg(DISTINCT claim_id) FROM claim_revisions WHERE brain_id=b AND id IN(SELECT id FROM revisions)),'[]'),
  'snapshots',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM snapshots),'[]'),
  'facts',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM facts),'[]'),
  'manifest_revisions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM manifests),'[]'),
  'rules',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM rules),'[]'),
  'decisions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM decisions),'[]'),
  'artifacts',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM artifacts),'[]'),
  'jobs',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM work),'[]'),
  'publication_fences',coalesce((SELECT jsonb_agg(jsonb_build_object('repository_id',repository_id,'revision',revision) ORDER BY id) FROM repository_snapshots WHERE brain_id=b AND id IN(SELECT id FROM snapshots)),'[]'),
  'independent_claim_revisions',(SELECT n FROM independent),
  'shared_sources',k='collection' AND EXISTS(SELECT 1 FROM evidence_memberships WHERE brain_id=b AND source_id IN(SELECT source_id FROM source_versions WHERE id IN(SELECT id FROM versions)) AND group_id<>t))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

CREATE FUNCTION recollect_privacy_target(b uuid,k text,t uuid) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT CASE k
 WHEN 'source' THEN EXISTS(SELECT 1 FROM sources WHERE brain_id=b AND id=t)
 WHEN 'claim' THEN EXISTS(SELECT 1 FROM claims WHERE brain_id=b AND id=t)
 WHEN 'snapshot' THEN EXISTS(SELECT 1 FROM repository_snapshots WHERE brain_id=b AND id=t)
 WHEN 'manifest' THEN EXISTS(SELECT 1 FROM revision_manifests WHERE brain_id=b AND id=t)
 WHEN 'collection' THEN EXISTS(SELECT 1 FROM evidence_groups WHERE brain_id=b AND id=t AND kind='collection')
 ELSE false END
$$;
REVOKE ALL ON FUNCTION recollect_privacy_target(uuid,text,uuid) FROM PUBLIC;
CREATE FUNCTION recollect_privacy_preview(b uuid,k text,t uuid) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF recollect_role(b) IS DISTINCT FROM 'admin' OR recollect_device() IS NOT NULL THEN RAISE insufficient_privilege; END IF;
 IF NOT recollect_privacy_target(b,k,t) THEN RETURN NULL; END IF;
 RETURN recollect_privacy_closure(b,k,t,'erase');
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_preview(uuid,text,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_privacy_preview(uuid,text,uuid) TO recollect_app;

CREATE FUNCTION recollect_privacy_ids(m jsonb,k text) RETURNS uuid[]
LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
 SELECT coalesce(array_agg(value::uuid),'{}'::uuid[]) FROM jsonb_array_elements_text(coalesce(m->k,'[]'))
$$;
REVOKE ALL ON FUNCTION recollect_privacy_ids(jsonb,text) FROM PUBLIC;

CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; v uuid[]; cr uuid[]; sn uuid[]; mr uuid[]; removal_state text;
BEGIN
 SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
 PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
 v:=recollect_privacy_ids(r.manifest,'source_versions'); cr:=recollect_privacy_ids(r.manifest,'claim_revisions');
 sn:=recollect_privacy_ids(r.manifest,'snapshots'); mr:=recollect_privacy_ids(r.manifest,'manifest_revisions');
 removal_state:=CASE WHEN r.cause='erase' THEN 'erased' ELSE 'expired' END;
 INSERT INTO privacy_artifacts(request_id,brain_id,artifact_id)
 SELECT r.id,r.brain_id,unnest(recollect_privacy_ids(r.manifest,'artifacts')) ON CONFLICT DO NOTHING;
 INSERT INTO privacy_publication_fences(brain_id,repository_id,revision,request_id)
 SELECT r.brain_id,(f->>'repository_id')::uuid,f->>'revision',r.id FROM jsonb_array_elements(r.manifest->'publication_fences') f
 ON CONFLICT DO NOTHING;
 -- Identity fences persist on canonical rows. No old revision is promoted.
 UPDATE source_versions SET title=initcap(removal_state)||' source',source_uri=NULL,observed_at=NULL,artifact_id=NULL,
   byte_length=0,media_type='text/plain',processing='missing',privacy_state=CASE WHEN privacy_state='erased' THEN 'erased' ELSE removal_state END,removed_at=coalesce(removed_at,r.created_at)
   WHERE brain_id=r.brain_id AND id=ANY(v);
 DELETE FROM source_chunks WHERE brain_id=r.brain_id AND version_id=ANY(v);
 UPDATE claim_revisions SET revision='{}',subject_key='',predicate_key='',value_key='',
   privacy_state=CASE WHEN privacy_state='erased' THEN 'erased' ELSE removal_state END,removed_at=coalesce(removed_at,r.created_at)
   WHERE brain_id=r.brain_id AND id=ANY(cr);
 UPDATE repository_snapshots SET adapter='',adapter_build='',extractor_version='',settings='{}',coverage='{}',
   file_count=0,fact_count=0,retained_file_count=0,privacy_state=CASE WHEN privacy_state='erased' THEN 'erased' ELSE removal_state END,removed_at=coalesce(removed_at,r.created_at)
   WHERE brain_id=r.brain_id AND id=ANY(sn);
 UPDATE repository_facts SET record='{}' WHERE brain_id=r.brain_id AND snapshot_id=ANY(sn);
 DELETE FROM repository_files WHERE brain_id=r.brain_id AND snapshot_id=ANY(sn);
 DELETE FROM repository_artifacts WHERE brain_id=r.brain_id AND snapshot_id=ANY(sn);
 UPDATE repository_contributions SET scope='{}',origin='',branch=NULL,dirty=false WHERE brain_id=r.brain_id AND snapshot_id=ANY(sn);
 UPDATE repository_jobs SET selection='{}' WHERE brain_id=r.brain_id AND snapshot_id=ANY(sn);
 UPDATE manifest_revisions SET revision='{}',privacy_state=CASE WHEN privacy_state='erased' THEN 'erased' ELSE removal_state END,removed_at=coalesce(removed_at,r.created_at)
   WHERE brain_id=r.brain_id AND id=ANY(mr);
 UPDATE revision_manifests SET name='Erased manifest '||id::text WHERE brain_id=r.brain_id AND current_revision=ANY(mr);
 UPDATE assertion_rules SET rule='{}',subject_key='',predicate_key='',value_key='',payload_erased=true
   WHERE brain_id=r.brain_id AND id=ANY(recollect_privacy_ids(r.manifest,'rules'));
 -- Preserve attribution and opaque transitions; remove all free-form review content.
 UPDATE memory_decisions SET decision=jsonb_set(jsonb_set(jsonb_set(decision,'{reason}','""'),'{actor_name}','""'),'{revalidation_basis}','null'),payload_erased=true
   WHERE brain_id=r.brain_id AND id=ANY(recollect_privacy_ids(r.manifest,'decisions'));
 UPDATE command_receipts SET input='{}',response=NULL,invalidated=true WHERE brain_id=r.brain_id;
 UPDATE jobs SET state='cancelled',progress=0,error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
   WHERE brain_id=r.brain_id AND target_id IN(SELECT unnest(v) UNION SELECT unnest(sn)) AND state IN('queued','running');
 INSERT INTO memory_epochs(brain_id,epoch) VALUES(r.brain_id,1) ON CONFLICT(brain_id) DO UPDATE SET epoch=memory_epochs.epoch+1;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;

CREATE FUNCTION recollect_erase(b uuid,k text,t uuid,expected_epoch bigint,rid uuid) RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; epoch bigint;
BEGIN
 IF recollect_role(b) IS DISTINCT FROM 'admin' OR recollect_device() IS NOT NULL THEN RAISE insufficient_privilege; END IF;
 PERFORM id FROM brains WHERE id=b FOR UPDATE;
 IF NOT recollect_privacy_target(b,k,t) THEN RETURN NULL; END IF;
 SELECT coalesce((SELECT e.epoch FROM memory_epochs e WHERE brain_id=b),0) INTO epoch;
 IF epoch<>expected_epoch THEN RAISE EXCEPTION 'privacy preview changed' USING ERRCODE='40001'; END IF;
 m:=recollect_privacy_closure(b,k,t,'erase');
 INSERT INTO privacy_requests(id,brain_id,actor_id,target,cause,manifest) VALUES(rid,b,recollect_actor(),jsonb_build_object('kind',k,'id',t),'erase',m);
 PERFORM recollect_privacy_apply(rid);
 INSERT INTO mutation_audit(id,brain_id,actor_id,action,target_id,disposition) VALUES(gen_random_uuid(),b,recollect_actor(),'memory.erase',rid,'cleanup_pending');
 RETURN rid;
END $$;
REVOKE ALL ON FUNCTION recollect_erase(uuid,text,uuid,bigint,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_erase(uuid,text,uuid,bigint,uuid) TO recollect_app;

-- System entry point: it can select only records that are due under canonical policy.
-- It accepts no caller-supplied identities or payloads.
CREATE FUNCTION recollect_expire_one() RETURNS uuid
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE candidate record; rid uuid; current_state text;
BEGIN
 SELECT * INTO candidate FROM (
   SELECT brain_id,id,'source_version'::text AS kind,recollect_retention_deadline(brain_id,retention_class,created_at) AS deadline
     FROM source_versions WHERE privacy_state='active'
   UNION ALL SELECT brain_id,id,'snapshot',recollect_retention_deadline(brain_id,'repository',created_at)
     FROM repository_snapshots WHERE privacy_state='active'
   UNION ALL SELECT brain_id,id,'claim_revision',recollect_retention_deadline(brain_id,'claim',recorded_at)
     FROM claim_revisions WHERE privacy_state='active'
 ) due WHERE deadline<=clock_timestamp() ORDER BY deadline,id LIMIT 1;
 IF NOT FOUND THEN RETURN NULL; END IF;
 PERFORM id FROM brains WHERE id=candidate.brain_id FOR UPDATE SKIP LOCKED;
 IF NOT FOUND THEN RETURN NULL; END IF;
 CASE candidate.kind
   WHEN 'source_version' THEN SELECT recollect_content_state(brain_id,retention_class,privacy_state,created_at) INTO current_state FROM source_versions WHERE id=candidate.id;
   WHEN 'snapshot' THEN SELECT recollect_content_state(brain_id,'repository',privacy_state,created_at) INTO current_state FROM repository_snapshots WHERE id=candidate.id;
   WHEN 'claim_revision' THEN SELECT recollect_content_state(brain_id,'claim',privacy_state,recorded_at) INTO current_state FROM claim_revisions WHERE id=candidate.id;
 END CASE;
 IF current_state<>'expired' THEN RETURN NULL; END IF;
 -- A second worker may have completed this expiry while the first was selecting it.
 IF EXISTS(SELECT 1 FROM privacy_requests WHERE brain_id=candidate.brain_id AND cause='expire' AND target=jsonb_build_object('kind',candidate.kind,'id',candidate.id)) THEN RETURN NULL; END IF;
 rid:=gen_random_uuid();
 INSERT INTO privacy_requests(id,brain_id,target,cause,manifest)
 VALUES(rid,candidate.brain_id,jsonb_build_object('kind',candidate.kind,'id',candidate.id),'expire',
        recollect_privacy_closure(candidate.brain_id,candidate.kind,candidate.id,'expire'));
 PERFORM recollect_privacy_apply(rid);
 RETURN rid;
END $$;
REVOKE ALL ON FUNCTION recollect_expire_one() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_one() TO recollect_app;

-- Minimal canonical work enumeration survives the initiating principal's revocation.
CREATE FUNCTION recollect_privacy_pending() RETURNS SETOF privacy_requests
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT * FROM privacy_requests WHERE state<>'complete' ORDER BY sequence LIMIT 20
$$;
CREATE FUNCTION recollect_privacy_progress(rid uuid,exported boolean,removed uuid[],failure text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF failure IS NOT NULL AND failure NOT IN('journal_unavailable','artifact_unavailable') THEN RAISE invalid_parameter_value; END IF;
 UPDATE privacy_artifacts SET removed_at=clock_timestamp() WHERE request_id=rid AND artifact_id=ANY(removed) AND removed_at IS NULL;
 UPDATE privacy_requests SET journaled=journaled OR exported,error_code=failure,
 state=CASE WHEN failure IS NOT NULL THEN 'error'
            WHEN (journaled OR exported) AND NOT EXISTS(SELECT 1 FROM privacy_artifacts WHERE request_id=rid AND removed_at IS NULL) THEN 'complete'
            ELSE 'pending' END WHERE id=rid;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_pending(),recollect_privacy_progress(uuid,boolean,uuid[],text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_privacy_pending(),recollect_privacy_progress(uuid,boolean,uuid[],text) TO recollect_app;

-- Entries whose Brain did not exist in an older backup still advance the restore barrier.
CREATE TABLE privacy_replayed_entries(id uuid PRIMARY KEY,sequence bigint NOT NULL UNIQUE);
CREATE FUNCTION recollect_privacy_positions() RETURNS TABLE(id uuid,sequence bigint,journaled boolean)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT id,sequence,journaled FROM privacy_requests
 UNION ALL SELECT id,sequence,true FROM privacy_replayed_entries
$$;
REVOKE ALL ON FUNCTION recollect_privacy_positions() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_privacy_positions() TO recollect_app;

-- Only the migration/operator database role may introduce a journal entry.
-- The application role may process committed canonical requests, never supplied closures.
CREATE FUNCTION recollect_privacy_replay(entry jsonb) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE rid uuid; b uuid; position bigint; found_entry privacy_requests;
BEGIN
 IF (entry->>'installation_id')::uuid IS DISTINCT FROM (SELECT id FROM privacy_installation) THEN RAISE invalid_parameter_value; END IF;
 rid:=(entry->>'id')::uuid; b:=(entry->>'brain_id')::uuid;position:=(entry->>'sequence')::bigint;
 SELECT * INTO found_entry FROM privacy_requests WHERE id=rid;
 IF FOUND THEN
   IF found_entry.brain_id<>b OR found_entry.sequence<>position OR found_entry.manifest<>entry->'manifest' THEN RAISE invalid_parameter_value; END IF;
   RETURN;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM brains WHERE id=b) THEN
   INSERT INTO privacy_replayed_entries(id,sequence) VALUES(rid,position) ON CONFLICT(id) DO NOTHING;
   PERFORM setval(pg_get_serial_sequence('privacy_requests','sequence'),greatest(position,(SELECT coalesce(max(sequence),1) FROM privacy_requests),(SELECT coalesce(max(sequence),1) FROM privacy_replayed_entries)),true);
   RETURN;
 END IF;
 PERFORM id FROM brains WHERE id=b FOR UPDATE;
 INSERT INTO privacy_requests(id,brain_id,sequence,target,cause,manifest,created_at,journaled) OVERRIDING SYSTEM VALUE
 VALUES(rid,b,position,entry->'target',entry->>'cause',entry->'manifest',(entry->>'created_at')::timestamptz,true);
 PERFORM recollect_privacy_apply(rid);
 PERFORM setval(pg_get_serial_sequence('privacy_requests','sequence'),greatest(position,(SELECT coalesce(max(sequence),1) FROM privacy_requests)),true);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_replay(jsonb) FROM PUBLIC;

CREATE FUNCTION recollect_expire_audit() RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 DELETE FROM mutation_audit WHERE id IN(
   SELECT a.id FROM mutation_audit a WHERE a.brain_id IS NOT NULL
     AND recollect_retention_deadline(a.brain_id,'audit',a.created_at)<=clock_timestamp()
     AND NOT EXISTS(SELECT 1 FROM jobs WHERE audit_id=a.id) ORDER BY a.created_at LIMIT 100)
$$;
REVOKE ALL ON FUNCTION recollect_expire_audit() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_audit() TO recollect_app;

CREATE FUNCTION recollect_privacy_device_ack(b uuid,p_position bigint) RETURNS SETOF privacy_device_positions
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 IF recollect_device() IS NULL OR recollect_role(b) IS NULL THEN RAISE insufficient_privilege; END IF;
 IF p_position<0 OR p_position>coalesce((SELECT max(sequence) FROM privacy_requests WHERE brain_id=b),0) THEN RAISE invalid_parameter_value; END IF;
 RETURN QUERY INSERT INTO privacy_device_positions(brain_id,device_id,sequence) VALUES(b,recollect_device(),p_position)
 ON CONFLICT(brain_id,device_id) DO UPDATE SET sequence=greatest(privacy_device_positions.sequence,excluded.sequence),checked_at=clock_timestamp()
 RETURNING *;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_device_ack(uuid,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_privacy_device_ack(uuid,bigint) TO recollect_app;
