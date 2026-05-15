CREATE TABLE memory_decisions (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL REFERENCES brains(id),
    decision jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id)
);
CREATE TABLE memory_decision_claims (
    decision_id uuid NOT NULL,
    claim_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    PRIMARY KEY(decision_id,claim_id),
    FOREIGN KEY(decision_id,brain_id) REFERENCES memory_decisions(id,brain_id) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(claim_id,brain_id) REFERENCES claims(id,brain_id)
);
CREATE TABLE assertion_rules (
    id uuid PRIMARY KEY,
    brain_id uuid NOT NULL,
    decision_id uuid NOT NULL,
    subject_key text NOT NULL,
    predicate_key text NOT NULL,
    value_key text NOT NULL,
    rule jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,brain_id),
    FOREIGN KEY(decision_id,brain_id) REFERENCES memory_decisions(id,brain_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX assertion_rule_match ON assertion_rules(brain_id,subject_key,predicate_key,value_key);
CREATE TABLE memory_rule_exceptions (
    rule_id uuid NOT NULL,
    revision_id uuid NOT NULL,
    decision_id uuid NOT NULL,
    brain_id uuid NOT NULL,
    PRIMARY KEY(rule_id,revision_id),
    FOREIGN KEY(rule_id,brain_id) REFERENCES assertion_rules(id,brain_id),
    FOREIGN KEY(revision_id,brain_id) REFERENCES claim_revisions(id,brain_id),
    FOREIGN KEY(decision_id,brain_id) REFERENCES memory_decisions(id,brain_id) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE memory_epochs (
    brain_id uuid PRIMARY KEY REFERENCES brains(id),
    epoch bigint NOT NULL CHECK(epoch>0)
);
DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['memory_decisions','memory_decision_claims','assertion_rules','memory_rule_exceptions','memory_epochs'] LOOP
    EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
    EXECUTE format('CREATE POLICY review_read ON %I FOR SELECT USING(recollect_role(brain_id) IS NOT NULL)',tab);
    EXECUTE format('CREATE POLICY review_insert ON %I FOR INSERT WITH CHECK(recollect_role(brain_id) IN (''writer'',''admin''))',tab);
    EXECUTE format('GRANT SELECT,INSERT ON %I TO recollect_app',tab);
  END LOOP;
END $$;
CREATE POLICY epoch_update ON memory_epochs FOR UPDATE USING(recollect_role(brain_id) IN ('writer','admin')) WITH CHECK(recollect_role(brain_id) IN ('writer','admin'));
GRANT UPDATE(epoch) ON memory_epochs TO recollect_app;
