-- Bit signatures for the large-scope semantic shortlist.
--
-- A signature is the sign-bit quantization of a stored embedding
-- (pgvector `binary_quantize`: component > 0 becomes 1). It is derived data:
-- the dimension trigger below recomputes it whenever the embedding, the
-- signature, the profile or the Brain changes, so every writer (publication,
-- reindex, fences, privacy erasure) keeps it consistent and an erased vector
-- (`embedding=NULL`) leaves no signature. It is stored inline (at most 384
-- bytes at 3,072 dimensions) while the 12 KB vector is stored out of line, so
-- a shortlist scan over a scope never reads a vector. No vector index is
-- created: the signature is read only after Brain/scope/eligibility filtering.
--
-- The migration runner applies every pending migration in one transaction
-- under a 10 s statement default. The backfill reads each stored vector once,
-- so it gets an explicit bound sized for large installations, runs in keyset
-- batches over the primary key (every statement keeps its transition tables
-- small and is index-ordered, never an O(n^2) rescan), and the runner's value
-- is restored before the next migration.
SELECT set_config('recollect.migration_statement_timeout',current_setting('statement_timeout'),true);
SET LOCAL statement_timeout='30min';

ALTER TABLE semantic_entries ADD COLUMN signature bit varying;

-- The backfill runs before the trigger changes: the existing trigger fires only
-- for embedding/profile/Brain updates, so this signature-only update does not
-- revalidate every dimension, and the table is locked exclusively by the
-- ALTER above, so no writer can interleave.
DO $$
DECLARE ids uuid[]; last_id uuid;
BEGIN
  LOOP
    SELECT array_agg(id ORDER BY id) INTO ids FROM (
      SELECT id FROM semantic_entries
      WHERE embedding IS NOT NULL AND (last_id IS NULL OR id>last_id)
      ORDER BY id LIMIT 1000) batch;
    EXIT WHEN ids IS NULL;
    UPDATE semantic_entries SET signature=binary_quantize(embedding) WHERE id=ANY(ids);
    last_id:=ids[array_upper(ids,1)];
  END LOOP;
END $$;

CREATE OR REPLACE FUNCTION recollect_semantic_dimensions() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
DECLARE expected integer;
BEGIN
  IF NEW.embedding IS NOT NULL THEN
    SELECT dimensions INTO expected FROM semantic_profiles
      WHERE id=NEW.profile_id AND brain_id=NEW.brain_id;
    IF expected IS NULL OR vector_dims(NEW.embedding)<>expected THEN
      RAISE EXCEPTION 'semantic profile dimension mismatch' USING ERRCODE='23514';
    END IF;
    NEW.signature:=binary_quantize(NEW.embedding);
  ELSE
    NEW.signature:=NULL;
  END IF;
  RETURN NEW;
END $$;
DROP TRIGGER semantic_dimensions ON semantic_entries;
CREATE TRIGGER semantic_dimensions BEFORE INSERT OR UPDATE OF embedding,signature,profile_id,brain_id
  ON semantic_entries FOR EACH ROW EXECUTE FUNCTION recollect_semantic_dimensions();

ALTER TABLE semantic_entries ADD CONSTRAINT semantic_signature_matches_embedding
  CHECK((embedding IS NULL)=(signature IS NULL));

SELECT set_config('statement_timeout',current_setting('recollect.migration_statement_timeout'),true);
