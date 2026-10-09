-- Preparation is a consistent, read-only snapshot, never a publication gate.
-- Keep ordinary fair admission unchanged and reject mutation in preparation.
CREATE OR REPLACE FUNCTION recollect_lock_brain(target uuid, exclusive boolean) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE admission integer;
BEGIN
  IF recollect_role(target) IS NULL THEN RETURN false; END IF;
  SELECT admission_key INTO admission FROM brains WHERE id=target;
  IF NOT FOUND THEN RETURN false; END IF;
  IF current_setting('recollect.preparation',true)='on' THEN
    IF current_setting('transaction_read_only')<>'on'
      OR current_setting('transaction_isolation')<>'repeatable read' OR exclusive THEN
      RAISE EXCEPTION 'Preparation requires a repeatable-read read-only transaction'
        USING ERRCODE='25006';
    END IF;
    RETURN true;
  END IF;
  IF exclusive THEN
    PERFORM pg_advisory_xact_lock(73241029,admission);
    PERFORM id FROM brains WHERE id=target FOR UPDATE;
  ELSE
    PERFORM pg_advisory_xact_lock_shared(73241029,admission);
    PERFORM id FROM brains WHERE id=target FOR SHARE;
  END IF;
  RETURN FOUND;
END $$;
REVOKE ALL ON FUNCTION recollect_lock_brain(uuid,boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_lock_brain(uuid,boolean) TO recollect_app;

-- These inputs affect canonical support/model eligibility without necessarily
-- changing a claim/source row. They must invalidate provisional graph buffers.
DO $$ DECLARE tab text; BEGIN
  FOREACH tab IN ARRAY ARRAY['model_policy_heads','model_input_fences','model_claim_fences'] LOOP
    EXECUTE format('CREATE TRIGGER analytics_insert AFTER INSERT ON %I REFERENCING NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
    EXECUTE format('CREATE TRIGGER analytics_update AFTER UPDATE ON %I REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
    EXECUTE format('CREATE TRIGGER analytics_delete AFTER DELETE ON %I REFERENCING OLD TABLE AS old_rows FOR EACH STATEMENT EXECUTE FUNCTION recollect_analytics_inputs_changed()',tab);
  END LOOP;
END $$;
