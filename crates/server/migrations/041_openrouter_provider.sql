-- Historical requests used the direct OpenAI adapter. Provider identity is
-- explicit for new attempts and remains attached after a Brain changes policy.
ALTER TABLE model_requests ADD COLUMN provider text NOT NULL DEFAULT 'openai'
    CHECK (provider IN ('openai','openrouter'));
ALTER TABLE model_requests ADD COLUMN cost_usd double precision
    CHECK (cost_usd >= 0 AND cost_usd < 1000000);
ALTER TABLE semantic_profiles DROP CONSTRAINT semantic_profiles_dimensions_check;
ALTER TABLE semantic_profiles ADD CONSTRAINT semantic_profiles_dimensions_check
    CHECK (dimensions BETWEEN 1 AND 4096);

-- Cost is recorded atomically with the existing one-attempt completion fence.
CREATE FUNCTION recollect_finish_model_request(rid uuid,token uuid,outcome text,reason text,returned text,it bigint,ot bigint,tt bigint,dims integer,cost double precision)
RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path=public,pg_temp AS $$
  WITH changed AS (
    UPDATE model_requests SET state=outcome,error_code=reason,returned_model=returned,
      input_tokens=it,output_tokens=ot,total_tokens=tt,dimensions=dims,cost_usd=cost,
      charged_tokens=coalesce(tt,reserved_tokens),finished_at=clock_timestamp()
    WHERE id=rid AND call_token=token AND state='running'
      AND outcome IN ('succeeded','failed','uncertain')
      AND (reason IS NULL OR reason IN ('provider_http','provider_rate_limited','provider_unavailable','provider_timeout','provider_transport','provider_refusal','provider_incomplete','provider_shape','provider_body_limit'))
      AND (returned IS NULL OR length(returned)<=120)
      AND coalesce(it>=0,true) AND coalesce(ot>=0,true) AND coalesce(tt>=0,true)
      AND (cost IS NULL OR (cost>=0 AND cost<1000000))
    RETURNING id
  ) SELECT EXISTS(SELECT 1 FROM changed)
$$;
REVOKE ALL ON FUNCTION recollect_finish_model_request(uuid,uuid,text,text,text,bigint,bigint,bigint,integer,double precision) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_finish_model_request(uuid,uuid,text,text,text,bigint,bigint,bigint,integer,double precision) TO recollect_app;
