-- Administrator-curated text guidance, distinct from learned memory.
CREATE TABLE managed_skills (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
 name text NOT NULL CHECK(length(name) BETWEEN 1 AND 64),
 version integer NOT NULL CHECK(version>0),
 environment_id uuid,
 enabled boolean NOT NULL,
 summary jsonb NOT NULL CHECK(jsonb_typeof(summary)='object'),
 UNIQUE(id,brain_id),
 UNIQUE(brain_id,name),
 FOREIGN KEY(environment_id,brain_id) REFERENCES evidence_groups(id,brain_id)
);
CREATE TABLE managed_skill_versions (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id) ON DELETE CASCADE,
 skill_id uuid NOT NULL,
 version integer NOT NULL CHECK(version>0),
 environment_id uuid,
 summary jsonb NOT NULL CHECK(jsonb_typeof(summary)='object'),
 files jsonb NOT NULL CHECK(jsonb_typeof(files)='array'),
 created_by uuid NOT NULL REFERENCES accounts(id),
 UNIQUE(skill_id,version),
 FOREIGN KEY(skill_id,brain_id) REFERENCES managed_skills(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(environment_id,brain_id) REFERENCES evidence_groups(id,brain_id)
);
CREATE INDEX managed_skills_scope ON managed_skills(brain_id,environment_id);
ALTER TABLE managed_skills ENABLE ROW LEVEL SECURITY;
CREATE POLICY skill_read ON managed_skills FOR SELECT USING(
 recollect_role(brain_id) IS NOT NULL AND (enabled OR (recollect_role(brain_id)='admin' AND recollect_device() IS NULL)));
CREATE POLICY skill_create ON managed_skills FOR INSERT WITH CHECK(
 recollect_role(brain_id)='admin' AND recollect_device() IS NULL);
CREATE POLICY skill_update ON managed_skills FOR UPDATE USING(
 recollect_role(brain_id)='admin' AND recollect_device() IS NULL) WITH CHECK(
 recollect_role(brain_id)='admin' AND recollect_device() IS NULL);
ALTER TABLE managed_skill_versions ENABLE ROW LEVEL SECURITY;
CREATE POLICY skill_version_read ON managed_skill_versions FOR SELECT USING(
 recollect_role(brain_id) IS NOT NULL AND EXISTS(SELECT 1 FROM managed_skills s WHERE s.brain_id=managed_skill_versions.brain_id AND s.id=skill_id));
CREATE POLICY skill_version_create ON managed_skill_versions FOR INSERT WITH CHECK(
 recollect_role(brain_id)='admin' AND recollect_device() IS NULL);
GRANT SELECT,INSERT,UPDATE ON managed_skills TO recollect_app;
GRANT SELECT,INSERT ON managed_skill_versions TO recollect_app;

INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (682,'managed_skill_versions'),(690,'managed_skills');
-- Include immutable guidance identities in the existing deletion preview fence.
ALTER FUNCTION recollect_brain_closure(uuid) RENAME TO recollect_pre_skills_brain_closure;
CREATE FUNCTION recollect_brain_closure(b uuid) RETURNS jsonb
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb;
BEGIN
 m:=recollect_pre_skills_brain_closure(b);
 IF m IS NULL THEN RETURN NULL; END IF;
 RETURN m||jsonb_build_object(
  'managed_skills',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM managed_skills WHERE brain_id=b),'[]'),
  'managed_skill_versions',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM managed_skill_versions WHERE brain_id=b),'[]'));
END $$;
REVOKE ALL ON FUNCTION recollect_pre_skills_brain_closure(uuid),recollect_brain_closure(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_closure(uuid) TO recollect_app;

-- Historical applicability remains meaningful while a bundle exists.
CREATE OR REPLACE FUNCTION recollect_mcp_environment_used(target_brain uuid, environment uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT recollect_role(target_brain) IS NOT NULL AND (
  EXISTS(SELECT 1 FROM mcp_connections WHERE brain_id=target_brain AND environment_id=environment) OR
  EXISTS(SELECT 1 FROM mcp_profiles WHERE brain_id=target_brain AND environment_id=environment) OR
  EXISTS(SELECT 1 FROM managed_skills WHERE brain_id=target_brain AND environment_id=environment) OR
  EXISTS(SELECT 1 FROM managed_skill_versions WHERE brain_id=target_brain AND environment_id=environment))
$$;
