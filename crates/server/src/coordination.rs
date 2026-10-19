//! Brain-scoped presence, matching and durable peer communication (ADR 0023).
pub mod a2a;
mod matching;
pub mod stream;
use crate::{
    AppState,
    auth::Auth,
    db::{
        self,
        admission::{self, Archived, BrainAccess, Lock, Transport},
    },
    error::{Error, Result},
    workspace,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction, types::Json as DbJson};
use std::collections::HashMap;
pub use stream::events;
use uuid::Uuid;
type Tx<'a> = Transaction<'a, Postgres>;
fn conflict() -> Error {
    Error(
        StatusCode::CONFLICT,
        "coordination_conflict",
        "Coordination state changed or an identity was reused. Refresh before retrying.",
    )
}
const PAGE_BYTES: usize = 32 * 1024 * 1024;
fn response_limit() -> Error {
    Error(
        StatusCode::PAYLOAD_TOO_LARGE,
        "coordination_response_limit",
        "Coordination response exceeds its byte limit. Reduce published declarations or use a smaller inbox page.",
    )
}
fn json_size(value: &impl serde::Serialize) -> Result<usize> {
    serde_json::to_vec(value)
        .map(|v| v.len())
        .map_err(|_| response_limit())
}
fn bounded(value: &impl serde::Serialize) -> Result<()> {
    if json_size(value)? > PAGE_BYTES {
        Err(response_limit())
    } else {
        Ok(())
    }
}
fn capacity() -> Error {
    Error(
        StatusCode::TOO_MANY_REQUESTS,
        "coordination_capacity",
        "Coordination capacity reached. Preserve request identity and retry later.",
    )
}
/// Device-only. Reads share the Brain lock and writes take it exclusively;
/// archived Brains are refused for both. The session is re-checked.
fn access(write: bool) -> BrainAccess {
    BrainAccess::READ
        .with_lock(if write { Lock::Exclusive } else { Lock::Shared })
        .with_transport(Transport::Device)
        .with_archived(Archived::Reject(conflict))
        .with_session_recheck()
}
async fn tx<'a>(state: &'a AppState, auth: &Auth, brain: Uuid, write: bool) -> Result<Tx<'a>> {
    Ok(admission::brain_tx(&state.pool, auth, brain, access(write))
        .await?
        .tx)
}
#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    brain_id: Uuid,
    account_id: Uuid,
    device_id: Uuid,
    task_id: Uuid,
    scope_id: Uuid,
    revision: i64,
    input: DbJson<CoordinationSessionInput>,
    projection: DbJson<CoordinationSession>,
    closed: bool,
    last_seen_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    next_sequence: i64,
    eligible: bool,
}
const SESSION: &str =
    "SELECT s.*,recollect_coordination_eligible(s.id) AS eligible FROM coordination_sessions s";
impl SessionRow {
    fn dto(&self, own: bool) -> CoordinationSession {
        let mut p = self.projection.0.clone();
        p.revision = self.revision;
        p.last_seen_at = self.last_seen_at;
        p.expires_at = self.expires_at;
        p.status = if self.closed {
            "closed"
        } else if self.eligible {
            "active"
        } else {
            "expired"
        }
        .into();
        p.task_id = own.then_some(self.task_id);
        p.scope_id = own.then_some(self.scope_id);
        p
    }
    fn owned(&self, auth: &Auth) -> bool {
        self.account_id == auth.user.id && Some(self.device_id) == auth.device_id
    }
}
async fn session(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<SessionRow> {
    sqlx::query_as(&format!("{SESSION} WHERE s.brain_id=$1 AND s.id=$2"))
        .bind(brain)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(Error::missing)
}
async fn owned(
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    id: Uuid,
    live: bool,
) -> Result<SessionRow> {
    let s = session(tx, brain, id).await?;
    if !s.owned(auth) {
        return Err(Error::missing());
    }
    if live && !s.eligible {
        return Err(conflict());
    }
    Ok(s)
}
async fn active(tx: &mut Tx<'_>, brain: Uuid) -> Result<Vec<SessionRow>> {
    Ok(sqlx::query_as(&format!("{SESSION} WHERE s.brain_id=$1 AND recollect_coordination_eligible(s.id) ORDER BY s.id LIMIT 500")).bind(brain).fetch_all(&mut **tx).await?)
}
async fn cleanup(tx: &mut Tx<'_>, brain: Uuid) -> Result<()> {
    // Work is bounded per mutation. No request creates an unbounded sweeper.
    for table in ["coordination_events", "coordination_messages"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE id IN(SELECT id FROM {table} WHERE brain_id=$1 AND expires_at<=now() ORDER BY expires_at LIMIT 500)")).bind(brain).execute(&mut **tx).await?;
    }
    sqlx::query("UPDATE coordination_overlaps SET active=false WHERE brain_id=$1 AND active AND (NOT recollect_coordination_eligible(left_id) OR NOT recollect_coordination_eligible(right_id))").bind(brain).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM coordination_sessions WHERE id IN(SELECT id FROM coordination_sessions WHERE brain_id=$1 AND expires_at<now()-interval '7 days' ORDER BY expires_at LIMIT 50)").bind(brain).execute(&mut **tx).await?;
    Ok(())
}
async fn emit(
    tx: &mut Tx<'_>,
    brain: Uuid,
    recipient: Uuid,
    kind: &str,
    payload: CoordinationPayload,
) -> Result<()> {
    if json_size(&payload)? > 4 * 1024 * 1024 - 512 {
        return Err(response_limit());
    }
    let count: i64 = sqlx::query_scalar("SELECT recollect_coordination_event_count($1)")
        .bind(recipient)
        .fetch_one(&mut **tx)
        .await?;
    if count >= 1000 {
        return Err(capacity());
    }
    let seq:i64=sqlx::query_scalar("UPDATE coordination_sessions SET next_sequence=next_sequence+1 WHERE id=$1 AND brain_id=$2 RETURNING next_sequence").bind(recipient).bind(brain).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO coordination_events(id,brain_id,session_id,sequence,kind,payload,expires_at) VALUES($1,$2,$3,$4,$5,$6,now()+interval '7 days')").bind(Uuid::new_v4()).bind(brain).bind(recipient).bind(seq).bind(kind).bind(DbJson(payload)).execute(&mut **tx).await?;
    Ok(())
}
fn overlap(id: Uuid, a: &SessionRow, b: &SessionRow, r: &matching::Reason) -> CoordinationOverlap {
    CoordinationOverlap {
        id,
        peer: b.dto(false),
        reason: r.reason.clone(),
        paths: r.paths.clone(),
        tokens: r.tokens.clone(),
        basis: r.basis.clone(),
        own_revision: a.revision,
        peer_revision: b.revision,
    }
}
/// `left_id`, `right_id`, `reason_key`, then the overlap `id`, `signature` and `active`.
type KnownOverlap = (Uuid, Uuid, String, Uuid, DbJson<Value>, bool);
async fn matches(
    tx: &mut Tx<'_>,
    brain: Uuid,
    own: &SessionRow,
    notify: bool,
) -> Result<Vec<CoordinationOverlap>> {
    let peers = active(tx, brain).await?;
    // One read of this session's known overlaps replaces a SELECT per peer and
    // reason. Reasons are unique per peer, and each key is written only after
    // it is read, so the snapshot stays exact for the loop.
    let known: Vec<KnownOverlap> = sqlx::query_as("SELECT left_id,right_id,reason_key,id,signature,active FROM coordination_overlaps WHERE brain_id=$1 AND (left_id=$2 OR right_id=$2)").bind(brain).bind(own.id).fetch_all(&mut **tx).await?;
    let mut known: HashMap<_, _> = known
        .into_iter()
        .map(|(l, r, k, id, sig, active)| ((l, r, k), (id, sig, active)))
        .collect();
    let mut result = Vec::new();
    let mut kept = Vec::new();
    for peer in peers {
        if peer.id == own.id {
            continue;
        }
        let a = own.dto(false);
        let b = peer.dto(false);
        for reason in matching::reasons(&a, &b) {
            let (left, right) = if own.id < peer.id {
                (own, &peer)
            } else {
                (&peer, own)
            };
            let previous = known.remove(&(left.id, right.id, reason.key.clone()));
            let relevant = |session: &SessionRow| {
                session
                    .projection
                    .0
                    .intents
                    .iter()
                    .filter(|i| reason.paths.contains(&i.path))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            let signature = json!({"reason":reason,"left_environment":left.projection.0.environment_id,"right_environment":right.projection.0.environment_id,"left_intents":relevant(left),"right_intents":relevant(right)});
            let id = previous.as_ref().map(|p| p.0).unwrap_or_else(Uuid::new_v4);
            // Content revisions, not lease refreshes, cause a new notice.
            let changed = previous.as_ref().is_none_or(|p| !p.2 || p.1.0 != signature);
            kept.push(id);
            if notify {
                sqlx::query("INSERT INTO coordination_overlaps(id,brain_id,left_id,right_id,reason_key,signature,active) VALUES($1,$2,$3,$4,$5,$6,true) ON CONFLICT(left_id,right_id,reason_key) DO UPDATE SET signature=excluded.signature,active=true").bind(id).bind(brain).bind(left.id).bind(right.id).bind(&reason.key).bind(DbJson(signature)).execute(&mut **tx).await?;
                if changed {
                    emit(
                        tx,
                        brain,
                        own.id,
                        "overlap",
                        CoordinationPayload::Overlap(overlap(id, own, &peer, &reason)),
                    )
                    .await?;
                    emit(
                        tx,
                        brain,
                        peer.id,
                        "overlap",
                        CoordinationPayload::Overlap(overlap(id, &peer, own, &reason)),
                    )
                    .await?;
                }
            }
            result.push(overlap(id, own, &peer, &reason));
        }
    }
    if notify {
        sqlx::query("UPDATE coordination_overlaps SET active=false WHERE brain_id=$1 AND (left_id=$2 OR right_id=$2) AND NOT(id=ANY($3))").bind(brain).bind(own.id).bind(kept).execute(&mut **tx).await?;
    }
    Ok(result)
}
#[utoipa::path(post,path="/api/brains/{brain}/coordination/sessions",params(("brain"=Uuid,Path)),request_body=CoordinationSessionInput,responses((status=200,body=CoordinationAnnouncement)))]
pub async fn announce(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(mut input): Json<CoordinationSessionInput>,
) -> Result<Json<CoordinationAnnouncement>> {
    matching::validate(&input)?;
    input.context_repository_ids.sort();
    input.context_repository_ids.dedup();
    let mut tx = tx(&state, &auth, brain, true).await?;
    cleanup(&mut tx, brain).await?;
    let row:Option<(Uuid,Option<Uuid>,bool,Uuid,DbJson<ScopeSnapshot>)>=sqlx::query_as("SELECT t.account_id,t.device_id,t.closed,t.current_scope,s.snapshot FROM workspace_tasks t JOIN scope_snapshots s ON s.id=t.current_scope WHERE t.id=$1 AND t.brain_id=$2").bind(input.task_id).bind(brain).fetch_optional(&mut *tx).await?;
    let (account, device, closed, current, scope) = row.ok_or_else(Error::missing)?;
    if account != auth.user.id || device != auth.device_id {
        return Err(Error::missing());
    }
    if closed
        || current != input.expected_scope
        || !workspace::selection_valid(&mut tx, brain, &scope.0.selection).await?
    {
        return Err(conflict());
    }
    for repository in &input.context_repository_ids {
        if !scope.0.selection.repository_ids.is_empty()
            && !scope.0.selection.repository_ids.contains(repository)
        {
            return Err(Error::invalid(
                "Context repository is outside this task scope.",
            ));
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM repositories WHERE id=$1 AND brain_id=$2)",
        )
        .bind(repository)
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(Error::missing());
        }
    }
    for intent in &input.intents {
        if !scope.0.selection.repository_ids.is_empty()
            && !scope
                .0
                .selection
                .repository_ids
                .contains(&intent.repository_id)
        {
            return Err(Error::invalid(
                "Intent repository is outside this task scope.",
            ));
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM repositories WHERE id=$1 AND brain_id=$2)",
        )
        .bind(intent.repository_id)
        .bind(brain)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(Error::missing());
        }
    }
    let previous: Option<SessionRow> = sqlx::query_as(&format!("{SESSION} WHERE s.id=$1"))
        .bind(input.id)
        .fetch_optional(&mut *tx)
        .await?;
    let mut revision = 1;
    if let Some(old) = previous.as_ref() {
        if old.brain_id != brain || !old.owned(&auth) || old.task_id != input.task_id || old.closed
        {
            return Err(conflict());
        }
        let mut comparable = input.clone();
        comparable.expected_revision = old.input.0.expected_revision;
        if comparable == old.input.0 {
            revision = old.revision;
        } else if input.expected_revision == Some(old.revision) {
            revision = old.revision + 1;
        } else {
            return Err(conflict());
        }
    } else {
        if input.expected_revision.is_some() {
            return Err(conflict());
        }
    }
    if previous.as_ref().is_none_or(|old| !old.eligible) {
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM coordination_sessions WHERE brain_id=$1 AND account_id=$2 AND id<>$3 AND NOT closed AND expires_at>now()").bind(brain).bind(auth.user.id).bind(input.id).fetch_one(&mut *tx).await?;
        let total:i64=sqlx::query_scalar("SELECT count(*) FROM coordination_sessions WHERE brain_id=$1 AND id<>$2 AND NOT closed AND expires_at>now()").bind(brain).bind(input.id).fetch_one(&mut *tx).await?;
        if count >= 100 || total >= 500 {
            return Err(capacity());
        }
    }
    let device_id = auth.device_id.ok_or_else(Error::forbidden)?;
    let device_name: String = sqlx::query_scalar("SELECT name FROM devices WHERE id=$1")
        .bind(device_id)
        .fetch_one(&mut *tx)
        .await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let environment_name = scope.0.environment.as_ref().map(|v| v.name.clone());
    let selection = scope.0.selection;
    let projection = CoordinationSession {
        id: input.id,
        revision,
        account_id: auth.user.id,
        account_name: auth.user.username.clone(),
        device_id,
        device_name,
        host_kind: input.host_kind.clone(),
        label: input.label.clone(),
        goal: input.goal.clone(),
        topics: input.topics.clone(),
        intents: input.intents.clone(),
        environment_id: selection.environment_id,
        environment_name,
        repository_ids: if selection.repository_ids.is_empty() {
            input.context_repository_ids.clone()
        } else {
            selection.repository_ids
        },
        area_ids: selection.area_ids,
        status: "active".into(),
        last_seen_at: now,
        expires_at: now + Duration::seconds(90),
        task_id: None,
        scope_id: None,
    };
    sqlx::query("INSERT INTO coordination_sessions(id,brain_id,account_id,device_id,task_id,scope_id,revision,input,projection,last_seen_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(id) DO UPDATE SET scope_id=excluded.scope_id,revision=excluded.revision,input=excluded.input,projection=excluded.projection,last_seen_at=excluded.last_seen_at,expires_at=excluded.expires_at")
 .bind(input.id).bind(brain).bind(auth.user.id).bind(device_id).bind(input.task_id).bind(input.expected_scope).bind(revision).bind(DbJson(&input)).bind(DbJson(projection)).bind(now).bind(now+Duration::seconds(90)).execute(&mut *tx).await.map_err(mutation_error)?;
    let own = session(&mut tx, brain, input.id).await?;
    let overlaps = matches(&mut tx, brain, &own, true).await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "coordination.announce",
        input.id,
        "published",
    )
    .await?;
    let out = CoordinationAnnouncement {
        session: own.dto(true),
        overlaps,
    };
    bounded(&out)?;
    tx.commit().await?;
    Ok(Json(out))
}
fn mutation_error(e: sqlx::Error) -> Error {
    if e.as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        conflict()
    } else {
        e.into()
    }
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerQuery {
    pub session_id: Option<Uuid>,
}
#[utoipa::path(get,path="/api/brains/{brain}/coordination/sessions",params(("brain"=Uuid,Path),("session_id"=Option<Uuid>,Query)),responses((status=200,body=CoordinationPeers)))]
pub async fn peers(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<PeerQuery>,
) -> Result<Json<CoordinationPeers>> {
    let mut tx = tx(&state, &auth, brain, false).await?;
    let overlaps = if let Some(id) = query.session_id {
        let own = owned(&mut tx, &auth, brain, id, true).await?;
        matches(&mut tx, brain, &own, false).await?
    } else {
        vec![]
    };
    let sessions = active(&mut tx, brain)
        .await?
        .iter()
        .map(|s| s.dto(false))
        .collect();
    let out = CoordinationPeers { sessions, overlaps };
    bounded(&out)?;
    tx.commit().await?;
    Ok(Json(out))
}
#[utoipa::path(post,path="/api/brains/{brain}/coordination/sessions/{id}/heartbeat",params(("brain"=Uuid,Path),("id"=Uuid,Path)),request_body=CoordinationHeartbeat,responses((status=200,body=CoordinationSession)))]
pub async fn heartbeat(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CoordinationHeartbeat>,
) -> Result<Json<CoordinationSession>> {
    // A one-row lease refresh. announce and close hold the exclusive Brain lock
    // and are the only writers of the revision and closed state it checks, so the
    // shared lock is enough; concurrent heartbeats serialize on the session row.
    let mut tx = tx(&state, &auth, brain, false).await?;
    let own = owned(&mut tx, &auth, brain, id, true).await?;
    if own.revision != input.expected_revision {
        return Err(conflict());
    }
    sqlx::query("UPDATE coordination_sessions SET last_seen_at=clock_timestamp(),expires_at=clock_timestamp()+interval '90 seconds' WHERE id=$1").bind(id).execute(&mut *tx).await?;
    let out = session(&mut tx, brain, id).await?.dto(true);
    tx.commit().await?;
    Ok(Json(out))
}
#[utoipa::path(post,path="/api/brains/{brain}/coordination/sessions/{id}/close",params(("brain"=Uuid,Path),("id"=Uuid,Path)),responses((status=200,body=Value)))]
pub async fn close(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let mut tx = tx(&state, &auth, brain, true).await?;
    let own = owned(&mut tx, &auth, brain, id, false).await?;
    if !own.closed {
        let recipients:Vec<Uuid>=sqlx::query_scalar("SELECT CASE WHEN left_id=$2 THEN right_id ELSE left_id END FROM coordination_overlaps WHERE brain_id=$1 AND active AND (left_id=$2 OR right_id=$2)").bind(brain).bind(id).fetch_all(&mut *tx).await?;
        sqlx::query(
            "UPDATE coordination_sessions SET closed=true,expires_at=clock_timestamp() WHERE id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        let peer = session(&mut tx, brain, id).await?.dto(false);
        let recipients: std::collections::BTreeSet<_> = recipients.into_iter().collect();
        for recipient in recipients {
            if session(&mut tx, brain, recipient).await?.eligible {
                emit(
                    &mut tx,
                    brain,
                    recipient,
                    "peer_closed",
                    CoordinationPayload::PeerClosed(CoordinationPeerClosed { peer: peer.clone() }),
                )
                .await?
            }
        }
        sqlx::query(
            "UPDATE coordination_overlaps SET active=false WHERE left_id=$1 OR right_id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        db::audit(
            &mut tx,
            auth.user.id,
            brain,
            "coordination.close",
            id,
            "closed",
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"state":"closed"})))
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxQuery {
    pub after: Option<i64>,
    pub limit: Option<i64>,
}
#[derive(sqlx::FromRow)]
struct EventRow {
    sequence: i64,
    id: Uuid,
    kind: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    payload: DbJson<CoordinationPayload>,
}
#[utoipa::path(get,path="/api/brains/{brain}/coordination/sessions/{id}/inbox",params(("brain"=Uuid,Path),("id"=Uuid,Path),("after"=Option<i64>,Query),("limit"=Option<i64>,Query)),responses((status=200,body=CoordinationInbox)))]
pub async fn inbox(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    Query(query): Query<InboxQuery>,
) -> Result<Json<CoordinationInbox>> {
    Ok(Json(
        read_inbox(&state, &auth, brain, id, query, false).await?,
    ))
}
async fn read_inbox(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    id: Uuid,
    query: InboxQuery,
    live: bool,
) -> Result<CoordinationInbox> {
    let after = query.after.unwrap_or(0);
    let limit = query.limit.unwrap_or(50);
    if after < 0 || !(1..=100).contains(&limit) {
        return Err(Error::invalid(
            "Use a nonnegative cursor and a limit from 1 to 100.",
        ));
    }
    let mut tx = tx(state, auth, brain, false).await?;
    let own = owned(&mut tx, auth, brain, id, live).await?;
    if after > own.next_sequence {
        return Err(Error::invalid("Cursor is ahead of this inbox."));
    }
    let rows:Vec<EventRow>=sqlx::query_as("SELECT sequence,id,kind,created_at,expires_at,payload FROM coordination_events WHERE brain_id=$1 AND session_id=$2 AND sequence>$3 AND expires_at>clock_timestamp() ORDER BY sequence LIMIT $4").bind(brain).bind(id).bind(after).bind(limit).fetch_all(&mut *tx).await?;
    let gap = rows
        .first()
        .map(|r| r.sequence > after + 1)
        .unwrap_or(own.next_sequence > after);
    let mut next_cursor = rows
        .last()
        .map(|r| r.sequence)
        .unwrap_or(own.next_sequence.max(after));
    let mut events: Vec<_> = rows
        .into_iter()
        .map(|r| CoordinationEvent {
            sequence: r.sequence,
            id: r.id,
            kind: r.kind,
            created_at: r.created_at,
            expires_at: r.expires_at,
            payload: r.payload.0,
        })
        .collect();
    for event in &mut events {
        match &mut event.payload {
            CoordinationPayload::Overlap(v) => refresh_peer(&mut tx, brain, &mut v.peer).await?,
            CoordinationPayload::Message(v) => {
                refresh_peer(&mut tx, brain, &mut v.sender).await?;
                refresh_peer(&mut tx, brain, &mut v.recipient).await?;
            }
            CoordinationPayload::PeerClosed(v) => refresh_peer(&mut tx, brain, &mut v.peer).await?,
        }
    }
    let peers = if gap {
        active(&mut tx, brain)
            .await?
            .iter()
            .map(|s| s.dto(false))
            .collect()
    } else {
        vec![]
    };
    let mut bytes = json_size(&peers)? + 512;
    if bytes > PAGE_BYTES {
        return Err(response_limit());
    }
    let mut included = 0;
    for event in &events {
        let size = json_size(event)? + 1;
        if bytes + size > PAGE_BYTES {
            break;
        }
        bytes += size;
        included += 1;
    }
    if included < events.len() {
        if included == 0 {
            return Err(response_limit());
        }
        events.truncate(included);
        next_cursor = events.last().map(|e| e.sequence).unwrap_or(after);
    }
    tx.commit().await?;
    Ok(CoordinationInbox {
        events,
        next_cursor,
        gap,
        peers,
    })
}
#[utoipa::path(post,path="/api/brains/{brain}/coordination/sessions/{id}/inbox/{event}/ack",params(("brain"=Uuid,Path),("id"=Uuid,Path),("event"=Uuid,Path)),responses((status=200,body=Value)))]
pub async fn ack(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id, event)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let mut tx = tx(&state, &auth, brain, true).await?;
    owned(&mut tx, &auth, brain, id, false).await?;
    let changed=sqlx::query("UPDATE coordination_events SET acknowledged_at=coalesce(acknowledged_at,clock_timestamp()) WHERE brain_id=$1 AND session_id=$2 AND id=$3 AND expires_at>clock_timestamp()").bind(brain).bind(id).bind(event).execute(&mut *tx).await?.rows_affected();
    if changed == 0 {
        return Err(Error::missing());
    }
    tx.commit().await?;
    Ok(Json(json!({"state":"acknowledged"})))
}
#[utoipa::path(post,path="/api/brains/{brain}/coordination/messages",params(("brain"=Uuid,Path)),request_body=CoordinationMessageInput,responses((status=200,body=CoordinationMessage)))]
pub async fn send(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Json(input): Json<CoordinationMessageInput>,
) -> Result<Json<CoordinationMessage>> {
    Ok(Json(send_message(&state, &auth, brain, input).await?))
}
pub(super) async fn send_message(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    input: CoordinationMessageInput,
) -> Result<CoordinationMessage> {
    if !matching::text(&input.text, 4000, false)
        || !matches!(
            input.kind.as_str(),
            "note" | "proposal" | "accept" | "decline"
        )
        || input.from_session_id == input.to_session_id
    {
        return Err(Error::invalid(
            "Use bounded printable peer text and note/proposal/accept/decline.",
        ));
    }
    let mut tx = tx(state, auth, brain, true).await?;
    cleanup(&mut tx, brain).await?;
    let sender = owned(&mut tx, auth, brain, input.from_session_id, true).await?;
    let recipient = session(&mut tx, brain, input.to_session_id).await?;
    if !recipient.eligible {
        return Err(Error::missing());
    }
    let prior:Option<(DbJson<CoordinationMessageInput>,DbJson<CoordinationMessage>)>=sqlx::query_as("SELECT input,payload FROM coordination_messages WHERE id=$1 AND brain_id=$2 AND expires_at>now()").bind(input.id).bind(brain).fetch_optional(&mut *tx).await?;
    if let Some((old, receipt)) = prior {
        if old.0 != input {
            return Err(conflict());
        }
        tx.commit().await?;
        return Ok(receipt.0);
    }
    if matches!(input.kind.as_str(), "accept" | "decline") && input.reply_to.is_none() {
        return Err(Error::invalid(
            "Accept/decline requires the other participant's proposal.",
        ));
    }
    if let Some(reply) = input.reply_to {
        let prior:DbJson<CoordinationMessage>=sqlx::query_scalar("SELECT payload FROM coordination_messages WHERE id=$1 AND brain_id=$2 AND expires_at>now()").bind(reply).bind(brain).fetch_optional(&mut *tx).await?.ok_or_else(Error::missing)?;
        let same = (prior.0.from_session_id == input.from_session_id
            && prior.0.to_session_id == input.to_session_id)
            || (prior.0.from_session_id == input.to_session_id
                && prior.0.to_session_id == input.from_session_id);
        if !same
            || matches!(input.kind.as_str(), "accept" | "decline")
                && (prior.0.kind != "proposal" || prior.0.to_session_id != input.from_session_id)
        {
            return Err(Error::invalid(
                "Reply must address a retained message between these participants.",
            ));
        }
    }
    let rate:i64=sqlx::query_scalar("SELECT count(*) FROM coordination_messages WHERE from_session_id=$1 AND created_at>now()-interval '1 minute'").bind(sender.id).fetch_one(&mut *tx).await?;
    if rate >= 60 {
        return Err(capacity());
    }
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let message = CoordinationMessage {
        id: input.id,
        from_session_id: sender.id,
        to_session_id: recipient.id,
        kind: input.kind.clone(),
        text: input.text.clone(),
        reply_to: input.reply_to,
        sender: sender.dto(false),
        recipient: recipient.dto(false),
        created_at: now,
        expires_at: now + Duration::days(7),
    };
    sqlx::query("INSERT INTO coordination_messages(id,brain_id,from_session_id,to_session_id,input,payload,created_at,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(input.id).bind(brain).bind(sender.id).bind(recipient.id).bind(DbJson(input)).bind(DbJson(&message)).bind(now).bind(message.expires_at).execute(&mut *tx).await.map_err(mutation_error)?;
    emit(
        &mut tx,
        brain,
        recipient.id,
        "message",
        CoordinationPayload::Message(message.clone()),
    )
    .await?;
    db::audit(
        &mut tx,
        auth.user.id,
        brain,
        "coordination.message",
        message.id,
        "queued",
    )
    .await?;
    tx.commit().await?;
    Ok(message)
}

async fn refresh_peer(tx: &mut Tx<'_>, brain: Uuid, peer: &mut CoordinationSession) -> Result<()> {
    let current = session(tx, brain, peer.id).await?;
    peer.status = current.dto(false).status;
    Ok(())
}
