-- Content-free admission order; clock movement and delayed provider responses
-- cannot turn an older request into the newest interpretation generation.
CREATE SEQUENCE knowledge_mapping_request_ordinal AS bigint;
CREATE TABLE knowledge_mapping_request_order (
 request_id uuid PRIMARY KEY,
 brain_id uuid NOT NULL REFERENCES brains(id),
 ordinal bigint NOT NULL UNIQUE,
 FOREIGN KEY(request_id,brain_id) REFERENCES model_requests(id,brain_id) ON DELETE CASCADE
);
INSERT INTO knowledge_mapping_request_order(request_id,brain_id,ordinal)
 SELECT id,brain_id,nextval('knowledge_mapping_request_ordinal') FROM model_requests ORDER BY created_at,id;
ALTER TABLE knowledge_mapping_request_order ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_mapping_request_order FORCE ROW LEVEL SECURITY;
CREATE POLICY mapping_request_order_read ON knowledge_mapping_request_order FOR SELECT
 USING(brain_id IN(SELECT id FROM brains));
GRANT SELECT ON knowledge_mapping_request_order TO recollect_app;
INSERT INTO brain_deletion_dependents(ordinal,relation) VALUES(606,'knowledge_mapping_request_order');
CREATE FUNCTION recollect_mapping_request_admitted() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 INSERT INTO knowledge_mapping_request_order(request_id,brain_id,ordinal)
 VALUES(NEW.id,NEW.brain_id,nextval('knowledge_mapping_request_ordinal'));
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_mapping_request_admitted() FROM PUBLIC;
CREATE TRIGGER mapping_request_admitted AFTER INSERT ON model_requests
 FOR EACH ROW EXECUTE FUNCTION recollect_mapping_request_admitted();

CREATE FUNCTION recollect_mapping_superseded(b uuid,q uuid,v uuid,context jsonb) RETURNS boolean
LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
 SELECT NOT EXISTS(SELECT 1 FROM knowledge_mapping_request_order WHERE brain_id=b AND request_id=q)
 OR EXISTS(SELECT 1 FROM knowledge_mapping_inputs i
  JOIN knowledge_mapping_contexts c ON c.brain_id=i.brain_id AND c.id=i.context_id
  JOIN knowledge_mapping_request_order newer ON newer.brain_id=i.brain_id AND newer.request_id=i.producer_request_id
  JOIN knowledge_mapping_request_order current_order ON current_order.brain_id=b AND current_order.request_id=q
  WHERE i.brain_id=b AND i.source_version_id=v AND i.state='published' AND c.context=$4 AND newer.ordinal>current_order.ordinal)
$$;
REVOKE ALL ON FUNCTION recollect_mapping_superseded(uuid,uuid,uuid,jsonb) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_mapping_superseded(uuid,uuid,uuid,jsonb) TO recollect_app;
ALTER TABLE knowledge_mapping_coverage DROP CONSTRAINT knowledge_mapping_coverage_state_check;
ALTER TABLE knowledge_mapping_coverage ADD CHECK(state IN ('missing','invalid','staged','blocked_profile','blocked_capacity','superseded','published'));
