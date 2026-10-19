use super::*;
use axum::{
    http::HeaderMap,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use futures_util::{Stream, stream};
use std::{
    collections::{BTreeMap, VecDeque},
    convert::Infallible,
    sync::Mutex,
    time::{Duration as StdDuration, Instant},
};

/// Each open stream re-checks authorization and reads its inbox on every poll,
/// so the interval and the per-Brain stream count bound that background load.
const POLL: StdDuration = StdDuration::from_secs(2);
const STREAMS_PER_BRAIN: usize = 32;
static STREAMS: Mutex<BTreeMap<Uuid, usize>> = Mutex::new(BTreeMap::new());

/// Held by a stream for its lifetime; dropping it frees the Brain's slot.
struct Slot(&'static Mutex<BTreeMap<Uuid, usize>>, Uuid);
impl Slot {
    fn acquire(brain: Uuid) -> Result<Self> {
        Self::acquire_in(&STREAMS, brain, STREAMS_PER_BRAIN)
    }
    fn acquire_in(
        streams: &'static Mutex<BTreeMap<Uuid, usize>>,
        brain: Uuid,
        limit: usize,
    ) -> Result<Self> {
        let mut open = streams.lock().unwrap_or_else(|e| e.into_inner());
        let count = open.entry(brain).or_default();
        if *count >= limit {
            return Err(capacity());
        }
        *count += 1;
        Ok(Self(streams, brain))
    }
}
impl Drop for Slot {
    fn drop(&mut self) {
        let mut open = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = open.get_mut(&self.1) {
            *count -= 1;
            if *count == 0 {
                open.remove(&self.1);
            }
        }
    }
}

#[utoipa::path(get,path="/api/brains/{brain}/coordination/sessions/{id}/events",params(("brain"=Uuid,Path),("id"=Uuid,Path),("after"=Option<i64>,Query)),responses((status=200,description="Bounded authenticated inbox SSE with cursor replay")))]
pub async fn events(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Query(mut query): Query<InboxQuery>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    if let Some(cursor) = headers.get("last-event-id") {
        query.after = Some(
            cursor
                .to_str()
                .ok()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| Error::invalid("Last-Event-ID must be an inbox sequence."))?,
        )
    }
    query.limit = Some(1);
    let slot = Slot::acquire(brain)?;
    let first = stream_inbox(&state, &auth, brain, id, query).await?;
    let pending = frames(&first);
    let cursor = first.next_cursor;
    let output = stream::unfold(
        (
            state,
            auth,
            brain,
            id,
            cursor,
            pending,
            Instant::now(),
            slot,
        ),
        |(state, auth, brain, id, mut cursor, mut pending, start, slot)| async move {
            loop {
                if start.elapsed() > StdDuration::from_secs(25) {
                    return None;
                }
                // A fresh publication transaction checks account/device/Brain/task scope before each frame.
                let next = match stream_inbox(
                    &state,
                    &auth,
                    brain,
                    id,
                    InboxQuery {
                        after: Some(cursor),
                        limit: Some(1),
                    },
                )
                .await
                {
                    Ok(v) => v,
                    Err(_) => return None,
                };
                if let Some(event) = pending.pop_front() {
                    return Some((
                        Ok(event),
                        (state, auth, brain, id, cursor, pending, start, slot),
                    ));
                }
                cursor = next.next_cursor;
                pending = frames(&next);
                if let Some(event) = pending.pop_front() {
                    return Some((
                        Ok(event),
                        (state, auth, brain, id, cursor, pending, start, slot),
                    ));
                }
                tokio::time::sleep(POLL).await;
            }
        },
    );
    Ok(Sse::new(output).keep_alive(KeepAlive::new().interval(StdDuration::from_secs(10))))
}
fn frames(inbox: &CoordinationInbox) -> VecDeque<Event> {
    let mut frames = VecDeque::new();
    if inbox.gap {
        if let Ok(event) = Event::default().event("gap").json_data(inbox) {
            frames.push_back(event)
        }
    }
    for item in &inbox.events {
        if let Ok(event) = Event::default()
            .id(item.sequence.to_string())
            .event(&item.kind)
            .json_data(item)
        {
            frames.push_back(event)
        }
    }
    frames
}

async fn stream_inbox(
    state: &AppState,
    auth: &Auth,
    brain: Uuid,
    id: Uuid,
    query: InboxQuery,
) -> Result<CoordinationInbox> {
    // One short shared-lock transaction checks live ownership and reads the page.
    let page = read_inbox(state, auth, brain, id, query, true).await?;
    if page.gap && json_size(&page)? > 4 * 1024 * 1024 - 512 {
        return Err(response_limit());
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_STREAMS: Mutex<BTreeMap<Uuid, usize>> = Mutex::new(BTreeMap::new());

    #[test]
    fn polling_is_at_least_two_seconds() {
        assert!(POLL >= StdDuration::from_secs(2));
    }

    #[test]
    fn stream_slots_are_capped_per_brain_and_released_on_drop() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let held: Vec<_> = (0..2)
            .map(|_| {
                Slot::acquire_in(&TEST_STREAMS, a, 2)
                    .ok()
                    .expect("free slot")
            })
            .collect();
        let over = Slot::acquire_in(&TEST_STREAMS, a, 2).err().unwrap();
        assert_eq!(
            (over.0, over.1),
            (StatusCode::TOO_MANY_REQUESTS, "coordination_capacity")
        );
        // Another Brain is unaffected.
        assert!(Slot::acquire_in(&TEST_STREAMS, b, 2).is_ok());
        drop(held);
        assert!(Slot::acquire_in(&TEST_STREAMS, a, 2).is_ok());
    }
}
