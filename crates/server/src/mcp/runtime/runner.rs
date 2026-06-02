use super::*;

#[derive(Clone)]
pub struct Runner {
    reference: String,
    epoch: Uuid,
    actor: Option<Uuid>,
    device: Option<Uuid>,
}
#[derive(sqlx::FromRow)]
struct Identity {
    id: Uuid,
    brain_id: Uuid,
    actor_id: Uuid,
    device_id: Option<Uuid>,
}
fn fenced() -> Error {
    runtime_conflict(
        "mcp_runner_fenced",
        "This runner or attempt no longer owns execution.",
    )
}
impl Runner {
    pub fn central(epoch: Uuid) -> Self {
        Self {
            reference: "central".into(),
            epoch,
            actor: None,
            device: None,
        }
    }
    fn local(auth: &Auth, epoch: Uuid, private: Option<Uuid>) -> Result<Self> {
        let device = auth
            .device_id
            .ok_or_else(|| Error::invalid("A paired device is required for the local runner."))?;
        Ok(Self {
            reference: private
                .map(|id| format!("private:{id}"))
                .unwrap_or_else(|| format!("device:{device}")),
            epoch,
            actor: Some(auth.user.id),
            device: Some(device),
        })
    }
    async fn tx<'a>(&self, pool: &'a sqlx::PgPool) -> Result<Tx<'a>> {
        match self.actor {
            Some(actor) => Ok(db::device_tx(pool, actor, self.device).await?),
            None => Ok(pool.begin().await?),
        }
    }
    async fn lock_local(&self, tx: &mut Tx<'_>) -> Result<()> {
        if self.reference.starts_with("private:") {
            let valid: bool = sqlx::query_scalar("SELECT recollect_mcp_private_lock($1,$2,$3)")
                .bind(&self.reference)
                .bind(self.device)
                .bind(self.actor)
                .fetch_one(&mut **tx)
                .await?;
            return if valid { Ok(()) } else { Err(fenced()) };
        }
        if let Some(device) = self.device {
            let valid:Option<bool>=sqlx::query_scalar("SELECT claimed AND revoked_at IS NULL AND expires_at>clock_timestamp() FROM devices WHERE id=$1 AND account_id=$2 FOR SHARE")
                .bind(device).bind(self.actor).fetch_optional(&mut **tx).await?;
            if valid != Some(true) {
                return Err(fenced());
            }
        }
        Ok(())
    }
    pub async fn register(&self, state: &AppState) -> Result<McpRunnerLease> {
        let mut tx = self.tx(&state.pool).await?;
        self.lock_local(&mut tx).await?;
        let registered: bool = sqlx::query_scalar("SELECT recollect_mcp_runner_register($1,$2)")
            .bind(&self.reference)
            .bind(self.epoch)
            .fetch_one(&mut *tx)
            .await?;
        if !registered {
            return Err(runtime_conflict(
                "mcp_runner_already_active",
                "This runner already has a live owner. Wait for its lease to expire.",
            ));
        }
        let lease = self.touch(&mut tx).await?;
        tx.commit().await?;
        Ok(lease)
    }
    async fn touch(&self, tx: &mut Tx<'_>) -> Result<McpRunnerLease> {
        sqlx::query_scalar::<_, Option<DbJson<McpRunnerLease>>>(
            "SELECT recollect_mcp_runner_touch($1,$2)",
        )
        .bind(&self.reference)
        .bind(self.epoch)
        .fetch_one(&mut **tx)
        .await?
        .map(|v| v.0)
        .ok_or_else(fenced)
    }
    pub async fn heartbeat(&self, state: &AppState) -> Result<McpRunnerLease> {
        let mut tx = self.tx(&state.pool).await?;
        let lease = self.touch(&mut tx).await?;
        tx.commit().await?;
        Ok(lease)
    }
    async fn identity(&self, state: &AppState, attempt: &McpAttempt) -> Result<Identity> {
        if attempt.epoch != self.epoch {
            return Err(fenced());
        }
        let mut tx = self.tx(&state.pool).await?;
        let identity =
            sqlx::query_as::<_, Identity>("SELECT * FROM recollect_mcp_attempt($1,$2,$3,$4)")
                .bind(&self.reference)
                .bind(self.epoch)
                .bind(attempt.call_id)
                .bind(attempt.attempt_token)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(fenced)?;
        if identity.brain_id != attempt.brain_id {
            return Err(fenced());
        }
        tx.commit().await?;
        Ok(identity)
    }
    pub async fn claim(&self, state: &AppState) -> Result<McpClaim> {
        let mut tx = self.tx(&state.pool).await?;
        let identity = sqlx::query_as::<_, Identity>("SELECT * FROM recollect_mcp_next($1,$2)")
            .bind(&self.reference)
            .bind(self.epoch)
            .fetch_optional(&mut *tx)
            .await?;
        tx.commit().await?;
        let Some(identity) = identity else {
            return Ok(McpClaim { plan: None });
        };
        match self.claim_authorized(state, &identity).await {
            Ok(plan) => Ok(McpClaim { plan }),
            Err(error) if error.0 != StatusCode::SERVICE_UNAVAILABLE => {
                let mut tx = self.tx(&state.pool).await?;
                sqlx::query("SELECT recollect_mcp_reject_queued($1,$2,$3,$4)")
                    .bind(&self.reference)
                    .bind(self.epoch)
                    .bind(identity.id)
                    .bind(error.1)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                Ok(McpClaim { plan: None })
            }
            Err(error) => Err(error),
        }
    }
    async fn claim_authorized(
        &self,
        state: &AppState,
        identity: &Identity,
    ) -> Result<Option<McpExecutionPlan>> {
        let mut tx = execution_tx(
            &state.pool,
            identity.actor_id,
            identity.device_id,
            identity.brain_id,
        )
        .await?;
        let row = load_call(&mut tx, identity.brain_id, identity.id, true).await?;
        if row.state != "queued" {
            return Ok(None);
        }
        self.lock_local(&mut tx).await?;
        let alive: bool = sqlx::query_scalar("SELECT recollect_mcp_runner_alive($1,$2)")
            .bind(&self.reference)
            .bind(self.epoch)
            .fetch_one(&mut *tx)
            .await?;
        if !alive || row.runner_reference != self.reference || row.queue_expires_at <= Utc::now() {
            return Err(fenced());
        }
        let input = stored_input(&mut tx, row.id).await?;
        let authorized =
            authorize(&mut tx, row.brain_id, row.actor_id, row.device_id, &input).await?;
        if !unchanged(&row, &authorized) {
            return Err(runtime_conflict(
                "mcp_configuration_changed",
                "Queued configuration changed before execution.",
            ));
        }
        let arguments = arguments(
            &input,
            &authorized.definition.manifest,
            row.id,
            row.reconciles_call_id.is_some(),
        )?;
        let token = Uuid::new_v4();
        let lease_until:DateTime<Utc>=sqlx::query_scalar("UPDATE mcp_calls SET state='starting',code=NULL,runner_epoch=$2,attempt_token=$3,lease_until=clock_timestamp()+interval '30 seconds',deadline=clock_timestamp()+interval '35 seconds',started_at=clock_timestamp() WHERE id=$1 RETURNING lease_until")
            .bind(row.id).bind(self.epoch).bind(token).fetch_one(&mut *tx).await?;
        db::audit(
            &mut tx,
            row.actor_id,
            row.brain_id,
            "mcp.startup",
            row.id,
            "starting",
        )
        .await?;
        tx.commit().await?;
        Ok(Some(McpExecutionPlan {
            call_id: row.id,
            brain_id: row.brain_id,
            actor_id: row.actor_id,
            device_id: row.device_id,
            profile_id: row.profile_id,
            connection_id: row.connection_id,
            client_session_id: row.client_session_id,
            runner_reference: self.reference.clone(),
            runner_epoch: self.epoch,
            attempt_token: token,
            lease_until,
            connection_revision: row.connection_revision,
            definition_revision: row.definition_revision,
            definition: authorized.definition.manifest.0,
            target: authorized.connection.target,
            configuration: authorized.connection.configuration,
            credential_alias: authorized.connection.credential_alias,
            tool_name: row.tool_name,
            arguments,
            timeout_seconds: row.timeout_seconds as u32,
        }))
    }
    pub async fn start(&self, state: &AppState, input: &McpStart) -> Result<McpStartPermit> {
        let identity = self.identity(state, &input.attempt).await?;
        match self.start_authorized(state, &identity, input).await {
            Ok(permit) => Ok(permit),
            Err(error) if error.0 != StatusCode::SERVICE_UNAVAILABLE => {
                let completion = McpCompletion {
                    attempt: input.attempt.clone(),
                    state: "failed".into(),
                    code: error.1.into(),
                    result: None,
                };
                self.complete(state, &completion).await?;
                Ok(McpStartPermit {
                    dispatch: false,
                    state: "failed".into(),
                    code: Some(error.1.into()),
                    deadline: None,
                })
            }
            Err(error) => Err(error),
        }
    }
    async fn start_authorized(
        &self,
        state: &AppState,
        identity: &Identity,
        input: &McpStart,
    ) -> Result<McpStartPermit> {
        let mut tx = execution_tx(
            &state.pool,
            identity.actor_id,
            identity.device_id,
            identity.brain_id,
        )
        .await?;
        self.lock_local(&mut tx).await?;
        let row = load_call(&mut tx, identity.brain_id, identity.id, true).await?;
        if row.runner_epoch != Some(self.epoch)
            || row.attempt_token != Some(input.attempt.attempt_token)
        {
            return Err(fenced());
        }
        // A replayed/lost start acknowledgment must never authorize another send.
        if row.state != "starting" {
            return Ok(McpStartPermit {
                dispatch: false,
                state: row.state,
                code: Some("dispatch_already_committed".into()),
                deadline: row.deadline,
            });
        }
        if row.cancel_requested {
            sqlx::query("UPDATE mcp_calls SET state='cancelled',code='cancelled_before_dispatch',completed_at=clock_timestamp(),payload_expires_at=clock_timestamp()+interval '1 hour' WHERE id=$1").bind(row.id).execute(&mut *tx).await?;
            db::audit(
                &mut tx,
                row.actor_id,
                row.brain_id,
                "mcp.completion",
                row.id,
                "cancelled",
            )
            .await?;
            tx.commit().await?;
            return Ok(McpStartPermit {
                dispatch: false,
                state: "cancelled".into(),
                code: Some("cancelled_before_dispatch".into()),
                deadline: None,
            });
        }
        let alive: bool = sqlx::query_scalar("SELECT recollect_mcp_runner_alive($1,$2)")
            .bind(&self.reference)
            .bind(self.epoch)
            .fetch_one(&mut *tx)
            .await?;
        if !alive
            || row.lease_until.is_none_or(|t| t <= Utc::now())
            || row.deadline.is_none_or(|t| t <= Utc::now())
        {
            return Err(fenced());
        }
        let request = stored_input(&mut tx, row.id).await?;
        let authorized =
            authorize(&mut tx, row.brain_id, row.actor_id, row.device_id, &request).await?;
        if !unchanged(&row, &authorized) {
            return Err(runtime_conflict(
                "mcp_configuration_changed",
                "The authorized scope or configuration changed before dispatch.",
            ));
        }
        arguments(
            &request,
            &authorized.definition.manifest,
            row.id,
            row.reconciles_call_id.is_some(),
        )?;
        let owned: bool = sqlx::query_scalar("SELECT recollect_mcp_instance_matches($1,$2)")
            .bind(input.instance_id)
            .bind(row.id)
            .fetch_one(&mut *tx)
            .await?;
        if !owned {
            return Err(fenced());
        }
        let deadline:DateTime<Utc>=sqlx::query_scalar("UPDATE mcp_calls SET state='running',instance_id=$2,dispatched_at=clock_timestamp(),deadline=clock_timestamp()+make_interval(secs=>timeout_seconds),lease_until=least(lease_until,clock_timestamp()+make_interval(secs=>timeout_seconds)) WHERE id=$1 RETURNING deadline")
            .bind(row.id).bind(input.instance_id).fetch_one(&mut *tx).await?;
        db::audit(
            &mut tx,
            row.actor_id,
            row.brain_id,
            "mcp.dispatch",
            row.id,
            "running",
        )
        .await?;
        tx.commit().await?;
        Ok(McpStartPermit {
            dispatch: true,
            state: "running".into(),
            code: None,
            deadline: Some(deadline),
        })
    }
    pub async fn credentials(&self, state: &AppState, input: &McpAttempt) -> Result<()> {
        let identity = self.identity(state, input).await?;
        let mut tx = execution_tx(
            &state.pool,
            identity.actor_id,
            identity.device_id,
            identity.brain_id,
        )
        .await?;
        self.lock_local(&mut tx).await?;
        let row = load_call(&mut tx, identity.brain_id, identity.id, true).await?;
        let alive: bool = sqlx::query_scalar("SELECT recollect_mcp_runner_alive($1,$2)")
            .bind(&self.reference)
            .bind(self.epoch)
            .fetch_one(&mut *tx)
            .await?;
        if !alive
            || !matches!(row.state.as_str(), "starting" | "running")
            || row.cancel_requested
            || row.runner_epoch != Some(self.epoch)
            || row.attempt_token != Some(input.attempt_token)
            || row.lease_until.is_none_or(|t| t <= Utc::now())
            || row.deadline.is_none_or(|t| t <= Utc::now())
        {
            return Err(fenced());
        }
        let request = stored_input(&mut tx, row.id).await?;
        let authorized =
            authorize(&mut tx, row.brain_id, row.actor_id, row.device_id, &request).await?;
        if !unchanged(&row, &authorized) {
            return Err(runtime_conflict(
                "mcp_configuration_changed",
                "The configuration changed before credential access.",
            ));
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn complete(&self, state: &AppState, input: &McpCompletion) -> Result<()> {
        self.identity(state, &input.attempt).await?;
        let response = matches!(input.state.as_str(), "succeeded" | "tool_error");
        let content_valid = input.result.as_ref().is_some_and(|value| {
            value.is_object()
                && value
                    .get("content")
                    .and_then(Value::as_array)
                    .is_some_and(|blocks| {
                        blocks
                            .iter()
                            .all(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                    })
                && (value
                    .get("isError")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    == (input.state == "tool_error"))
        });
        if !matches!(
            input.state.as_str(),
            "succeeded" | "tool_error" | "failed" | "cancelled" | "unknown"
        ) || (response && !content_valid)
            || (!response && input.result.is_some())
            || input.code.is_empty()
            || input.code.len() > 100
            || !input
                .code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || input
                .result
                .as_ref()
                .is_some_and(|r| serde_json::to_vec(r).map_or(true, |v| v.len() > 262144))
        {
            return Err(Error::invalid(
                "Return a bounded supported call disposition.",
            ));
        }
        let safe = input
            .result
            .as_ref()
            .map(|r| sanitize_capture_value(r, &publication::configured_secrets(state)));
        let mut tx = self.tx(&state.pool).await?;
        let locked: bool = sqlx::query_scalar("SELECT recollect_mcp_observation_lock($1,$2,$3,$4)")
            .bind(&self.reference)
            .bind(self.epoch)
            .bind(input.attempt.call_id)
            .bind(input.attempt.attempt_token)
            .fetch_one(&mut *tx)
            .await?;
        if !locked {
            return Err(fenced());
        }
        let removed: bool = sqlx::query_scalar("SELECT recollect_mcp_observation_fenced($1,$2)")
            .bind(input.attempt.brain_id)
            .bind(input.attempt.call_id)
            .fetch_one(&mut *tx)
            .await?;
        let accepted: bool =
            sqlx::query_scalar("SELECT recollect_mcp_complete($1,$2,$3,$4,$5,$6,$7,$8)")
                .bind(&self.reference)
                .bind(self.epoch)
                .bind(input.attempt.call_id)
                .bind(input.attempt.attempt_token)
                .bind(&input.state)
                .bind(&input.code)
                .bind(if removed { None } else { safe })
                .bind(Uuid::new_v4())
                .fetch_one(&mut *tx)
                .await?;
        if !accepted {
            return Err(fenced());
        }
        observations::completed(state, &mut tx, &self.reference, input).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn receipt_removals(
        &self,
        state: &AppState,
        input: McpReceiptCheck,
    ) -> Result<McpReceiptRemovals> {
        if input.call_ids.len() > 64 {
            return Err(Error::invalid("Check at most 64 receipt identities."));
        }
        let mut tx = self.tx(&state.pool).await?;
        let call_ids = sqlx::query_scalar("SELECT * FROM recollect_mcp_receipt_removals($1,$2)")
            .bind(&self.reference)
            .bind(&input.call_ids)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(McpReceiptRemovals { call_ids })
    }
    pub async fn defer(&self, state: &AppState, input: &McpAttempt) -> Result<()> {
        self.identity(state, input).await?;
        let mut tx = self.tx(&state.pool).await?;
        let accepted: bool = sqlx::query_scalar("SELECT recollect_mcp_defer($1,$2,$3,$4)")
            .bind(&self.reference)
            .bind(self.epoch)
            .bind(input.call_id)
            .bind(input.attempt_token)
            .fetch_one(&mut *tx)
            .await?;
        if !accepted {
            return Err(fenced());
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn instance(&self, state: &AppState, input: &McpInstanceUpdate) -> Result<()> {
        let identity = self.identity(state, &input.attempt).await?;
        if input.active_calls > 4 || input.idle_seconds > 86400 {
            return Err(Error::invalid("Instance activity exceeds runtime bounds."));
        }
        if input.state == "starting" {
            let mut tx = execution_tx(
                &state.pool,
                identity.actor_id,
                identity.device_id,
                identity.brain_id,
            )
            .await?;
            self.lock_local(&mut tx).await?;
            let row = load_call(&mut tx, identity.brain_id, identity.id, true).await?;
            if row.state != "starting"
                || row.cancel_requested
                || row.lease_until.is_none_or(|until| until <= Utc::now())
                || row.deadline.is_none_or(|until| until <= Utc::now())
            {
                return Err(fenced());
            }
            let request = stored_input(&mut tx, row.id).await?;
            let authorized =
                authorize(&mut tx, row.brain_id, row.actor_id, row.device_id, &request).await?;
            if !unchanged(&row, &authorized) {
                return Err(runtime_conflict(
                    "mcp_configuration_changed",
                    "The configuration changed before provider startup.",
                ));
            }
            // Retain authority locks in this one transaction. Rebind solely for
            // the narrow runner-owned write, avoiding a nested pool acquisition.
            sqlx::query("SELECT set_config('recollect.actor',$1,true),set_config('recollect.device',$2,true)")
                .bind(self.actor.map(|id|id.to_string()).unwrap_or_default())
                .bind(self.device.map(|id|id.to_string()).unwrap_or_default()).execute(&mut *tx).await?;
            self.instance_write_tx(&mut tx, input).await?;
            tx.commit().await?;
        } else {
            self.instance_write(state, input).await?;
        }
        Ok(())
    }
    async fn instance_write(&self, state: &AppState, input: &McpInstanceUpdate) -> Result<()> {
        let mut tx = self.tx(&state.pool).await?;
        self.instance_write_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(())
    }
    async fn instance_write_tx(&self, tx: &mut Tx<'_>, input: &McpInstanceUpdate) -> Result<()> {
        let accepted: bool =
            sqlx::query_scalar("SELECT recollect_mcp_instance($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                .bind(&self.reference)
                .bind(self.epoch)
                .bind(input.attempt.call_id)
                .bind(input.attempt.attempt_token)
                .bind(input.instance_id)
                .bind(input.credential_generation)
                .bind(&input.state)
                .bind(input.active_calls as i32)
                .bind(input.idle_seconds as i64)
                .fetch_one(&mut **tx)
                .await?;
        if !accepted {
            return Err(fenced());
        }
        Ok(())
    }
}
async fn stored_input(tx: &mut Tx<'_>, id: Uuid) -> Result<McpCallInput> {
    sqlx::query_scalar::<_, DbJson<McpCallInput>>(
        "SELECT request FROM mcp_call_payloads WHERE call_id=$1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .map(|v| v.0)
    .ok_or_else(denied)
}
pub async fn maintain(state: &AppState) -> Result<()> {
    sqlx::query("SELECT recollect_mcp_maintain()")
        .execute(&state.pool)
        .await?;
    Ok(())
}

#[utoipa::path(post,path="/api/mcp/runner/receipt-removals",operation_id="mcpReceiptRemovals",params(McpRunnerSelection),request_body=McpReceiptCheck,responses((status=200,body=McpReceiptRemovals)))]
pub async fn receipt_removals(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpReceiptCheck>,
) -> Result<Json<McpReceiptRemovals>> {
    Ok(Json(
        Runner::local(&auth, Uuid::nil(), selection.private_runner)?
            .receipt_removals(&state, input)
            .await?,
    ))
}

#[utoipa::path(post,path="/api/mcp/runner/register",operation_id="registerMcpRunner",params(McpRunnerSelection),responses((status=200,body=McpRunnerLease)))]
pub async fn register(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
) -> Result<Json<McpRunnerLease>> {
    Ok(Json(
        Runner::local(&auth, Uuid::new_v4(), selection.private_runner)?
            .register(&state)
            .await?,
    ))
}
#[utoipa::path(post,path="/api/mcp/runner/heartbeat",operation_id="heartbeatMcpRunner",params(McpRunnerSelection),request_body=McpRunnerEpoch,responses((status=200,body=McpRunnerLease)))]
pub async fn heartbeat(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpRunnerEpoch>,
) -> Result<Json<McpRunnerLease>> {
    Ok(Json(
        Runner::local(&auth, input.epoch, selection.private_runner)?
            .heartbeat(&state)
            .await?,
    ))
}
#[utoipa::path(post,path="/api/mcp/runner/claim",operation_id="claimMcpCall",params(McpRunnerSelection),request_body=McpRunnerEpoch,responses((status=200,body=McpClaim)))]
pub async fn claim(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpRunnerEpoch>,
) -> Result<Json<McpClaim>> {
    Ok(Json(
        Runner::local(&auth, input.epoch, selection.private_runner)?
            .claim(&state)
            .await?,
    ))
}
#[utoipa::path(post,path="/api/mcp/runner/start",operation_id="startMcpCall",params(McpRunnerSelection),request_body=McpStart,responses((status=200,body=McpStartPermit)))]
pub async fn start(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpStart>,
) -> Result<Json<McpStartPermit>> {
    Ok(Json(
        Runner::local(&auth, input.attempt.epoch, selection.private_runner)?
            .start(&state, &input)
            .await?,
    ))
}
#[utoipa::path(post,path="/api/mcp/runner/complete",operation_id="completeMcpCall",params(McpRunnerSelection),request_body=McpCompletion,responses((status=200,body=serde_json::Value)))]
pub async fn complete(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpCompletion>,
) -> Result<Json<Value>> {
    Runner::local(&auth, input.attempt.epoch, selection.private_runner)?
        .complete(&state, &input)
        .await?;
    Ok(Json(json!({"accepted":true})))
}
#[utoipa::path(post,path="/api/mcp/runner/defer",operation_id="deferMcpCall",params(McpRunnerSelection),request_body=McpAttempt,responses((status=200,body=serde_json::Value)))]
pub async fn defer(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpAttempt>,
) -> Result<Json<Value>> {
    Runner::local(&auth, input.epoch, selection.private_runner)?
        .defer(&state, &input)
        .await?;
    Ok(Json(json!({"deferred":true})))
}
#[utoipa::path(post,path="/api/mcp/runner/instance",operation_id="updateMcpInstance",params(McpRunnerSelection),request_body=McpInstanceUpdate,responses((status=200,body=serde_json::Value)))]
pub async fn instance(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpInstanceUpdate>,
) -> Result<Json<Value>> {
    Runner::local(&auth, input.attempt.epoch, selection.private_runner)?
        .instance(&state, &input)
        .await?;
    Ok(Json(json!({"accepted":true})))
}

#[utoipa::path(post,path="/api/mcp/runner/credentials",operation_id="authorizeMcpCredentials",params(McpRunnerSelection),request_body=McpAttempt,responses((status=200,body=serde_json::Value)))]
pub async fn credentials(
    State(state): State<AppState>,
    auth: Auth,
    Query(selection): Query<McpRunnerSelection>,
    Json(input): Json<McpAttempt>,
) -> Result<Json<Value>> {
    Runner::local(&auth, input.epoch, selection.private_runner)?
        .credentials(&state, &input)
        .await?;
    Ok(Json(json!({"authorized":true})))
}
