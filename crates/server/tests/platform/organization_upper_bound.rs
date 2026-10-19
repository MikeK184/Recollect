//! Frozen idealized extractor input, actual canonical source qualification and
//! native GDS. The synthetic ledger is fixture data, never a provider receipt.
//! This comparison cannot select or qualify any production producer or recipe.
use super::*;
use axum::extract::FromRequestParts;
use recollect_server::{
    auth::Auth,
    graph::analytics,
    knowledge_mapping::{self, affinity, fields, storage},
};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS: &str = include_str!(
    "../../../../docs/research/graph-organization-adversarial-fixtures-2026-10-18.json"
);
const RECIPE: &str = include_str!("../fixtures/organization_upper_bound_full_recipe.json");

async fn hash(h: &Harness, bytes: &[u8]) -> String {
    sqlx::query_scalar("SELECT encode(sha256($1::bytea),'hex')")
        .bind(bytes)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn auth(h: &Harness, owner: &Login) -> Auth {
    let (mut parts, _) = Request::builder()
        .method("POST")
        .header("cookie", &owner.cookie)
        .header("x-csrf-token", &owner.csrf)
        .body(())
        .unwrap()
        .into_parts();
    Auth::from_request_parts(&mut parts, &h.state)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1))
}
fn bid(base: &str) -> Uuid {
    base.rsplit('/').next().unwrap().parse().unwrap()
}

/// Deliberately synthetic durable extraction ledger: no gateway invocation.
/// The real candidate parser/storage and complete current-origin checks still
/// run; this is an extractor upper bound, never automatic producer proof.
async fn seed_gold(
    h: &Harness,
    owner: &Login,
    base: &str,
    source: &Value,
    selection: &Value,
    document: &Value,
    profile: Uuid,
) -> Option<Uuid> {
    let version: Uuid = source["version"]["id"].as_str().unwrap().parse().unwrap();
    let mentions: Vec<_> = document["mentions"].as_array().unwrap().iter().map(|m|
        json!({"key":m[0],"kind":m[1],"span":{"byte_start":m[2],"byte_end":m[3],"quote":m[4]},"confidence":1.0})
    ).collect();
    let payload = json!({"schema":"source-mentions-1","adapter_revision":"gold-upper-bound-1","source_version_id":version,"language":document["language"],"mentions":mentions,"aliases":[],"relations":[]});
    let encoded = serde_json::to_vec(&payload).unwrap();
    if document["language"] != "en" {
        assert_eq!(
            knowledge_mapping::source_candidates(
                version,
                document["text"].as_str().unwrap(),
                &encoded
            )
            .unwrap_err(),
            knowledge_mapping::CandidateFailure::UnsupportedLanguage
        );
        return None;
    }
    let principal = auth(h, owner).await;
    let prepared = storage::prepare_source(&h.state, &principal, bid(base), version, &encoded)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let input = storage::stage_source(&h.state, &principal, prepared)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    let run=ok(h,"POST",&format!("{base}/learning"),owner,json!({"source_version_id":version,"selection":selection,"manifest_revision_id":null,"operation_id":null})).await;
    let run_id: Uuid = run["id"].as_str().unwrap().parse().unwrap();
    let request = Uuid::new_v4();
    let brain = bid(base);
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("INSERT INTO model_requests(id,brain_id,actor_id,operation_id,policy_id,purpose,provider,model,returned_model,prompt_label,schema_label,state,call_token,reserved_tokens,charged_tokens,input_tokens,output_tokens,total_tokens,finished_at) SELECT $1,r.brain_id,r.actor_id,r.id,r.policy_id,'extraction',p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model','source-facts-2','source-facts-2','succeeded',gen_random_uuid(),1,0,0,0,0,clock_timestamp() FROM learning_runs r JOIN model_policies p ON p.brain_id=r.brain_id AND p.id=r.policy_id WHERE r.id=$2")
        .bind(request).bind(run_id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO model_request_inputs(request_id,brain_id,kind,input_id) VALUES($1,$2,'source_version',$3)")
        .bind(request).bind(brain).bind(version).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE learning_runs SET state='succeeded',request_id=$2,finished_at=clock_timestamp() WHERE id=$1")
        .bind(run_id).bind(request).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE jobs SET state='succeeded',progress=100,lease_token=NULL,lease_until=NULL WHERE id=(SELECT job_id FROM learning_runs WHERE id=$1)")
        .bind(run_id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO learning_support_stages(run_id,brain_id,extraction_request_id,assessment_operation_id,verifier_version,payload) VALUES($1,$2,$3,gen_random_uuid(),'source-support-3','{}')")
        .bind(run_id).bind(brain).bind(request).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE knowledge_mapping_inputs SET producer_request_id=$2,producer_policy_id=(SELECT policy_id FROM learning_runs WHERE id=$3),producer_change_id=$4 WHERE id=$1")
        .bind(input).bind(request).bind(run_id).bind(profile).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO knowledge_mapping_origins(input_id,brain_id,run_id,request_id,policy_id,producer_change_id,offered_inputs) SELECT $1,$2,$3,$4,policy_id,$5,jsonb_build_array(jsonb_build_object('kind','source_version','id',$6::uuid)) FROM learning_runs WHERE id=$3")
        .bind(input).bind(brain).bind(run_id).bind(request).bind(profile).bind(version).execute(&mut *tx).await.unwrap();
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(principal.user.id.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let current: bool = sqlx::query_scalar("SELECT recollect_mapping_origin_current($1,$2)")
        .bind(brain)
        .bind(input)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(
        current,
        "Gold fixture still passes exact canonical source/context/current policy/origin gates"
    );
    sqlx::query("INSERT INTO knowledge_mapping_coverage(run_id,brain_id,request_id,state,input_id) VALUES($1,$2,$3,'staged',$4)")
        .bind(run_id).bind(brain).bind(request).bind(input).execute(&mut *tx).await.unwrap();
    let audit = recollect_server::db::audit(
        &mut tx,
        principal.user.id,
        brain,
        "organization.upper_bound.seed",
        input,
        "synthetic_gold_no_provider",
    )
    .await
    .unwrap_or_else(|e| panic!("{}", e.1));
    recollect_server::jobs::enqueue_work(
        &mut tx,
        principal.user.id,
        brain,
        audit,
        input,
        "knowledge.map",
        "heavy",
    )
    .await
    .unwrap_or_else(|e| panic!("{}", e.1));
    tx.commit().await.unwrap();
    Some(input)
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL/Neo4j/GDS; zero model/provider calls"]
async fn organization_frozen_twenty_document_gold_upper_bound_native_comparison() {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    let recipe: Value = serde_json::from_str(RECIPE).unwrap();
    let (h, owner, alpha, p, server) = setup().await;
    assert_eq!(
        hash(&h, CORPUS.as_bytes()).await,
        recipe["corpus_sha256"].as_str().unwrap()
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        hash(
            &h,
            &std::fs::read(root.join("scripts/score-topics.py")).unwrap()
        )
        .await,
        recipe["scorer_sha256"].as_str().unwrap(),
        "Freeze scorer before native output; never tune held-out outcomes"
    );
    let docs = corpus["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 20);

    let beta_value = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Gold upper-bound beta"}),
    )
    .await;
    let beta = format!("/api/brains/{}", beta_value["id"].as_str().unwrap());
    let bases = BTreeMap::from([("alpha".to_string(), alpha), ("beta".to_string(), beta)]);
    let principal = auth(&h, &owner).await;
    let mut profiles = BTreeMap::new();
    for (name, base) in &bases {
        allow(&h, &owner, base, |policy| {
            policy["autonomous_memory"] = json!(false);
            policy["purposes"] = json!(["extraction"]);
        })
        .await;
        let profile = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_mapping_producer_profiles(change_id,brain_id,provider,requested_model,returned_model,prompt_label,schema_label,adapter_revision,mapping_schema,quality_state,quality_receipt_sha256) SELECT $1,$2,p.policy->>'provider',p.policy->>'text_model',p.policy->>'text_model','source-facts-2','source-facts-2','gold-upper-bound-1','source-mentions-1','qualified',repeat('d',64) FROM model_policy_heads ph JOIN model_policies p ON p.brain_id=ph.brain_id AND p.id=ph.policy_id WHERE ph.brain_id=$2")
            .bind(profile).bind(bid(base)).execute(&h.admin).await.unwrap();
        sqlx::query(
            "INSERT INTO knowledge_mapping_producer_heads(brain_id,change_id) VALUES($1,$2)",
        )
        .bind(bid(base))
        .bind(profile)
        .execute(&h.admin)
        .await
        .unwrap();
        profiles.insert(name.clone(), profile);
    }
    let mut repositories = BTreeMap::<(String, String), (Uuid, String)>::new();
    let mut environments = BTreeMap::<(String, String), (Uuid, String)>::new();
    let mut sources = BTreeMap::<String, (Value, Value)>::new();
    let mut metadata = BTreeMap::<String, Vec<Value>>::new();
    let mut versions = BTreeMap::<Uuid, String>::new();
    for doc in docs {
        let id = doc["id"].as_str().unwrap().to_string();
        let scope: Vec<_> = doc["scope"].as_str().unwrap().split('/').collect();
        let base = &bases[scope[0]];
        let brain = bid(base);
        let mut groups = vec![];
        let mut repository_ids = vec![];
        let mut environment = None;
        if scope[1] != "unknown" {
            let key = (scope[0].to_string(), scope[1].to_string());
            if !repositories.contains_key(&key) {
                let rid = Uuid::new_v4();
                let origin = format!(
                    "https://organization-fixture.invalid/{}/{}.git",
                    scope[0], scope[1]
                );
                sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)").bind(rid).bind(brain).bind(&origin).bind(principal.user.id).execute(&h.admin).await.unwrap();
                repositories.insert(key.clone(), (rid, origin));
            }
            let (rid, label) = &repositories[&key];
            repository_ids.push(*rid);
            groups.push(json!({"id":format!("metadata:{brain}:repository:{rid}"),"label":label,"provenance_kind":"canonical_scope","canonical_reference":{"kind":"repository","id":rid}}));
        }
        if scope[2] != "unknown" {
            let key = (scope[0].to_string(), scope[2].to_string());
            if !environments.contains_key(&key) {
                let group = ok(
                    &h,
                    "POST",
                    &format!("{base}/evidence/groups"),
                    &owner,
                    json!({"kind":"environment","name":scope[2]}),
                )
                .await;
                environments.insert(
                    key.clone(),
                    (
                        group["id"].as_str().unwrap().parse().unwrap(),
                        group["name"].as_str().unwrap().to_string(),
                    ),
                );
            }
            let (eid, label) = &environments[&key];
            environment = Some(*eid);
            groups.push(json!({"id":format!("metadata:{brain}:environment:{eid}"),"label":label,"provenance_kind":"canonical_scope","canonical_reference":{"kind":"environment","id":eid}}));
        }
        let s = source(&h, &owner, base, doc["text"].as_str().unwrap()).await;
        if let Some(eid) = environment {
            ok(
                &h,
                "PUT",
                &format!("{base}/sources/{}/groups", s["id"].as_str().unwrap()),
                &owner,
                json!({"group_ids":[eid]}),
            )
            .await;
        }
        let version: Uuid = s["version"]["id"].as_str().unwrap().parse().unwrap();
        let selection =
            json!({"repository_ids":repository_ids,"area_ids":[],"environment_id":environment});
        sqlx::query(
            "INSERT INTO source_import_scopes(version_id,brain_id,selection) VALUES($1,$2,$3)",
        )
        .bind(version)
        .bind(brain)
        .bind(sqlx::types::Json(&selection))
        .execute(&h.admin)
        .await
        .unwrap();
        versions.insert(version, id.clone());
        metadata.insert(id.clone(), groups);
        sources.insert(id, (s, selection));
    }
    production_mapping::process(&h).await;
    let mut inputs = BTreeMap::new();
    for doc in docs {
        let id = doc["id"].as_str().unwrap();
        let realm = doc["scope"].as_str().unwrap().split('/').next().unwrap();
        let (s, selection) = &sources[id];
        if let Some(input) = seed_gold(
            &h,
            &owner,
            &bases[realm],
            s,
            selection,
            doc,
            profiles[realm],
        )
        .await
        {
            inputs.insert(input, id.to_string());
        }
    }
    // Independently run the actual canonical field pipeline before clustering.
    for _ in 0..20 {
        let n = fields::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("fields {}", e.1));
        while worker::run_once(&h.state, "heavy").await.unwrap() {}
        if n == 0 {
            break;
        }
    }
    let publication_rows: Vec<(Uuid,String,bool,String,Option<String>)>=sqlx::query_as("SELECT i.id,i.state,o.published_at IS NOT NULL,j.state,j.error_code FROM knowledge_mapping_inputs i JOIN knowledge_mapping_origins o ON o.brain_id=i.brain_id AND o.input_id=i.id JOIN jobs j ON j.brain_id=i.brain_id AND j.target_id=i.id AND j.kind='knowledge.map' WHERE i.id=ANY($1) ORDER BY i.id")
        .bind(inputs.keys().copied().collect::<Vec<_>>()).fetch_all(&h.admin).await.unwrap();
    let publication_outcomes:Vec<_>=publication_rows.iter().map(|(input,state,sealed,job,code)|json!({"document_id":inputs[input],"input_state":state,"origin_sealed":sealed,"job_state":job,"job_error":code})).collect();
    std::fs::write(
        root.join(".cache/automatic-mapping/organization-gold-publication-controls.json"),
        serde_json::to_vec_pretty(&publication_outcomes).unwrap(),
    )
    .unwrap();
    assert_eq!(
        publication_rows.len(),
        19,
        "Every English document, including empty d12, needs one actual publication job"
    );
    assert!(
        publication_rows
            .iter()
            .all(|(_, state, sealed, job, code)| state == "published"
                && *sealed
                && job == "succeeded"
                && code.is_none()),
        "Publication failures are retained separately and cannot be counted as ordinary unassigned documents"
    );
    let mut field_receipts = vec![];
    for (name, base) in &bases {
        let response = ok(
            &h,
            "POST",
            &format!("{base}/organization/fields/view"),
            &owner,
            json!({"scope":{"kind":"knowledge"},"entity_id":null,"cursor":null}),
        )
        .await;
        assert!(
            response["next_cursor"].is_null(),
            "The independent metadata receipt must cover the complete fixture"
        );
        for doc in docs
            .iter()
            .filter(|doc| doc["scope"].as_str().unwrap().split('/').next() == Some(name.as_str()))
        {
            let id = doc["id"].as_str().unwrap();
            for group in &metadata[id] {
                let reference = &group["canonical_reference"];
                let entity=response["entities"].as_array().unwrap().iter().find(|e|e["reference_id"]==reference["id"] && e["kind"]==reference["kind"]).expect("Metadata baseline reference must be independently visible through the canonical typed field API");
                assert_eq!(entity["label"], group["label"]);
                assert!(
                    response["origins"].as_array().unwrap().iter().any(
                        |origin| origin["entity_id"] == entity["id"]
                            && origin["value"] == reference["id"]
                            && origin["revision_id"] == sources[id].0["version"]["id"]
                    ),
                    "Every metadata membership needs its own exact source-context field origin"
                );
            }
        }
        field_receipts.push(json!({"brain":name,"view":response}));
        let recipe_id = Uuid::new_v4();
        sqlx::query("INSERT INTO knowledge_organization_recipes(change_id,brain_id,recipe,gds_version,quality_state,quality_receipt_sha256) VALUES($1,$2,$3,'2026.08.1','qualified',repeat('e',64))").bind(recipe_id).bind(bid(base)).bind(affinity::RECIPE).execute(&h.admin).await.unwrap();
        sqlx::query("INSERT INTO knowledge_organization_heads(brain_id,change_id) VALUES($1,$2)")
            .bind(bid(base))
            .bind(recipe_id)
            .execute(&h.admin)
            .await
            .unwrap();
    }
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        0,
        "No provider is used by any baseline"
    );
    assert_eq!(analytics::run_once(&h.state).await.unwrap(), 2);
    while worker::run_once(&h.state, "heavy").await.unwrap() {}
    let mut native_groups = BTreeMap::<String, Vec<Value>>::new();
    let mut native_runs = vec![];
    let mut native_failed = BTreeSet::new();
    for (name, base) in &bases {
        let brain = bid(base);
        let status:(Uuid,String,Option<String>,Option<String>,i32,i32,sqlx::types::Json<Value>)=sqlx::query_as("SELECT r.id,r.state,r.error_code,r.gds_version,r.node_count,r.edge_count,c.projection FROM analytics_reports r JOIN knowledge_organization_calculations c ON c.report_id=r.id WHERE r.brain_id=$1 AND r.purpose='organization' ORDER BY r.created_at DESC LIMIT 1").bind(brain).fetch_one(&h.admin).await.unwrap();
        native_runs.push(json!({"brain":name,"state":status.1,"error_code":status.2,"gds_version":status.3,"nodes":status.4,"edges":status.5,"projection_sha256":hash(&h,&serde_json::to_vec(&status.6.0).unwrap()).await}));
        if status.1 != "ready" {
            native_failed.insert(name.clone());
            continue;
        }
        assert_eq!(status.3.as_deref(), Some("2026.08.1"));
        let page = ok(
            &h,
            "POST",
            &format!("{base}/organization/view"),
            &owner,
            json!({"scope":{"kind":"knowledge"},"limit":50}),
        )
        .await;
        let topics:Vec<(Uuid,String,Vec<Uuid>)>=sqlx::query_as("SELECT t.id,t.kind,array_agg(DISTINCT i.source_version_id ORDER BY i.source_version_id) FROM knowledge_topics t JOIN knowledge_topic_memberships m ON m.brain_id=t.brain_id AND m.topic_id=t.id JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE t.brain_id=$1 AND t.kind IN ('community','facet') GROUP BY t.id,t.kind").bind(brain).fetch_all(&h.admin).await.unwrap();
        for (topic, family, members) in topics {
            let card = page["topics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == json!(topic))
                .expect("Actual native topic must pass qualified API");
            let label = card["label"].as_str().unwrap();
            let (version,start,end,quote):(Uuid,i32,i32,String)=sqlx::query_as("SELECT i.source_version_id,m.byte_start,m.byte_end,m.quote FROM knowledge_topic_memberships tm JOIN knowledge_mentions m ON m.brain_id=tm.brain_id AND m.input_id=tm.input_id AND m.entity_id=tm.entity_id JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE tm.brain_id=$1 AND tm.topic_id=$2 AND m.quote=$3 ORDER BY m.id LIMIT 1").bind(brain).bind(topic).bind(label).fetch_one(&h.admin).await.unwrap();
            let group = json!({"id":format!("native:{brain}:{topic}"),"family":family,"label":label,"provenance_kind":"exact_entity_mention","label_evidence":{"document_id":versions[&version],"byte_start":start,"byte_end":end,"quote":quote}});
            for version in members {
                native_groups
                    .entry(versions[&version].clone())
                    .or_default()
                    .push(group.clone());
            }
        }
    }
    let mut entity_groups = BTreeMap::<String, Vec<Value>>::new();
    let rows:Vec<(Uuid,Uuid,Uuid,i32,i32,String)>=sqlx::query_as("SELECT DISTINCT ON(e.id,i.id) e.brain_id,e.id,i.source_version_id,m.byte_start,m.byte_end,m.quote FROM knowledge_mentions m JOIN knowledge_entities e ON e.brain_id=m.brain_id AND e.id=m.entity_id JOIN knowledge_mapping_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE i.state='published' AND e.kind IN ('technology','concept') ORDER BY e.id,i.id,m.id").fetch_all(&h.admin).await.unwrap();
    let mut representative = BTreeMap::new();
    for (brain, entity, version, start, end, quote) in &rows {
        representative.entry(*entity).or_insert_with(||json!({"id":format!("entity:{brain}:{entity}"),"label":quote,"provenance_kind":"exact_entity_mention","label_evidence":{"document_id":versions[version],"byte_start":start,"byte_end":end,"quote":quote}}));
    }
    for (_, entity, version, _, _, _) in rows {
        entity_groups
            .entry(versions[&version].clone())
            .or_default()
            .push(representative[&entity].clone());
    }
    let outcome_rows =
        |members: &BTreeMap<String, Vec<Value>>, extractor: bool, native: bool| -> Vec<Value> {
            docs.iter()
                .map(|doc| {
                    let id = doc["id"].as_str().unwrap();
                    let brain = doc["scope"].as_str().unwrap().split('/').next().unwrap();
                    let groups = members.get(id).cloned().unwrap_or_default();
                    let outcome = if native && native_failed.contains(brain) {
                        "failed"
                    } else if doc["language"] != "en" && extractor {
                        "unsupported"
                    } else if groups.is_empty() {
                        "unassigned"
                    } else {
                        "assigned"
                    };
                    json!({"id":id,"outcome":outcome,"groups":groups})
                })
                .collect()
        };
    let receipt = json!({"corpus_sha256":hash(&h,CORPUS.as_bytes()).await,"recipe_sha256":hash(&h,RECIPE.as_bytes()).await,"scorer_sha256":recipe["scorer_sha256"],"input_mode":"gold_upper_bound","gds_version":"2026.08.1","model_calls":p.calls.load(Ordering::SeqCst),"paid_model_calls":0,"production_recipe_qualified":false,"native_runs":native_runs,"publication_outcomes":publication_outcomes,"canonical_field_receipts":field_receipts,"baselines":[
        {"name":"native_weighted","state":if native_failed.is_empty(){"completed"}else{"failed"},"documents":outcome_rows(&native_groups,true,true)},
        {"name":"metadata","state":"completed","documents":outcome_rows(&metadata,false,false)},
        {"name":"entity_overlap","state":"completed","documents":outcome_rows(&entity_groups,true,false)},
        {"name":"eligible_vector","state":"not_measured","reason":"No real retained compatible vectors in this independent gold fixture; no embeddings generated","documents":docs.iter().map(|d|json!({"id":d["id"],"outcome":"unassigned","groups":[]})).collect::<Vec<_>>()}]});
    let path = root.join(".cache/automatic-mapping/organization-gold-upper-bound.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_eq!(p.calls.load(Ordering::SeqCst), 0);
    assert!(
        native_failed.is_empty(),
        "All native outcomes are retained in the receipt"
    );
    server.abort();
    h.finish().await;
}
