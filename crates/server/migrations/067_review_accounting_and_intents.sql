-- Review follow-up: provider token accounting, deferrable automatic learning,
-- evidence artifact intents and persisted mapping failure reasons.

-- 1. Token accounting. A reservation is an admission estimate, not a charge.
-- Failed or uncertain outcomes without provider usage are charged the
-- estimated input only; a succeeded call without usage keeps the full
-- reservation because its output size is unknown.
ALTER TABLE model_requests ADD COLUMN IF NOT EXISTS estimated_input_tokens bigint CHECK(estimated_input_tokens>0);

CREATE FUNCTION recollect_model_charge(outcome text,tt bigint,it bigint,ot bigint,estimated bigint,reserved bigint)
RETURNS bigint LANGUAGE sql IMMUTABLE SET search_path=public,pg_temp AS $$
  SELECT coalesce(tt,
    CASE WHEN it IS NOT NULL THEN it+coalesce(ot,0) END,
    CASE WHEN outcome<>'succeeded' THEN estimated END,
    reserved)
$$;
REVOKE ALL ON FUNCTION recollect_model_charge(text,bigint,bigint,bigint,bigint,bigint) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_model_charge(text,bigint,bigint,bigint,bigint,bigint) TO recollect_app;

CREATE OR REPLACE FUNCTION recollect_finish_model_request(rid uuid,token uuid,outcome text,reason text,returned text,it bigint,ot bigint,tt bigint,dims integer,cost double precision)
RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  WITH changed AS (
    UPDATE model_requests SET state=outcome,error_code=reason,returned_model=returned,
      input_tokens=it,output_tokens=ot,total_tokens=tt,dimensions=dims,cost_usd=cost,
      charged_tokens=recollect_model_charge(outcome,tt,it,ot,estimated_input_tokens,reserved_tokens),
      finished_at=clock_timestamp()
    WHERE id=rid AND call_token=token AND state='running'
      AND outcome IN ('succeeded','failed','uncertain')
      AND (reason IS NULL OR reason IN ('provider_http','provider_rate_limited','provider_unavailable','provider_timeout','provider_transport','provider_refusal','provider_incomplete','provider_shape','provider_body_limit'))
      AND (returned IS NULL OR length(returned)<=120)
      AND coalesce(it>=0,true) AND coalesce(ot>=0,true) AND coalesce(tt>=0,true)
      AND (cost IS NULL OR (cost>=0 AND cost<1000000))
    RETURNING id
  ) SELECT EXISTS(SELECT 1 FROM changed)
$$;

-- Deadline expiry marks an unrecorded call uncertain without a usage report.
CREATE FUNCTION recollect_model_request_settled() RETURNS trigger
LANGUAGE plpgsql SET search_path=public,pg_temp AS $$
BEGIN
  NEW.charged_tokens:=recollect_model_charge(NEW.state,NEW.total_tokens,NEW.input_tokens,NEW.output_tokens,NEW.estimated_input_tokens,NEW.reserved_tokens);
  RETURN NEW;
END $$;
CREATE TRIGGER model_request_settled BEFORE UPDATE OF state ON model_requests
  FOR EACH ROW WHEN (OLD.state='running' AND NEW.state IN ('failed','uncertain'))
  EXECUTE FUNCTION recollect_model_request_settled();

-- 2. A failed automatic source may be attempted again after a cooldown (the
-- application bounds attempts); a failed run no longer owns the identity.
DROP INDEX learning_automatic_once;
CREATE UNIQUE INDEX learning_automatic_once ON learning_runs(brain_id,source_version_id,policy_id)
  WHERE automatic AND retry_of IS NULL AND state<>'failed';

-- 3. Evidence artifacts: the file path is recorded in its own committed
-- transaction before any bytes are written, so a failed canonical commit can
-- be swept and a Brain-wide erasure still names the file. Rows store only
-- opaque identifiers.
CREATE TABLE evidence_artifact_intents (
 artifact_id uuid PRIMARY KEY,brain_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT clock_timestamp(),removed_at timestamptz
);
ALTER TABLE evidence_artifact_intents ENABLE ROW LEVEL SECURITY;
ALTER TABLE evidence_artifact_intents FORCE ROW LEVEL SECURITY;
CREATE POLICY evidence_intent_read ON evidence_artifact_intents FOR SELECT USING(recollect_role(brain_id) IS NOT NULL);
CREATE POLICY evidence_intent_write ON evidence_artifact_intents FOR INSERT WITH CHECK(
 recollect_role(brain_id) IN ('writer','admin') AND removed_at IS NULL
 AND NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=evidence_artifact_intents.brain_id AND v.artifact_id=evidence_artifact_intents.artifact_id));
CREATE POLICY evidence_intent_delete ON evidence_artifact_intents FOR DELETE USING(
 recollect_role(brain_id) IN ('writer','admin') AND EXISTS(
 SELECT 1 FROM source_versions v WHERE v.brain_id=evidence_artifact_intents.brain_id AND v.artifact_id=evidence_artifact_intents.artifact_id));
GRANT SELECT,INSERT,DELETE ON evidence_artifact_intents TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(992,'evidence_artifact_intents');

-- Files whose canonical row never committed. A 30 minute grace period keeps an
-- in-flight request from losing the file it is about to commit.
CREATE FUNCTION recollect_pending_evidence_artifacts() RETURNS TABLE(brain_id uuid,artifact_id uuid)
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT i.brain_id,i.artifact_id FROM evidence_artifact_intents i
 WHERE recollect_actor() IS NULL AND recollect_device() IS NULL AND i.removed_at IS NULL
  AND i.created_at<clock_timestamp()-interval '30 minutes'
  AND NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=i.brain_id AND v.artifact_id=i.artifact_id)
 ORDER BY i.created_at,i.artifact_id LIMIT 100
$$;
-- The opaque identity survives file removal so a later Brain erasure still
-- scrubs a backup taken before cleanup.
CREATE FUNCTION recollect_forget_evidence_artifact(b uuid,a uuid) RETURNS void
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 UPDATE evidence_artifact_intents i SET removed_at=clock_timestamp() WHERE i.brain_id=b AND i.artifact_id=a
  AND recollect_actor() IS NULL AND recollect_device() IS NULL AND i.removed_at IS NULL
  AND i.created_at<clock_timestamp()-interval '30 minutes'
  AND NOT EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=b AND v.artifact_id=a)
$$;
-- Committed intents are normally deleted by their own transaction.
CREATE FUNCTION recollect_settle_evidence_intents() RETURNS integer
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 WITH settled AS (
  DELETE FROM evidence_artifact_intents i WHERE recollect_actor() IS NULL AND recollect_device() IS NULL
   AND i.created_at<clock_timestamp()-interval '30 minutes'
   AND EXISTS(SELECT 1 FROM source_versions v WHERE v.brain_id=i.brain_id AND v.artifact_id=i.artifact_id)
  RETURNING 1)
 SELECT count(*)::integer FROM settled
$$;
REVOKE ALL ON FUNCTION recollect_pending_evidence_artifacts(),recollect_forget_evidence_artifact(uuid,uuid),recollect_settle_evidence_intents() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_pending_evidence_artifacts(),recollect_forget_evidence_artifact(uuid,uuid),recollect_settle_evidence_intents() TO recollect_app;

ALTER FUNCTION recollect_privacy_closure(uuid,text,uuid,text) RENAME TO recollect_pre_evidence_intent_closure;
CREATE FUNCTION recollect_privacy_closure(b uuid,k text,t uuid,c text) RETURNS jsonb
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE m jsonb; BEGIN
 m:=recollect_pre_evidence_intent_closure(b,k,t,c);
 IF k='brain' THEN
  m:=m||jsonb_build_object('artifacts',coalesce((SELECT jsonb_agg(id ORDER BY id) FROM (
   SELECT unnest(recollect_privacy_ids(m,'artifacts')) id
   UNION SELECT i.artifact_id FROM evidence_artifact_intents i WHERE i.brain_id=b) pending),'[]'::jsonb));
 END IF;
 RETURN m;
END $$;
REVOKE ALL ON FUNCTION recollect_privacy_closure(uuid,text,uuid,text) FROM PUBLIC;

-- 4. Static reason for an invalid mapping batch (a CandidateFailure code or an
-- operational reason); never provider or source text.
ALTER TABLE knowledge_mapping_coverage ADD COLUMN IF NOT EXISTS failure text CHECK(failure ~ '^[a-z_]{1,40}$');

-- 5. A Brain admin's retry must not run other Brains' erasure work.
CREATE FUNCTION recollect_privacy_pending_for(b uuid) RETURNS SETOF privacy_requests
LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
 SELECT * FROM privacy_requests WHERE brain_id=b AND state<>'complete' ORDER BY sequence LIMIT 20
$$;
REVOKE ALL ON FUNCTION recollect_privacy_pending_for(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_privacy_pending_for(uuid) TO recollect_app;
