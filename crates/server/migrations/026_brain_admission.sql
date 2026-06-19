-- Tuple share lockers can bypass a waiting tuple writer. Queue admission in
-- PostgreSQL's transaction lock manager first, while retaining the row lock
-- and authorization as the canonical mutation/read boundary.
ALTER TABLE brains ADD COLUMN admission_key integer GENERATED ALWAYS AS IDENTITY UNIQUE;
GRANT USAGE ON SEQUENCE brains_admission_key_seq TO recollect_app;

CREATE OR REPLACE FUNCTION recollect_lock_brain(target uuid, exclusive boolean) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE admission integer;
BEGIN
  IF recollect_role(target) IS NULL THEN RETURN false; END IF;
  SELECT admission_key INTO admission FROM brains WHERE id=target;
  IF NOT FOUND THEN RETURN false; END IF;
  -- A reserved two-integer namespace is distinct from existing bigint lane
  -- and journal locks. Identity keys need no hashing and survive restoration.
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
