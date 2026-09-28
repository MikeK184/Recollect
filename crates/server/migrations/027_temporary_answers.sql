ALTER TABLE model_requests DROP CONSTRAINT model_requests_purpose_check;
ALTER TABLE model_requests ADD CONSTRAINT model_requests_purpose_check
    CHECK(purpose IN ('extraction','synthesis','embedding','reranking','answering'));

-- Operational identities only. Never put the query, transcript, retrieved
-- context, response, or a content hash in this table.
CREATE TABLE answer_requests (
    id uuid NOT NULL,
    brain_id uuid NOT NULL REFERENCES brains(id),
    actor_id uuid NOT NULL REFERENCES accounts(id),
    call_token uuid NOT NULL,
    state text NOT NULL CHECK(state IN ('retrieving','answering','completed','no_evidence',
        'unavailable','failed','cancelled','stale','uncertain','detail_expired')),
    cancel_requested boolean NOT NULL DEFAULT false,
    policy_id uuid,
    memory_epoch bigint,
    expires_at timestamptz,
    model_request_id uuid,
    failure_code text,
    detail_expired boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    deadline timestamptz NOT NULL DEFAULT clock_timestamp()+interval '150 seconds',
    finished_at timestamptz,
    PRIMARY KEY(brain_id,actor_id,id),
    FOREIGN KEY(model_request_id,brain_id) REFERENCES model_requests(id,brain_id)
);
CREATE INDEX answer_requests_active ON answer_requests(brain_id,deadline)
    WHERE state IN ('retrieving','answering');
ALTER TABLE answer_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE answer_requests FORCE ROW LEVEL SECURITY;
CREATE POLICY own_answers ON answer_requests FOR ALL
    USING(actor_id=recollect_actor() AND recollect_device() IS NULL AND recollect_role(brain_id) IS NOT NULL)
    WITH CHECK(actor_id=recollect_actor() AND recollect_device() IS NULL AND recollect_role(brain_id) IS NOT NULL);
GRANT SELECT,INSERT,UPDATE ON answer_requests TO recollect_app;

-- Admitted work must finish metadata accounting even after access is revoked.
-- A random admission token conveys only this bounded settlement authority.
CREATE FUNCTION recollect_finish_answer(b uuid,a uuid,r uuid,token uuid,outcome text,reason text)
RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
    UPDATE answer_requests SET
      state=CASE WHEN cancel_requested THEN 'cancelled' ELSE outcome END,
      failure_code=CASE WHEN cancel_requested THEN 'answer_cancelled' ELSE reason END,
      finished_at=clock_timestamp()
    WHERE brain_id=b AND actor_id=a AND id=r AND call_token=token
      AND state IN ('retrieving','answering')
      AND outcome IN ('completed','no_evidence','unavailable','failed','cancelled','stale','uncertain')
      AND (reason IS NULL OR reason ~ '^[a-z_]{1,80}$')
$$;
REVOKE ALL ON FUNCTION recollect_finish_answer(uuid,uuid,uuid,uuid,text,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_finish_answer(uuid,uuid,uuid,uuid,text,text) TO recollect_app;

ALTER FUNCTION recollect_expire_model_details() RENAME TO recollect_expire_pre_answer_details;
CREATE FUNCTION recollect_expire_model_details() RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
    PERFORM recollect_expire_pre_answer_details();
    UPDATE answer_requests SET state=CASE WHEN cancel_requested THEN 'cancelled' ELSE 'uncertain' END,
      failure_code='answer_interrupted',finished_at=clock_timestamp()
      WHERE (brain_id,actor_id,id) IN (SELECT brain_id,actor_id,id FROM answer_requests
        WHERE state IN ('retrieving','answering') AND deadline<=clock_timestamp() LIMIT 100);
    UPDATE answer_requests SET state='detail_expired',detail_expired=true,policy_id=NULL,
      memory_epoch=NULL,expires_at=NULL,model_request_id=NULL,failure_code=NULL
      WHERE (brain_id,actor_id,id) IN (SELECT brain_id,actor_id,id FROM answer_requests
        WHERE NOT detail_expired AND state NOT IN ('retrieving','answering')
          AND recollect_retention_deadline(brain_id,'audit',created_at)<=clock_timestamp() LIMIT 100);
END $$;
REVOKE ALL ON FUNCTION recollect_expire_model_details() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_expire_model_details() TO recollect_app;
