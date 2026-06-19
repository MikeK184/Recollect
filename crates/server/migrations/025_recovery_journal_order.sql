-- Before-statement runs before evaluating identity defaults. Serialize privacy
-- inserts until commit so an exported predecessor cannot later acquire a newly
-- committed lower-sequence neighbor. Rollbacks can still leave harmless numeric
-- gaps; explicit journal replay retains its original identities unchanged.
CREATE FUNCTION recollect_privacy_insert_order() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 PERFORM pg_advisory_xact_lock(73241028);
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_insert_order() FROM PUBLIC;
CREATE TRIGGER privacy_sequence_order BEFORE INSERT ON privacy_requests
 FOR EACH STATEMENT EXECUTE FUNCTION recollect_privacy_insert_order();
