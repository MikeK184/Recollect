-- No recipe is selected by this migration. An immutable qualified receipt must
-- precede selection; synthetic integration profiles are not production quality.
ALTER TABLE analytics_reports ADD COLUMN purpose text NOT NULL DEFAULT 'topology'
 CHECK(purpose IN ('topology','organization'));
CREATE UNIQUE INDEX organization_calculation_active ON analytics_reports(brain_id)
 WHERE purpose='organization' AND state IN ('queued','running');

CREATE TABLE knowledge_organization_recipes (
 change_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 recipe text NOT NULL CHECK(recipe='entity-idf-bipartite-leiden-1'),
 gds_version text NOT NULL CHECK(octet_length(gds_version) BETWEEN 1 AND 64),
 quality_state text NOT NULL DEFAULT 'blocked' CHECK(quality_state IN ('blocked','qualified')),
 quality_receipt_sha256 text CHECK(quality_receipt_sha256 ~ '^[a-f0-9]{64}$'),
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
 UNIQUE(change_id,brain_id),
 CHECK(quality_state<>'qualified' OR quality_receipt_sha256 IS NOT NULL)
);
CREATE TABLE knowledge_organization_heads (
 brain_id uuid PRIMARY KEY REFERENCES brains(id),
 change_id uuid NOT NULL,
 FOREIGN KEY(change_id,brain_id) REFERENCES knowledge_organization_recipes(change_id,brain_id)
);
CREATE TABLE knowledge_organization_outbox (
 brain_id uuid PRIMARY KEY REFERENCES brains(id),
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0),
 handled_revision bigint NOT NULL DEFAULT 0 CHECK(handled_revision>=0),
 state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','blocked_recipe','empty','queued','ready','failed')),
 report_id uuid REFERENCES analytics_reports(id) ON DELETE SET NULL,
 deadline timestamptz,
 updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE knowledge_organization_calculations (
 report_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 recipe_change_id uuid NOT NULL,
 outbox_revision bigint NOT NULL,
 projection jsonb NOT NULL CHECK(octet_length(projection::text)<=8388608),
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(recipe_change_id,brain_id) REFERENCES knowledge_organization_recipes(change_id,brain_id)
);
CREATE TABLE knowledge_organization_inputs (
 brain_id uuid NOT NULL,
 report_id uuid NOT NULL,
 input_id uuid NOT NULL,
 PRIMARY KEY(report_id,input_id),
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(input_id,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE
);
-- Lineage stores only exact IDs and a surviving source-bound dependency. Native
-- community numbers and labels are never used as canonical identities.
CREATE TABLE knowledge_organization_lineage (
 id uuid PRIMARY KEY,
 brain_id uuid NOT NULL,
 topic_id uuid NOT NULL,
 previous_topic_id uuid NOT NULL,
 basis_input uuid NOT NULL,
 report_id uuid NOT NULL,
 FOREIGN KEY(topic_id,brain_id) REFERENCES knowledge_topics(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(basis_input,brain_id) REFERENCES knowledge_mapping_inputs(id,brain_id) ON DELETE CASCADE,
 FOREIGN KEY(report_id,brain_id) REFERENCES analytics_reports(id,brain_id) ON DELETE CASCADE
);
ALTER TABLE knowledge_topics ADD COLUMN organization_report_id uuid
 REFERENCES analytics_reports(id) ON DELETE SET NULL;
CREATE FUNCTION recollect_organization_recipe_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN RAISE EXCEPTION 'Organization recipe revisions are immutable'; END $$;
CREATE TRIGGER organization_recipe_immutable BEFORE UPDATE ON knowledge_organization_recipes
 FOR EACH ROW EXECUTE FUNCTION recollect_organization_recipe_immutable();
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['knowledge_organization_recipes','knowledge_organization_heads','knowledge_organization_outbox',
  'knowledge_organization_calculations','knowledge_organization_inputs','knowledge_organization_lineage'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY organization_calculation_read ON %I FOR SELECT USING(brain_id IN(SELECT id FROM brains))',tab);
  EXECUTE format('GRANT SELECT ON %I TO recollect_app',tab);
  IF tab NOT IN ('knowledge_organization_recipes','knowledge_organization_heads') THEN
   EXECUTE format('CREATE POLICY organization_calculation_write ON %I FOR ALL USING(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin''))) WITH CHECK(brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN (''writer'',''admin'')))',tab);
   EXECUTE format('GRANT INSERT,UPDATE,DELETE ON %I TO recollect_app',tab);
  END IF;
 END LOOP;
END $$;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES
 (550,'knowledge_organization_lineage'),(551,'knowledge_organization_inputs'),
 (552,'knowledge_organization_calculations'),(9990,'knowledge_organization_outbox'),
 (554,'knowledge_organization_heads'),(555,'knowledge_organization_recipes');

-- Routing contains identities only. Native grouping of already interpreted
-- evidence is also automatic after an explicitly requested learning run; it
-- does not require permission for new autonomous model calls.
CREATE FUNCTION recollect_organization_brains() RETURNS TABLE(brain_id uuid,actor_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT b.id,p.created_by FROM brains b
 JOIN knowledge_organization_outbox o ON o.brain_id=b.id
 JOIN model_policy_heads h ON h.brain_id=b.id
 JOIN model_policies p ON p.brain_id=h.brain_id AND p.id=h.policy_id
 JOIN accounts a ON a.id=p.created_by
 WHERE NOT b.archived AND a.enabled AND p.policy->>'enabled'='true'
  AND p.policy->'purposes' ? 'extraction'
  AND recollect_effective_role(b.id,p.created_by) IN ('writer','admin')
 ORDER BY b.id
$$;
REVOKE ALL ON FUNCTION recollect_organization_brains() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_organization_brains() TO recollect_app;

-- All existing canonical invalidations also coalesce a restart-safe semantic
-- calculation request. This is one content-free row per Brain, not one job per
-- mention/write. Existing job-before-report lock order remains unchanged.
CREATE OR REPLACE FUNCTION recollect_analytics_invalidate(affected uuid[]) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[];
BEGIN
 UPDATE brains SET analytics_epoch=analytics_epoch+1 WHERE id=ANY(affected);
 SELECT array_agg(id) INTO reports FROM analytics_reports WHERE brain_id=ANY(affected) AND state IN ('queued','running','ready');
 UPDATE jobs SET state='cancelled',error_code='analytics_inputs_changed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE brain_id=ANY(affected) AND kind='graph.analyze' AND state IN ('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='stale',scores=NULL,error_code='analytics_inputs_changed' WHERE id=ANY(reports);
 INSERT INTO knowledge_organization_outbox(brain_id)
 SELECT DISTINCT id FROM brains WHERE id=ANY(affected)
 ON CONFLICT(brain_id) DO UPDATE SET revision=knowledge_organization_outbox.revision+1,state='pending',updated_at=clock_timestamp();
END $$;
CREATE TRIGGER organization_recipe_selected_insert AFTER INSERT ON knowledge_organization_heads
 REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed();
CREATE TRIGGER organization_recipe_selected_update AFTER UPDATE ON knowledge_organization_heads
 REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed();
CREATE TRIGGER organization_recipe_selected_delete AFTER DELETE ON knowledge_organization_heads
 REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed();

-- A human edit can invalidate a frozen calculation epoch. Coalesce that real
-- edit into a new request only when a calculation is active; successful
-- publication never calls this helper and cannot create its own retry loop.
CREATE FUNCTION recollect_organization_override_changed(b uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; BEGIN
 IF recollect_role(b) NOT IN ('writer','admin') OR recollect_role(b) IS NULL THEN RAISE insufficient_privilege; END IF;
 SELECT array_agg(id) INTO reports FROM analytics_reports WHERE brain_id=b AND purpose='organization' AND state IN ('queued','running');
 IF reports IS NOT NULL THEN
  UPDATE jobs SET state='cancelled',error_code='analytics_inputs_changed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
   WHERE brain_id=b AND kind='graph.analyze' AND target_id=ANY(reports) AND state IN ('queued','running');
  UPDATE analytics_reports SET state='stale',scores=NULL,error_code='analytics_inputs_changed' WHERE id=ANY(reports);
  UPDATE knowledge_organization_outbox SET revision=revision+1,state='pending',updated_at=clock_timestamp() WHERE brain_id=b;
 END IF;
END $$;
REVOKE ALL ON FUNCTION recollect_organization_override_changed(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_organization_override_changed(uuid) TO recollect_app;

-- A removed exact contributor invalidates the whole provisional weighted
-- aggregate, including opaque descriptor/score data, rather than salvaging it.
CREATE FUNCTION recollect_organization_calculation_input_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE reports uuid[]; BEGIN
 SELECT array_agg(DISTINCT report_id) INTO reports FROM organization_removed_calculation_inputs;
 UPDATE jobs SET state='cancelled',error_code='content_removed',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
 WHERE kind='graph.analyze' AND state IN ('queued','running') AND target_id=ANY(reports);
 UPDATE analytics_reports SET state='removed',selection=NULL,provenance=NULL,descriptor=NULL,scores=NULL,error_code='content_removed'
 WHERE id=ANY(reports);
 DELETE FROM knowledge_organization_calculations WHERE report_id=ANY(reports);
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_organization_calculation_input_removed() FROM PUBLIC;
CREATE TRIGGER organization_calculation_input_removed AFTER DELETE ON knowledge_organization_inputs
 REFERENCING OLD TABLE AS organization_removed_calculation_inputs
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_organization_calculation_input_removed();
CREATE FUNCTION recollect_organization_calculation_removed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 DELETE FROM knowledge_organization_calculations WHERE report_id=NEW.id;
 DELETE FROM knowledge_organization_inputs WHERE report_id=NEW.id;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_organization_calculation_removed() FROM PUBLIC;
CREATE TRIGGER organization_calculation_removed AFTER UPDATE OF state ON analytics_reports
 FOR EACH ROW WHEN(NEW.state='removed' AND OLD.state IS DISTINCT FROM NEW.state)
 EXECUTE FUNCTION recollect_organization_calculation_removed();
CREATE FUNCTION recollect_organization_calculation_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 UPDATE knowledge_organization_outbox SET state=CASE WHEN revision>handled_revision THEN 'pending' ELSE 'failed' END,updated_at=clock_timestamp()
  WHERE brain_id=NEW.brain_id AND report_id=NEW.id;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_organization_calculation_state() FROM PUBLIC;
CREATE TRIGGER organization_calculation_state AFTER UPDATE OF state ON analytics_reports
 FOR EACH ROW WHEN(NEW.purpose='organization' AND NEW.state IN ('failed','cancelled','stale','removed') AND OLD.state IS DISTINCT FROM NEW.state)
 EXECUTE FUNCTION recollect_organization_calculation_state();

-- A current community's publication record is canonical provenance, not the
-- seven-day disposable topology history. Keep it while topics still reference
-- it; superseded/unreferenced calculations retain normal bounded cleanup.
CREATE OR REPLACE FUNCTION recollect_analytics_maintain() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE b uuid; reports uuid[];
BEGIN
 FOR b IN SELECT id FROM brains WHERE EXISTS(SELECT 1 FROM analytics_reports r WHERE r.brain_id=brains.id AND r.state IN ('queued','running','ready') AND r.expires_at<=clock_timestamp()) ORDER BY id LIMIT 100 FOR UPDATE SKIP LOCKED LOOP
  SELECT array_agg(id) INTO reports FROM analytics_reports WHERE brain_id=b AND state IN ('queued','running','ready') AND expires_at<=clock_timestamp();
  UPDATE jobs SET state='cancelled',error_code='analytics_inputs_expired',lease_token=NULL,lease_until=NULL,updated_at=clock_timestamp()
   WHERE brain_id=b AND kind='graph.analyze' AND state IN ('queued','running') AND target_id=ANY(reports);
  UPDATE analytics_reports SET state='stale',scores=NULL,error_code='analytics_inputs_expired' WHERE id=ANY(reports);
 END LOOP;
 DELETE FROM analytics_reports WHERE id IN (SELECT r.id FROM analytics_reports r
  WHERE r.state NOT IN ('queued','running') AND r.created_at<clock_timestamp()-interval '7 days'
   AND NOT EXISTS(SELECT 1 FROM analytics_attempts a WHERE a.report_id=r.id AND a.cleaned_at IS NULL)
   AND NOT EXISTS(SELECT 1 FROM knowledge_topics t WHERE t.organization_report_id=r.id) LIMIT 100);
END $$;
INSERT INTO knowledge_organization_outbox(brain_id) SELECT id FROM brains;

-- This is a query-shape hint, never an authority cache or a supplied Brain ID.
-- Default execution retains each exact existing policy. Only bounded bulk
-- qualification uses equivalent current visible/writable Brain sets once per
-- statement; ordinary one-row canonical lookups avoid repeated whole-set setup.
-- Actor/device checks, command types, roles and RLS enforcement remain intact.
DO $$ DECLARE p record; old_expr text; bulk_expr text; clause text; statement text;
BEGIN
 FOR p IN SELECT c.relname,q.polname,pg_get_expr(q.polqual,q.polrelid) AS predicate,
   pg_get_expr(q.polwithcheck,q.polrelid) AS check_predicate
  FROM pg_policy q JOIN pg_class c ON c.oid=q.polrelid JOIN pg_namespace n ON n.oid=c.relnamespace
  WHERE n.nspname='public' AND c.relname=ANY(ARRAY['sources','source_versions','source_chunks',
   'evidence_groups','evidence_memberships','learning_runs','learning_support_stages',
   'model_requests','model_request_inputs','model_policies','model_policy_heads',
   'source_excerpts','source_import_scopes','retention_policies'])
 LOOP
  statement:=format('ALTER POLICY %I ON %I',p.polname,p.relname);
  FOREACH clause IN ARRAY ARRAY['USING','WITH CHECK'] LOOP
   old_expr:=CASE WHEN clause='USING' THEN p.predicate ELSE p.check_predicate END;
   IF old_expr IS NULL THEN CONTINUE; END IF;
   bulk_expr:=replace(old_expr,
    'recollect_role(brain_id) = ANY (ARRAY[''writer''::text, ''admin''::text])',
    'brain_id IN(SELECT id FROM brains WHERE recollect_role(id) IN(''writer'',''admin''))');
   bulk_expr:=replace(bulk_expr,'recollect_role(brain_id) = ''admin''::text',
    'brain_id IN(SELECT id FROM brains WHERE recollect_role(id)=''admin'')');
   bulk_expr:=replace(bulk_expr,'recollect_role(brain_id) IS NOT NULL','brain_id IN(SELECT id FROM brains)');
   IF bulk_expr IS DISTINCT FROM old_expr THEN
    statement:=statement||format(' %s (CASE WHEN current_setting(''recollect.bulk_qualification'',true)=''on'' THEN (%s) ELSE (%s) END)',clause,bulk_expr,old_expr);
   ELSE
    statement:=statement||format(' %s (%s)',clause,old_expr);
   END IF;
  END LOOP;
  EXECUTE statement;
 END LOOP;
END $$;
