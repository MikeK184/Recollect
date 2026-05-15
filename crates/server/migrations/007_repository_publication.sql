ALTER TABLE brains ADD COLUMN allow_repository_content boolean NOT NULL DEFAULT false;
ALTER TABLE operation_bindings ADD CONSTRAINT operation_brain_identity UNIQUE(id,brain_id);
CREATE TABLE repository_snapshots (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    repository_id uuid NOT NULL,
    revision text NOT NULL,
    adapter text NOT NULL,
    adapter_build text NOT NULL,
    extractor_version text NOT NULL,
    settings jsonb NOT NULL,
    coverage jsonb NOT NULL,
    file_count bigint NOT NULL,
    fact_count bigint NOT NULL,
    retained_file_count bigint NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id),
    FOREIGN KEY(repository_id,brain_id) REFERENCES repositories(id,brain_id)
);
CREATE INDEX repository_snapshot_revision ON repository_snapshots(repository_id,revision,created_at DESC,id);
CREATE TABLE repository_artifacts (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    snapshot_id uuid NOT NULL,
    kind text NOT NULL CHECK(kind IN ('facts','insights','receipt')),
    byte_length integer NOT NULL CHECK(byte_length BETWEEN 0 AND 10485760),
    UNIQUE(snapshot_id,kind),
    FOREIGN KEY(snapshot_id,brain_id) REFERENCES repository_snapshots(id,brain_id)
);
CREATE TABLE repository_files (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    snapshot_id uuid NOT NULL,
    path text NOT NULL,
    object_id text NOT NULL,
    mode text NOT NULL,
    size bigint,
    status text NOT NULL,
    extraction text NOT NULL,
    artifact_id uuid UNIQUE,
    byte_length integer NOT NULL CHECK(byte_length BETWEEN 0 AND 1048576),
    UNIQUE(snapshot_id,path),
    FOREIGN KEY(snapshot_id,brain_id) REFERENCES repository_snapshots(id,brain_id)
);
CREATE TABLE repository_contributions (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    snapshot_id uuid NOT NULL,
    actor_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid NOT NULL REFERENCES devices(id),
    operation_id uuid NOT NULL,
    scope jsonb NOT NULL,
    origin text NOT NULL,
    branch text,
    dirty boolean NOT NULL,
    captured_at timestamptz NOT NULL,
    accepted_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(snapshot_id,operation_id),
    FOREIGN KEY(snapshot_id,brain_id) REFERENCES repository_snapshots(id,brain_id),
    FOREIGN KEY(operation_id,brain_id) REFERENCES operation_bindings(id,brain_id)
);
CREATE TABLE repository_facts (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    snapshot_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK(ordinal>=0),
    record jsonb NOT NULL,
    UNIQUE(snapshot_id,ordinal),
    FOREIGN KEY(snapshot_id,brain_id) REFERENCES repository_snapshots(id,brain_id)
);
CREATE TABLE repository_jobs (
    job_id uuid PRIMARY KEY REFERENCES jobs(id),
    brain_id uuid NOT NULL,
    snapshot_id uuid NOT NULL,
    operation_id uuid,
    selection jsonb NOT NULL,
    FOREIGN KEY(snapshot_id,brain_id) REFERENCES repository_snapshots(id,brain_id),
    FOREIGN KEY(operation_id,brain_id) REFERENCES operation_bindings(id,brain_id)
);
CREATE TABLE revision_manifests (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    environment_id uuid NOT NULL,
    name text NOT NULL,
    current_revision uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id),
    FOREIGN KEY(environment_id,brain_id) REFERENCES evidence_groups(id,brain_id)
);
CREATE UNIQUE INDEX revision_manifest_name ON revision_manifests(environment_id,lower(name));
CREATE TABLE manifest_revisions (
    id uuid PRIMARY KEY,
    manifest_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    revision jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,manifest_id,brain_id),
    FOREIGN KEY(manifest_id,brain_id) REFERENCES revision_manifests(id,brain_id)
);
ALTER TABLE revision_manifests ADD CONSTRAINT manifest_current_revision
    FOREIGN KEY(current_revision,id,brain_id) REFERENCES manifest_revisions(id,manifest_id,brain_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX manifest_revision_history ON manifest_revisions(manifest_id,created_at DESC,id);
CREATE INDEX repository_contributor_history ON repository_contributions(snapshot_id,accepted_at DESC,id);
CREATE INDEX jobs_repository_history ON jobs(brain_id,target_id,created_at DESC,id DESC) WHERE kind='repository.process';
DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['repository_snapshots','repository_artifacts','repository_files','repository_contributions','repository_facts','repository_jobs','revision_manifests','manifest_revisions'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY publication_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY publication_insert ON %I FOR INSERT WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('GRANT SELECT,INSERT ON %I TO recollect_app',tab);
  END LOOP;
END $$;
CREATE POLICY manifest_update ON revision_manifests FOR UPDATE USING(recollect_role(brain_id) IN ('writer','admin')) WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT UPDATE(current_revision) ON revision_manifests TO recollect_app;
