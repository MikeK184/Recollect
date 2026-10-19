-- Explicit shared projection; private workspace tables retain their existing RLS.
CREATE TABLE coordination_sessions (
 id uuid PRIMARY KEY, brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
 account_id uuid NOT NULL REFERENCES accounts(id), device_id uuid NOT NULL REFERENCES devices(id),
 task_id uuid NOT NULL, scope_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0),
 input jsonb NOT NULL, projection jsonb NOT NULL, closed boolean NOT NULL DEFAULT false,
 last_seen_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
 next_sequence bigint NOT NULL DEFAULT 0,
 UNIQUE(id,brain_id),
 FOREIGN KEY(task_id,brain_id,account_id) REFERENCES workspace_tasks(id,brain_id,account_id),
 FOREIGN KEY(scope_id,task_id,brain_id,account_id) REFERENCES scope_snapshots(id,task_id,brain_id,account_id)
);
CREATE INDEX coordination_presence ON coordination_sessions(brain_id,expires_at);
CREATE TABLE coordination_overlaps (
 id uuid PRIMARY KEY, brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
 left_id uuid NOT NULL, right_id uuid NOT NULL, reason_key text NOT NULL,
 signature jsonb NOT NULL, active boolean NOT NULL,
 UNIQUE(left_id,right_id,reason_key),
 FOREIGN KEY(left_id,brain_id) REFERENCES coordination_sessions(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(right_id,brain_id) REFERENCES coordination_sessions(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE coordination_messages (
 id uuid PRIMARY KEY, brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
 from_session_id uuid NOT NULL, to_session_id uuid NOT NULL, input jsonb NOT NULL,
 payload jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
 FOREIGN KEY(from_session_id,brain_id) REFERENCES coordination_sessions(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(to_session_id,brain_id) REFERENCES coordination_sessions(id,brain_id) ON DELETE CASCADE
);
CREATE TABLE coordination_events (
 id uuid PRIMARY KEY, brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
 session_id uuid NOT NULL, sequence bigint NOT NULL CHECK(sequence>0), kind text NOT NULL,
 payload jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
 acknowledged_at timestamptz, UNIQUE(session_id,sequence),
 FOREIGN KEY(session_id,brain_id) REFERENCES coordination_sessions(id,brain_id) ON DELETE CASCADE
);
CREATE INDEX coordination_message_expiry ON coordination_messages(brain_id,expires_at);
CREATE INDEX coordination_event_expiry ON coordination_events(brain_id,expires_at);
-- A minimal predicate exposes eligibility, never another account's private task.
CREATE FUNCTION recollect_coordination_eligible(sid uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT coalesce((SELECT NOT s.closed AND s.expires_at>clock_timestamp()
  AND NOT b.archived AND a.enabled AND (a.auth_kind='local' OR a.membership_until>now())
  AND d.claimed AND d.revoked_at IS NULL AND d.expires_at>now()
  AND NOT t.closed AND t.current_scope=s.scope_id AND t.device_id=s.device_id
  AND recollect_effective_role(s.brain_id,s.account_id) IS NOT NULL
  AND recollect_role(s.brain_id) IS NOT NULL
  AND recollect_selection_valid(s.brain_id,s.projection)
 FROM coordination_sessions s JOIN brains b ON b.id=s.brain_id
 JOIN accounts a ON a.id=s.account_id JOIN devices d ON d.id=s.device_id
 JOIN workspace_tasks t ON t.id=s.task_id WHERE s.id=sid),false)
$$;
REVOKE ALL ON FUNCTION recollect_coordination_eligible(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_coordination_eligible(uuid) TO recollect_app;
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['coordination_sessions','coordination_overlaps','coordination_messages','coordination_events'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY coordination_brain ON %I FOR ALL USING(recollect_role(brain_id) IS NOT NULL) WITH CHECK(recollect_role(brain_id) IS NOT NULL)',tab);
  EXECUTE format('GRANT SELECT,INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
 END LOOP;
END $$;
CREATE POLICY coordination_inbox_private ON coordination_events AS RESTRICTIVE FOR SELECT USING(
 EXISTS(SELECT 1 FROM coordination_sessions s WHERE s.id=session_id AND s.account_id=recollect_actor() AND s.device_id=recollect_device()));
CREATE POLICY coordination_message_private ON coordination_messages AS RESTRICTIVE FOR SELECT USING(
 EXISTS(SELECT 1 FROM coordination_sessions s WHERE s.id IN(from_session_id,to_session_id) AND s.account_id=recollect_actor() AND s.device_id=recollect_device()));
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (641,'coordination_events'),(642,'coordination_messages'),(643,'coordination_overlaps'),(644,'coordination_sessions');
ALTER FUNCTION recollect_brain_closure(uuid) RENAME TO recollect_pre_coordination_brain_closure;
CREATE FUNCTION recollect_brain_closure(b uuid) RETURNS jsonb
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; BEGIN
 m:=recollect_pre_coordination_brain_closure(b);
 IF m IS NULL THEN RETURN NULL; END IF;
 RETURN m||jsonb_build_object(
 'coordination_sessions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_sessions WHERE brain_id=b),'[]'),
 'coordination_messages',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_messages WHERE brain_id=b),'[]'),
 'coordination_events',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_events WHERE brain_id=b),'[]'),
 'coordination_overlaps',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM coordination_overlaps WHERE brain_id=b),'[]'));
END $$;
REVOKE ALL ON FUNCTION recollect_pre_coordination_brain_closure(uuid),recollect_brain_closure(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_closure(uuid) TO recollect_app;
CREATE FUNCTION recollect_coordination_event_count(sid uuid) RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT count(*) FROM coordination_events e JOIN coordination_sessions s ON s.id=e.session_id
 WHERE s.id=sid AND recollect_role(s.brain_id) IS NOT NULL AND e.expires_at>now()
$$;
REVOKE ALL ON FUNCTION recollect_coordination_event_count(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_coordination_event_count(uuid) TO recollect_app;
