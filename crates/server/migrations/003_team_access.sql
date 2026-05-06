ALTER TABLE accounts ADD COLUMN auth_kind text NOT NULL DEFAULT 'local' CHECK (auth_kind IN ('local','oidc'));
ALTER TABLE accounts ADD COLUMN credential_id uuid;
ALTER TABLE accounts ADD COLUMN oidc_issuer text;
ALTER TABLE accounts ADD COLUMN oidc_subject text;
ALTER TABLE accounts ADD COLUMN oidc_groups text[] NOT NULL DEFAULT '{}';
ALTER TABLE accounts ADD COLUMN membership_until timestamptz;
ALTER TABLE accounts ADD CONSTRAINT oidc_identity UNIQUE(oidc_issuer,oidc_subject);
ALTER TABLE accounts ADD CONSTRAINT account_identity_kind CHECK (
  (auth_kind='local' AND oidc_issuer IS NULL AND oidc_subject IS NULL) OR
  (auth_kind='oidc' AND oidc_issuer IS NOT NULL AND oidc_subject IS NOT NULL AND NOT installation_owner));

CREATE TABLE invitations (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    token uuid NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL DEFAULT now()+interval '24 hours',
    consumed_at timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX invitations_account ON invitations(account_id);
CREATE TABLE oidc_flows (
    state text PRIMARY KEY,
    nonce text NOT NULL,
    verifier text NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT now()+interval '5 minutes'
);
CREATE TABLE brain_group_grants (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    issuer text NOT NULL,
    group_name text NOT NULL CHECK(length(group_name) BETWEEN 1 AND 200),
    role text NOT NULL CHECK(role IN ('reader','writer','admin')),
    UNIQUE(brain_id,group_name)
);

CREATE FUNCTION recollect_effective_role(target uuid, principal uuid) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT CASE max(weight) WHEN 3 THEN 'admin' WHEN 2 THEN 'writer' WHEN 1 THEN 'reader' END
    FROM (
        SELECT 3 AS weight FROM brains WHERE id=target AND owner_id=principal
        UNION ALL
        SELECT CASE role WHEN 'admin' THEN 3 WHEN 'writer' THEN 2 ELSE 1 END
        FROM brain_grants WHERE brain_id=target AND account_id=principal
        UNION ALL
        SELECT CASE g.role WHEN 'admin' THEN 3 WHEN 'writer' THEN 2 ELSE 1 END
        FROM brain_group_grants g JOIN accounts a ON a.id=principal
        WHERE g.brain_id=target AND a.auth_kind='oidc' AND a.membership_until>now()
            AND g.issuer=a.oidc_issuer AND g.group_name=ANY(a.oidc_groups)
    ) roles WHERE EXISTS(SELECT 1 FROM accounts WHERE id=principal AND enabled)
$$;
REVOKE ALL ON FUNCTION recollect_effective_role(uuid,uuid) FROM PUBLIC;

CREATE OR REPLACE FUNCTION recollect_role(target uuid) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT recollect_effective_role(target,recollect_actor())
$$;
CREATE FUNCTION recollect_account_role(target uuid, principal uuid) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT CASE WHEN recollect_role(target)='admin' OR principal=recollect_actor()
        THEN recollect_effective_role(target,principal) END
$$;
REVOKE ALL ON FUNCTION recollect_account_role(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_account_role(uuid,uuid) TO recollect_app;

ALTER TABLE brain_group_grants ENABLE ROW LEVEL SECURITY;
CREATE POLICY group_read ON brain_group_grants FOR SELECT USING(recollect_role(brain_id)='admin');
CREATE POLICY group_write ON brain_group_grants FOR ALL USING(recollect_role(brain_id)='admin')
    WITH CHECK(recollect_role(brain_id)='admin');
CREATE POLICY system_audit_read ON mutation_audit FOR SELECT USING(brain_id IS NULL AND
    EXISTS(SELECT 1 FROM accounts WHERE id=recollect_actor() AND enabled AND installation_owner));
CREATE POLICY system_audit_create ON mutation_audit FOR INSERT WITH CHECK(brain_id IS NULL AND actor_id=recollect_actor() AND
    (EXISTS(SELECT 1 FROM accounts WHERE id=recollect_actor() AND enabled AND installation_owner)
     OR (target_id=recollect_actor() AND action IN ('account.enroll','auth.oidc'))));

GRANT SELECT,INSERT,UPDATE,DELETE ON invitations,oidc_flows,brain_group_grants TO recollect_app;
