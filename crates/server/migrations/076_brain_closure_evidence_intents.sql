-- Brain deletion names the files of the Brain's evidence artifact intents (ADR
-- 0016, contract docs/contracts/platform-brain-deletion.md#closure-computation).
--
-- 067 records an evidence artifact's identity before its bytes are written, so
-- a failed canonical commit leaves an intent that the evidence sweep removes
-- and that "a Brain-wide erasure still names". It added the identities only in
-- a k='brain' branch of recollect_privacy_closure, which the Brain closure
-- never passes: 029 unions the member targets' closures and 074 applies their
-- rules to Brain-wide sets. recollect_brain_remove then deleted the intent
-- rows, so recollect_pending_evidence_artifacts could no longer find an
-- uncommitted upload of a deleted Brain and its file stayed on disk.
--
-- Every intent of the Brain is named, pending or already swept, exactly as the
-- 067 branch wrote: the journal then lets a restore replay remove a file that
-- an older artifact backup brings back, and an absent file counts as removed.
-- This is the one deliberate departure from the iterated per-target union
-- (recollect_iterated_brain_closure). The document, and so the preview counter,
-- changes only for a Brain that holds intent rows, and a new upload between
-- preview and DELETE now requires a fresh preview. The equivalence test in
-- crates/server/tests/brain_deletion.rs compares against that union plus
-- these identities.
--
-- The 039 excerpt intents need no such branch: each one names a committed
-- parent version of the Brain, which the closure already holds. The other
-- dead k='brain' branches (045, 054, 056, 057, 058, 060, 062) name rows only;
-- the apply chain's Brain target and recollect_brain_remove delete those rows.
ALTER FUNCTION recollect_brain_closure(uuid) RENAME TO recollect_pre_evidence_intent_brain_closure;
REVOKE ALL ON FUNCTION recollect_pre_evidence_intent_brain_closure(uuid) FROM PUBLIC, recollect_app;
CREATE FUNCTION recollect_brain_closure(b uuid) RETURNS jsonb
LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb;
BEGIN
  m:=recollect_pre_evidence_intent_brain_closure(b);
  IF m IS NULL THEN RETURN NULL; END IF;
  RETURN m||jsonb_build_object('artifacts',recollect_brain_set(m->'artifacts'||coalesce(
    (SELECT jsonb_agg(i.artifact_id) FROM evidence_artifact_intents i WHERE i.brain_id=b),'[]')));
END $$;
REVOKE ALL ON FUNCTION recollect_brain_closure(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_brain_closure(uuid) TO recollect_app;
