-- Brain deletion without per-row derived-state upkeep for the Brain being
-- removed (ADR 0016, contract
-- docs/contracts/platform-brain-deletion.md#closure-computation).
--
-- A Brain-wide request runs the whole per-class apply chain and then
-- recollect_brain_remove deletes every Brain relation in the same
-- transaction. While the chain redacted rows, the row triggers below re-derived
-- semantic, learning, handover, support and epoch state for each one: 15,500
-- semantic, 13,000 learning, 13,000 support, 10,000 epoch and 7,500
-- reconciliation and handover firings for a Brain of 3,000 sources and 5,000
-- claims, about 23 of its 32 seconds. Every row those triggers write carries
-- the same brain_id and belongs to a relation in brain_deletion_dependents, so
-- recollect_brain_remove deletes it moments later and no committed state can
-- observe the upkeep.
--
-- The apply chain still runs every per-class statement: redaction, the
-- retained artifact cleanup queue, publication fences, receipt invalidation
-- and the journal are unchanged. Only these row triggers skip, and only while
-- recollect.removing_brain names the Brain-wide erase request of that same
-- Brain. The setting is transaction-local and is checked against the journal,
-- which only recollect_brain_delete and journal replay write, so another
-- caller setting it changes nothing. Statement triggers keep firing once per
-- statement.
CREATE FUNCTION recollect_brain_removing(b uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
  SELECT coalesce(current_setting('recollect.removing_brain',true),'')<>'' AND EXISTS(
    SELECT 1 FROM privacy_requests r
    WHERE r.id=nullif(current_setting('recollect.removing_brain',true),'')::uuid
      AND r.brain_id=b AND r.cause='erase' AND r.target->>'kind'='brain')
$$;
REVOKE ALL ON FUNCTION recollect_brain_removing(uuid) FROM PUBLIC;
-- Trigger conditions run as the writing role.
GRANT EXECUTE ON FUNCTION recollect_brain_removing(uuid) TO recollect_app;

-- The outermost apply names a Brain-wide erase request for its own duration;
-- live deletion and journal replay both enter here.
ALTER FUNCTION recollect_privacy_apply(uuid) RENAME TO recollect_pre_brain_gate_apply;
CREATE FUNCTION recollect_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  IF r.cause='erase' AND r.target->>'kind'='brain' THEN
    PERFORM set_config('recollect.removing_brain',r.id::text,true);
    PERFORM recollect_pre_brain_gate_apply(rid);
    PERFORM set_config('recollect.removing_brain','',true);
  ELSE
    PERFORM recollect_pre_brain_gate_apply(rid);
  END IF;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_apply(uuid),recollect_pre_brain_gate_apply(uuid) FROM PUBLIC;

-- The same triggers, names (so firing order), events and functions, with the
-- removal condition added.
DROP TRIGGER source_semantic_removed ON source_versions;
CREATE TRIGGER source_semantic_removed AFTER UPDATE OF privacy_state ON source_versions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_semantic_input_removed();
DROP TRIGGER chunk_semantic_removed ON source_chunks;
CREATE TRIGGER chunk_semantic_removed BEFORE DELETE ON source_chunks FOR EACH ROW
  WHEN (NOT recollect_brain_removing(OLD.brain_id)) EXECUTE FUNCTION recollect_semantic_input_removed();
DROP TRIGGER claim_semantic_removed ON claim_revisions;
CREATE TRIGGER claim_semantic_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_semantic_input_removed();
DROP TRIGGER snapshot_semantic_removed ON repository_snapshots;
CREATE TRIGGER snapshot_semantic_removed AFTER UPDATE OF privacy_state ON repository_snapshots FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_semantic_input_removed();
DROP TRIGGER manifest_semantic_removed ON manifest_revisions;
CREATE TRIGGER manifest_semantic_removed AFTER UPDATE OF privacy_state ON manifest_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_semantic_input_removed();
DROP TRIGGER semantic_source_fenced ON model_input_fences;
CREATE TRIGGER semantic_source_fenced AFTER INSERT ON model_input_fences FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_semantic_fenced();
DROP TRIGGER semantic_claim_fenced ON model_claim_fences;
CREATE TRIGGER semantic_claim_fenced AFTER INSERT ON model_claim_fences FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_semantic_fenced();
DROP TRIGGER source_learning_removed ON source_versions;
CREATE TRIGGER source_learning_removed AFTER UPDATE OF privacy_state ON source_versions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_learning_input_removed();
DROP TRIGGER claim_model_removed ON claim_revisions;
CREATE TRIGGER claim_model_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_learning_input_removed();
DROP TRIGGER snapshot_model_removed ON repository_snapshots;
CREATE TRIGGER snapshot_model_removed AFTER UPDATE OF privacy_state ON repository_snapshots FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_learning_input_removed();
DROP TRIGGER handover_input_removed ON claim_revisions;
CREATE TRIGGER handover_input_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_handover_input_removed();
DROP TRIGGER reconciliation_input_removed ON claim_revisions;
CREATE TRIGGER reconciliation_input_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_reconciliation_input_removed();
DROP TRIGGER support_source_removed ON source_versions;
CREATE TRIGGER support_source_removed AFTER UPDATE OF privacy_state ON source_versions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_support_stage_removed();
DROP TRIGGER support_revision_removed ON claim_revisions;
CREATE TRIGGER support_revision_removed AFTER UPDATE OF privacy_state ON claim_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_support_stage_removed();
DROP TRIGGER support_snapshot_removed ON repository_snapshots;
CREATE TRIGGER support_snapshot_removed AFTER UPDATE OF privacy_state ON repository_snapshots FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_support_stage_removed();
DROP TRIGGER support_manifest_removed ON manifest_revisions;
CREATE TRIGGER support_manifest_removed AFTER UPDATE OF privacy_state ON manifest_revisions FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_support_stage_removed();
DROP TRIGGER revision_support_changed ON memory_support_assessments;
CREATE TRIGGER revision_support_changed BEFORE INSERT OR UPDATE ON memory_support_assessments FOR EACH ROW
  WHEN (NOT recollect_brain_removing(NEW.brain_id)) EXECUTE FUNCTION recollect_revision_support_changed();

-- The 014 capture redaction matched every Brain event against every fence of
-- the manifest (2,501 events x 1,252 fences took 1.2 s), so it grew with the
-- square of a Brain-wide manifest. The same rows are selected by event
-- identity or by binding and native key, each fence evaluated once.
CREATE OR REPLACE FUNCTION recollect_pre_observation_privacy_apply(rid uuid) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE r privacy_requests;
BEGIN
  SELECT * INTO STRICT r FROM privacy_requests WHERE id=rid;
  PERFORM id FROM brains WHERE id=r.brain_id FOR UPDATE;
  INSERT INTO privacy_capture_fences(brain_id,event_id,binding_id,native_key,request_id)
    SELECT r.brain_id,(f->>'event_id')::uuid,(f->>'binding_id')::uuid,f->>'native_key',r.id
    FROM jsonb_array_elements(coalesce(r.manifest->'capture_event_fences','[]')) f ON CONFLICT DO NOTHING;
  WITH fences AS MATERIALIZED (
    SELECT (f->>'event_id')::uuid AS event_id,(f->>'binding_id')::uuid AS binding_id,f->>'native_key' AS native_key
    FROM jsonb_array_elements(coalesce(r.manifest->'capture_event_fences','[]')) f)
  UPDATE capture_events e SET metadata=NULL,admission_policy=NULL,
    expires_at=least(e.expires_at,recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)),
    state=CASE WHEN r.cause='erase' OR e.state='removed' THEN 'removed' ELSE 'expired' END
    WHERE e.brain_id=r.brain_id AND (e.id IN(SELECT event_id FROM fences)
      OR (e.binding_id,e.native_key) IN(SELECT binding_id,native_key FROM fences));
  PERFORM recollect_autonomous_privacy_apply(rid);
END $$;
REVOKE ALL ON FUNCTION recollect_pre_observation_privacy_apply(uuid) FROM PUBLIC;
