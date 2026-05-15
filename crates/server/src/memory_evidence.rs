use crate::{
    AppState, artifacts,
    auth::Auth,
    db,
    error::{Error, Result},
    publication,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use recollect_protocol::*;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(crate) type Tx<'a> = Transaction<'a, Postgres>;
pub(crate) async fn read_tx<'a>(state: &'a AppState, auth: &Auth, brain: Uuid) -> Result<Tx<'a>> {
    let mut tx = auth.tx(&state.pool).await?;
    db::lock_brain(&mut tx, brain, false).await?;
    db::require_role(&mut tx, brain, false).await?;
    Ok(tx)
}
#[derive(sqlx::FromRow)]
pub(crate) struct EvidenceRow {
    pub kind: String,
    pub id: Uuid,
    label: String,
    pub source_id: Option<Uuid>,
    pub repository_id: Option<Uuid>,
    pub snapshot_id: Option<Uuid>,
    revision: Option<String>,
    reference: Option<String>,
    observed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub artifact_id: Option<Uuid>,
    pub byte_length: i32,
    pub data: Value,
    pub privacy_state: String,
    pub expires_at: Option<DateTime<Utc>>,
}
impl EvidenceRow {
    pub fn privacy(&self) -> &str {
        crate::retention::effective_state(&self.privacy_state, self.expires_at)
    }
    pub async fn choice(&self, state: &AppState, brain: Uuid) -> ClaimEvidenceChoice {
        let removed = self.privacy() != "active";
        let availability = if removed {
            self.privacy()
        } else if self.kind == "source_version" {
            if let Some(artifact) = self.artifact_id {
                match artifacts::read(
                    &state.config.artifact_dir,
                    brain,
                    artifact,
                    self.byte_length,
                )
                .await
                {
                    Ok(_) => "retained",
                    Err(artifacts::ReadFailure::Missing) => "missing",
                    Err(artifacts::ReadFailure::Invalid) => "unreadable",
                    Err(artifacts::ReadFailure::Unavailable) => "unavailable",
                }
            } else {
                "reference_only"
            }
        } else {
            "retained"
        };
        let removed = self.privacy() != "active";
        let availability = if removed {
            self.privacy()
        } else {
            availability
        };
        ClaimEvidenceChoice {
            kind: self.kind.clone(),
            id: self.id,
            label: if removed {
                format!("{} evidence", self.privacy())
            } else {
                self.label.clone()
            },
            source_id: self.source_id,
            repository_id: self.repository_id,
            snapshot_id: self.snapshot_id,
            revision: self.revision.clone(),
            reference: if removed {
                None
            } else {
                self.reference.clone()
            },
            observed_at: if removed { None } else { self.observed_at },
            created_at: self.created_at,
            availability: availability.into(),
        }
    }
}
const EVIDENCE: &str = "
SELECT 'source_version'::text AS kind,v.id,CASE WHEN p.state='active' THEN v.title ELSE p.state||' source' END AS label,v.source_id,NULL::uuid AS repository_id,NULL::uuid AS snapshot_id,NULL::text AS revision,CASE WHEN p.state='active' THEN v.source_uri END AS reference,CASE WHEN p.state='active' THEN v.observed_at END AS observed_at,v.recorded_at AS created_at,CASE WHEN p.state='active' THEN v.artifact_id END AS artifact_id,CASE WHEN p.state='active' THEN v.byte_length ELSE 0 END AS byte_length,CASE WHEN p.state='active' THEN jsonb_build_object('source_id',v.source_id,'media_type',v.media_type,'contributor',a.username,'device_id',v.device_id) ELSE jsonb_build_object('privacy_state',p.state) END AS data,p.state AS privacy_state,recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at) AS expires_at
FROM recollect_source_knowledge v JOIN accounts a ON a.id=v.created_by CROSS JOIN LATERAL (SELECT recollect_content_state(v.brain_id,v.retention_class,v.privacy_state,v.created_at) AS state) p WHERE v.brain_id=$1
UNION ALL
SELECT 'repository_fact',f.id,CASE WHEN p.state='active' THEN left(coalesce(f.record->>'name',f.record->>'kind','Repository fact'),500) ELSE p.state||' fact' END,NULL,s.repository_id,s.id,s.revision,NULL,NULL,s.created_at,NULL,0,CASE WHEN p.state='active' THEN f.record ELSE jsonb_build_object('privacy_state',p.state) END,p.state,recollect_retention_deadline(s.brain_id,'repository',s.created_at)
FROM repository_facts f JOIN repository_snapshots s ON s.id=f.snapshot_id CROSS JOIN LATERAL (SELECT recollect_content_state(s.brain_id,'repository',s.privacy_state,s.created_at) AS state) p WHERE f.brain_id=$1
UNION ALL
SELECT 'manifest_revision',r.id,coalesce(r.revision->>'name','Erased manifest'),NULL,NULL,NULL,r.id::text,r.revision->>'observation_reference',(r.revision->>'observed_at')::timestamptz,r.created_at,NULL,0,r.revision,r.privacy_state,NULL
FROM manifest_revisions r WHERE r.brain_id=$1";
pub(crate) async fn row(tx: &mut Tx<'_>, brain: Uuid, kind: &str, id: Uuid) -> Result<EvidenceRow> {
    sqlx::query_as(&format!(
        "SELECT * FROM ({EVIDENCE}) e WHERE kind=$2 AND id=$3"
    ))
    .bind(brain)
    .bind(kind)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(Error::missing)
}
#[derive(Default, Deserialize)]
pub struct EvidenceQuery {
    kind: Option<String>,
    repository_id: Option<Uuid>,
    search: Option<String>,
    offset: Option<i64>,
}
#[utoipa::path(get,path="/api/brains/{brain}/claim-evidence",operation_id="claimEvidence",params(("brain"=Uuid,Path),("kind"=Option<String>,Query),("repository_id"=Option<Uuid>,Query),("search"=Option<String>,Query),("offset"=Option<i64>,Query)),responses((status=200,body=ClaimEvidencePage)))]
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(brain): Path<Uuid>,
    Query(query): Query<EvidenceQuery>,
) -> Result<Json<ClaimEvidencePage>> {
    let offset = publication::offset(&publication::Page {
        offset: query.offset,
    })?;
    if query.kind.as_ref().is_some_and(|k| {
        !matches!(
            k.as_str(),
            "source_version" | "repository_fact" | "manifest_revision"
        )
    }) || query.search.as_ref().is_some_and(|s| s.len() > 256)
    {
        return Err(Error::invalid(
            "Choose a supported evidence kind and search text within 256 bytes.",
        ));
    }
    let mut tx = read_tx(&state, &auth, brain).await?;
    if let Some(repo) = query.repository_id {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM repositories WHERE brain_id=$1 AND id=$2)",
        )
        .bind(brain)
        .bind(repo)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(Error::missing());
        }
    }
    let filter = format!(
        "FROM ({EVIDENCE}) e WHERE ($2::text IS NULL OR kind=$2) AND ($3::uuid IS NULL OR repository_id=$3) AND ($4::text IS NULL OR strpos(lower(label),lower($4))>0)"
    );
    let total = sqlx::query_scalar(&format!("SELECT count(*) {filter}"))
        .bind(brain)
        .bind(&query.kind)
        .bind(query.repository_id)
        .bind(&query.search)
        .fetch_one(&mut *tx)
        .await?;
    let rows: Vec<EvidenceRow> = sqlx::query_as(&format!(
        "SELECT * {filter} ORDER BY created_at DESC,kind,id LIMIT 20 OFFSET $5"
    ))
    .bind(brain)
    .bind(query.kind)
    .bind(query.repository_id)
    .bind(query.search)
    .bind(offset)
    .fetch_all(&mut *tx)
    .await?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.choice(&state, brain).await);
    }
    tx.commit().await?;
    Ok(Json(ClaimEvidencePage {
        items,
        total,
        offset,
    }))
}
#[utoipa::path(get,path="/api/brains/{brain}/claim-evidence/{kind}/{evidence}",operation_id="claimEvidenceDetail",params(("brain"=Uuid,Path),("kind"=String,Path),("evidence"=Uuid,Path)),responses((status=200,body=ClaimEvidenceDetail)))]
pub async fn detail(
    State(state): State<AppState>,
    auth: Auth,
    Path((brain, kind, id)): Path<(Uuid, String, Uuid)>,
) -> Result<Json<ClaimEvidenceDetail>> {
    let mut tx = read_tx(&state, &auth, brain).await?;
    let row = row(&mut tx, brain, &kind, id).await?;
    let mut evidence = row.choice(&state, brain).await;
    let mut text = if let Some(artifact) = row.artifact_id.filter(|_| row.privacy() == "active") {
        match artifacts::read(&state.config.artifact_dir, brain, artifact, row.byte_length).await {
            Ok(text) => Some(text),
            Err(_) => {
                evidence.availability = "unavailable".into();
                None
            }
        }
    } else {
        None
    };
    tx.commit().await?;
    if row.privacy() != "active" {
        evidence = row.choice(&state, brain).await;
        text = None;
    }
    Ok(Json(ClaimEvidenceDetail {
        evidence,
        data: if row.privacy() == "active" {
            row.data
        } else {
            serde_json::json!({"privacy_state":row.privacy()})
        },
        text,
        expires_at: row.expires_at,
    }))
}
