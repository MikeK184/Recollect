//! GDS calculates topology; canonical qualification owns every input/result.
use super::*;
use crate::{auth::Auth, db, jobs, worker::ClaimedJob};
use chrono::{DateTime, Utc};
use serde_json::Value;

mod api;
mod journal;
mod native;
mod work;
pub use api::{__path_get, __path_queue, __path_view, get, queue, view};
pub use journal::{erase, initialize, reconcile, run_once};
pub use work::execute;

const NODES: usize = 10_000;
const EDGES: usize = 50_000;
const MEMORY: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Frozen {
    nodes: Vec<String>,
    edges: Vec<GraphEdge>,
    generations: Vec<Uuid>,
}
impl Frozen {
    fn from_selected(selected: &read::Selected) -> Self {
        let mut edges = selected.edges.clone();
        edges.sort_by_key(|e| e.id);
        let mut generations: Vec<_> = selected.view.inputs.iter().map(|g| g.id).collect();
        generations.push(selected.view.generation.id);
        generations.sort();
        generations.dedup();
        Self {
            nodes: selected.nodes.keys().cloned().collect(),
            edges,
            generations,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Score {
    key: String,
    score: Option<f64>,
    group: Option<i64>,
}
fn parameters(algorithm: &str) -> Result<Value> {
    Ok(match algorithm {
        "pagerank" => {
            json!({"concurrency":1,"dampingFactor":0.85,"maxIterations":50,"tolerance":0.000001,"scaler":"None"})
        }
        "leiden" => {
            json!({"concurrency":1,"randomSeed":42,"maxLevels":10,"gamma":1.0,"theta":0.01,"tolerance":0.0001})
        }
        "wcc" => json!({"concurrency":1}),
        _ => {
            return Err(Error::invalid(
                "Choose PageRank, Leiden or connected components.",
            ));
        }
    })
}
fn validate(input: &AnalyticsRequest) -> Result<Value> {
    let config = parameters(&input.algorithm)?;
    if !matches!(input.direction.as_str(), "outgoing" | "incoming" | "both")
        || (input.algorithm != "pagerank" && input.direction != "both")
        || input.scope.relations.is_empty()
        || input.scope.relations.iter().any(|r| {
            if input.scope.kind == "knowledge" {
                !KNOWLEDGE.contains(&r.as_str())
            } else {
                !(STRUCTURAL.contains(&r.as_str())
                    || input.scope.kind == "combined" && r == "terraform_module")
            }
        })
    {
        return Err(Error::invalid(
            "Select meaningful relations for this graph. Communities/components require both directions; PageRank also accepts outgoing or incoming.",
        ));
    }
    Ok(config)
}
async fn analytics_epoch(tx: &mut Tx<'_>, brain: Uuid) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&mut **tx)
            .await?,
    )
}
async fn privacy_sequence(tx: &mut Tx<'_>, brain: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(max(sequence),0) FROM privacy_requests WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_one(&mut **tx)
    .await?)
}
async fn selected(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    brain: Uuid,
    scope: GraphSelection,
) -> Result<read::Selected> {
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let mut input = read::select(state, tx, auth, brain, scope, NODES, EDGES).await?;
    // A withheld conflict may expire and admit a different vertex without a
    // write. Preserve all Brain TTL transitions after qualification started.
    let ttl:Option<DateTime<Utc>>=sqlx::query_scalar("WITH deadlines AS (
     SELECT recollect_retention_deadline(brain_id,retention_class,created_at) AS at FROM source_versions WHERE brain_id=$1 AND privacy_state='active'
     UNION ALL SELECT recollect_retention_deadline(brain_id,'claim',recorded_at) FROM claim_revisions WHERE brain_id=$1 AND privacy_state='active'
     UNION ALL SELECT recollect_retention_deadline(brain_id,'repository',created_at) FROM repository_snapshots WHERE brain_id=$1 AND privacy_state='active')
     SELECT min(at) FROM deadlines WHERE at>$2")
     .bind(brain).bind(at).fetch_one(&mut **tx).await?;
    input.deadline = [input.deadline, ttl].into_iter().flatten().min();
    Ok(input)
}
async fn report(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<AnalyticsReport> {
    let row:Option<Json<AnalyticsReport>>=sqlx::query_scalar("SELECT to_jsonb(r)-'descriptor'-'scores' FROM analytics_reports r WHERE brain_id=$1 AND id=$2")
      .bind(brain).bind(id).fetch_optional(&mut **tx).await?;
    row.map(|r| r.0).ok_or_else(Error::missing)
}
async fn descriptor(tx: &mut Tx<'_>, brain: Uuid, id: Uuid) -> Result<Frozen> {
    let row: Option<Json<Frozen>> =
        sqlx::query_scalar("SELECT descriptor FROM analytics_reports WHERE brain_id=$1 AND id=$2")
            .bind(brain)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    row.map(|r| r.0).ok_or_else(changed)
}
fn changed() -> Error {
    Error(
        StatusCode::CONFLICT,
        "analytics_inputs_changed",
        "This analysis no longer matches eligible inputs. Queue a new report from the current selection.",
    )
}
async fn deadline(tx: &mut Tx<'_>, at: Option<DateTime<Utc>>) -> Result<()> {
    let valid: bool = sqlx::query_scalar("SELECT $1::timestamptz IS NULL OR $1>clock_timestamp()")
        .bind(at)
        .fetch_one(&mut **tx)
        .await?;
    if !valid {
        return Err(changed());
    }
    Ok(())
}
async fn current(
    state: &AppState,
    tx: &mut Tx<'_>,
    auth: &Auth,
    r: &AnalyticsReport,
    operation: Option<Uuid>,
) -> Result<read::Selected> {
    if !matches!(r.state.as_str(), "queued" | "running" | "ready")
        || analytics_epoch(tx, r.brain_id).await? != r.analytics_epoch
    {
        return Err(changed());
    }
    deadline(tx, r.expires_at).await?;
    let mut scope = r.selection.clone().ok_or_else(changed)?;
    scope.operation_id = operation;
    let input = selected(state, tx, auth, r.brain_id, scope).await?;
    if Frozen::from_selected(&input) != descriptor(tx, r.brain_id, r.id).await? {
        return Err(changed());
    }
    deadline(tx, input.deadline).await?;
    Ok(input)
}
async fn lease(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<()> {
    let owns:Option<bool>=sqlx::query_scalar("SELECT lease_until>clock_timestamp() FROM jobs WHERE id=$1 AND brain_id=$2 AND state='running' AND lease_token=$3 FOR UPDATE")
     .bind(job.id).bind(job.brain_id).bind(job.lease_token).fetch_optional(&mut **tx).await?;
    if owns != Some(true) {
        return Err(failure(
            "analytics_lease_lost",
            "This analysis no longer owns its worker lease.",
        ));
    }
    Ok(())
}
async fn job_auth(tx: &mut Tx<'_>, job: &ClaimedJob) -> Result<Auth> {
    let user: Json<User> = sqlx::query_scalar("SELECT to_jsonb(a) FROM accounts a WHERE id=$1")
        .bind(job.actor_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(Auth {
        user: user.0,
        session: Uuid::nil(),
        csrf: Uuid::nil(),
        device_id: job.device_id,
    })
}
