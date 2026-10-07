ALTER TABLE workspace_tasks ADD COLUMN continuation_of_task_id uuid;
ALTER TABLE workspace_tasks ADD CONSTRAINT task_continuation_owner
 FOREIGN KEY(continuation_of_task_id,brain_id,account_id) REFERENCES workspace_tasks(id,brain_id,account_id);
CREATE FUNCTION recollect_task_continuation_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
 IF TG_OP='UPDATE' AND NEW.continuation_of_task_id IS DISTINCT FROM OLD.continuation_of_task_id THEN RAISE EXCEPTION 'immutable task continuation'; END IF;
 IF TG_OP='INSERT' AND NEW.continuation_of_task_id IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM workspace_tasks t WHERE t.id=NEW.continuation_of_task_id AND t.brain_id=NEW.brain_id AND t.account_id=NEW.account_id
   AND t.device_id IS NOT DISTINCT FROM NEW.device_id AND t.closed) THEN RAISE EXCEPTION 'invalid task continuation'; END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_task_continuation_immutable() FROM PUBLIC;
CREATE TRIGGER task_continuation_immutable BEFORE INSERT OR UPDATE ON workspace_tasks FOR EACH ROW EXECUTE FUNCTION recollect_task_continuation_immutable();
