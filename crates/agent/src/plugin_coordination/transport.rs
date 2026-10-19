use super::*;
use crate::{Client, StoredDevice};
use recollect_protocol::{
    CoordinationAnnouncement, CoordinationInbox, CoordinationIntent, CoordinationMessageInput,
    CoordinationPeers, CoordinationSession, CoordinationSessionInput, WorkspaceCatalogue,
};
use reqwest::Method;
use std::{io::Read, process::Stdio};

fn base(config: &Config) -> String {
    format!("/api/brains/{}/coordination", config.brain)
}
async fn request<T: serde::de::DeserializeOwned>(
    client: &Client,
    device: &StoredDevice,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<T> {
    let mut response = client.send(method, path, Some(device.token), body).await?;
    ensure!(
        response.status().is_success(),
        "coordination_request_rejected:{}",
        response.status().as_u16()
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 32 * 1024 * 1024,
            "coordination_response_too_large"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn wake(root: &Path, id: Uuid) -> Result<()> {
    // Locks, rather than a PID file, survive crashes and suppress duplicate workers.
    let lock = plugin_storage::private_file(&directory(root)?.join(format!("{id}.lock")), false)?;
    if lock.try_lock().is_err() {
        return Ok(());
    }
    drop(lock);
    let mut child = std::process::Command::new(std::env::current_exe()?);
    child
        .args(["coordination-worker", &id.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        child.process_group(0);
    }
    child.spawn()?;
    Ok(())
}

async fn intentions(
    client: &Client,
    device: &StoredDevice,
    desired: &Desired,
) -> Result<(Vec<CoordinationIntent>, Vec<Uuid>)> {
    let catalogue: WorkspaceCatalogue = request(
        client,
        device,
        Method::GET,
        &format!("/api/brains/{}/workspace", desired.config.brain),
        None,
    )
    .await?;
    let mut roots: std::collections::BTreeMap<PathBuf, Option<Uuid>> =
        std::collections::BTreeMap::new();
    let mut context_repositories = vec![];
    if desired.session.scope.selection.repository_ids.is_empty()
        && let Some(root) = desired
            .session
            .setup
            .working_directory
            .ancestors()
            .find(|p| p.join(".git").exists())
    {
        let observed = crate::workspace::observe_checkout(root).await;
        let id = registered_repository(&catalogue.repositories, observed.origin.as_deref());
        if let Some(id) = id {
            context_repositories.push(id);
        }
        roots.insert(root.to_owned(), id);
    }
    let mut result = vec![];
    for declared in &desired.paths {
        // A new file may not exist. Find the nearest existing parent, resolving
        // symlinks there, and append only lexical normal components afterwards.
        let Some((root, relative)) = repository_path(&declared.path) else {
            continue;
        };
        let repository = if let Some(id) = roots.get(&root) {
            *id
        } else {
            let observed = crate::workspace::observe_checkout(&root).await;
            let id = registered_repository(&catalogue.repositories, observed.origin.as_deref());
            roots.insert(root, id);
            id
        };
        if let Some(repository_id) = repository {
            let selected = &desired.session.scope.selection.repository_ids;
            if !selected.is_empty() && !selected.contains(&repository_id) {
                continue;
            }
            let path = relative.to_string_lossy().replace('\\', "/");
            if path.is_empty() || path.chars().count() > 1024 {
                continue;
            }
            result.push(CoordinationIntent {
                repository_id,
                path,
                mode: "write".into(),
                phase: declared.phase.clone(),
            });
        }
    }
    result.sort_by(|a, b| (&a.repository_id, &a.path).cmp(&(&b.repository_id, &b.path)));
    result.dedup_by(|a, b| a.repository_id == b.repository_id && a.path == b.path);
    Ok((result, context_repositories))
}

fn registered_repository(
    repositories: &[recollect_protocol::Repository],
    origin: Option<&str>,
) -> Option<Uuid> {
    let origin = origin?;
    let ids = repositories
        .iter()
        .filter(|r| r.canonical_origin == origin || r.origins.iter().any(|alias| alias == origin))
        .map(|r| r.id)
        .collect::<Vec<_>>();
    (ids.len() == 1).then(|| ids[0])
}

fn repository_path(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut existing = path;
    while !existing.exists() {
        existing = existing.parent()?;
    }
    let suffix = path.strip_prefix(existing).ok()?;
    let resolved = existing.canonicalize().ok()?.join(suffix);
    let parent = resolved.parent()?;
    let root = parent
        .ancestors()
        .find(|p| p.join(".git").exists())?
        .to_owned();
    let relative = resolved.strip_prefix(&root).ok()?.to_owned();
    if relative
        .components()
        .any(|p| !matches!(p, Component::Normal(_)))
    {
        return None;
    }
    Some((root, relative))
}

async fn pull(
    client: &Client,
    device: &StoredDevice,
    config: &Config,
    id: Uuid,
    inbox: &mut Inbox,
) -> Result<()> {
    let page: CoordinationInbox = request(
        client,
        device,
        Method::GET,
        &format!(
            "{}/sessions/{id}/inbox?after={}&limit=10",
            base(config),
            inbox.cursor()?
        ),
        None,
    )
    .await?;
    inbox.receive(&page.events, page.next_cursor)?;
    inbox.put(
        "authorized_until",
        &(Utc::now() + chrono::Duration::seconds(90)),
    )?;
    if page.gap {
        inbox.put("gap", &true)?;
    }
    acknowledge(client, device, config, id, inbox).await
}
async fn acknowledge(
    client: &Client,
    device: &StoredDevice,
    config: &Config,
    id: Uuid,
    inbox: &Inbox,
) -> Result<()> {
    let ids = {
        let mut stmt = inbox
            .db
            .prepare("SELECT id FROM events WHERE acked=0 ORDER BY sequence LIMIT 100")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    for event in ids {
        let _: Value = request(
            client,
            device,
            Method::POST,
            &format!("{}/sessions/{id}/inbox/{event}/ack", base(config)),
            None,
        )
        .await?;
        inbox
            .db
            .execute("UPDATE events SET acked=1 WHERE id=?1", [event])?;
    }
    Ok(())
}

#[derive(Default)]
struct Sse {
    bytes: Vec<u8>,
}
enum Frame {
    Event(Box<CoordinationEvent>),
    Gap(CoordinationInbox),
}

fn display_watch(inbox: &Inbox) -> Result<()> {
    let cursor = inbox.get::<i64>("watch_cursor")?.unwrap_or(0);
    let events = {
        let mut statement = inbox.db.prepare(
            "SELECT body FROM events WHERE sequence>?1 AND expires>?2 ORDER BY sequence LIMIT 100",
        )?;
        statement
            .query_map(params![cursor, Utc::now().to_rfc3339()], |r| {
                r.get::<_, String>(0)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };
    for text in events {
        let event: CoordinationEvent = serde_json::from_str(&text)?;
        eprintln!("{}", human_event(&event));
        inbox.put("watch_cursor", &event.sequence)?;
    }
    Ok(())
}
impl Sse {
    fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Frame>> {
        ensure!(
            self.bytes.len() + bytes.len() <= 4 * 1024 * 1024,
            "coordination_stream_frame_too_large"
        );
        self.bytes.extend_from_slice(bytes);
        let mut events = vec![];
        loop {
            let delimiter = self
                .bytes
                .windows(2)
                .position(|v| v == b"\n\n")
                .map(|i| (i, 2))
                .or_else(|| {
                    self.bytes
                        .windows(4)
                        .position(|v| v == b"\r\n\r\n")
                        .map(|i| (i, 4))
                });
            let Some((end, length)) = delimiter else {
                break;
            };
            let frame = String::from_utf8(self.bytes.drain(..end + length).collect())?;
            let data = frame
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("data:")
                        .map(|v| v.strip_prefix(' ').unwrap_or(v))
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !data.is_empty() {
                let kind = frame
                    .lines()
                    .find_map(|line| line.strip_prefix("event:").map(str::trim));
                events.push(if kind == Some("gap") {
                    Frame::Gap(serde_json::from_str(&data)?)
                } else {
                    Frame::Event(Box::new(serde_json::from_str(&data)?))
                });
            }
        }
        Ok(events)
    }
}

async fn stream(
    client: &Client,
    device: &StoredDevice,
    config: &Config,
    id: Uuid,
    inbox: &mut Inbox,
    generation: Option<Uuid>,
    visible: bool,
) -> Result<()> {
    let mut response = client
        .http
        .get(format!(
            "{}{}/sessions/{id}/events",
            client.endpoint,
            base(config)
        ))
        .bearer_auth(device.token)
        .header("Accept", "text/event-stream")
        .header("Last-Event-ID", inbox.cursor()?.to_string())
        .timeout(Duration::from_secs(28))
        .send()
        .await?;
    ensure!(
        response.status().is_success(),
        "coordination_stream_rejected:{}",
        response.status().as_u16()
    );
    ensure!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.starts_with("text/event-stream")),
        "coordination_stream_invalid"
    );
    inbox.put(
        "transport",
        &json!({"state":"streaming","checked_at":Utc::now()}),
    )?;
    let until = tokio::time::Instant::now() + Duration::from_secs(25);
    let mut parser = Sse::default();
    loop {
        tokio::select! {
            _=tokio::time::sleep_until(until)=>return Ok(()),
            _=tokio::time::sleep(Duration::from_secs(1))=>{
                if visible {display_watch(inbox)?;}
                if generation.is_some() && inbox.get::<Uuid>("generation")?!=generation {return Ok(());}
            }
            result=response.chunk()=>{
                let Some(chunk)=result? else {return Ok(())};
                for frame in parser.feed(&chunk)? {
                    let event=match frame {
                        Frame::Gap(page)=>{
                            inbox.receive(&page.events,page.next_cursor)?;
                            inbox.put("gap",&true)?;
                            inbox.put("gap_peers",&page.peers)?;
                            if visible {eprintln!("Recollect coordination retained-history gap; current peer snapshot is available through peers.");}
                            continue;
                        }
                        Frame::Event(event)=>*event,
                    };
                    inbox.receive(std::slice::from_ref(&event),event.sequence)?;
                    inbox.put("authorized_until", &(Utc::now()+chrono::Duration::seconds(90)))?;
                    if visible {display_watch(inbox)?;}
                }
            }
        }
    }
}

fn destination_current(config: &Config) -> bool {
    plugin_storage::config().is_ok_and(|c| {
        c.endpoint == config.endpoint && c.device == config.device && c.profile == config.profile
    })
}

/// Each worker belongs to one exact native session. A silent/crashed host does
/// not acquire an immortal presence lease: hooks must remain recently observed.
pub async fn worker(id: Uuid) -> Result<Value> {
    let root = plugin_storage::root()?;
    let lock = plugin_storage::private_file(&directory(&root)?.join(format!("{id}.lock")), false)?;
    if lock.try_lock().is_err() {
        return Ok(json!({"state":"already_running"}));
    }
    // The configured Brain can differ from the immutable workspace-selected Brain.
    // Read the binding first; credentials still come from the current matching device.
    let mut config = plugin_storage::config()?;
    let path = directory(&root)?.join(format!("{id}.sqlite"));
    plugin_storage::private_file(&path, false)?;
    let db = Connection::open(&path)?;
    let brain: String = db.query_row("SELECT brain FROM binding", [], |r| r.get(0))?;
    config.brain = brain.parse()?;
    drop(db);
    let mut inbox = Inbox::open(&root, &config, id)?;
    let desired = inbox
        .get::<Desired>("desired")?
        .ok_or_else(|| anyhow!("coordination_native_session_missing"))?;
    config = desired.config.clone();
    ensure!(
        destination_current(&config),
        "coordination_destination_changed"
    );
    let device = crate::plugin_runtime::credential(&config)?;
    let client = Client::new(&config.endpoint)?;
    let mut last_heartbeat = tokio::time::Instant::now() - Duration::from_secs(30);
    let mut failures = 0;
    let mut force_announce = true;
    let mut observed_generation = inbox.get::<Uuid>("generation")?;
    loop {
        observed_generation = inbox.get::<Uuid>("generation")?.or(observed_generation);
        let desired = inbox
            .get::<Desired>("desired")?
            .ok_or_else(|| anyhow!("coordination_native_session_missing"))?;
        if !destination_current(&config) {
            inbox.invalidate("destination_changed")?;
            break;
        }
        if !desired.session.ended && !fresh(&desired) {
            inbox.put("transport",&json!({"state":"host_activity_stale","checked_at":Utc::now(),"note":"Silent tools may outlast observed host coverage; presence expires without recent hooks."}))?;
            break;
        }
        let generation = inbox.get::<Uuid>("generation")?;
        let result = async {
            if desired.session.ended {
                let _: Value = request(
                    &client,
                    &device,
                    Method::POST,
                    &format!("{}/sessions/{id}/close", base(&config)),
                    None,
                )
                .await?;
                return Ok::<bool, anyhow::Error>(true);
            }
            let previous = inbox.get::<CoordinationSessionInput>("announced")?;
            let mut input = CoordinationSessionInput {
                id,
                task_id: desired.session.setup.task_id,
                expected_scope: desired.session.scope.id,
                expected_revision: inbox.get("revision")?,
                host_kind: desired.session.setup.host.clone(),
                label: short(&format!("{} session", desired.session.setup.host), 120),
                goal: desired.goal.clone(),
                topics: vec![],
                context_repository_ids: previous
                    .as_ref()
                    .map(|p| p.context_repository_ids.clone())
                    .unwrap_or_default(),
                intents: previous
                    .as_ref()
                    .map(|p| p.intents.clone())
                    .unwrap_or_default(),
            };
            if inbox.get::<Uuid>("sent_generation")? != generation {
                (input.intents, input.context_repository_ids) =
                    intentions(&client, &device, &desired).await?;
                inbox.put(
                    "context_coverage",
                    &if desired.session.scope.selection.repository_ids.is_empty()
                        && input.context_repository_ids.is_empty()
                    {
                        "checkout_context_unresolved"
                    } else {
                        "registered_context"
                    },
                )?;
            }
            let same = previous.as_ref().is_some_and(|p| {
                let mut p = p.clone();
                p.expected_revision = input.expected_revision;
                p == input
            });
            if same {
                inbox.put("sent_generation", &generation)?;
            }
            if force_announce
                || !same
                || inbox
                    .get::<CoordinationSessionInput>("pending_announce")?
                    .is_some()
            {
                // Freeze retries before sending, so a lost response replays the same
                // expected revision and payload instead of inventing a later write.
                let pending = inbox
                    .get::<CoordinationSessionInput>("pending_announce")?
                    .filter(|p| p.expected_scope == input.expected_scope);
                if pending.is_none() {
                    inbox.put("pending_generation", &generation)?;
                }
                let input = pending.unwrap_or(input);
                inbox.put("pending_announce", &input)?;
                let response: CoordinationAnnouncement = request(
                    &client,
                    &device,
                    Method::POST,
                    &format!("{}/sessions", base(&config)),
                    Some(serde_json::to_value(&input)?),
                )
                .await?;
                ensure!(response.session.id == id, "coordination_session_mismatch");
                inbox.put("revision", &response.session.revision)?;
                force_announce = false;
                inbox.put("announced", &input)?;
                inbox.put("sent_generation", &inbox.get::<Uuid>("pending_generation")?)?;
                inbox
                    .db
                    .execute("DELETE FROM state WHERE key='pending_announce'", [])?;
                last_heartbeat = tokio::time::Instant::now();
            } else if last_heartbeat.elapsed() >= Duration::from_secs(25) {
                let response: CoordinationSession = request(
                    &client,
                    &device,
                    Method::POST,
                    &format!("{}/sessions/{id}/heartbeat", base(&config)),
                    Some(json!({"expected_revision":input.expected_revision})),
                )
                .await?;
                inbox.put("revision", &response.revision)?;
                last_heartbeat = tokio::time::Instant::now();
            }
            pull(&client, &device, &config, id, &mut inbox).await?;
            stream(&client, &device, &config, id, &mut inbox, generation, false).await?;
            Ok(false)
        }
        .await;
        match result {
            Ok(true) => {
                inbox.put(
                    "transport",
                    &json!({"state":"closed","checked_at":Utc::now()}),
                )?;
                break;
            }
            Ok(false) => failures = 0,
            Err(error) => {
                if [":401", ":403", ":404"]
                    .iter()
                    .any(|code| error.to_string().ends_with(code))
                {
                    inbox.invalidate("denied")?;
                    break;
                }
                if error.to_string().ends_with(":409") {
                    force_announce = true;
                    if let Ok(peers) = request::<CoordinationPeers>(
                        &client,
                        &device,
                        Method::GET,
                        &format!("{}/sessions?session_id={id}", base(&config)),
                        None,
                    )
                    .await
                        && let Some(own) = peers.sessions.iter().find(|s| s.id == id)
                    {
                        inbox.put("revision", &own.revision)?;
                        inbox.db.execute(
                            "DELETE FROM state WHERE key IN ('pending_announce','sent_generation')",
                            [],
                        )?;
                    }
                }
                failures += 1;
                inbox.put(
                    "transport",
                    &json!({"state":"degraded","checked_at":Utc::now(),"retry":failures}),
                )?;
                if failures >= 10 {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
    // Cover an enqueue racing with idle exit and lock release.
    drop(lock);
    if inbox.get::<Uuid>("generation")? != observed_generation {
        wake(&root, id)?;
    }
    Ok(json!({"state":"stopped","session_id":id}))
}

fn option(args: &[String], name: &str) -> Result<Option<String>> {
    let values: Vec<_> = args
        .windows(2)
        .filter(|v| v[0] == name)
        .map(|v| v[1].clone())
        .collect();
    ensure!(values.len() <= 1, "duplicate_coordination_option");
    Ok(values.first().cloned())
}
fn input<T: serde::de::DeserializeOwned>() -> Result<T> {
    let mut bytes = vec![];
    std::io::stdin().take(131073).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 131072, "coordination_input_too_large");
    Ok(serde_json::from_slice(&bytes)?)
}

pub async fn run(
    config: &Config,
    client: &Client,
    device: &StoredDevice,
    args: &[String],
) -> Result<Value> {
    let action = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| anyhow!(USAGE))?;
    let session = option(args, "--session")?
        .map(|s| s.parse::<Uuid>())
        .transpose()?;
    match action {
        "announce" => {
            let body: CoordinationSessionInput = input()?;
            request(
                client,
                device,
                Method::POST,
                &format!("{}/sessions", base(config)),
                Some(serde_json::to_value(body)?),
            )
            .await
        }
        "send" => {
            let body: CoordinationMessageInput = input()?;
            request(
                client,
                device,
                Method::POST,
                &format!("{}/messages", base(config)),
                Some(serde_json::to_value(body)?),
            )
            .await
        }
        "peers" => {
            request(
                client,
                device,
                Method::GET,
                &format!(
                    "{}/sessions{}",
                    base(config),
                    session
                        .map(|id| format!("?session_id={id}"))
                        .unwrap_or_default()
                ),
                None,
            )
            .await
        }
        "inbox" => {
            let id = session.ok_or_else(|| anyhow!(USAGE))?;
            let after = option(args, "--after")?
                .unwrap_or_else(|| "0".into())
                .parse::<i64>()?;
            request(
                client,
                device,
                Method::GET,
                &format!("{}/sessions/{id}/inbox?after={after}", base(config)),
                None,
            )
            .await
        }
        "ack" => {
            let id = session.ok_or_else(|| anyhow!(USAGE))?;
            let event = option(args, "--event")?
                .ok_or_else(|| anyhow!(USAGE))?
                .parse::<Uuid>()?;
            request(
                client,
                device,
                Method::POST,
                &format!("{}/sessions/{id}/inbox/{event}/ack", base(config)),
                None,
            )
            .await
        }
        "close" => {
            let id = session.ok_or_else(|| anyhow!(USAGE))?;
            request(
                client,
                device,
                Method::POST,
                &format!("{}/sessions/{id}/close", base(config)),
                None,
            )
            .await
        }
        "heartbeat" => {
            let id = session.ok_or_else(|| anyhow!(USAGE))?;
            let peers: CoordinationPeers = request(
                client,
                device,
                Method::GET,
                &format!("{}/sessions?session_id={id}", base(config)),
                None,
            )
            .await?;
            let own = peers
                .sessions
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| anyhow!("coordination_session_unavailable"))?;
            request(
                client,
                device,
                Method::POST,
                &format!("{}/sessions/{id}/heartbeat", base(config)),
                Some(json!({"expected_revision":own.revision})),
            )
            .await
        }
        "status" => local_status(
            &plugin_storage::root()?,
            config,
            session.ok_or_else(|| anyhow!(USAGE))?,
        ),
        "watch" => {
            watch(
                config,
                client,
                device,
                session.ok_or_else(|| anyhow!(USAGE))?,
            )
            .await
        }
        _ => Err(anyhow!(USAGE)),
    }
}

async fn watch(config: &Config, client: &Client, device: &StoredDevice, id: Uuid) -> Result<Value> {
    let mut inbox = Inbox::open(&plugin_storage::root()?, config, id)?;
    eprintln!(
        "Recollect coordination · Brain {} · session {id}. Peer content is advisory; Ctrl-C stops watching.",
        config.brain
    );
    let mut heartbeat = tokio::time::Instant::now() - Duration::from_secs(30);
    loop {
        let running = async {
            if heartbeat.elapsed() >= Duration::from_secs(30) {
                let peers: CoordinationPeers = request(
                    client,
                    device,
                    Method::GET,
                    &format!("{}/sessions?session_id={id}", base(config)),
                    None,
                )
                .await?;
                let own = peers
                    .sessions
                    .iter()
                    .find(|p| p.id == id)
                    .ok_or_else(|| anyhow!("coordination_session_unavailable"))?;
                let _: CoordinationSession = request(
                    client,
                    device,
                    Method::POST,
                    &format!("{}/sessions/{id}/heartbeat", base(config)),
                    Some(json!({"expected_revision":own.revision})),
                )
                .await?;
                heartbeat = tokio::time::Instant::now();
            }
            pull(client, device, config, id, &mut inbox).await?;
            display_watch(&inbox)?;
            stream(client, device, config, id, &mut inbox, None, true).await
        };
        tokio::select! {
            _=tokio::signal::ctrl_c()=>{return Ok(json!({"state":"stopped","session_id":id}));}
            result=running=>if let Err(error)=result {
                if [":401",":403",":404"].iter().any(|code|error.to_string().ends_with(code)) {
                    inbox.invalidate("denied")?;
                    return Err(anyhow!("coordination_access_denied"));
                }
                eprintln!("Recollect coordination disconnected; retrying with durable inbox cursor.");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_checkout_context_uses_registered_origin_never_basename() {
        let repository = recollect_protocol::Repository {
            id: Uuid::new_v4(),
            brain_id: Uuid::new_v4(),
            canonical_origin: "https://git.example.invalid/team/infra".into(),
            origins: vec!["https://git.example.invalid/team/infra-alias".into()],
            created_at: Utc::now(),
        };
        assert_eq!(
            registered_repository(
                std::slice::from_ref(&repository),
                Some("https://git.example.invalid/team/infra")
            ),
            Some(repository.id)
        );
        assert_eq!(
            registered_repository(
                std::slice::from_ref(&repository),
                Some("https://git.example.invalid/team/infra-alias")
            ),
            Some(repository.id)
        );
        assert_eq!(
            registered_repository(
                std::slice::from_ref(&repository),
                Some("https://git.example.invalid/other/infra")
            ),
            None
        );
        assert_eq!(
            registered_repository(std::slice::from_ref(&repository), None),
            None
        );
    }
    #[test]
    fn visible_watch_has_an_independent_delivery_cursor() {
        let root = crate::publication::project_root()
            .join(".cache")
            .join(format!("coordination-watch-{}", Uuid::new_v4()));
        let config = Config {
            endpoint: "http://localhost:9999".into(),
            device: Uuid::new_v4(),
            brain: Uuid::new_v4(),
            profile: "test".into(),
            with_runner: false,
            runner_id: None,
        };
        let mut inbox = Inbox::open(&root, &config, Uuid::new_v4()).unwrap();
        let event = super::super::tests::event_fixture(1);
        // The background worker received this before watch read its stream.
        inbox.receive(std::slice::from_ref(&event), 1).unwrap();
        display_watch(&inbox).unwrap();
        assert_eq!(inbox.get::<i64>("watch_cursor").unwrap(), Some(1));
        // Watch did not consume the host hook's independent delivery receipt.
        assert_eq!(inbox.undelivered().unwrap().len(), 1);
        display_watch(&inbox).unwrap();
        assert_eq!(inbox.get::<i64>("watch_cursor").unwrap(), Some(1));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn sse_partial_frames_comments_and_bounds() {
        let mut parser = Sse::default();
        assert!(parser.feed(b": heartbeat\r\n\r\n").unwrap().is_empty());
        assert!(parser.feed(b"data: ").unwrap().is_empty());
        assert!(parser.feed(b"not json\n\n").is_err());
        assert!(
            Sse::default()
                .feed(&vec![b'x'; 4 * 1024 * 1024 + 1])
                .is_err()
        );
        let event = super::super::tests::event_fixture(1);
        let frame = format!(
            "id: 1\r\nevent: message\r\ndata: {}\r\n\r\n",
            serde_json::to_string(&event).unwrap()
        );
        let mut parser = Sse::default();
        let mut received = vec![];
        for part in frame.as_bytes().chunks(7) {
            received.extend(parser.feed(part).unwrap());
        }
        assert_eq!(received.len(), 1);
        assert!(matches!(&received[0],Frame::Event(received) if received.id==event.id));
        let gap = CoordinationInbox {
            events: vec![],
            next_cursor: 7,
            gap: true,
            peers: vec![],
        };
        let frames = parser
            .feed(
                format!(
                    "event: gap\ndata: {}\n\n{frame}",
                    serde_json::to_string(&gap).unwrap()
                )
                .as_bytes(),
            )
            .unwrap();
        assert!(
            matches!(&frames[0],Frame::Gap(received) if received.gap && received.next_cursor==7)
        );
        assert!(matches!(&frames[1],Frame::Event(received) if received.id==event.id));
    }
}
