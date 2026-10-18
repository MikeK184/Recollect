//! Exact packed evidence for the answering consumer. Construction is private
//! to the server; client-supplied RecallResponse objects never enter this path.
use super::*;
use crate::{answers, model_gateway::InputRef};

pub(crate) struct Bundle {
    auth: Auth,
    request_id: Uuid,
    input: RecallRequest,
    response: RecallResponse,
}

impl Bundle {
    pub(crate) fn new(
        auth: &Auth,
        request_id: Uuid,
        input: RecallRequest,
        response: RecallResponse,
    ) -> Self {
        Self {
            auth: Auth {
                user: auth.user.clone(),
                session: auth.session,
                csrf: Uuid::nil(),
                device_id: auth.device_id,
            },
            request_id,
            input,
            response,
        }
    }

    pub(crate) fn response(&self) -> &RecallResponse {
        &self.response
    }

    /// Re-check every selected item and graph witness; returns the graph
    /// evidence as currently qualified so `resolve` need not evaluate it again.
    pub(crate) async fn revalidate(
        &self,
        state: &AppState,
        tx: &mut Tx<'_>,
    ) -> Result<Vec<RecallItem>> {
        let brain = self.response.brain_id;
        answers::active(tx, &self.auth, brain, self.request_id).await?;
        let epoch: i64 = sqlx::query_scalar(
            "SELECT coalesce((SELECT epoch FROM memory_epochs WHERE brain_id=$1),0)",
        )
        .bind(brain)
        .fetch_one(&mut **tx)
        .await?;
        if epoch != self.response.memory_epoch
            || self.response.expires_at.is_some_and(|at| at <= Utc::now())
        {
            return Err(answers::stale());
        }
        authority(tx, &self.auth, brain, &self.input).await?;
        let manifest =
            selected_manifest(tx, brain, &self.input, self.response.knowledge_at).await?;
        let context = ReadContext {
            brain,
            input: &self.input,
            at: self.response.knowledge_at,
            manifest: manifest.as_ref(),
        };
        // Reapply the canonical filters/item gate at the original frozen time
        // and exact revision/span for every item with a single pass over the
        // candidate pipeline. Do not rank, embed or substitute.
        let items = &self.response.context.items;
        let spans: Vec<_> = items
            .iter()
            .map(|original| {
                original
                    .provenance
                    .iter()
                    .find(|p| p.kind == "source_version" && p.id == original.revision_id)
            })
            .collect();
        let kinds: Vec<&str> = items.iter().map(|i| i.kind.as_str()).collect();
        let revisions: Vec<Uuid> = items.iter().map(|i| i.revision_id).collect();
        let byte_from: Vec<Option<i32>> = spans.iter().map(|p| p.and_then(|p| p.byte_from)).collect();
        let byte_to: Vec<Option<i32>> = spans.iter().map(|p| p.and_then(|p| p.byte_to)).collect();
        let mut rows: Vec<Candidate> = vec![];
        if !items.is_empty() {
            let sql = format!("{} SELECT kind,id,revision_id,chunk_id,label,text,recorded_at,selection,source_id,repository_id,
                snapshot_id,revision,path,line_from,line_to,byte_from,byte_to,artifact_id,byte_length,
                processing,data,expires_at,exact_match,lexical_match,rank FROM (
                  SELECT m.kind,m.id,m.revision_id,m.chunk_id,m.label,m.text,m.recorded_at,m.selection,m.source_id,m.repository_id,
                    m.snapshot_id,m.revision,m.path,m.line_from,m.line_to,m.byte_from,m.byte_to,m.artifact_id,m.byte_length,
                    m.processing,m.data,m.expires_at,m.exact_match,m.lexical_match,m.rank,
                    row_number() OVER (PARTITION BY w.ordinal) AS candidate_number
                  FROM matched m JOIN unnest($11::text[],$12::uuid[],$13::integer[],$14::integer[])
                    WITH ORDINALITY AS w(kind,revision_id,byte_from,byte_to,ordinal)
                    ON m.kind=w.kind AND m.revision_id=w.revision_id
                   AND (w.byte_from IS NULL OR m.byte_from=w.byte_from)
                   AND (w.byte_to IS NULL OR m.byte_to=w.byte_to)
                  WHERE m.status_eligible
                ) candidates WHERE candidate_number<=2", include_str!("../retrieval_candidates.sql"));
            rows = sqlx::query_as(&sql)
                .bind(brain)
                .bind(self.response.knowledge_at)
                .bind(&self.input.query)
                .bind(SqlJson(&self.input.selection))
                .bind(self.input.collection_id)
                .bind(manifest.as_ref().map(SqlJson))
                .bind(&self.input.mode)
                .bind(self.input.exact.as_ref().map(|r| r.kind.as_str()))
                .bind(self.input.exact.as_ref().map(|r| r.id))
                .bind(&self.input.channels)
                .bind(&kinds)
                .bind(&revisions)
                .bind(&byte_from)
                .bind(&byte_to)
                .fetch_all(&mut **tx)
                .await?;
        }
        for (index, original) in items.iter().enumerate() {
            let mut matched = false;
            for row in rows.iter().filter(|row| {
                row.kind == original.kind
                    && row.revision_id == original.revision_id
                    && byte_from[index].is_none_or(|from| row.byte_from == Some(from))
                    && byte_to[index].is_none_or(|to| row.byte_to == Some(to))
            }) {
                if let Some((current, _)) =
                    item(state, tx, &context, row, &mut RecallCoverage::default()).await?
                    && current.id == original.id
                    && current.revision_id == original.revision_id
                    && current.text == original.text
                    && current.label == original.label
                    && current.recorded_at == original.recorded_at
                    && current.selection == original.selection
                    && current
                        .qualifications
                        .iter()
                        .all(|q| original.qualifications.contains(q))
                    && serde_json::to_value(&current.provenance).ok()
                        == serde_json::to_value(&original.provenance).ok()
                    && serde_json::to_value(&current.claim).ok()
                        == serde_json::to_value(&original.claim).ok()
                {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return Err(answers::stale());
            }
        }
        // Graph witnesses include intermediate nodes and edges, not merely
        // the final matching record. Check the whole selected witness again.
        let graph_items = crate::graph::recall::bundle_evidence(
            state,
            tx,
            &self.auth,
            &self.input,
            &self.response,
        )
        .await?;
        if self.response.expires_at.is_some_and(|at| at <= Utc::now()) {
            return Err(answers::stale());
        }
        Ok(graph_items)
    }

    pub(crate) async fn resolve(
        &self,
        state: &AppState,
        tx: &mut Tx<'_>,
    ) -> Result<(String, Vec<String>, Vec<InputRef>)> {
        let graph_items = self.revalidate(state, tx).await?;
        let brain = self.response.brain_id;
        let mut classes = BTreeSet::new();
        let mut dependencies = Vec::new();
        for item in self.response.context.items.iter().chain(graph_items.iter()) {
            match item.kind.as_str() {
                "claim" => {
                    classes.insert("claim".to_owned());
                    dependencies.push(InputRef {
                        kind: "claim_revision".into(),
                        id: item.revision_id,
                    });
                }
                "source_version" => {
                    let class: String = sqlx::query_scalar(
                        "SELECT retention_class FROM source_versions WHERE brain_id=$1 AND id=$2",
                    )
                    .bind(brain)
                    .bind(item.revision_id)
                    .fetch_one(&mut **tx)
                    .await?;
                    classes.insert(class);
                    let original: Option<String> = sqlx::query_scalar("SELECT original_class FROM automatic_support_excerpts WHERE brain_id=$1 AND version_id=$2 AND privacy_state='active'")
                        .bind(brain).bind(item.revision_id).fetch_optional(&mut **tx).await?;
                    if let Some(original) = original {
                        classes.insert(original);
                    }

                    dependencies.push(InputRef {
                        kind: "source_version".into(),
                        id: item.revision_id,
                    });
                }
                "repository_fact" => {
                    classes.insert("repository".to_owned());
                    dependencies.push(InputRef {
                        kind: "repository_fact".into(),
                        id: item.revision_id,
                    });
                }
                "manifest_revision" => {
                    classes.insert("repository".to_owned());
                }
                _ => return Err(answers::stale()),
            }
            dependencies.extend(
                item.provenance
                    .iter()
                    .filter(|p| {
                        matches!(p.kind.as_str(), "source_version" | "repository_fact")
                            && p.availability == "retained"
                    })
                    .map(|p| InputRef {
                        kind: p.kind.clone(),
                        id: p.id,
                    }),
            );
        }
        dependencies.sort_by(|a, b| (&a.kind, a.id).cmp(&(&b.kind, b.id)));
        dependencies.dedup();
        for dependency in &dependencies {
            let fenced = match dependency.kind.as_str() {
                "source_version" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM model_input_fences WHERE brain_id=$1 AND source_version_id=$2)")
                    .bind(brain).bind(dependency.id).fetch_one(&mut **tx).await?,
                "claim_revision" => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM model_claim_fences WHERE brain_id=$1 AND revision_id=$2)")
                    .bind(brain).bind(dependency.id).fetch_one(&mut **tx).await?,
                _ => false,
            };
            if fenced {
                return Err(answers::stale());
            }
        }
        let data = json!({
            "instruction": self.response.context.instruction,
            "mode": self.response.context.mode,
            "selection": self.response.selection,
            "manifest_revision_id": self.response.manifest_revision_id,
            "knowledge_at": self.response.knowledge_at,
            "fact_at": self.response.fact_at,
            "memory_epoch": self.response.memory_epoch,
            "expires_at": self.response.expires_at,
            "coverage": self.response.coverage,
            "evidence": self.citations(),
        })
        .to_string();
        publication::safe_payload(state, &json!(&data))?;
        Ok((data, classes.into_iter().collect(), dependencies))
    }

    pub(crate) fn citations(&self) -> Vec<AnswerCitation> {
        self.response
            .context
            .items
            .iter()
            .enumerate()
            .map(|(index, evidence)| AnswerCitation {
                id: format!("E{}", index + 1),
                evidence: evidence.clone(),
            })
            .collect()
    }

    pub(crate) async fn admitted(&self, tx: &mut Tx<'_>, model_request: Uuid) -> Result<()> {
        sqlx::query("UPDATE answer_requests SET state='answering',model_request_id=$4 WHERE brain_id=$1 AND actor_id=$2 AND id=$3 AND state='retrieving' AND NOT cancel_requested")
            .bind(self.response.brain_id).bind(self.auth.user.id).bind(self.request_id).bind(model_request)
            .execute(&mut **tx).await?;
        Ok(())
    }
}
