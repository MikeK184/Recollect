CREATE TABLE mcp_definitions (
    key text PRIMARY KEY CHECK(length(key) BETWEEN 1 AND 64),
    id uuid NOT NULL UNIQUE,
    manifest jsonb NOT NULL CHECK(jsonb_typeof(manifest)='object'),
    enabled boolean NOT NULL DEFAULT true,
    approved_by uuid NOT NULL REFERENCES accounts(id),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
-- Only the operator/migration principal can approve implementations.
GRANT SELECT ON mcp_definitions TO recollect_app;

CREATE TABLE mcp_connections (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
    description text NOT NULL DEFAULT '' CHECK(length(description)<=2000),
    definition_key text NOT NULL REFERENCES mcp_definitions(key),
    environment_id uuid,
    target text NOT NULL CHECK(length(target) BETWEEN 1 AND 2048),
    placement text NOT NULL CHECK(placement IN ('central','local','private')),
    runner_reference text,
    credential_alias text,
    configuration jsonb NOT NULL CHECK(jsonb_typeof(configuration)='object'),
    enabled boolean NOT NULL DEFAULT true,
    revision uuid NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id),
    FOREIGN KEY(environment_id,brain_id) REFERENCES evidence_groups(id,brain_id),
    CHECK ((placement='central' AND runner_reference IS NULL) OR
           (placement<>'central' AND length(runner_reference) BETWEEN 1 AND 200))
);
CREATE UNIQUE INDEX mcp_connection_name ON mcp_connections(brain_id,lower(name));

CREATE TABLE mcp_profiles (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 120),
    description text NOT NULL DEFAULT '' CHECK(length(description)<=2000),
    environment_id uuid,
    enabled boolean NOT NULL DEFAULT true,
    revision uuid NOT NULL,
    created_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id),
    FOREIGN KEY(environment_id,brain_id) REFERENCES evidence_groups(id,brain_id)
);
CREATE UNIQUE INDEX mcp_profile_name ON mcp_profiles(brain_id,lower(name));
CREATE TABLE mcp_profile_connections (
    brain_id uuid NOT NULL,
    profile_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    PRIMARY KEY(profile_id,connection_id),
    FOREIGN KEY(profile_id,brain_id) REFERENCES mcp_profiles(id,brain_id),
    FOREIGN KEY(connection_id,brain_id) REFERENCES mcp_connections(id,brain_id)
);
CREATE TABLE mcp_profile_grants (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    profile_id uuid NOT NULL,
    account_id uuid REFERENCES accounts(id),
    issuer text,
    group_name text,
    can_use boolean NOT NULL,
    can_manage boolean NOT NULL,
    can_share boolean NOT NULL,
    FOREIGN KEY(profile_id,brain_id) REFERENCES mcp_profiles(id,brain_id),
    CHECK ((account_id IS NOT NULL AND issuer IS NULL AND group_name IS NULL) OR
           (account_id IS NULL AND issuer IS NOT NULL AND length(group_name) BETWEEN 1 AND 200)),
    CHECK(can_use OR can_manage OR can_share)
);
CREATE UNIQUE INDEX mcp_direct_grant ON mcp_profile_grants(profile_id,account_id) WHERE account_id IS NOT NULL;
CREATE UNIQUE INDEX mcp_group_grant ON mcp_profile_grants(profile_id,issuer,group_name) WHERE account_id IS NULL;

CREATE FUNCTION recollect_mcp_effective(target uuid, principal uuid, power text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT coalesce((SELECT
        recollect_effective_role(p.brain_id,principal) IS NOT NULL AND
        ((power IN ('manage','share') AND recollect_effective_role(p.brain_id,principal)='admin') OR
         EXISTS(SELECT 1 FROM mcp_profile_grants g JOIN accounts a ON a.id=principal
                WHERE g.profile_id=p.id AND a.enabled AND
                  (g.account_id=a.id OR (g.account_id IS NULL AND a.auth_kind='oidc' AND
                    a.membership_until>now() AND g.issuer=a.oidc_issuer AND g.group_name=ANY(a.oidc_groups))) AND
                  CASE power WHEN 'use' THEN g.can_use WHEN 'manage' THEN g.can_manage
                             WHEN 'share' THEN g.can_share ELSE false END))
        FROM mcp_profiles p WHERE p.id=target),false)
$$;
REVOKE ALL ON FUNCTION recollect_mcp_effective(uuid,uuid,text) FROM PUBLIC;
CREATE FUNCTION recollect_mcp_can(target uuid, power text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT recollect_mcp_effective(target,recollect_actor(),power)
$$;
REVOKE ALL ON FUNCTION recollect_mcp_can(uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_can(uuid,text) TO recollect_app;
CREATE FUNCTION recollect_mcp_account_can(target uuid, principal uuid, power text) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT CASE WHEN principal=recollect_actor() OR recollect_mcp_can(target,'share')
                THEN recollect_mcp_effective(target,principal,power) ELSE false END
$$;
REVOKE ALL ON FUNCTION recollect_mcp_account_can(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_account_can(uuid,uuid,text) TO recollect_app;
CREATE FUNCTION recollect_mcp_account_brain_role(target uuid, principal uuid) RETURNS text
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT CASE WHEN principal=recollect_actor() OR recollect_mcp_can(target,'share')
                THEN recollect_effective_role(p.brain_id,principal) END
    FROM mcp_profiles p WHERE p.id=target
$$;
REVOKE ALL ON FUNCTION recollect_mcp_account_brain_role(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_account_brain_role(uuid,uuid) TO recollect_app;

ALTER TABLE mcp_connections ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_connection_read ON mcp_connections FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY mcp_connection_insert ON mcp_connections FOR INSERT WITH CHECK(recollect_role(brain_id)='admin');
CREATE POLICY mcp_connection_update ON mcp_connections FOR UPDATE USING(recollect_role(brain_id)='admin') WITH CHECK(recollect_role(brain_id)='admin');
ALTER TABLE mcp_profiles ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_profile_read ON mcp_profiles FOR SELECT USING(
    recollect_mcp_can(id,'use') OR recollect_mcp_can(id,'manage') OR recollect_mcp_can(id,'share'));
CREATE POLICY mcp_profile_insert ON mcp_profiles FOR INSERT WITH CHECK(recollect_role(brain_id)='admin');
CREATE POLICY mcp_profile_update ON mcp_profiles FOR UPDATE USING(recollect_mcp_can(id,'manage')) WITH CHECK(recollect_mcp_can(id,'manage'));
ALTER TABLE mcp_profile_connections ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_membership_read ON mcp_profile_connections FOR SELECT USING(
    recollect_mcp_can(profile_id,'use') OR recollect_mcp_can(profile_id,'manage') OR recollect_mcp_can(profile_id,'share'));
CREATE POLICY mcp_membership_write ON mcp_profile_connections FOR ALL USING(recollect_mcp_can(profile_id,'manage')) WITH CHECK(recollect_mcp_can(profile_id,'manage'));
ALTER TABLE mcp_profile_grants ENABLE ROW LEVEL SECURITY;
CREATE POLICY mcp_grant_read ON mcp_profile_grants FOR SELECT USING(recollect_mcp_can(profile_id,'share'));
CREATE POLICY mcp_grant_write ON mcp_profile_grants FOR ALL USING(recollect_mcp_can(profile_id,'share')) WITH CHECK(recollect_mcp_can(profile_id,'share'));
GRANT SELECT,INSERT,UPDATE ON mcp_connections,mcp_profiles TO recollect_app;
GRANT SELECT,INSERT,UPDATE,DELETE ON mcp_profile_connections,mcp_profile_grants TO recollect_app;

CREATE FUNCTION recollect_mcp_environment_used(target_brain uuid, environment uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
    SELECT recollect_role(target_brain) IS NOT NULL AND
       (EXISTS(SELECT 1 FROM mcp_connections WHERE brain_id=target_brain AND environment_id=environment) OR
        EXISTS(SELECT 1 FROM mcp_profiles WHERE brain_id=target_brain AND environment_id=environment))
$$;
REVOKE ALL ON FUNCTION recollect_mcp_environment_used(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mcp_environment_used(uuid,uuid) TO recollect_app;
