-- Preserve existing vectors while supporting separate exact generations.
ALTER TABLE semantic_profiles DROP CONSTRAINT semantic_profiles_dimensions_check;
ALTER TABLE semantic_profiles ADD CONSTRAINT semantic_profiles_dimensions_check
    CHECK(dimensions BETWEEN 1 AND 3072);
ALTER TABLE semantic_entries ALTER COLUMN embedding TYPE vector;

CREATE FUNCTION recollect_semantic_dimensions() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
DECLARE expected integer;
BEGIN
  IF NEW.embedding IS NOT NULL THEN
    SELECT dimensions INTO expected FROM semantic_profiles
      WHERE id=NEW.profile_id AND brain_id=NEW.brain_id;
    IF expected IS NULL OR vector_dims(NEW.embedding)<>expected THEN
      RAISE EXCEPTION 'semantic profile dimension mismatch' USING ERRCODE='23514';
    END IF;
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER semantic_dimensions BEFORE INSERT OR UPDATE OF embedding,profile_id,brain_id
  ON semantic_entries FOR EACH ROW EXECUTE FUNCTION recollect_semantic_dimensions();

-- Profiles are immutable identities. Rebuilds create a new row and move the head.
CREATE FUNCTION recollect_semantic_profile_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
  IF (NEW.provider,NEW.model,NEW.dimensions,NEW.representation,NEW.brain_id)
      IS DISTINCT FROM (OLD.provider,OLD.model,OLD.dimensions,OLD.representation,OLD.brain_id) THEN
    RAISE EXCEPTION 'semantic profiles are immutable' USING ERRCODE='23514';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER semantic_profile_immutable BEFORE UPDATE ON semantic_profiles
  FOR EACH ROW EXECUTE FUNCTION recollect_semantic_profile_immutable();
