//! Unmocked browser/API/worker proof in an owned disposable database. The model
//! is a loopback HTTP fixture; its selected quality profile is synthetic plumbing.
use super::*;
use recollect_server::autonomous;
use std::{io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf, time::Duration};

/// Exercise the actual UI after a production publisher converted an unmatched
/// edited topic. The caller owns the native grouping/replacement fixture.
pub(super) async fn assert_manual_overlay_browser(
    h: &Harness,
    owner: &Login,
    base: &str,
    topic: Uuid,
    label: &str,
    versions: &[Uuid],
) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dist = std::env::var_os("RECOLLECT_ORGANIZATION_UI_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("web/dist"));
    assert!(
        dist.join("index.html").is_file(),
        "Build the current frontend before native overlay UI proof"
    );
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    let sources: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,title FROM source_versions WHERE brain_id=$1 AND id=ANY($2) ORDER BY id",
    )
    .bind(brain)
    .bind(versions)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(sources.len(), versions.len());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut config = (*h.state.config).clone();
    config.public_origin = origin.clone();
    config.static_dir = dist.to_string_lossy().into_owned();
    let state = AppState::new(h.state.pool.clone(), config).unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app(state)).await.unwrap() });
    let (name, value) = owner.cookie.split_once('=').unwrap();
    let cache = root.join(".cache/graph-implementation");
    std::fs::create_dir_all(&cache).unwrap();
    let fixture = cache.join(format!(
        "organization-overlay-native-private-{}.json",
        Uuid::new_v4()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&fixture)
        .unwrap();
    file.write_all(&serde_json::to_vec(&json!({
        "origin":origin,"brain_id":brain,"topic_id":topic,"label":label,
        "session_cookie":{"name":name,"value":value},
        "sources":sources.iter().map(|(version,title)|json!({"key":format!("source_version:{version}"),"title":title})).collect::<Vec<_>>()
    })).unwrap()).unwrap();
    drop(file);
    let mut command = tokio::process::Command::new("node");
    command
        .arg(root.join("web/scripts/organization-overlay-native-proof.mjs"))
        .current_dir(&root)
        .env("RECOLLECT_ORGANIZATION_NATIVE_FIXTURE", &fixture)
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENROUTER_API_KEY")
        .kill_on_drop(true);
    let proof = tokio::time::timeout(Duration::from_secs(60), command.output()).await;
    std::fs::remove_file(&fixture).unwrap();
    server.abort();
    let proof = proof
        .expect("Native overlay UI proof exceeded its bounded deadline")
        .unwrap();
    assert!(
        proof.status.success(),
        "Native overlay UI failed: stdout={} stderr={}",
        String::from_utf8_lossy(&proof.stdout),
        String::from_utf8_lossy(&proof.stderr)
    );
}

fn output(text: &str, alias: bool) -> Value {
    let (terms, kind) = if alias {
        (["app", "service"], "service")
    } else {
        (["Rust", "PostgreSQL"], "technology")
    };
    let span = |quote: &str| json!({"quote":quote,"line_from":1,"line_to":1,"occurrence":null});
    json!({"claims":[],"retirements":[],"mapping":{"language":"en","mentions":[
        {"key":"a","kind":kind,"span":span(terms[0]),"confidence":0.99},
        {"key":"b","kind":kind,"span":span(terms[1]),"confidence":0.99}],
        "relations":if alias {json!([])}else{json!([{"from":"a","to":"b","relation":"related_to","span":span(text)}])},
        "aliases":if alias {json!([{"left":"a","right":"b","span":span(text)}])}else{json!([])} }})
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Requires owned PostgreSQL/Neo4j, current built frontend and local Playwright"]
async fn organization_native_browser_uses_automatic_worker_and_unmocked_scoped_api() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dist = std::env::var_os("RECOLLECT_ORGANIZATION_UI_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("web/dist"));
    assert!(
        dist.join("index.html").is_file(),
        "Build current frontend before native browser proof"
    );
    let script = root.join("web/scripts/organization-native-proof.mjs");
    assert!(
        script.is_file(),
        "Native Playwright proof script must exist"
    );
    let (h, owner, base, p, provider_server) = setup().await;
    let brain: Uuid = base.rsplit('/').next().unwrap().parse().unwrap();
    allow(&h, &owner, &base, |policy| {
        policy["autonomous_memory"] = json!(true);
        policy["purposes"] = json!(["extraction", "synthesis", "embedding"]);
        policy["automatic_embedding"] = json!(false);
    })
    .await;
    production_mapping::synthetic_profile(&h, &base, true).await;
    let prod = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"PROD"}),
    )
    .await;
    let dev = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"DEV"}),
    )
    .await;
    let mut sources = vec![];
    for (n, (title, text, environment, alias)) in [
        ("Platform guide", "Rust uses PostgreSQL.", &prod, false),
        (
            "Platform handover",
            "Rust depends on PostgreSQL.",
            &dev,
            false,
        ),
        (
            "Service alias",
            "app is also known as service.",
            &prod,
            true,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        *p.candidates.lock().unwrap() = output(text, alias);
        let source = ok(
            &h,
            "POST",
            &format!("{base}/sources"),
            &owner,
            json!({"title":title,"media_type":"text/plain","content":text,"retain_content":true}),
        )
        .await;
        ok(
            &h,
            "PUT",
            &format!("{base}/sources/{}/groups", source["id"].as_str().unwrap()),
            &owner,
            json!({"group_ids":[environment["id"]]}),
        )
        .await;
        production_mapping::process(&h).await;
        assert!(
            autonomous::run_once(&h.state)
                .await
                .unwrap_or_else(|e| panic!("{}", e.1))
                > 0
        );
        model_job(&h).await;
        let mut published = 0;
        for _ in 0..30 {
            published=sqlx::query_scalar::<_,i64>("SELECT count(*) FROM knowledge_mapping_inputs WHERE brain_id=$1 AND state='published'")
                .bind(base.rsplit('/').next().unwrap().parse::<Uuid>().unwrap()).fetch_one(&h.admin).await.unwrap();
            if published == n as i64 + 1 {
                break;
            }
            assert!(
                worker::run_once(&h.state, "heavy").await.unwrap(),
                "Actual production mapping job must make progress"
            );
        }
        assert_eq!(
            published,
            n as i64 + 1,
            "No browser staging or administrative publication shortcut"
        );
        assert_eq!(
            p.calls.load(Ordering::SeqCst),
            n + 1,
            "Mapping reuses one existing extraction call per source"
        );
        sources.push(json!({"id":source["id"],"version_id":source["version"]["id"],"title":title,"exact_text":text,"expected_quote":if alias {"app"}else{"Rust"}}));
    }
    // Pure canonical metadata preparation uses no additional model request.
    for _ in 0..30 {
        let queued = recollect_server::knowledge_mapping::fields::run_once(&h.state)
            .await
            .unwrap_or_else(|e| panic!("field schedule {}", e.1));
        let mut worked = false;
        for _ in 0..40 {
            if !worker::run_once(&h.state, "heavy").await.unwrap() {
                break;
            }
            worked = true;
        }
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM knowledge_field_outbox WHERE revision>handled_revision",
        )
        .fetch_one(&h.admin)
        .await
        .unwrap();
        if pending == 0 && queued == 0 && !worked {
            break;
        }
    }
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Canonical references add zero extraction requests"
    );
    let field_ready: bool = sqlx::query_scalar(
        "SELECT bool_and(state='ready' AND revision=handled_revision) FROM knowledge_field_outbox",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!(
        field_ready,
        "Canonical references must settle before the native browser proof"
    );
    // This owned browser fixture selects explicitly synthetic finite retained
    // vectors and a synthetic qualified recipe to exercise native source-group
    // plumbing. These rows are not actual embedding/model-quality evidence and
    // never select a production installation profile. No provider call is made.
    let dimensions: i32 = sqlx::query_scalar("SELECT (policy->>'embedding_dimensions')::int FROM model_policies p JOIN model_policy_heads h ON h.brain_id=p.brain_id AND h.policy_id=p.id WHERE p.brain_id=$1")
        .bind(brain).fetch_one(&h.admin).await.unwrap();
    assert!((2..=4096).contains(&dimensions));
    let retained_profile = organization_hybrid::retained_profile(&h, brain).await;
    for (n, source) in sources.iter().enumerate() {
        let mut vector = vec![0.0; dimensions as usize];
        // The two PROD contributors can group; DEV remains independent. The
        // existing entity facet still separately exercises its exact sources.
        vector[usize::from(n == 1)] = 1.0;
        organization_hybrid::retained(
            &h,
            brain,
            retained_profile,
            source["version_id"].as_str().unwrap().parse().unwrap(),
            &vector,
        )
        .await;
    }
    let synthetic_recipe = organization_hybrid::select(&h, brain).await;
    let source_report = organization_hybrid::calculate(&h, &h.state, brain).await;
    let mut source_rows: Vec<(Uuid, Uuid, Uuid)> = sqlx::query_as("SELECT t.id,m.input_id,i.source_version_id FROM knowledge_topics t JOIN knowledge_topic_sources m ON m.brain_id=t.brain_id AND m.topic_id=t.id JOIN knowledge_source_inputs i ON i.brain_id=m.brain_id AND i.id=m.input_id WHERE t.brain_id=$1 AND t.kind='community' AND t.organization_report_id=$2 ORDER BY t.id,m.input_id")
        .bind(brain).bind(source_report).fetch_all(&h.admin).await.unwrap();
    let mut group_sizes = std::collections::BTreeMap::<Uuid, usize>::new();
    for row in &source_rows {
        *group_sizes.entry(row.0).or_default() += 1;
    }
    let selected_topic = group_sizes
        .iter()
        .max_by_key(|(id, count)| (**count, **id))
        .filter(|(_, count)| **count >= 2)
        .map(|(id, _)| *id)
        .expect(
            "The native plumbing fixture provides an actual multi-source group for UI commands",
        );
    source_rows.retain(|row| row.0 == selected_topic);
    assert!(
        (2..=3).contains(&source_rows.len()),
        "Native grouping retains actual source contributors"
    );
    assert!(source_rows.iter().all(|r| r.0 == source_rows[0].0));
    let prod_versions: std::collections::BTreeSet<Uuid> = [0, 2]
        .into_iter()
        .map(|n| sources[n]["version_id"].as_str().unwrap().parse().unwrap())
        .collect();
    let actual_versions: std::collections::BTreeSet<_> = source_rows.iter().map(|r| r.2).collect();
    let known_versions: std::collections::BTreeSet<Uuid> = sources
        .iter()
        .map(|s| s["version_id"].as_str().unwrap().parse().unwrap())
        .collect();
    assert!(actual_versions.is_subset(&known_versions));
    // Shared entity features can also connect DEV in the global view. Record
    // actual native membership rather than tuning the fixture to force a group.
    let actual_titles: Vec<_> = sources
        .iter()
        .filter(|s| {
            actual_versions.contains(&s["version_id"].as_str().unwrap().parse::<Uuid>().unwrap())
        })
        .map(|s| s["title"].clone())
        .collect();
    let source_groups_expected = json!({"topic_id":source_rows[0].0,
        "source_titles":actual_titles,
        "input_ids":source_rows.iter().map(|r|r.1).collect::<Vec<_>>(),
        "source_keys":source_rows.iter().map(|r|format!("source_version:{}",r.2)).collect::<Vec<_>>(),
        "report_id":source_report,"recipe_id":synthetic_recipe,"semantic_profile_id":retained_profile,
        "prod_source_keys":prod_versions.iter().map(|v|format!("source_version:{v}")).collect::<Vec<_>>(),
        "prod_qualified_sources":2,
        "synthetic_quality":true,"quality_measured":false,"embedding_http_calls":0});
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Native source groups reuse existing inputs without embedding or extraction HTTP"
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut config = (*h.state.config).clone();
    config.public_origin = origin.clone();
    config.static_dir = dist.to_string_lossy().into_owned();
    let state = AppState::new(h.state.pool.clone(), config).unwrap();
    let router = app(state);
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let (cookie_name, cookie_value) = owner.cookie.split_once('=').unwrap();
    let cache = root.join(".cache/graph-implementation");
    std::fs::create_dir_all(&cache).unwrap();
    let fixture = cache.join(format!(
        "organization-native-private-{}.json",
        Uuid::new_v4()
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&fixture)
        .unwrap();
    file.write_all(&serde_json::to_vec(&json!({"origin":origin,"brain_id":base.rsplit('/').next().unwrap(),
        "session_cookie":{"name":cookie_name,"value":cookie_value},"sources":sources,"related_titles":["Platform guide","Platform handover"],
        "source_groups_expected":source_groups_expected,
        "alias":{"source_title":"Service alias","quote":"app is also known as service.","from_label":"app","to_label":"service"},
        "environment":{"id":prod["id"],"name":"PROD","expected_titles":["Platform guide","Service alias"]}})).unwrap()).unwrap();
    drop(file);
    let mut command = tokio::process::Command::new("node");
    command
        .arg(&script)
        .current_dir(&root)
        .env("RECOLLECT_ORGANIZATION_NATIVE_FIXTURE", &fixture)
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENROUTER_API_KEY")
        .kill_on_drop(true);
    let proof = tokio::time::timeout(Duration::from_secs(150), command.output())
        .await
        .expect("Native browser proof exceeded bounded child-process deadline")
        .unwrap();
    assert!(
        proof.status.success(),
        "Native UI proof failed: stdout={} stderr={}",
        String::from_utf8_lossy(&proof.stdout),
        String::from_utf8_lossy(&proof.stderr)
    );
    assert_eq!(
        p.calls.load(Ordering::SeqCst),
        3,
        "Browser navigation/alias decisions make no additional model calls"
    );
    let topic_audits: Vec<String> = sqlx::query_scalar(
        "SELECT disposition FROM mutation_audit WHERE brain_id=$1 AND action='organization.topic' ORDER BY disposition",
    )
    .bind(brain)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        topic_audits,
        vec!["exclude", "rename", "split"],
        "The actual browser must commit exactly one audited source rename, split and exclusion"
    );
    let receipt = json!({"fixture":"actual-worker-unmocked-api-synthetic-loopback-model","source_count":3,"extraction_calls":3,"extra_mapping_calls":0,"embedding_http_calls":0,"source_group_contributors":source_rows.len(),"source_topic_audits":topic_audits,"synthetic_retained_vector_and_recipe_quality":true,"quality_measured":false,"paid_model_calls":0,"browser_exit_success":true});
    std::fs::write(
        cache.join("organization-native-browser-receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    std::fs::remove_file(&fixture).unwrap();
    server.abort();
    provider_server.abort();
    h.finish().await;
}
