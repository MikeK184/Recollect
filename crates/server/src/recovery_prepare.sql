-- A queued snapshot call may already have executed in the lost future. Never
-- redispatch it. Keep its IDs/attempt receipts for receipt-only reconciliation.
WITH fenced AS (
 UPDATE mcp_calls SET state='unknown',code='installation_restored',
 completed_at=clock_timestamp(),payload_expires_at=clock_timestamp()+interval '1 hour',lease_until=NULL
 WHERE state IN ('queued','starting','running') RETURNING *
)
INSERT INTO mutation_audit(id,actor_id,device_id,brain_id,action,target_id,disposition)
 SELECT gen_random_uuid(),actor_id,device_id,brain_id,'mcp.recovery',id,'unknown' FROM fenced;
UPDATE mcp_instances SET state='lost',active_calls=0,updated_at=clock_timestamp()
 WHERE state IN ('starting','ready','draining');
UPDATE mcp_runners SET epoch=gen_random_uuid(),lease_until='epoch';
UPDATE mcp_observation_outbox SET lease_token=NULL,lease_until=NULL,next_attempt_at=clock_timestamp()
 WHERE state IN ('pending','error');

UPDATE model_requests SET state='uncertain',error_code='installation_restored',
 charged_tokens=greatest(charged_tokens,reserved_tokens),finished_at=clock_timestamp(),call_token=gen_random_uuid()
 WHERE state='running';
-- Automatic work admitted before the checkpoint may have billed afterward.
-- Preserve its failed attempt rather than automatically making a second call.
UPDATE jobs SET state='failed',error_code='installation_restored',lease_token=NULL,
 lease_until=NULL,updated_at=clock_timestamp()
 WHERE state IN ('queued','running') AND kind IN ('source.learn','semantic.generate','graph.analyze');
UPDATE analytics_reports SET state='stale',error_code='installation_restored'
 WHERE state='ready';

-- Deterministic local work can resume under a fresh lease and current authority.
UPDATE jobs SET state='queued',lease_token=NULL,lease_until=NULL,not_before=clock_timestamp(),
 updated_at=clock_timestamp() WHERE state='running';
-- Existing canonical graph jobs rebuild their original inputs on the new graph
-- store. Removed/superseded inputs remain removed; publication rechecks rules,
-- authority and input epochs. No analytical scratch graph is replayed.
UPDATE jobs j SET state='queued',attempts=0,progress=0,error_code=NULL,
 lease_token=NULL,lease_until=NULL,not_before=clock_timestamp(),updated_at=clock_timestamp()
 FROM graph_generations g WHERE g.job_id=j.id AND g.state NOT IN ('removed','superseded')
 AND NOT EXISTS(SELECT 1 FROM graph_generations newer WHERE newer.brain_id=g.brain_id
   AND newer.kind=g.kind AND newer.snapshot_id IS NOT DISTINCT FROM g.snapshot_id
   AND newer.input_snapshot_ids=g.input_snapshot_ids
   AND (newer.created_at,newer.id)>(g.created_at,g.id));
UPDATE graph_generations g SET state='queued',descriptor=NULL,published_at=NULL,error_code=NULL,
 node_count=0,edge_count=0,unresolved=0,ambiguous=0,unsupported=0,cleaned_at=NULL
 FROM jobs j WHERE j.id=g.job_id AND j.state='queued';
UPDATE graph_generations SET state='superseded',descriptor=NULL,cleaned_at=clock_timestamp()
 WHERE state IN ('ready','running');
