CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE accounts (
    id uuid PRIMARY KEY,
    username text NOT NULL UNIQUE,
    installation_owner boolean NOT NULL DEFAULT false,
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX single_installation_owner ON accounts (installation_owner) WHERE installation_owner;

CREATE TABLE sessions (
    token uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    csrf_token uuid NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sessions_account ON sessions(account_id);

CREATE TABLE brains (
    id uuid PRIMARY KEY,
    owner_id uuid NOT NULL REFERENCES accounts(id),
    name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 120),
    description text NOT NULL DEFAULT '' CHECK (length(description) <= 2000),
    archived boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE brain_grants (
    brain_id uuid NOT NULL REFERENCES brains(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    role text NOT NULL CHECK (role IN ('reader', 'writer', 'admin')),
    PRIMARY KEY (brain_id, account_id)
);
CREATE TABLE mutation_audit (
    id uuid PRIMARY KEY,
    brain_id uuid REFERENCES brains(id),
    actor_id uuid NOT NULL REFERENCES accounts(id),
    action text NOT NULL,
    target_id uuid NOT NULL,
    disposition text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_brain_time ON mutation_audit(brain_id, created_at DESC);

CREATE FUNCTION recollect_actor() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT nullif(current_setting('recollect.actor', true), '')::uuid
$$;

CREATE FUNCTION recollect_role(target uuid) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp AS $$
    SELECT CASE WHEN b.owner_id = recollect_actor() THEN 'admin' ELSE g.role END
    FROM brains b LEFT JOIN brain_grants g
      ON g.brain_id = b.id AND g.account_id = recollect_actor()
    WHERE b.id = target
$$;
REVOKE ALL ON FUNCTION recollect_role(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_role(uuid) TO recollect_app;

ALTER TABLE brains ENABLE ROW LEVEL SECURITY;
CREATE POLICY brain_read ON brains FOR SELECT USING (recollect_role(id) IS NOT NULL);
CREATE POLICY brain_create ON brains FOR INSERT WITH CHECK (owner_id = recollect_actor());
CREATE POLICY brain_update ON brains FOR UPDATE USING (recollect_role(id) = 'admin')
    WITH CHECK (recollect_role(id) = 'admin');

ALTER TABLE brain_grants ENABLE ROW LEVEL SECURITY;
CREATE POLICY grant_read ON brain_grants FOR SELECT USING (recollect_role(brain_id) IS NOT NULL);
CREATE POLICY grant_write ON brain_grants FOR ALL USING (recollect_role(brain_id) = 'admin')
    WITH CHECK (recollect_role(brain_id) = 'admin');

ALTER TABLE mutation_audit ENABLE ROW LEVEL SECURITY;
CREATE POLICY audit_read ON mutation_audit FOR SELECT USING (recollect_role(brain_id) = 'admin');
CREATE POLICY audit_create ON mutation_audit FOR INSERT
    WITH CHECK (actor_id = recollect_actor() AND recollect_role(brain_id) IS NOT NULL);

GRANT SELECT, INSERT, UPDATE ON accounts TO recollect_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON sessions TO recollect_app;
GRANT SELECT, INSERT, UPDATE ON brains TO recollect_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON brain_grants TO recollect_app;
GRANT SELECT, INSERT ON mutation_audit TO recollect_app;
