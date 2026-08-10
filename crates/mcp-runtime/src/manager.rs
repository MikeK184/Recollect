use crate::{
    CancellationToken, Connected, DispatchOutcome, Result, RuntimeError, USEFUL_IDLE,
    credentials::ResolvedCredentials,
};
use recollect_protocol::McpDefinitionManifest;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    future::Future,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

/// All fields come from the authorized immutable execution plan, not tool JSON.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceKey {
    pub brain_id: Uuid,
    pub profile_id: Uuid,
    pub connection_id: Uuid,
    pub actor_id: Uuid,
    pub device_id: Option<Uuid>,
    pub client_session_id: Uuid,
    pub runner_epoch: Uuid,
    pub connection_revision: Uuid,
    pub definition_revision: String,
    pub credential_generation: Uuid,
}
impl InstanceKey {
    fn same_session_connection(&self, other: &Self) -> bool {
        self.brain_id == other.brain_id
            && self.profile_id == other.profile_id
            && self.connection_id == other.connection_id
            && self.actor_id == other.actor_id
            && self.device_id == other.device_id
            && self.client_session_id == other.client_session_id
            && self.runner_epoch == other.runner_epoch
    }
}
struct State {
    connection: Option<Arc<Connected>>,
    active: usize,
    last_use: Instant,
    started: bool,
    draining: bool,
    closing: bool,
}
struct Slot {
    id: Uuid,
    key: InstanceKey,
    state: Mutex<State>,
    startup: AsyncMutex<()>,
    permits: Arc<Semaphore>,
}
#[derive(Debug, Clone)]
pub struct InstanceStatus {
    pub id: Uuid,
    pub key: InstanceKey,
    pub state: &'static str,
    pub active_calls: usize,
    pub idle_seconds: u64,
    pub last_use: Instant,
}
#[derive(Clone, Copy)]
pub struct StartSpec<'a> {
    pub definition: &'a McpDefinitionManifest,
    pub target: &'a str,
    pub configuration: &'a Value,
    pub supervisor: &'a Path,
}
pub struct RuntimeManager {
    slots: Mutex<BTreeMap<InstanceKey, Arc<Slot>>>,
    permits: Arc<Semaphore>,
}
impl Default for RuntimeManager {
    fn default() -> Self {
        Self {
            slots: Mutex::new(BTreeMap::new()),
            permits: Arc::new(Semaphore::new(16)),
        }
    }
}

struct Reservation {
    slot: Arc<Slot>,
    useful: bool,
    uncertain: bool,
    _runner: OwnedSemaphorePermit,
    _instance: OwnedSemaphorePermit,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let mut state = self.slot.state.lock().expect("runtime state lock");
        state.active -= 1;
        if self.uncertain {
            state.draining = true;
        }
        if self.useful {
            state.last_use = Instant::now();
        }
    }
}
struct StartupGuard<'a> {
    slot: &'a Slot,
    completed: bool,
}
impl Drop for StartupGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.slot.state.lock().expect("runtime state lock").draining = true;
        }
    }
}
pub struct OperationLease {
    reservation: Reservation,
    connected: Arc<Connected>,
}
impl OperationLease {
    pub fn instance_id(&self) -> Uuid {
        self.reservation.slot.id
    }
    pub fn validate(&self, name: &str, arguments: &Value) -> Result<()> {
        self.connected.validate(name, arguments)
    }
    /// Consume one operation lease for one tool attempt. A receipt upload is a
    /// separate coordinator operation; it must never call this method again.
    pub async fn call(
        mut self,
        name: &str,
        arguments: Value,
        timeout: Duration,
        cancel: CancellationToken,
    ) -> DispatchOutcome {
        if self.connected.validate(name, &arguments).is_ok() && !cancel.is_cancelled() {
            self.reservation.useful = true;
            self.reservation.uncertain = true;
        }
        let result = self.connected.call(name, arguments, timeout, cancel).await;
        self.reservation.uncertain = false;
        if result.state == "unknown" {
            self.reservation
                .slot
                .state
                .lock()
                .expect("runtime state lock")
                .draining = true;
        }
        result
    }
}
impl RuntimeManager {
    /// The owner callback commits instance identity before this runner spawns or
    /// connects. Runner fencing/authority is the coordinator's responsibility.
    pub async fn acquire<F, Fut>(
        &self,
        key: InstanceKey,
        spec: StartSpec<'_>,
        credentials: Arc<ResolvedCredentials>,
        record_owner: F,
    ) -> Result<OperationLease>
    where
        F: FnOnce(Uuid) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        if key.credential_generation != credentials.generation {
            return Err(RuntimeError("credential_generation_mismatch"));
        }
        let runner = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| RuntimeError("runtime_capacity"))?;
        let reservation = {
            let mut slots = self.slots.lock().expect("runtime slots lock");
            for (old_key, slot) in slots.iter() {
                if old_key != &key && old_key.same_session_connection(&key) {
                    slot.state.lock().expect("runtime state lock").draining = true;
                }
            }
            if !slots.contains_key(&key) && slots.len() >= 16 {
                return Err(RuntimeError("runtime_capacity"));
            }
            let slot = slots
                .entry(key.clone())
                .or_insert_with(|| {
                    Arc::new(Slot {
                        id: Uuid::new_v4(),
                        key,
                        state: Mutex::new(State {
                            connection: None,
                            active: 0,
                            last_use: Instant::now(),
                            started: false,
                            draining: false,
                            closing: false,
                        }),
                        startup: AsyncMutex::new(()),
                        permits: Arc::new(Semaphore::new(4)),
                    })
                })
                .clone();
            let permit = slot
                .permits
                .clone()
                .try_acquire_owned()
                .map_err(|_| RuntimeError("instance_capacity"))?;
            {
                let mut state = slot.state.lock().expect("runtime state lock");
                if state.draining {
                    return Err(RuntimeError("instance_draining"));
                }
                state.active += 1;
            }
            Reservation {
                slot,
                useful: false,
                uncertain: false,
                _runner: runner,
                _instance: permit,
            }
        };
        let connected = {
            let _startup = reservation.slot.startup.lock().await;
            let existing = {
                let state = reservation.slot.state.lock().expect("runtime state lock");
                if state.draining {
                    return Err(RuntimeError("instance_draining"));
                }
                state.connection.clone()
            };
            if let Some(connected) = existing {
                if !connected.reusable() {
                    reservation
                        .slot
                        .state
                        .lock()
                        .expect("runtime state lock")
                        .draining = true;
                    return Err(RuntimeError("provider_instance_stale"));
                }
                connected
            } else {
                {
                    let mut state = reservation.slot.state.lock().expect("runtime state lock");
                    if state.started {
                        return Err(RuntimeError("provider_startup_interrupted"));
                    }
                    state.started = true;
                }
                let mut guard = StartupGuard {
                    slot: &reservation.slot,
                    completed: false,
                };
                record_owner(reservation.slot.id).await?;
                let connected = Arc::new(
                    Connected::open(
                        spec.definition,
                        spec.target,
                        spec.configuration,
                        credentials,
                        spec.supervisor,
                    )
                    .await?,
                );
                reservation
                    .slot
                    .state
                    .lock()
                    .expect("runtime state lock")
                    .connection = Some(connected.clone());
                guard.completed = true;
                connected
            }
        };
        Ok(OperationLease {
            reservation,
            connected,
        })
    }
    pub fn statuses(&self, now: Instant) -> Vec<InstanceStatus> {
        self.slots
            .lock()
            .expect("runtime slots lock")
            .values()
            .map(|slot| {
                let state = slot.state.lock().expect("runtime state lock");
                InstanceStatus {
                    id: slot.id,
                    key: slot.key.clone(),
                    active_calls: state.active,
                    state: if state.draining {
                        "draining"
                    } else if state.connection.is_some() {
                        "ready"
                    } else {
                        "starting"
                    },
                    idle_seconds: now.saturating_duration_since(state.last_use).as_secs(),
                    last_use: state.last_use,
                }
            })
            .collect()
    }
    /// Mark owned instances for draining; existing operation leases remain valid.
    pub fn release_session(&self, actor: Uuid, session: Uuid) {
        for slot in self.slots.lock().expect("runtime slots lock").values() {
            if slot.key.actor_id == actor && slot.key.client_session_id == session {
                slot.state.lock().expect("runtime state lock").draining = true;
            }
        }
    }
    pub fn drain_all(&self) {
        for slot in self.slots.lock().expect("runtime slots lock").values() {
            slot.state.lock().expect("runtime state lock").draining = true;
        }
    }
    pub fn drain_instance(&self, id: Uuid) {
        for slot in self.slots.lock().expect("runtime slots lock").values() {
            if slot.id == id {
                slot.state.lock().expect("runtime state lock").draining = true;
            }
        }
    }
    pub fn release_brain_session(&self, brain: Uuid, actor: Uuid, session: Uuid) {
        for slot in self.slots.lock().expect("runtime slots lock").values() {
            if slot.key.brain_id == brain
                && slot.key.actor_id == actor
                && slot.key.client_session_id == session
            {
                slot.state.lock().expect("runtime state lock").draining = true;
            }
        }
    }
    /// The executor supplies its monotonic time, never a caller timestamp. This
    /// also permits proving the exact idle threshold without wall-clock sleeps.
    pub async fn maintain(&self, now: Instant) -> Vec<(Uuid, Result<()>)> {
        let closing = {
            let slots = self.slots.lock().expect("runtime slots lock");
            slots
                .values()
                .filter_map(|slot| {
                    let mut state = slot.state.lock().expect("runtime state lock");
                    if !state.closing
                        && state.active == 0
                        && (state.draining
                            || now.saturating_duration_since(state.last_use) >= USEFUL_IDLE
                            || state.connection.as_ref().is_some_and(|c| !c.reusable()))
                    {
                        state.draining = true;
                        state.closing = true;
                        Some(slot.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        let mut results = Vec::with_capacity(closing.len());
        for slot in closing {
            let connection = slot
                .state
                .lock()
                .expect("runtime state lock")
                .connection
                .clone();
            let result = match connection {
                Some(connection) => connection.close().await,
                None => Ok(()),
            };
            // Closing ownership continues to consume capacity. An interrupted or
            // failed close is quarantined until executor shutdown, never reused.
            if result.is_ok() {
                let mut slots = self.slots.lock().expect("runtime slots lock");
                if slots.get(&slot.key).is_some_and(|s| s.id == slot.id) {
                    slots.remove(&slot.key);
                }
            }
            results.push((slot.id, result));
        }
        results
    }
}
