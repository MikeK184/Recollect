//! One candidate pass per read transaction. The canonical candidate chain
//! (`retrieval_candidates.sql`) is evaluated once into a transaction-local table;
//! coverage flags, ranking, semantic admission/scoring and the project brief all
//! read that table. It is `ON COMMIT DROP`, written only here and never a source
//! of truth: a new transaction (after query embedding) rebuilds it.
use super::*;

pub(super) const TABLE: &str = "recall_pool";

const FULL_DDL: &str = "CREATE TEMP TABLE recall_pool (
  kind text,id uuid,revision_id uuid,chunk_id uuid,label text,text text,recorded_at timestamptz,
  selection jsonb,source_id uuid,repository_id uuid,snapshot_id uuid,revision text,path text,
  line_from integer,line_to integer,byte_from integer,byte_to integer,artifact_id uuid,
  byte_length integer,processing text,data jsonb,expires_at timestamptz,
  exact_match boolean,lexical_match boolean,rank real,status_eligible boolean
) ON COMMIT DROP";
const FULL_COLUMNS: &str = "kind,id,revision_id,chunk_id,label,text,recorded_at,selection,source_id,repository_id,snapshot_id,revision,path,line_from,line_to,byte_from,byte_to,artifact_id,byte_length,processing,data,expires_at,exact_match,lexical_match,rank,status_eligible";
// Admission needs identities only. Canonical text stays out of the table so a
// large fresh projection can be refused within the read deadline.
const SLIM_DDL: &str = "CREATE TEMP TABLE recall_pool (
  kind text,id uuid,revision_id uuid,chunk_id uuid,status_eligible boolean
) ON COMMIT DROP";
const SLIM_COLUMNS: &str = "kind,id,revision_id,chunk_id,status_eligible";

/// Columns every full-pool consumer selects as a `Candidate`.
pub(super) const CANDIDATE: &str = "kind,id,revision_id,chunk_id,label,text,recorded_at,selection,source_id,repository_id,snapshot_id,revision,path,line_from,line_to,byte_from,byte_to,artifact_id,byte_length,processing,data,expires_at,exact_match,lexical_match,rank";

/// How much of the Brain the pass must cover. A lexical window reaches only rows
/// that can match (through the `recall_vector` GIN indexes), exact identifiers
/// and coverage rows; semantic scoring and the project brief rank populations
/// that do not depend on the query text, so they need every eligible row.
#[derive(Clone, Copy)]
pub(super) struct Breadth {
    pub claims: bool,
    pub rest: bool,
}

// Window arguments: the parsed query when the lexical channel is requested, the
// literal text and the exact identifier when the exact channel is. A NULL
// argument disables its branch inside the window function.
const WINDOW_QUERY: &str =
    "CASE WHEN 'lexical'=ANY($10::text[]) AND $3<>'' THEN websearch_to_tsquery('simple',$3) END";
const WINDOW_LITERAL: &str = "CASE WHEN 'exact'=ANY($10::text[]) AND $3<>'' THEN $3 END";

fn window(function: &str, kind: &str) -> String {
    format!(
        "SELECT w FROM {function}($1,{WINDOW_QUERY},{WINDOW_LITERAL},CASE WHEN 'exact'=ANY($10::text[]) AND $8='{kind}' THEN $9 END) w"
    )
}

fn requested(breadth: Breadth) -> (String, String) {
    let mut ctes = String::new();
    let mut windows = String::new();
    // Row-level security keeps the app role from using the GIN and literal
    // indexes directly (their operators are not leakproof), so the windows are
    // resolved by SECURITY DEFINER functions that return identifiers only.
    if !breadth.claims {
        ctes += &format!(
            "recall_requested_claims AS MATERIALIZED ({}),\n",
            window("recollect_window_claims", "claim")
                .replace("SELECT w FROM", "SELECT w AS claim_id FROM")
        );
    }
    if !breadth.rest {
        ctes += &format!(
            "recall_requested_sources AS MATERIALIZED ({}),
recall_requested_facts AS MATERIALIZED ({}),
recall_requested_manifests AS MATERIALIZED ({}),\n",
            window("recollect_window_sources", "source_version")
                .replace("SELECT w FROM", "SELECT w AS source_id FROM"),
            window("recollect_window_facts", "repository_fact")
                .replace("SELECT w FROM", "SELECT w AS id FROM"),
            window("recollect_window_manifests", "manifest_revision")
                .replace("SELECT w FROM", "SELECT w AS id FROM"),
        );
        windows += "recall_facts AS MATERIALIZED (
  SELECT f.* FROM repository_facts f WHERE f.brain_id=$1 AND f.id IN (SELECT id FROM recall_requested_facts)
), recall_manifests AS MATERIALIZED (
  SELECT r.* FROM manifest_revisions r WHERE r.brain_id=$1 AND r.id IN (SELECT id FROM recall_requested_manifests)
),\n";
    }
    (ctes, windows)
}

/// The shared candidate chain with its optional lexical windows applied through
/// the same markers the graph channel uses for its entity windows.
pub(super) fn chain(breadth: Breadth) -> String {
    let mut sql = include_str!("../retrieval_candidates.sql").to_string();
    let (ctes, windows) = requested(breadth);
    sql = sql
        .replace("/* graph_requested_ctes */", &ctes)
        .replace("/* graph_window_ctes */", &windows);
    if !breadth.claims {
        sql = sql.replace(
            "/* graph_window:claim_histories */",
            "AND claim_id IN (SELECT claim_id FROM recall_requested_claims)",
        );
    }
    if !breadth.rest {
        sql = sql
            .replace(
                "/* graph_window:capture_sources */",
                "AND e.source_id IN (SELECT source_id FROM recall_requested_sources)",
            )
            .replace(
                "/* graph_window:source_knowledge */",
                "AND v.source_id IN (SELECT source_id FROM recall_requested_sources)",
            )
            // Latest-per-logical-identity lists keep their whole histories but
            // only for the identities the windows reached.
            .replace(
                "/* recall_window:known_snapshots */",
                "AND repository_id IN (SELECT s.repository_id FROM repository_snapshots s WHERE s.brain_id=$1 AND s.id IN (SELECT snapshot_id FROM recall_facts))",
            )
            .replace(
                "/* recall_window:known_manifests */",
                "AND manifest_id IN (SELECT m.manifest_id FROM manifest_revisions m WHERE m.brain_id=$1 AND m.id IN (SELECT id FROM recall_manifests))",
            )
            .replace("/* graph_window:facts */ repository_facts", "recall_facts")
            .replace(
                "/* graph_window:manifests */ manifest_revisions",
                "recall_manifests",
            );
    }
    sql
}

/// Build the pool for this transaction. `slim` keeps identities only.
pub(super) async fn build(
    tx: &mut Tx<'_>,
    context: &ReadContext<'_>,
    breadth: Breadth,
    slim: bool,
) -> Result<()> {
    let (ddl, columns) = if slim {
        (SLIM_DDL, SLIM_COLUMNS)
    } else {
        (FULL_DDL, FULL_COLUMNS)
    };
    sqlx::query(ddl).execute(&mut **tx).await?;
    let input = context.input;
    let sql = format!(
        "INSERT INTO {TABLE}({columns}) {} SELECT {columns} FROM matched",
        chain(breadth)
    );
    sqlx::query(&sql)
        .bind(context.brain)
        .bind(context.at)
        .bind(&input.query)
        .bind(SqlJson(&input.selection))
        .bind(input.collection_id)
        .bind(context.manifest.map(SqlJson))
        .bind(&input.mode)
        .bind(input.exact.as_ref().map(|e| e.kind.as_str()))
        .bind(input.exact.as_ref().map(|e| e.id))
        .bind(&input.channels)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Plan of the lexical-window pass for `query` over `brain`: `EXPLAIN` only, so
/// nothing is read or written beyond an empty temporary table in the caller's
/// transaction. Lets a populated-Brain test prove the GIN indexes are eligible.
pub(super) async fn explain(tx: &mut Tx<'_>, brain: Uuid, query: &str) -> Result<Vec<String>> {
    sqlx::query(FULL_DDL).execute(&mut **tx).await?;
    let at: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let sql = format!(
        "EXPLAIN INSERT INTO {TABLE}({FULL_COLUMNS}) {} SELECT {FULL_COLUMNS} FROM matched",
        chain(Breadth {
            claims: false,
            rest: false
        })
    );
    Ok(sqlx::query_scalar(&sql)
        .bind(brain)
        .bind(at)
        .bind(query)
        .bind(SqlJson(ScopeSelection::default()))
        .bind(Option::<Uuid>::None)
        .bind(Option::<SqlJson<ManifestRevision>>::None)
        .bind("investigation")
        .bind(Option::<&str>::None)
        .bind(Option::<Uuid>::None)
        .bind(vec!["exact".to_string(), "lexical".to_string()])
        .fetch_all(&mut **tx)
        .await?)
}
