//! One fenced runner's bounded execution loop. The coordinator is authoritative;
//! retries below upload stored receipts only and never repeat a tool invocation.
use crate::{
    CancellationToken, Result, RuntimeError,
    credentials::CredentialResolver,
    manager::{InstanceKey, RuntimeManager, StartSpec},
    outbox::Outbox,
};
use recollect_protocol::*;
use std::{
    collections::BTreeMap,
    future::Future,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
use tokio::task::JoinSet;
use uuid::Uuid;

pub trait Coordinator: Clone + Send + Sync + 'static {
    fn heartbeat(&self) -> impl Future<Output = Result<McpRunnerLease>> + Send;
    fn claim(&self) -> impl Future<Output = Result<McpClaim>> + Send;
    fn credentials(&self, input: McpAttempt) -> impl Future<Output = Result<()>> + Send;
    fn instance(&self, input: McpInstanceUpdate) -> impl Future<Output = Result<()>> + Send;
    fn start(&self, input: McpStart) -> impl Future<Output = Result<McpStartPermit>> + Send;
    fn complete(&self, input: McpCompletion) -> impl Future<Output = Result<()>> + Send;
    fn receipt_removals(
        &self,
        input: McpReceiptCheck,
    ) -> impl Future<Output = Result<McpReceiptRemovals>> + Send;
    fn defer(&self, input: McpAttempt) -> impl Future<Output = Result<()>> + Send;
}
async fn control<T>(future: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .map_err(|_| RuntimeError("runner_control_timeout"))?
}
type Instances = Arc<Mutex<BTreeMap<Uuid, (McpAttempt, Uuid)>>>;
#[derive(Clone)]
pub struct Executor<C: Coordinator> {
    coordinator: C,
    manager: Arc<RuntimeManager>,
    credentials: Arc<CredentialResolver>,
    supervisor: Arc<PathBuf>,
    outbox: Arc<Outbox>,
    instances: Instances,
    blocked: Arc<AtomicBool>,
}
impl<C: Coordinator> Executor<C> {
    pub fn new(
        coordinator: C,
        credentials: CredentialResolver,
        supervisor: PathBuf,
        outbox: Outbox,
    ) -> Self {
        Self {
            coordinator,
            manager: Arc::new(RuntimeManager::default()),
            credentials: Arc::new(credentials),
            supervisor: Arc::new(supervisor),
            outbox: Arc::new(outbox),
            instances: Arc::new(Mutex::new(BTreeMap::new())),
            blocked: Arc::new(AtomicBool::new(true)),
        }
    }
    async fn receipt(&self, completion: McpCompletion) -> Result<()> {
        // A late outcome can arrive after its earlier unknown evidence was
        // erased. Check identities without transmitting the retained output.
        let removals = control(self.coordinator.receipt_removals(McpReceiptCheck {
            call_ids: vec![completion.attempt.call_id],
        }))
        .await;
        if removals
            .as_ref()
            .is_ok_and(|r| r.call_ids.contains(&completion.attempt.call_id))
        {
            return self.outbox.remove(vec![completion.attempt.call_id]).await;
        }
        if let Err(error) = self.outbox.store(&completion, SystemTime::now()).await {
            // Reporting directly may still preserve the known outcome. A storage
            // fault stops admission even if that reporting attempt succeeds.
            if removals.is_ok() {
                let _ = control(self.coordinator.complete(completion)).await;
            }
            return Err(error);
        }
        if removals.is_err() {
            self.blocked.store(true, Ordering::Release);
            return Ok(());
        }
        match control(self.coordinator.complete(completion.clone())).await {
            Ok(()) => self.outbox.acknowledge(&completion).await,
            Err(RuntimeError("mcp_runner_fenced")) => self.outbox.quarantine(&completion).await,
            Err(_) => {
                self.blocked.store(true, Ordering::Release);
                Ok(())
            }
        }
    }
    async fn upload(&self, stop: CancellationToken) -> Result<()> {
        loop {
            let identities = self.outbox.identities(SystemTime::now()).await?;
            let mut unavailable = false;
            if !identities.is_empty() {
                let result = tokio::select! {
                    _=stop.cancelled()=>return Ok(()),
                    result=control(self.coordinator.receipt_removals(McpReceiptCheck {call_ids: identities.clone()}))=>result,
                };
                match result {
                    Ok(removals) if removals.call_ids.iter().all(|id| identities.contains(id)) => {
                        self.outbox.remove(removals.call_ids).await?
                    }
                    _ => unavailable = true,
                }
            }
            // Re-read after removal, and never upload a concurrently added body
            // whose identity was absent from this pass's removal check.
            let pending = if unavailable {
                vec![]
            } else {
                self.outbox.pending(SystemTime::now()).await?
            };
            for receipt in pending
                .into_iter()
                .filter(|r| identities.contains(&r.attempt.call_id))
            {
                let result = tokio::select! {
                    _=stop.cancelled()=>return Ok(()),
                    result=control(self.coordinator.complete(receipt.clone()))=>result,
                };
                match result {
                    Ok(()) => self.outbox.acknowledge(&receipt).await?,
                    Err(RuntimeError("mcp_runner_fenced")) => {
                        self.outbox.quarantine(&receipt).await?
                    }
                    Err(_) => {
                        unavailable = true;
                        break;
                    }
                }
            }
            self.blocked.store(unavailable, Ordering::Release);
            tokio::select! { _=stop.cancelled()=>return Ok(()),_=tokio::time::sleep(Duration::from_secs(1))=>{} }
        }
    }
    async fn execute(&self, plan: McpExecutionPlan, cancel: CancellationToken) -> Result<()> {
        let attempt = McpAttempt {
            epoch: plan.runner_epoch,
            brain_id: plan.brain_id,
            call_id: plan.call_id,
            attempt_token: plan.attempt_token,
        };
        let preparation = async {
            let credentials = self
                .credentials
                .resolve_with(
                    plan.connection_id,
                    plan.credential_alias.as_deref(),
                    &plan.runner_reference,
                    || control(self.coordinator.credentials(attempt.clone())),
                )
                .await?;
            let coordinator = self.coordinator.clone();
            let credential_attempt = attempt.clone();
            let credential_operation = credentials.operation(move || {
                let coordinator = coordinator.clone();
                let attempt = credential_attempt.clone();
                async move { control(coordinator.credentials(attempt)).await }
            });
            let generation = credentials.generation;
            let key = InstanceKey {
                brain_id: plan.brain_id,
                profile_id: plan.profile_id,
                connection_id: plan.connection_id,
                actor_id: plan.actor_id,
                device_id: plan.device_id,
                client_session_id: plan.client_session_id,
                runner_epoch: plan.runner_epoch,
                connection_revision: plan.connection_revision,
                definition_revision: plan.definition_revision.to_rfc3339(),
                credential_generation: generation,
            };
            let attempt = attempt.clone();
            let coordinator = self.coordinator.clone();
            let owners = self.instances.clone();
            let lease = self
                .manager
                .acquire(
                    key,
                    StartSpec {
                        definition: &plan.definition,
                        target: &plan.target,
                        configuration: &plan.configuration,
                        supervisor: &self.supervisor,
                    },
                    credentials,
                    move |id| async move {
                        control(coordinator.instance(McpInstanceUpdate {
                            attempt: attempt.clone(),
                            instance_id: id,
                            credential_generation: generation,
                            state: "starting".into(),
                            active_calls: 1,
                            idle_seconds: 0,
                        }))
                        .await?;
                        owners
                            .lock()
                            .expect("instance ownership lock")
                            .insert(id, (attempt, generation));
                        Ok(())
                    },
                )
                .await?;
            Ok((lease, credential_operation))
        };
        let prepared = tokio::select! {
            _=cancel.cancelled()=>Err(RuntimeError("cancelled_before_dispatch")),
            result=tokio::time::timeout(Duration::from_secs(30),preparation)=>result.unwrap_or(Err(RuntimeError("provider_startup_timeout"))),
        };
        let (lease, credential_operation) = match prepared {
            Ok(lease) => lease,
            Err(RuntimeError(
                "runtime_capacity"
                | "instance_capacity"
                | "instance_draining"
                | "provider_instance_stale",
            )) => {
                return control(self.coordinator.defer(attempt)).await;
            }
            Err(error) => {
                return self
                    .receipt(McpCompletion {
                        attempt,
                        state: if cancel.is_cancelled() {
                            "cancelled"
                        } else {
                            "failed"
                        }
                        .into(),
                        code: error.0.into(),
                        result: None,
                    })
                    .await;
            }
        };
        if let Err(error) = lease.validate(&plan.tool_name, &plan.arguments) {
            drop(credential_operation);
            drop(lease);
            return self
                .receipt(McpCompletion {
                    attempt,
                    state: "failed".into(),
                    code: error.0.into(),
                    result: None,
                })
                .await;
        }
        if cancel.is_cancelled() {
            drop(credential_operation);
            drop(lease);
            return self
                .receipt(McpCompletion {
                    attempt,
                    state: "cancelled".into(),
                    code: "cancelled_before_dispatch".into(),
                    result: None,
                })
                .await;
        }
        let requested = Instant::now();
        let permit = match control(self.coordinator.start(McpStart {
            attempt: attempt.clone(),
            instance_id: lease.instance_id(),
        }))
        .await
        {
            Ok(permit) => permit,
            Err(_) => {
                drop(credential_operation);
                drop(lease);
                return self
                    .receipt(McpCompletion {
                        attempt,
                        state: "unknown".into(),
                        code: "dispatch_acknowledgment_lost".into(),
                        result: None,
                    })
                    .await;
            }
        };
        if !permit.dispatch {
            return Ok(());
        }
        // Subtract the entire pre-send round trip conservatively, avoiding clock
        // skew between a companion and its server while respecting the deadline.
        let remaining =
            Duration::from_secs(plan.timeout_seconds as u64).saturating_sub(requested.elapsed());
        if remaining.is_zero() {
            drop(credential_operation);
            drop(lease);
            return self
                .receipt(McpCompletion {
                    attempt,
                    state: "failed".into(),
                    code: "deadline_before_send".into(),
                    result: None,
                })
                .await;
        }
        let result = lease
            .call(&plan.tool_name, plan.arguments, remaining, cancel)
            .await;
        // Receipt persistence/retries are separate from the tool operation and
        // must never keep its dynamic credentials renewing in the background.
        drop(credential_operation);
        self.receipt(McpCompletion {
            attempt,
            state: result.state,
            code: result.code,
            result: result.result,
        })
        .await
    }
    async fn report_instances(&self) -> Result<()> {
        let mut failure = None;
        for status in self.manager.statuses(Instant::now()) {
            // Startup ownership is written before opening the SDK connection.
            // A periodic starting snapshot can race a successful dispatch.
            if status.state == "starting" {
                continue;
            }
            let owner = self
                .instances
                .lock()
                .expect("instance ownership lock")
                .get(&status.id)
                .cloned();
            if let Some((attempt, generation)) = owner
                && let Err(error) = control(self.coordinator.instance(McpInstanceUpdate {
                    attempt,
                    instance_id: status.id,
                    credential_generation: generation,
                    state: status.state.into(),
                    active_calls: status.active_calls as u32,
                    idle_seconds: status.idle_seconds.min(86400),
                }))
                .await
            {
                failure = Some(error);
            }
        }
        for (id, closed) in self.manager.maintain(Instant::now()).await {
            let owner = self
                .instances
                .lock()
                .expect("instance ownership lock")
                .get(&id)
                .cloned();
            if let Some((attempt, generation)) = owner
                && let Err(error) = control(
                    self.coordinator.instance(McpInstanceUpdate {
                        attempt,
                        instance_id: id,
                        credential_generation: generation,
                        state: if closed.is_ok() {
                            "stopped"
                        } else {
                            "draining"
                        }
                        .into(),
                        active_calls: 0,
                        idle_seconds: 0,
                    }),
                )
                .await
            {
                failure = Some(error);
            }
            if let Err(error) = closed {
                failure = Some(error);
            }
            self.instances
                .lock()
                .expect("instance ownership lock")
                .remove(&id);
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
    /// Stop admits no new calls and drains current work. Lost authority instead
    /// requests cancellation, retains receipts and lets durable recovery arbitrate.
    pub async fn run(&self, lease: McpRunnerLease, stop: CancellationToken) -> Result<()> {
        let mut calls = JoinSet::new();
        let mut maintenance = JoinSet::new();
        let mut uploader = JoinSet::new();
        let upload_stop = CancellationToken::new();
        let upload = self.clone();
        let uploaded_stop = upload_stop.clone();
        uploader.spawn(async move { upload.upload(uploaded_stop).await });
        let mut tokens: BTreeMap<Uuid, CancellationToken> = BTreeMap::new();
        let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut poll = tokio::time::interval(Duration::from_millis(500));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut stopping = false;
        let mut final_cleanup = false;
        let mut failure = None;
        loop {
            if stopping && calls.is_empty() && maintenance.is_empty() {
                if final_cleanup {
                    break;
                }
                final_cleanup = true;
                self.manager.drain_all();
                let worker = self.clone();
                maintenance.spawn(async move { worker.report_instances().await });
            }
            tokio::select! {
                biased;
                // Shutdown and completed cleanup precede renewal. A blackholed
                // five-second control request must not starve these branches at
                // the equally frequent heartbeat interval.
                _=stop.cancelled(),if !stopping=>{ stopping=true; },
                finished=calls.join_next(),if !calls.is_empty()=>{
                    match finished {
                        Some(Ok((id,result)))=>{ tokens.remove(&id); if let Err(error)=result { failure=Some(error); stopping=true; } },
                        _=>{ failure=Some(RuntimeError("runner_task_interrupted")); stopping=true; },
                    }
                },
                finished=maintenance.join_next(),if !maintenance.is_empty()=>{
                    if let Some(error)=match finished { Some(Ok(Ok(())))=>None,Some(Ok(Err(error)))=>Some(error),_=>Some(RuntimeError("runner_maintenance_interrupted")) } {
                        failure=Some(error); stopping=true;
                    }
                },
                finished=uploader.join_next(),if !uploader.is_empty()=>{
                    failure=Some(match finished {Some(Ok(Err(error)))=>error,_=>RuntimeError("runner_uploader_interrupted")}); stopping=true;
                },
                _=heartbeat.tick(),if failure.is_none()=>{
                    match control(self.coordinator.heartbeat()).await {
                        Ok(current) if current.epoch==lease.epoch && current.runner_reference==lease.runner_reference=>{
                            for id in current.cancel_calls { if let Some(cancel)=tokens.get(&id) { cancel.cancel(); } }
                            for id in current.drain_instances { self.manager.drain_instance(id); }
                            for released in current.released_sessions { self.manager.release_brain_session(released.brain_id,released.actor_id,released.client_session_id); }
                            if maintenance.is_empty() && !stopping { let worker=self.clone(); maintenance.spawn(async move { worker.report_instances().await }); }
                        },
                        result=>{
                            failure=Some(result.err().unwrap_or(RuntimeError("runner_epoch_changed"))); stopping=true;
                            for cancel in tokens.values() { cancel.cancel(); } self.manager.drain_all();
                        },
                    }
                },
                _=poll.tick(),if !stopping && calls.len()<16=>{
                    if self.blocked.load(Ordering::Acquire) { continue; }
                    match self.outbox.count(SystemTime::now()).await {
                        Ok(pending) if pending+calls.len()<64=>{},
                        Ok(_)=>continue,
                        Err(error)=>{ failure=Some(error); stopping=true; continue; },
                    }
                    match control(self.coordinator.claim()).await {
                        Ok(McpClaim { plan:Some(plan) }) if plan.runner_epoch==lease.epoch && plan.runner_reference==lease.runner_reference && (1..=3600).contains(&plan.timeout_seconds)=>{
                            let id=plan.call_id; let cancel=CancellationToken::new(); tokens.insert(id,cancel.clone()); let worker=self.clone();
                            calls.spawn(async move { let result=worker.execute(plan,cancel).await; (id,result) });
                        },
                        Ok(McpClaim { plan:None })=>{},
                        result=>{ failure=Some(result.err().unwrap_or(RuntimeError("runner_plan_mismatch"))); stopping=true; },
                    }
                },
            }
            if failure.is_some() {
                for cancel in tokens.values() {
                    cancel.cancel();
                }
            }
        }
        upload_stop.cancel();
        while uploader.join_next().await.is_some() {}
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone)]
    struct Blackhole;
    impl Coordinator for Blackhole {
        async fn receipt_removals(&self, _: McpReceiptCheck) -> Result<McpReceiptRemovals> {
            unreachable!()
        }
        async fn heartbeat(&self) -> Result<McpRunnerLease> {
            std::future::pending().await
        }
        async fn claim(&self) -> Result<McpClaim> {
            panic!("lost heartbeat must prevent admission")
        }
        async fn credentials(&self, _: McpAttempt) -> Result<()> {
            unreachable!()
        }
        async fn instance(&self, _: McpInstanceUpdate) -> Result<()> {
            unreachable!()
        }
        async fn start(&self, _: McpStart) -> Result<McpStartPermit> {
            unreachable!()
        }
        async fn complete(&self, _: McpCompletion) -> Result<()> {
            unreachable!()
        }
        async fn defer(&self, _: McpAttempt) -> Result<()> {
            unreachable!()
        }
    }
    #[tokio::test]
    async fn heartbeat_timeout_cannot_starve_executor_cleanup() {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache/mcp-executor-proof")
            .join(Uuid::new_v4().to_string());
        let outbox = Outbox::open(directory.clone(), "blackhole-fixture".into())
            .await
            .unwrap();
        let executor = Executor::new(
            Blackhole,
            CredentialResolver::new(None),
            PathBuf::new(),
            outbox,
        );
        let lease = McpRunnerLease {
            runner_reference: "central".into(),
            epoch: Uuid::new_v4(),
            lease_until: Default::default(),
            cancel_calls: vec![],
            drain_instances: vec![],
            released_sessions: vec![],
        };
        tokio::time::pause();
        let result = tokio::time::timeout(
            Duration::from_secs(20),
            executor.run(lease, CancellationToken::new()),
        )
        .await;
        assert_eq!(
            result.expect("cleanup must not wait on another heartbeat"),
            Err(RuntimeError("runner_control_timeout"))
        );
        drop(executor);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
