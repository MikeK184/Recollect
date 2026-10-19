-- One-statement Brain admission. recollect_lock_brain first proves the caller
-- can see the Brain (no lock is taken for an invisible or deleted Brain), then
-- queues in the transaction lock manager and takes the row lock. The role and
-- archive flag are read afterwards in separate plpgsql statements: each takes a
-- fresh READ COMMITTED snapshot, so both reflect whatever committed while this
-- transaction waited for the lock. A single SQL SELECT would read them with the
-- snapshot taken before the wait. A NULL role means "unavailable" (404).
CREATE OR REPLACE FUNCTION recollect_admit_brain(target uuid, exclusive boolean)
RETURNS TABLE(role text, archived boolean)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE
  effective text;
  is_archived boolean;
BEGIN
  IF NOT recollect_lock_brain(target, exclusive) THEN
    RETURN QUERY SELECT NULL::text, NULL::boolean;
    RETURN;
  END IF;
  effective := recollect_role(target);
  SELECT b.archived INTO is_archived FROM brains b WHERE b.id = target;
  IF effective IS NULL OR NOT FOUND THEN
    RETURN QUERY SELECT NULL::text, NULL::boolean;
    RETURN;
  END IF;
  RETURN QUERY SELECT effective, is_archived;
END $$;
REVOKE ALL ON FUNCTION recollect_admit_brain(uuid,boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_admit_brain(uuid,boolean) TO recollect_app;
