CREATE TABLE devices (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    name text NOT NULL CHECK(length(name) BETWEEN 1 AND 120),
    token uuid NOT NULL UNIQUE,
    claimed boolean NOT NULL DEFAULT false,
    revoked_at timestamptz,
    expires_at timestamptz NOT NULL DEFAULT now()+interval '30 days',
    last_used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX devices_account ON devices(account_id);
CREATE TABLE device_pairings (
    id uuid PRIMARY KEY,
    device_code uuid NOT NULL UNIQUE,
    user_code text NOT NULL UNIQUE,
    name text NOT NULL,
    state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','approved','declined','cancelled','claimed')),
    device_id uuid REFERENCES devices(id),
    expires_at timestamptz NOT NULL DEFAULT now()+interval '5 minutes',
    last_polled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
GRANT SELECT,INSERT,UPDATE,DELETE ON device_pairings TO recollect_app;
GRANT SELECT,INSERT,UPDATE ON devices TO recollect_app;

CREATE FUNCTION recollect_device() RETURNS uuid LANGUAGE sql STABLE AS $$
  SELECT nullif(current_setting('recollect.device',true),'')::uuid
$$;
ALTER TABLE jobs ADD COLUMN device_id uuid REFERENCES devices(id) DEFAULT recollect_device();
ALTER TABLE mutation_audit ADD COLUMN device_id uuid REFERENCES devices(id) DEFAULT recollect_device();
CREATE POLICY device_audit_create ON mutation_audit FOR INSERT WITH CHECK(brain_id IS NULL AND actor_id=recollect_actor() AND
  action IN ('device.approve','device.decline','device.revoke') AND
  (action='device.decline' OR EXISTS(SELECT 1 FROM devices WHERE id=target_id AND account_id=recollect_actor())));
CREATE POLICY device_audit_read ON mutation_audit FOR SELECT USING(brain_id IS NULL AND actor_id=recollect_actor() AND action LIKE 'device.%');
