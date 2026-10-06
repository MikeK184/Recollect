-- Active inputs are selected independently of the recent window, so an old
-- running source cannot disappear behind newer completed captures.
WITH recent_versions AS (
    SELECT id FROM source_versions WHERE brain_id=$1 AND privacy_state='active'
    AND coalesce(recollect_retention_deadline(brain_id,retention_class,created_at)>clock_timestamp(),true)
    ORDER BY created_at DESC,id DESC LIMIT 31
), recent_events AS (
    SELECT * FROM capture_events WHERE brain_id=$1 AND state='accepted'
    AND recollect_retention_deadline(brain_id,retention_class,captured_at)>clock_timestamp()
    ORDER BY received_at DESC,id DESC LIMIT 31
), candidates AS (
    SELECT id FROM recent_versions
    UNION SELECT source_version_id FROM recent_events WHERE source_version_id IS NOT NULL
    UNION SELECT target_id FROM jobs WHERE brain_id=$1 AND kind='source.process' AND state IN ('queued','running')
    UNION SELECT r.source_version_id FROM learning_runs r JOIN jobs j ON j.id=r.job_id AND j.brain_id=r.brain_id
      WHERE r.brain_id=$1 AND j.state IN ('queued','running') AND r.state IN ('queued','running')
    -- A recent retry/completion of an old version is also recent activity.
    UNION SELECT source_version_id FROM (
      SELECT source_version_id FROM learning_runs WHERE brain_id=$1 AND state<>'removed'
      ORDER BY coalesce(finished_at,created_at) DESC,id DESC LIMIT 31
    ) recent_runs
    UNION SELECT target_id FROM (SELECT target_id FROM jobs WHERE brain_id=$1 AND kind='source.process' ORDER BY updated_at DESC,id DESC LIMIT 31) recent_processing
) , inputs AS (
    SELECT coalesce(e.id,v.id) id,e.id capture_id,e.binding_id,v.source_id,v.id source_version_id,
      coalesce(b.actor_id,v.created_by) actor_id,coalesce(b.device_id,v.device_id) device_id,
      b.host,e.metadata,v.title,coalesce(e.received_at,v.created_at) received_at,
      least(recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at),
        recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)) expires_at,
      coalesce(e.metadata->>'kind',v.retention_class) kind,v.processing
    FROM source_versions v JOIN candidates c ON c.id=v.id
    LEFT JOIN capture_events e ON e.brain_id=v.brain_id AND e.source_version_id=v.id
    LEFT JOIN capture_bindings b ON b.id=e.binding_id AND b.brain_id=e.brain_id
    WHERE v.brain_id=$1 AND v.privacy_state='active'
      AND coalesce(recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)>clock_timestamp(),true)
      AND (e.id IS NULL OR (e.state='accepted' AND recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at)>clock_timestamp()))
    UNION ALL
    SELECT e.id,e.id,e.binding_id,NULL::uuid,NULL::uuid,b.actor_id,b.device_id,b.host,e.metadata,
      coalesce(e.metadata->>'host_event','Capture received'),e.received_at,
      recollect_retention_deadline(e.brain_id,e.retention_class,e.captured_at),
      coalesce(e.metadata->>'kind','lifecycle'),'no_source'
    FROM recent_events e JOIN capture_bindings b ON b.id=e.binding_id AND b.brain_id=e.brain_id
    WHERE e.source_version_id IS NULL
), joined AS (
    SELECT i.*,a.username contributor,d.name agent_name,coalesce(i.host,d.host_kind) resolved_host,
      CASE WHEN p.id IS NOT NULL THEN jsonb_build_object('id',p.id,'state',p.state,
        'updated_at',p.updated_at,'lease_until',p.lease_until,'error_code',p.error_code) END processing_job,
      CASE WHEN l.id IS NOT NULL THEN jsonb_build_object('id',l.id,'state',l.state,
        'created_at',l.created_at,'finished_at',l.finished_at,
        'accepted',l.accepted,'proposed',l.proposed,'blocked',l.blocked,'conflicting',l.conflicting,
        'reused',l.reused,'revised',l.revised,'retired',l.retired,'claim_ids',l.claim_ids,
        'job',jsonb_build_object('id',lj.id,'state',lj.state,'updated_at',lj.updated_at,
          'lease_until',lj.lease_until,'error_code',lj.error_code)) END learning,
      greatest(i.received_at,p.updated_at,lj.updated_at,l.finished_at) activity_at,
      coalesce(p.state IN ('queued','running'),false) OR coalesce(l.state IN ('queued','running') AND lj.state IN ('queued','running'),false) active
    FROM inputs i JOIN accounts a ON a.id=i.actor_id
    -- Device aliases are account metadata; shared published capture host labels
    -- remain usable without broadening device-history access.
    LEFT JOIN devices d ON d.id=i.device_id AND (d.account_id=$2 OR $3)
    LEFT JOIN LATERAL (SELECT * FROM jobs WHERE brain_id=$1 AND target_id=i.source_version_id
      AND kind='source.process' ORDER BY (state IN ('queued','running')) DESC,updated_at DESC,created_at DESC,id DESC LIMIT 1) p ON true
    LEFT JOIN LATERAL (SELECT * FROM learning_runs WHERE brain_id=$1 AND source_version_id=i.source_version_id
      AND state<>'removed' ORDER BY (state IN ('queued','running')) DESC,coalesce(finished_at,created_at) DESC,id DESC LIMIT 1) l ON true
    LEFT JOIN jobs lj ON lj.id=l.job_id AND lj.brain_id=$1
)
SELECT jsonb_build_object('id',id,'capture_id',capture_id,'binding_id',binding_id,
  'source_id',source_id,'source_version_id',source_version_id,'actor_id',actor_id,
  'contributor',contributor,'device_id',device_id,'agent_name',agent_name,'host',resolved_host,
  'host_session_id',metadata->>'host_session_id','agent_id',metadata->>'agent_id',
  'coverage',coalesce(metadata->'coverage','[]'::jsonb),'kind',kind,'tool_name',metadata->>'tool_name',
  'title',title,'received_at',received_at,'activity_at',activity_at,'expires_at',expires_at,
  'processing',processing,'processing_job',processing_job,'learning',learning,'active',active)
FROM joined ORDER BY active DESC,activity_at DESC,id DESC LIMIT 31
