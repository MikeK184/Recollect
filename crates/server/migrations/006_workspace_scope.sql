CREATE TABLE repositories (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    canonical_origin text NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id),
    UNIQUE(brain_id,canonical_origin)
);
CREATE TABLE repository_origins (
    brain_id uuid NOT NULL,
    origin text NOT NULL,
    repository_id uuid NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(brain_id,origin),
    FOREIGN KEY(repository_id,brain_id) REFERENCES repositories(id,brain_id)
);
CREATE TABLE workspace_registrations (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid NOT NULL REFERENCES devices(id),
    root text NOT NULL,
    complete boolean NOT NULL,
    notes jsonb NOT NULL,
    refreshed_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(brain_id,device_id,root),
    UNIQUE(id,brain_id,account_id,device_id),
    UNIQUE(id,brain_id,account_id)
);
CREATE TABLE checkout_registrations (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    account_id uuid NOT NULL,
    device_id uuid NOT NULL,
    repository_id uuid,
    local_path text NOT NULL,
    observation jsonb NOT NULL,
    present boolean NOT NULL DEFAULT true,
    observed_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(workspace_id,local_path),
    FOREIGN KEY(workspace_id,brain_id,account_id,device_id) REFERENCES workspace_registrations(id,brain_id,account_id,device_id),
    FOREIGN KEY(repository_id,brain_id) REFERENCES repositories(id,brain_id)
);
CREATE TABLE workspace_tasks (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    device_id uuid REFERENCES devices(id),
    parent_task_id uuid,
    workspace_id uuid,
    label text NOT NULL,
    closed boolean NOT NULL DEFAULT false,
    current_scope uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id,account_id),
    FOREIGN KEY(parent_task_id,brain_id,account_id) REFERENCES workspace_tasks(id,brain_id,account_id),
    FOREIGN KEY(workspace_id,brain_id,account_id) REFERENCES workspace_registrations(id,brain_id,account_id)
);
CREATE TABLE scope_snapshots (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    task_id uuid NOT NULL,
    account_id uuid NOT NULL,
    snapshot jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY(task_id,brain_id,account_id) REFERENCES workspace_tasks(id,brain_id,account_id),
    UNIQUE(id,task_id,brain_id,account_id)
);
ALTER TABLE workspace_tasks ADD CONSTRAINT task_current_scope
    FOREIGN KEY(current_scope,id,brain_id,account_id) REFERENCES scope_snapshots(id,task_id,brain_id,account_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE operation_bindings (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    task_id uuid NOT NULL,
    scope_id uuid NOT NULL,
    account_id uuid NOT NULL,
    device_id uuid REFERENCES devices(id),
    kind text NOT NULL CHECK(kind IN ('context','retrieval','write','capture','tool')),
    created_at timestamptz NOT NULL DEFAULT now(),
    FOREIGN KEY(scope_id,task_id,brain_id,account_id) REFERENCES scope_snapshots(id,task_id,brain_id,account_id)
);
CREATE INDEX workspace_task_owner ON workspace_tasks(brain_id,account_id,created_at DESC);
CREATE INDEX scope_task_history ON scope_snapshots(task_id,created_at DESC,id DESC);
CREATE INDEX operation_task_history ON operation_bindings(task_id,created_at DESC,id DESC);

DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['repositories','repository_origins'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY repository_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY repository_insert ON %I FOR INSERT WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin'') AND created_by=recollect_actor())',tab);
    EXECUTE format('GRANT SELECT,INSERT ON %I TO recollect_app',tab);
  END LOOP;
  FOREACH tab IN ARRAY ARRAY['workspace_registrations','checkout_registrations','workspace_tasks','scope_snapshots','operation_bindings'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY workspace_read ON %I FOR SELECT USING(account_id=recollect_actor() AND recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY workspace_insert ON %I FOR INSERT WITH CHECK(account_id=recollect_actor() AND recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('GRANT SELECT,INSERT ON %I TO recollect_app',tab);
  END LOOP;
  FOREACH tab IN ARRAY ARRAY['workspace_registrations','checkout_registrations'] LOOP
    EXECUTE format('CREATE POLICY checkout_update ON %I FOR UPDATE USING(account_id=recollect_actor() AND device_id=recollect_device() AND recollect_role(brain_id) IN (''writer'',''admin'')) WITH CHECK(account_id=recollect_actor() AND device_id=recollect_device() AND recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('CREATE POLICY checkout_insert_authority ON %I AS RESTRICTIVE FOR INSERT WITH CHECK(device_id=recollect_device() AND recollect_role(brain_id) IN (''writer'',''admin''))',tab);
  END LOOP;
END $$;
CREATE POLICY task_update ON workspace_tasks FOR UPDATE USING(account_id=recollect_actor() AND recollect_role(brain_id) IS NOT NULL)
    WITH CHECK(account_id=recollect_actor() AND recollect_role(brain_id) IS NOT NULL);
GRANT UPDATE(complete,notes,refreshed_at) ON workspace_registrations TO recollect_app;
GRANT UPDATE(repository_id,observation,present,observed_at) ON checkout_registrations TO recollect_app;
GRANT UPDATE(current_scope,closed,updated_at) ON workspace_tasks TO recollect_app;
