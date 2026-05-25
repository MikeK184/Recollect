ALTER TABLE brains ADD COLUMN graph_link_epoch bigint NOT NULL DEFAULT 0;
ALTER TABLE brains ADD COLUMN graph_discovery_cursor smallint NOT NULL DEFAULT 0
 CHECK(graph_discovery_cursor BETWEEN 0 AND 2);
CREATE FUNCTION recollect_graph_origin_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 UPDATE brains SET graph_link_epoch=graph_link_epoch+1 WHERE id=NEW.brain_id;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION recollect_graph_origin_changed() FROM PUBLIC;
CREATE TRIGGER graph_origin_changed AFTER INSERT ON repository_origins
 FOR EACH ROW EXECUTE FUNCTION recollect_graph_origin_changed();

ALTER TABLE graph_generations DROP CONSTRAINT graph_generations_kind_check;
ALTER TABLE graph_generations ADD CONSTRAINT graph_generations_kind_check
 CHECK(kind IN ('repository','knowledge','combined'));
ALTER TABLE graph_generations ADD COLUMN input_snapshot_ids uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE graph_generations ADD COLUMN descriptor_bytes integer
 GENERATED ALWAYS AS (coalesce(octet_length(descriptor::text),0)) STORED;
ALTER TABLE graph_generations ADD CONSTRAINT graph_combined_inputs
 CHECK((kind='combined')=(cardinality(input_snapshot_ids)>0)
   AND (kind<>'combined' OR cardinality(input_snapshot_ids) BETWEEN 2 AND 100));
CREATE INDEX graph_combined_history ON graph_generations(brain_id,input_snapshot_ids,input_epoch,created_at DESC,id DESC)
 WHERE kind='combined';
