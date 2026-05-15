ALTER TABLE repository_facts ADD CONSTRAINT fact_brain_identity UNIQUE(id,brain_id);
ALTER TABLE manifest_revisions ADD CONSTRAINT manifest_revision_brain_identity UNIQUE(id,brain_id);
-- Evidence knowledge timestamps must follow acquisition of the Brain write lock,
-- not the start of a transaction that may have waited behind another writer.
ALTER TABLE source_versions ALTER COLUMN created_at SET DEFAULT clock_timestamp();
ALTER TABLE manifest_revisions ALTER COLUMN created_at SET DEFAULT clock_timestamp();
CREATE TABLE claims (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    created_by uuid NOT NULL REFERENCES accounts(id),
    current_revision uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(id,brain_id)
);
CREATE TABLE claim_revisions (
    id uuid PRIMARY KEY,
    claim_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    recorded_at timestamptz NOT NULL,
    revision jsonb NOT NULL,
    subject_key text NOT NULL,
    predicate_key text NOT NULL,
    value_key text NOT NULL,
    UNIQUE(id,claim_id,brain_id),
    UNIQUE(id,brain_id),
    UNIQUE(claim_id,recorded_at),
    FOREIGN KEY(claim_id,brain_id) REFERENCES claims(id,brain_id)
);
ALTER TABLE claims ADD CONSTRAINT claim_current_revision
    FOREIGN KEY(current_revision,id,brain_id) REFERENCES claim_revisions(id,claim_id,brain_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX claim_knowledge_history ON claim_revisions(claim_id,recorded_at DESC);
CREATE INDEX claim_assertion ON claim_revisions(brain_id,subject_key,predicate_key);
CREATE TABLE claim_supports (
    revision_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK(ordinal BETWEEN 0 AND 19),
    source_version_id uuid,
    fact_id uuid,
    manifest_revision_id uuid,
    PRIMARY KEY(revision_id,ordinal),
    CHECK(num_nonnulls(source_version_id,fact_id,manifest_revision_id)=1),
    FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
    FOREIGN KEY(source_version_id,brain_id) REFERENCES source_versions(id,brain_id),
    FOREIGN KEY(fact_id,brain_id) REFERENCES repository_facts(id,brain_id),
    FOREIGN KEY(manifest_revision_id,brain_id) REFERENCES manifest_revisions(id,brain_id)
);
DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['claims','claim_revisions','claim_supports'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY memory_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY memory_insert ON %I FOR INSERT WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('GRANT SELECT,INSERT ON %I TO recollect_app',tab);
  END LOOP;
END $$;
CREATE POLICY claim_update ON claims FOR UPDATE USING(recollect_role(brain_id) IN ('writer','admin')) WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT UPDATE(current_revision) ON claims TO recollect_app;
