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
    collections::VecDeque,
    convert::Infallible,
    time::{Duration as StdDuration, Instant},
};

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
    let first = stream_inbox(&state, &auth, brain, id, query).await?;
    let pending = frames(&first);
    let cursor = first.next_cursor;
    let output = stream::unfold(
        (state, auth, brain, id, cursor, pending, Instant::now()),
        |(state, auth, brain, id, mut cursor, mut pending, start)| async move {
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
                    return Some((Ok(event), (state, auth, brain, id, cursor, pending, start)));
                }
                cursor = next.next_cursor;
                pending = frames(&next);
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (state, auth, brain, id, cursor, pending, start)));
                }
                tokio::time::sleep(StdDuration::from_millis(500)).await;
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
    let mut guard = tx(state, auth, brain, false).await?;
    owned(&mut guard, auth, brain, id, true).await?;
    guard.commit().await?;
    let page = read_inbox(state, auth, brain, id, query).await?;
    if page.gap && json_size(&page)? > 4 * 1024 * 1024 - 512 {
        return Err(response_limit());
    }
    Ok(page)
}
