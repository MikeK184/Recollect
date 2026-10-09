-- Scheduling progress only: no copied content or cached support verdict.
CREATE TABLE memory_support_discovery (
 brain_id uuid NOT NULL REFERENCES brains(id),
 policy_id uuid NOT NULL,
 verifier_version text NOT NULL,
 last_revision_id uuid,
 version bigint NOT NULL CHECK(version>0),
 PRIMARY KEY(brain_id,policy_id,verifier_version),
 FOREIGN KEY(policy_id,brain_id) REFERENCES model_policies(id,brain_id)
);
ALTER TABLE memory_support_discovery ENABLE ROW LEVEL SECURITY;
ALTER TABLE memory_support_discovery FORCE ROW LEVEL SECURITY;
CREATE POLICY support_discovery_read ON memory_support_discovery FOR SELECT
 USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY support_discovery_write ON memory_support_discovery FOR ALL
 USING(recollect_role(brain_id) IN ('writer','admin'))
 WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT SELECT,INSERT,UPDATE ON memory_support_discovery TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation)
 VALUES(680,'memory_support_discovery');
CREATE FUNCTION recollect_support_discovery_changed() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF NEW.brain_id<>OLD.brain_id OR NEW.policy_id<>OLD.policy_id
  OR NEW.verifier_version<>OLD.verifier_version OR NEW.version<>OLD.version+1 THEN
  RAISE EXCEPTION 'Immutable discovery identity and monotonic progress required';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER support_discovery_changed BEFORE UPDATE ON memory_support_discovery
 FOR EACH ROW EXECUTE FUNCTION recollect_support_discovery_changed();
-- The walk must resolve a bounded identity window before canonical qualification.
CREATE INDEX support_discovery_revision_walk ON claim_revisions(brain_id,id);
