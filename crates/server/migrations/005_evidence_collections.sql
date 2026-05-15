ALTER TABLE brains ADD COLUMN allow_document_content boolean NOT NULL DEFAULT true;

CREATE TABLE evidence_groups (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    kind text NOT NULL CHECK(kind IN ('collection','area','environment')),
    name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
    description text NOT NULL DEFAULT '' CHECK(length(description)<=2000),
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id)
);
CREATE UNIQUE INDEX evidence_group_name ON evidence_groups(brain_id,kind,lower(name));
CREATE TABLE sources (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    current_version uuid,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id)
);
CREATE TABLE source_versions (
    id uuid PRIMARY KEY,
    source_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    title text NOT NULL CHECK(length(btrim(title)) BETWEEN 1 AND 120),
    media_type text NOT NULL,
    source_uri text,
    observed_at timestamptz,
    artifact_id uuid UNIQUE,
    byte_length integer NOT NULL CHECK(byte_length BETWEEN 0 AND 1048576),
    created_by uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id) DEFAULT recollect_device(),
    created_at timestamptz NOT NULL DEFAULT now(),
    processing text NOT NULL CHECK(processing IN ('queued','ready','reference_only','missing')),
    FOREIGN KEY(source_id,brain_id) REFERENCES sources(id,brain_id),
    UNIQUE(id,brain_id),
    UNIQUE(id,source_id,brain_id)
);
ALTER TABLE sources ADD CONSTRAINT source_current_version FOREIGN KEY(current_version,id,brain_id)
    REFERENCES source_versions(id,source_id,brain_id) DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX source_history ON source_versions(source_id,created_at DESC,id);
CREATE INDEX jobs_source_history ON jobs(brain_id,target_id,created_at DESC,id DESC) WHERE kind='source.process';
CREATE TABLE evidence_memberships (
    brain_id uuid NOT NULL,
    source_id uuid NOT NULL,
    group_id uuid NOT NULL,
    PRIMARY KEY(source_id,group_id),
    FOREIGN KEY(source_id,brain_id) REFERENCES sources(id,brain_id),
    FOREIGN KEY(group_id,brain_id) REFERENCES evidence_groups(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE source_chunks (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    version_id uuid NOT NULL,
    ordinal integer NOT NULL,
    byte_start integer NOT NULL,
    byte_end integer NOT NULL CHECK(byte_end>byte_start),
    line_start integer NOT NULL,
    line_end integer NOT NULL,
    content text NOT NULL,
    FOREIGN KEY(version_id,brain_id) REFERENCES source_versions(id,brain_id),
    UNIQUE(version_id,ordinal)
);

DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['evidence_groups','sources','source_versions','evidence_memberships','source_chunks'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY evidence_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY evidence_insert ON %I FOR INSERT WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('CREATE POLICY evidence_update ON %I FOR UPDATE USING(recollect_role(brain_id) IN (''writer'',''admin'')) WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('CREATE POLICY evidence_delete ON %I FOR DELETE USING(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('GRANT SELECT,INSERT ON %I TO recollect_app',tab);
  END LOOP;
END $$;
GRANT UPDATE,DELETE ON evidence_groups,evidence_memberships,source_chunks TO recollect_app;
GRANT UPDATE(current_version,updated_at) ON sources TO recollect_app;
GRANT UPDATE(processing) ON source_versions TO recollect_app;

CREATE OR REPLACE FUNCTION recollect_fail_job(job_id uuid, token uuid, outcome text, reason text) RETURNS boolean
LANGUAGE sql SECURITY DEFINER SET search_path = public, pg_temp AS $$
    WITH changed AS (
        UPDATE jobs SET state=CASE WHEN outcome='queued' AND attempts>=max_attempts THEN 'failed' ELSE outcome END,
            error_code=reason, progress=0, lease_token=NULL, lease_until=NULL,
            not_before=clock_timestamp()+make_interval(secs=>least(attempts,2)), updated_at=now()
        WHERE id=job_id AND state='running' AND lease_token=token AND lease_until>clock_timestamp()
          AND outcome IN ('queued','failed','cancelled')
          AND reason IN ('database_error','permission_revoked','unsupported_kind','input_missing','artifact_unavailable')
        RETURNING id
    ) SELECT EXISTS(SELECT 1 FROM changed)
$$;

CREATE FUNCTION recollect_lock_brain(target uuid, exclusive boolean) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path = public, pg_temp AS $$
BEGIN
  IF recollect_role(target) IS NULL THEN RETURN false; END IF;
  IF exclusive THEN
    PERFORM id FROM brains WHERE id=target FOR UPDATE;
  ELSE
    PERFORM id FROM brains WHERE id=target FOR SHARE;
  END IF;
  RETURN FOUND;
END $$;
REVOKE ALL ON FUNCTION recollect_lock_brain(uuid,boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_lock_brain(uuid,boolean) TO recollect_app;
