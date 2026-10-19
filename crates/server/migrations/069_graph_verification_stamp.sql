-- Native graph verification stamp.
--
-- A ready generation's Neo4j copy is checked on every read by comparing its
-- sealing token (native_attempt_token) and its stored node/edge counts. This
-- column records when the complete descriptor was last verified against Neo4j:
-- at readiness, by the periodic deep check, or never again after a read saw
-- damage (cleared). It is bookkeeping about native storage, not canonical input.
ALTER TABLE graph_generations ADD COLUMN native_verified_at timestamptz;

-- Stamping is an UPDATE of a ready row, which the statement-level analytics
-- trigger treats as a changed input (epoch bump, cancelled and stale reports,
-- rejected in-flight reads). A stamp-only change must not do that. Everything
-- else is unchanged from migration 020; only the graph_generations UPDATE branch
-- differs. Row comparison happens only for rows that are ready before and after,
-- so state transitions (including readiness) never serialize the descriptor.
CREATE OR REPLACE FUNCTION recollect_analytics_inputs_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
DECLARE affected uuid[];
BEGIN
 IF TG_TABLE_NAME='graph_generations' THEN
  IF TG_OP='INSERT' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM new_rows WHERE state='ready';
  ELSIF TG_OP='DELETE' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM old_rows WHERE state='ready';
  ELSE SELECT array_agg(DISTINCT n.brain_id) INTO affected FROM new_rows n JOIN old_rows o ON o.id=n.id
   WHERE (n.state='ready' OR o.state='ready') AND CASE
     WHEN n.state<>'ready' OR o.state<>'ready' THEN true
     ELSE (to_jsonb(n)-'native_verified_at') IS DISTINCT FROM (to_jsonb(o)-'native_verified_at') END;
  END IF;
 ELSIF TG_OP='INSERT' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM new_rows;
 ELSIF TG_OP='DELETE' THEN SELECT array_agg(DISTINCT brain_id) INTO affected FROM old_rows;
 ELSE SELECT array_agg(DISTINCT brain_id) INTO affected FROM (
  SELECT brain_id FROM new_rows UNION SELECT brain_id FROM old_rows) x;
 END IF;
 IF affected IS NOT NULL THEN PERFORM recollect_analytics_invalidate(affected); END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION recollect_analytics_inputs_changed() FROM PUBLIC;

-- Generations that were already published are treated as verified when they
-- became ready; the first maintenance pass after 24 hours re-checks them.
UPDATE graph_generations SET native_verified_at=published_at WHERE published_at IS NOT NULL;

-- A graph read that failed exact verification cannot rebuild, but it can ask
-- maintenance to re-verify that exact generation first: clear its stamp. Only a
-- ready generation carrying the sealing token the read observed is affected.
CREATE FUNCTION recollect_graph_suspect(b uuid, generation uuid, token uuid) RETURNS boolean
LANGUAGE plpgsql SECURITY DEFINER SET search_path=public,pg_temp AS $$
BEGIN
 PERFORM set_config('lock_timeout','250ms',true);
 UPDATE graph_generations SET native_verified_at=NULL
  WHERE brain_id=b AND id=generation AND state='ready'
   AND native_attempt_token IS NOT DISTINCT FROM token AND native_verified_at IS NOT NULL;
 RETURN FOUND;
END $$;
REVOKE ALL ON FUNCTION recollect_graph_suspect(uuid,uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION recollect_graph_suspect(uuid,uuid,uuid) TO recollect_app;
