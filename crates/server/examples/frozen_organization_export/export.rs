//! Strict scalar projection from freshly authenticated ShadowResults only.
use super::{
    extraction::Observed,
    io,
    owned::{Owned, api, http},
};
use anyhow::{Context, Result, ensure};
use recollect_server::{
    auth::Auth,
    graph::analytics::{
        self,
        shadow::{self, Mode, ShadowResult, ShadowSource},
    },
    knowledge_mapping::source_features::HybridParameters,
    worker,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use uuid::Uuid;
pub const MODES: [Mode; 5] = [
    Mode::NativeHybrid,
    Mode::NativeWeighted,
    Mode::EntityOverlap,
    Mode::Metadata,
    Mode::EligibleVector,
];
pub fn parameters() -> HybridParameters {
    HybridParameters {
        neighbors: 3,
        minimum_cosine: 0.65,
        vector_scale: 0.25,
    }
}
pub fn group_id(brain: Uuid, id: &str) -> Result<String> {
    let id = format!("{brain}:{id}");
    ensure!(id.len() <= 256, "Encoded group ID exceeds bound");
    Ok(id)
}
pub fn scalar(
    mode: Mode,
    all: &[Observed],
    results: &[ShadowResult],
    failed: &BTreeSet<Uuid>,
) -> Result<Value> {
    ensure!(all.len() <= 250, "Source count exceeded");
    let by_version: BTreeMap<_, _> = all
        .iter()
        .filter_map(|s| {
            s.imported
                .as_ref()
                .map(|i| (i.version, (s.toy.id, i.brain)))
        })
        .collect();
    ensure!(
        by_version.len() == all.iter().filter(|s| s.imported.is_some()).count(),
        "Duplicate source versions"
    );
    let mut rows = BTreeMap::new();
    let mut groups = vec![];
    let mut ids = BTreeSet::new();
    let mut memberships = 0usize;
    for r in results {
        ensure!(
            r.mode == mode && r.parameters == parameters() && !failed.contains(&r.brain_id),
            "Result tuple/Brain mismatch"
        );
        let expected: BTreeSet<_> = by_version
            .iter()
            .filter(|(_, (_, b))| *b == r.brain_id)
            .map(|(id, _)| *id)
            .collect();
        ensure!(
            r.sources
                .iter()
                .map(|s| s.source_version_id)
                .collect::<BTreeSet<_>>()
                == expected
                && r.sources.len() == expected.len(),
            "Result cohort foreign/stale/missing"
        );
        let mut actual_groups = BTreeMap::new();
        for g in &r.groups {
            let encoded = group_id(r.brain_id, &g.id)?;
            ensure!(
                ids.insert(encoded.clone()) && !g.source_versions.is_empty(),
                "Duplicate/empty native group"
            );
            let mut members = vec![];
            let mut unique = BTreeSet::new();
            for v in &g.source_versions {
                let (id, brain) = by_version.get(v).context("Foreign group member")?;
                ensure!(
                    *brain == r.brain_id && unique.insert(*v),
                    "Cross-Brain or duplicate native member"
                );
                members.push(*id);
            }
            members.sort();
            memberships += members.len();
            ensure!(
                actual_groups.insert(g.id.clone(), unique).is_none(),
                "Repeated original group ID"
            );
            groups.push(json!({"id":encoded,"members":members}));
        }
        for s in &r.sources {
            let (id, brain) = by_version[&s.source_version_id];
            ensure!(brain == r.brain_id, "Foreign source Brain");
            let expected_groups: BTreeSet<_> = actual_groups
                .iter()
                .filter(|(_, m)| m.contains(&s.source_version_id))
                .map(|(id, _)| id.clone())
                .collect();
            ensure!(
                s.groups.iter().cloned().collect::<BTreeSet<_>>() == expected_groups
                    && s.groups.len() == expected_groups.len(),
                "Bidirectional membership mismatch"
            );
            ensure!(
                s.state
                    == if s.groups.is_empty() {
                        "unassigned"
                    } else {
                        "assigned"
                    },
                "Native state/membership mismatch"
            );
            let bound = all
                .iter()
                .find(|v| v.toy.id == id)
                .context("Document missing")?;
            ensure!(
                s.candidate_id == bound.candidate
                    && s.request_id
                        == bound.provenance["request"]["request_id"]
                            .as_str()
                            .filter(|_| bound.candidate.is_some())
                            .map(str::parse::<Uuid>)
                            .transpose()?,
                "Native candidate/request tuple changed"
            );
            ensure!(
                !s.ner_feature_available || bound.extraction["state"] == "validated",
                "Fabricated NER flag"
            );
            ensure!(rows.insert(id,json!({"document_id":id,"state":s.state,"ner_feature_available":s.ner_feature_available,"vector_feature_available":s.vector_feature_available,"groups":s.groups.iter().map(|g|group_id(r.brain_id,g)).collect::<Result<Vec<_>>>()?})).is_none(),"Repeated native source");
        }
    }
    for s in all {
        if let Some(i) = &s.imported {
            if failed.contains(&i.brain) {
                ensure!(!rows.contains_key(s.toy.id), "Failed Brain also succeeded");
                rows.insert(s.toy.id,json!({"document_id":s.toy.id,"state":"failed","ner_feature_available":false,"vector_feature_available":false,"groups":[]}));
            } else {
                ensure!(rows.contains_key(s.toy.id), "Imported source omitted");
            }
        } else {
            ensure!(
                s.toy.text.is_empty() && s.extraction["state"] == "not_attempted",
                "Only exact empty can skip import"
            );
            rows.insert(s.toy.id,json!({"document_id":s.toy.id,"state":"ineligible","ner_feature_available":false,"vector_feature_available":false,"groups":[]}));
        }
    }
    ensure!(
        groups.len() <= 10000 && memberships <= 50000,
        "Native output bounds exceeded"
    );
    groups.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(
        json!({"name":mode,"parameters":parameters(),"sources":rows.into_values().collect::<Vec<_>>(),"groups":groups}),
    )
}
pub async fn isolate(o: &Owned, out: &Path) -> Result<()> {
    o.verify().await?;
    let jobs:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'brain_id',brain_id,'kind',kind,'state',state) FROM jobs WHERE lane='heavy' AND state IN('queued','running') ORDER BY created_at,id").fetch_all(&o.admin).await?;
    ensure!(
        jobs.iter().all(|j| j["state"] == "queued"
            && matches!(
                j["kind"].as_str(),
                Some("graph.project" | "knowledge.fields")
            )),
        "Unrelated heavy work"
    );
    io::json(
        &out.join("initial-heavy-jobs.json"),
        &json!({"jobs":jobs,"owned_database":o.database}),
    )?;
    for j in jobs {
        let path = format!(
            "/api/brains/{}/jobs/{}/cancel",
            j["brain_id"].as_str().context("Brain")?,
            j["id"].as_str().context("Job")?
        );
        http(&o.state, Some(&o.login), "POST", &path, Value::Null).await?;
    }
    Ok(())
}
pub async fn cancel(o: &Owned, auth: &Auth, brain: Uuid, report: Uuid, job: Uuid) -> Result<Value> {
    o.verify().await?;
    let exact:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs j JOIN analytics_reports r ON r.job_id=j.id AND r.brain_id=j.brain_id WHERE j.id=$1 AND j.brain_id=$2 AND j.target_id=$3 AND j.actor_id=$4 AND j.kind='graph.analyze' AND r.id=$3 AND r.purpose='organization_shadow')").bind(job).bind(brain).bind(report).bind(o.actor).fetch_one(&o.admin).await?;
    ensure!(exact, "Cancellation exact ownership mismatch");
    analytics::reconcile(&o.state.config, &o.state.pool).await?;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM analytics_attempts WHERE brain_id=$1 AND report_id=$2 AND cleaned_at IS NULL").bind(brain).bind(report).fetch_one(&o.admin).await?;
    ensure!(n == 0, "Owned journal still open");
    let state: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=$1")
        .bind(job)
        .fetch_one(&o.admin)
        .await?;
    if matches!(state.as_str(), "queued" | "running") {
        let _ = api(recollect_server::jobs::cancel(
            axum::extract::State(o.state.clone()),
            auth.clone(),
            axum::extract::Path((brain, job)),
        )
        .await)?;
    }
    Ok(
        json!({"report_id":report,"job_id":job,"journal_closed":true,"cancelled_only_exact_owned_job":true}),
    )
}
pub async fn mode(
    o: &Owned,
    auth: &Auth,
    all: &[Observed],
    mode: Mode,
    gds: &str,
) -> Result<(Value, Value, Vec<ShadowResult>)> {
    let mut inputs = BTreeMap::<Uuid, Vec<ShadowSource>>::new();
    for s in all {
        if let Some(i) = &s.imported {
            inputs.entry(i.brain).or_default().push(ShadowSource {
                source_version_id: i.version,
                candidate_id: s.candidate,
            });
        }
    }
    let mut results = vec![];
    let mut failed = BTreeSet::new();
    let mut records = vec![];
    for (brain, mut sources) in inputs {
        sources.sort_by_key(|s| s.source_version_id);
        let queued = Box::pin(shadow::queue(
            &o.state,
            auth,
            brain,
            &sources,
            mode,
            parameters(),
            gds,
        ))
        .await;
        let report = match queued {
            Ok(id) => id,
            Err(e) => {
                failed.insert(brain);
                records.push(json!({"brain_id":brain,"state":"queue_rejected","error_code":e.1,"sources":sources}));
                continue;
            }
        };
        let job_id:Uuid=sqlx::query_scalar("SELECT job_id FROM analytics_reports WHERE brain_id=$1 AND id=$2 AND purpose='organization_shadow'").bind(brain).bind(report).fetch_one(&o.admin).await?;
        let pending: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM jobs WHERE lane='heavy' AND state IN('queued','running') ORDER BY id",
        )
        .fetch_all(&o.admin)
        .await?;
        ensure!(
            pending == vec![job_id],
            "Unrelated heavy job prevents exact claim"
        );
        let job = worker::claim(&o.state.pool, "heavy")
            .await?
            .context("Exact heavy job missing")?;
        ensure!(
            job.id == job_id
                && job.brain_id == brain
                && job.target_id == report
                && job.actor_id == o.actor
                && job.kind == "graph.analyze",
            "Heavy claim mismatch"
        );
        let execution = Box::pin(worker::execute(&o.state, &job)).await;
        let fresh = if execution.is_ok() {
            shadow::result(&o.state, auth, brain, report).await
        } else {
            Err(recollect_server::error::Error::invalid(
                "Native worker failed",
            ))
        };
        let mut record = report_provenance(o, brain, report, job_id).await?;
        record["expected_gds_version"] = json!(gds);
        match fresh {
            Ok(result) => {
                record["state"] = json!("freshly_qualified");
                record["result"] = serde_json::to_value(&result)?;
                record["group_id_encoding"]=json!(result.groups.iter().map(|g|Ok(json!({"scalar_id":group_id(brain,&g.id)?,"brain_id":brain,"original_group_id":g.id}))).collect::<Result<Vec<_>>>()?);
                results.push(result);
            }
            Err(e) => {
                failed.insert(brain);
                record["state"] = json!("failed");
                record["error_code"] = json!(e.1);
                record["worker_failure"] = json!(execution.err().map(|f| format!("{f:?}")));
                record["owned_cleanup"] = cancel(o, auth, brain, report, job_id).await?;
            }
        }
        records.push(record);
    }
    Ok((
        scalar(mode, all, &results, &failed)?,
        json!({"name":mode,"parameters":parameters(),"reports":records}),
        results,
    ))
}
async fn report_provenance(o: &Owned, brain: Uuid, report: Uuid, job: Uuid) -> Result<Value> {
    let report_row:Value=sqlx::query_scalar("SELECT jsonb_build_object('report_id',id,'job_id',job_id,'brain_id',brain_id,'actor_id',actor_id,'state',state,'error_code',error_code,'returned_gds_version',gds_version,'scores_sha256',CASE WHEN scores IS NULL THEN NULL ELSE encode(sha256(convert_to(scores::text,'UTF8')),'hex') END,'parameters',parameters,'expires_at',expires_at,'descriptor_sha256',encode(sha256(convert_to(descriptor::text,'UTF8')),'hex'),'analytics_epoch',analytics_epoch,'privacy_sequence',privacy_sequence) FROM analytics_reports WHERE brain_id=$1 AND id=$2 AND job_id=$3").bind(brain).bind(report).bind(job).fetch_one(&o.admin).await?;
    let calc:Value=sqlx::query_scalar("SELECT jsonb_build_object('projection_sha256',encode(sha256(convert_to((preparation->'projection')::text,'UTF8')),'hex'),'stored_result_sha256',CASE WHEN result IS NULL THEN NULL ELSE encode(sha256(convert_to(result::text,'UTF8')),'hex') END,'preparation_sha256',encode(sha256(convert_to(preparation::text,'UTF8')),'hex')) FROM knowledge_shadow_calculations WHERE brain_id=$1 AND report_id=$2").bind(brain).bind(report).fetch_one(&o.admin).await?;
    let attempts:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('attempt_id',id,'brain_id',brain_id,'report_id',report_id,'job_id',job_id,'cleaned_at',cleaned_at,'created_at',created_at) FROM analytics_attempts WHERE brain_id=$1 AND report_id=$2 ORDER BY created_at,id").bind(brain).bind(report).fetch_all(&o.admin).await?;
    Ok(json!({"report":report_row,"stored_digests":calc,"native_attempts":attempts}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_group_numbers_are_brain_qualified() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        assert_ne!(group_id(a, "0").unwrap(), group_id(b, "0").unwrap());
        assert!(group_id(a, &"g".repeat(230)).is_err());
        assert_eq!(parameters().neighbors, 3);
    }
}

#[cfg(test)]
mod strict_projection_tests {
    use super::*;
    use crate::{scopes::Imported, toy};
    use shadow::{Group, LabelProvenance, SourceOutcome};
    fn one() -> (Vec<Observed>, ShadowResult) {
        let brain = Uuid::from_u128(1);
        let version = Uuid::from_u128(2);
        let candidate = Uuid::from_u128(3);
        let request = Uuid::from_u128(4);
        let observed = Observed {
            toy: toy::fixtures().remove(0),
            imported: Some(Imported {
                brain,
                source: Uuid::from_u128(5),
                version,
                device: Uuid::from_u128(6),
                operation: Uuid::from_u128(7),
                policy: Uuid::from_u128(8),
                selection: serde_json::from_value(
                    json!({"repository_ids":[],"area_ids":[],"environment_id":null}),
                )
                .unwrap(),
            }),
            extraction: json!({"state":"validated"}),
            candidate: Some(candidate),
            provenance: json!({"request":{"request_id":request}}),
        };
        let result = ShadowResult {
            brain_id: brain,
            report_id: Uuid::from_u128(9),
            mode: Mode::NativeHybrid,
            parameters: parameters(),
            gds_version: Some("2026.08.1".into()),
            expires_at: chrono::Utc::now(),
            sources: vec![SourceOutcome {
                source_version_id: version,
                candidate_id: Some(candidate),
                request_id: Some(request),
                state: "unassigned".into(),
                ner_feature_available: true,
                vector_feature_available: false,
                groups: vec![],
            }],
            groups: vec![],
            vector_profile_id: None,
            vector_entry_ids: vec![],
            vector_coverage: "test".into(),
        };
        (vec![observed], result)
    }
    #[test]
    fn scalar_rejects_wrong_tuple_stale_candidate_request_and_foreign_source() {
        let (all, r) = one();
        assert!(scalar(r.mode, &all, std::slice::from_ref(&r), &BTreeSet::new()).is_ok());
        let mut wrong = r.clone();
        wrong.parameters.neighbors = 5;
        assert!(scalar(r.mode, &all, &[wrong], &BTreeSet::new()).is_err());
        for field in 0..3 {
            let mut wrong = r.clone();
            if field == 0 {
                wrong.sources[0].candidate_id = Some(Uuid::from_u128(30));
            } else if field == 1 {
                wrong.sources[0].request_id = Some(Uuid::from_u128(40));
            } else {
                wrong.sources[0].source_version_id = Uuid::from_u128(20);
            }
            assert!(scalar(r.mode, &all, &[wrong], &BTreeSet::new()).is_err());
        }
    }
    #[test]
    fn scalar_requires_bidirectional_membership_and_keeps_exact_empty() {
        let (mut all, mut r) = one();
        let v = r.sources[0].source_version_id;
        r.groups.push(Group {
            id: "local:0".into(),
            kind: "native_community".into(),
            label: "controlled".into(),
            label_provenance: LabelProvenance {
                kind: "canonical_title".into(),
                source_version_id: v,
                byte_start: None,
                byte_end: None,
                quote: None,
                field_kind: None,
                field_id: None,
            },
            source_versions: vec![v],
        });
        assert!(scalar(r.mode, &all, &[r.clone()], &BTreeSet::new()).is_err());
        r.sources[0].groups = vec!["local:0".into()];
        r.sources[0].state = "assigned".into();
        let empty = toy::fixtures()
            .into_iter()
            .find(|t| t.id == "empty_a")
            .unwrap();
        all.push(Observed {
            toy: empty,
            imported: None,
            extraction: json!({"state":"not_attempted"}),
            candidate: None,
            provenance: Value::Null,
        });
        let value = scalar(r.mode, &all, &[r.clone()], &BTreeSet::new()).unwrap();
        assert_eq!(value["sources"].as_array().unwrap().len(), 2);
        assert_eq!(value["sources"][0]["state"], "ineligible");
        r.groups[0].source_versions.push(Uuid::from_u128(500));
        assert!(scalar(r.mode, &all, &[r], &BTreeSet::new()).is_err());
    }
}
