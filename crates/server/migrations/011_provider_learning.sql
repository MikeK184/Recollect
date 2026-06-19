CREATE TABLE model_policies (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    policy jsonb NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id)
);
CREATE TABLE model_policy_heads (
    brain_id uuid PRIMARY KEY REFERENCES brains(id),
    policy_id uuid NOT NULL,
    FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id)
);
CREATE TABLE model_requests (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    actor_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    operation_id uuid NOT NULL,
    policy_id uuid NOT NULL,
    purpose text NOT NULL CHECK(purpose IN ('extraction','synthesis','embedding','reranking')),
    model text NOT NULL,
    returned_model text,
    prompt_label text NOT NULL,
    schema_label text NOT NULL,
    state text NOT NULL CHECK(state IN ('running','succeeded','failed','uncertain')),
    error_code text,
    call_token uuid NOT NULL,
    reserved_tokens bigint NOT NULL CHECK(reserved_tokens>0),
    charged_tokens bigint NOT NULL CHECK(charged_tokens>=0),
    input_tokens bigint CHECK(input_tokens>=0),
    output_tokens bigint CHECK(output_tokens>=0),
    total_tokens bigint CHECK(total_tokens>=0),
    dimensions integer,
    suppressed boolean NOT NULL DEFAULT false,
    detail_expired boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    deadline timestamptz NOT NULL DEFAULT clock_timestamp()+interval '50 seconds',
    finished_at timestamptz,
    UNIQUE(brain_id,actor_id,operation_id,purpose),
    UNIQUE(id,brain_id),
    FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id)
);
CREATE INDEX model_request_usage ON model_requests(brain_id,created_at DESC);
CREATE TABLE model_request_inputs (
    request_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    kind text NOT NULL CHECK(kind IN ('source_version','repository_fact','claim_revision')),
    input_id uuid NOT NULL,
    PRIMARY KEY(request_id,kind,input_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
CREATE TABLE learning_runs (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    source_version_id uuid NOT NULL,
    policy_id uuid NOT NULL,
    actor_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    operation_id uuid,
    selection jsonb NOT NULL,
    manifest_revision_id uuid,
    job_id uuid NOT NULL UNIQUE REFERENCES jobs(id),
    automatic boolean NOT NULL DEFAULT false,
    state text NOT NULL CHECK(state IN ('queued','running','succeeded','failed','cancelled','removed')),
    error_code text,
    request_id uuid,
    claim_ids uuid[] NOT NULL DEFAULT '{}',
    proposed integer NOT NULL DEFAULT 0,
    accepted integer NOT NULL DEFAULT 0,
    blocked integer NOT NULL DEFAULT 0,
    conflicting integer NOT NULL DEFAULT 0,
    reused integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    finished_at timestamptz,
    UNIQUE(id,brain_id),
    FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id),
    FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
CREATE UNIQUE INDEX learning_automatic_once ON learning_runs(brain_id,source_version_id) WHERE automatic;
CREATE TABLE claim_model_derivations (
    revision_id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    run_id uuid NOT NULL,
    request_id uuid NOT NULL,
    FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
    FOREIGN KEY(run_id,brain_id) REFERENCES learning_runs(id,brain_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
CREATE TABLE model_input_fences (
    brain_id uuid NOT NULL REFERENCES brains(id),
    source_version_id uuid NOT NULL,
    request_id uuid NOT NULL,
    PRIMARY KEY(brain_id,source_version_id),
    FOREIGN KEY(request_id,brain_id) REFERENCES privacy_requests(id,brain_id)
);
ALTER TABLE model_input_fences ENABLE ROW LEVEL SECURITY;
ALTER TABLE model_input_fences FORCE ROW LEVEL SECURITY;
CREATE POLICY model_fences_read ON model_input_fences FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
GRANT SELECT ON model_input_fences TO recollect_app;
DO $$
DECLARE tab text;
BEGIN
  FOREACH tab IN ARRAY ARRAY['model_policies','model_policy_heads','model_requests','model_request_inputs','learning_runs','claim_model_derivations'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY read_rows ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('GRANT SELECT,INSERT,UPDATE ON %I TO recollect_app',tab);
    IF tab IN ('model_policies','model_policy_heads') THEN
      EXECUTE format('CREATE POLICY write_rows ON %I FOR ALL USING(recollect_role(brain_id)=''admin'' AND recollect_device() IS NULL) WITH CHECK(recollect_role(brain_id)=''admin'' AND recollect_device() IS NULL)',tab);
    ELSIF tab='model_requests' THEN
      EXECUTE format('CREATE POLICY write_rows ON %I FOR ALL USING(recollect_role(brain_id) IS NOT NULL AND actor_id=recollect_actor()) WITH CHECK(recollect_role(brain_id) IS NOT NULL AND actor_id=recollect_actor())',tab);
    ELSIF tab='model_request_inputs' THEN
      EXECUTE format('CREATE POLICY write_rows ON %I FOR ALL USING(recollect_role(brain_id) IS NOT NULL AND EXISTS(SELECT 1 FROM model_requests r WHERE r.id=request_id AND r.actor_id=recollect_actor())) WITH CHECK(recollect_role(brain_id) IS NOT NULL AND EXISTS(SELECT 1 FROM model_requests r WHERE r.id=request_id AND r.actor_id=recollect_actor()))',tab);
    ELSE
      EXECUTE format('CREATE POLICY write_rows ON %I FOR ALL USING(recollect_role(brain_id) IN (''writer'',''admin'')) WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    END IF;
  END LOOP;
END $$;

-- Result accounting belongs to a previously admitted call even after revocation.
CREATE FUNCTION recollect_finish_model_request(rid uuid,token uuid,outcome text,reason text,returned text,it bigint,ot bigint,tt bigint,dims integer)
RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  WITH changed AS (
    UPDATE model_requests SET state=outcome,error_code=reason,returned_model=returned,
      input_tokens=it,output_tokens=ot,total_tokens=tt,dimensions=dims,
      charged_tokens=coalesce(tt,reserved_tokens),finished_at=clock_timestamp()
    WHERE id=rid AND call_token=token AND state='running'
      AND outcome IN ('succeeded','failed','uncertain')
      AND (reason IS NULL OR reason IN ('provider_http','provider_timeout','provider_transport','provider_refusal','provider_incomplete','provider_shape','provider_body_limit'))
      AND (returned IS NULL OR length(returned)<=120)
      AND coalesce(it>=0,true) AND coalesce(ot>=0,true) AND coalesce(tt>=0,true)
    RETURNING id
  ) SELECT EXISTS(SELECT 1 FROM changed)
$$;
REVOKE ALL ON FUNCTION recollect_finish_model_request(uuid,uuid,text,text,text,bigint,bigint,bigint,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_finish_model_request(uuid,uuid,text,text,text,bigint,bigint,bigint,integer) TO recollect_app;
CREATE FUNCTION recollect_suppress_model_request(rid uuid,token uuid) RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  UPDATE model_requests SET suppressed=true WHERE id=rid AND call_token=token
$$;
REVOKE ALL ON FUNCTION recollect_suppress_model_request(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_suppress_model_request(uuid,uuid) TO recollect_app;

CREATE FUNCTION recollect_expire_model_requests(b uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF recollect_role(b) IS NULL THEN RAISE EXCEPTION 'model access denied'; END IF;
  UPDATE model_requests SET state='uncertain',error_code='provider_timeout',finished_at=clock_timestamp()
    WHERE brain_id=b AND state='running' AND deadline<=clock_timestamp();
END $$;
REVOKE ALL ON FUNCTION recollect_expire_model_requests(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_model_requests(uuid) TO recollect_app;
CREATE FUNCTION recollect_expire_model_details() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  UPDATE model_requests SET state='uncertain',error_code='provider_timeout',finished_at=clock_timestamp()
    WHERE id IN(SELECT id FROM model_requests WHERE state='running' AND deadline<=clock_timestamp() ORDER BY deadline LIMIT 100);
  UPDATE model_requests SET model='detail_expired',returned_model=NULL,prompt_label='detail_expired',schema_label='detail_expired',
    error_code=NULL,input_tokens=NULL,output_tokens=NULL,total_tokens=NULL,dimensions=NULL,detail_expired=true
    WHERE id IN(SELECT id FROM model_requests WHERE NOT detail_expired AND state<>'running'
      AND recollect_retention_deadline(brain_id,'audit',created_at)<=clock_timestamp() ORDER BY created_at LIMIT 100);
END $$;
REVOKE ALL ON FUNCTION recollect_expire_model_details() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_model_details() TO recollect_app;

CREATE FUNCTION recollect_learning_input_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NEW.privacy_state='active' THEN RETURN NEW; END IF;
  IF TG_TABLE_NAME='source_versions' THEN
    UPDATE learning_runs SET state='removed',error_code='content_removed',selection='{}',finished_at=clock_timestamp()
      WHERE brain_id=NEW.brain_id AND source_version_id=NEW.id;
    UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
      WHERE brain_id=NEW.brain_id AND target_id IN(SELECT id FROM learning_runs WHERE source_version_id=NEW.id)
      AND state IN ('queued','running');
    UPDATE model_requests SET suppressed=true WHERE brain_id=NEW.brain_id AND id IN
      (SELECT request_id FROM model_request_inputs WHERE brain_id=NEW.brain_id AND kind='source_version' AND input_id=NEW.id);
  ELSIF TG_TABLE_NAME='claim_revisions' THEN
    UPDATE model_requests SET suppressed=true WHERE brain_id=NEW.brain_id AND id IN
      (SELECT request_id FROM model_request_inputs WHERE brain_id=NEW.brain_id AND kind='claim_revision' AND input_id=NEW.id);
  ELSE
    UPDATE model_requests SET suppressed=true WHERE brain_id=NEW.brain_id AND id IN
      (SELECT i.request_id FROM model_request_inputs i JOIN repository_facts f ON f.id=i.input_id AND f.brain_id=i.brain_id
       WHERE i.brain_id=NEW.brain_id AND i.kind='repository_fact' AND f.snapshot_id=NEW.id);
  END IF;
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_learning_input_removed() FROM PUBLIC;
CREATE TRIGGER source_learning_removed AFTER UPDATE OF privacy_state ON source_versions FOR EACH ROW EXECUTE FUNCTION recollect_learning_input_removed();
CREATE TRIGGER claim_model_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW EXECUTE FUNCTION recollect_learning_input_removed();
CREATE TRIGGER snapshot_model_removed AFTER UPDATE OF privacy_state ON repository_snapshots FOR EACH ROW EXECUTE FUNCTION recollect_learning_input_removed();

CREATE FUNCTION recollect_fail_learning_job(jid uuid,token uuid,reason text) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE target uuid;
BEGIN
  IF length(reason)>80 OR reason !~ '^[a-z_]+$' THEN RAISE EXCEPTION 'invalid learning failure'; END IF;
  UPDATE jobs SET state='failed',error_code=reason,progress=0,lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
    WHERE id=jid AND kind='source.learn' AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
    RETURNING target_id INTO target;
  IF target IS NOT NULL THEN
    UPDATE learning_runs SET state='failed',error_code=reason,finished_at=clock_timestamp(),
      request_id=(SELECT id FROM model_requests WHERE brain_id=learning_runs.brain_id AND actor_id=learning_runs.actor_id AND operation_id=learning_runs.id AND purpose='extraction')
      WHERE id=target AND state='failed';
  END IF;
END $$;
REVOKE ALL ON FUNCTION recollect_fail_learning_job(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_fail_learning_job(uuid,uuid,text) TO recollect_app;

CREATE FUNCTION recollect_learning_job_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
  IF NEW.kind='source.learn' AND NEW.state IN ('cancelled','failed') THEN
    UPDATE learning_runs SET state=NEW.state,error_code=NEW.error_code,finished_at=clock_timestamp()
      WHERE job_id=NEW.id AND state IN ('queued','running');
  END IF;
  RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_learning_job_state() FROM PUBLIC;
CREATE TRIGGER learning_job_state AFTER UPDATE OF state ON jobs FOR EACH ROW EXECUTE FUNCTION recollect_learning_job_state();

-- Extend the minimal journal closure; original evidence inputs remain retained.
ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_evidence_privacy_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; sources uuid[]; work uuid[];
BEGIN
  m:=recollect_evidence_privacy_closure(b,k,t,c);
  IF c='erase' THEN
    SELECT coalesce(array_agg(DISTINCT source_version_id ORDER BY source_version_id),'{}') INTO sources
      FROM claim_supports WHERE brain_id=b AND revision_id=ANY(recollect_privacy_ids(m,'claim_revisions')) AND source_version_id IS NOT NULL;
    IF cardinality(sources)>0 THEN m:=m||jsonb_build_object('model_input_sources',sources); END IF;
  END IF;
  SELECT coalesce(array_agg(DISTINCT id ORDER BY id),'{}') INTO work FROM (
    SELECT unnest(recollect_privacy_ids(m,'jobs')) AS id
    UNION
    SELECT j.id FROM jobs j JOIN learning_runs r ON r.job_id=j.id
      WHERE r.brain_id=b AND r.source_version_id=ANY(coalesce(sources,'{}')||recollect_privacy_ids(m,'source_versions')) AND j.state IN ('queued','running')
  ) x;
  RETURN jsonb_set(m,'{jobs}',to_jsonb(work));
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_evidence_privacy_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests; sources uuid[];
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
  sources:=recollect_privacy_ids(r.manifest,'model_input_sources');
  IF r.cause='erase' THEN
    SELECT coalesce(array_agg(DISTINCT id),'{}') INTO sources FROM (
      SELECT unnest(sources) AS id
      UNION
      SELECT source_version_id FROM claim_supports WHERE brain_id=r.brain_id
        AND revision_id=ANY(recollect_privacy_ids(r.manifest,'claim_revisions')) AND source_version_id IS NOT NULL
    ) s;
    INSERT INTO model_input_fences(brain_id,source_version_id,request_id)
      SELECT r.brain_id,unnest(sources),r.id ON CONFLICT DO NOTHING;
    UPDATE learning_runs SET state='removed',error_code='model_input_fenced',selection='{}',finished_at=clock_timestamp()
      WHERE brain_id=r.brain_id AND source_version_id=ANY(sources) AND state IN ('queued','running');
    UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
      WHERE brain_id=r.brain_id AND target_id IN(SELECT id FROM learning_runs WHERE brain_id=r.brain_id AND source_version_id=ANY(sources))
      AND state IN ('queued','running');
    UPDATE model_requests SET suppressed=true WHERE brain_id=r.brain_id AND id IN
      (SELECT request_id FROM model_request_inputs WHERE brain_id=r.brain_id AND kind='source_version' AND input_id=ANY(sources));
  END IF;
  PERFORM recollect_evidence_privacy_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid) FROM PUBLIC;
-- Existing local erasures can be upgraded from their retained opaque support links.
INSERT INTO model_input_fences(brain_id,source_version_id,request_id)
SELECT DISTINCT ON(s.brain_id,s.source_version_id) s.brain_id,s.source_version_id,p.id
  FROM claim_supports s JOIN claim_revisions c ON c.id=s.revision_id AND c.brain_id=s.brain_id
  JOIN privacy_requests p ON p.brain_id=s.brain_id AND p.cause='erase' AND s.revision_id=ANY(recollect_privacy_ids(p.manifest,'claim_revisions'))
  WHERE c.privacy_state='erased' AND s.source_version_id IS NOT NULL
  ORDER BY s.brain_id,s.source_version_id,p.sequence DESC
ON CONFLICT DO NOTHING;
