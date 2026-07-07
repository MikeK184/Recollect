use super::*;

fn module(name: &str, target: &str, revision: char) -> Value {
    json!({"id":name,"kind":"symbol","name":name,"file":"main.tf","line":1,"props":{"hcl_block":"module","external":true,"recollect_module_source":{"parser":"synthetic-parsed-fixture","state":"verified_literal","line_from":1,"line_to":1,"target":{"origin":target,"revision":revision.to_string().repeat(40),"directory":"."}}}})
}
pub(super) fn directory(relations: Value) -> Value {
    json!({"id":"root","kind":"module","name":".","file":".","props":{"language":"hcl","recollect_hcl_directory":{"parser":"synthetic-parsed-fixture","state":"parsed","directory":"."}},"relations":relations})
}
pub(super) async fn publish(
    h: &Harness,
    owner: &Login,
    base: &str,
    origin: &str,
    revision: char,
    facts: Value,
) -> (Value, Value, Vec<Value>) {
    let (_, token) = h.pair_device(owner, "Combined publication").await;
    let (status,response)=h.bearer("POST",&format!("{base}/workspace/checkouts"),&token,json!({"workspace_root":"/fixture","complete":false,"notes":[],"checkouts":[{"local_path":format!("/fixture/{revision}"),"origin":format!("https://{origin}.git"),"head":null,"branch":"main","dirty":false,"status":"available"}]})).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    let workspace = h
        .bearer("GET", &format!("{base}/workspace"), &token, Value::Null)
        .await
        .1;
    let repository = workspace["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["canonical_origin"] == origin)
        .unwrap()["id"]
        .clone();
    let operation = publication::begin(h, base, &token, &repository).await;
    let mut input = publication::input(&repository, &operation, &revision.to_string().repeat(40));
    input["captured_at"] = json!(chrono::Utc::now());
    input["origin"] = json!(origin);
    let files: std::collections::BTreeSet<String> = facts
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| {
            let path = f["file"].as_str()?;
            Some(if f["kind"] == "module" {
                if path == "." {
                    "main.tf".into()
                } else {
                    format!("{path}/main.tf")
                }
            } else {
                path.into()
            })
        })
        .collect();
    input["files"] = json!(files.into_iter().map(|path| json!({"path":path,"object_id":"1".repeat(40),"mode":"100644","size":100,"status":"materialized","extraction":"facts_emitted","content":null})).collect::<Vec<_>>());
    input["settings"] =
        json!({"retained_files":[],"module_source_parser":"synthetic-parsed-fixture"});
    input["receipt"]["fact_count"] = json!(facts.as_array().unwrap().len());
    input["facts"] = facts;
    let (status, response) = h
        .bearer(
            "POST",
            &format!(
                "{base}/repositories/{}/snapshots",
                repository.as_str().unwrap()
            ),
            &token,
            input,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    let snapshot = response["snapshot"]["id"].clone();
    for _ in 0..20 {
        let status = ok(
            h,
            "GET",
            &format!(
                "{base}/repository-snapshots/{}/facts",
                snapshot.as_str().unwrap()
            ),
            owner,
            Value::Null,
        )
        .await;
        if status["processing"] == "ready" {
            return (
                repository,
                snapshot,
                status["items"].as_array().unwrap().clone(),
            );
        }
        assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    }
    panic!("Synthetic publication did not materialize");
}
pub(super) async fn manifest(
    h: &Harness,
    owner: &Login,
    base: &str,
    environment: &Value,
    name: &str,
    entries: Value,
) -> Value {
    ok(h,"POST",&format!("{base}/revision-manifests"),owner,json!({"name":name,"environment_id":environment,"kind":"committed","entries":entries,"notes":"","base_revision":null})).await
}
pub(super) async fn combined_build(
    h: &Harness,
    owner: &Login,
    base: &str,
    manifest: &Value,
) -> Value {
    let g = ok(
        h,
        "POST",
        &format!("{base}/graph/rebuild"),
        owner,
        json!({"kind":"combined","manifest_revision_id":manifest}),
    )
    .await;
    for _ in 0..30 {
        assert!(worker::run_once(&h.state, "heavy").await.unwrap());
        let status = ok(h, "GET", &format!("{base}/graph"), owner, Value::Null).await;
        if let Some(current) = status["generations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["id"] == g["id"] && x["state"] != "queued" && x["state"] != "running")
        {
            assert_eq!(current["state"], "ready", "{status}");
            return current.clone();
        }
    }
    panic!("Combined generation did not publish");
}
pub(super) fn entity(facts: &[Value], name: &str) -> String {
    format!(
        "repository_fact:{}",
        facts.iter().find(|f| f["record"]["name"] == name).unwrap()["id"]
            .as_str()
            .unwrap()
    )
}

fn declares(target: &str) -> Value {
    json!({"kind":"declares","target":target,"target_id":target})
}
fn subdirectory(path: &str, relations: Value) -> Value {
    let mut fact = directory(relations);
    fact["id"] = json!(path);
    fact["name"] = json!(path);
    fact["file"] = json!(path);
    fact["props"]["recollect_hcl_directory"]["directory"] = json!(path);
    fact
}
fn pinned_directory(name: &str, origin: &str, revision: char, path: &str) -> Value {
    let mut fact = module(name, origin, revision);
    fact["props"]["recollect_module_source"]["target"]["directory"] = json!(path);
    fact
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_combined_union_limits_apply_before_loading_and_allow_narrower_inputs() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (a, sa, _) = publish(
        &h,
        &owner,
        &base,
        "example.test/a/core",
        'a',
        json!([
            directory(json!([])),
            module("module.target", "example.test/b/core", 'b')
        ]),
    )
    .await;
    let (b, sb, _) = publish(
        &h,
        &owner,
        &base,
        "example.test/b/core",
        'b',
        json!([directory(json!([]))]),
    )
    .await;
    for snapshot in [&sa, &sb] {
        let id: Uuid = snapshot.as_str().unwrap().parse().unwrap();
        sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('id','bulk-'||n,'kind','symbol','name','bulk_'||n,'file','main.tf','line',1) FROM generate_series(10,2609) n")
            .bind(brain).bind(id).execute(&h.admin).await.unwrap();
        sqlx::query("UPDATE repository_snapshots SET fact_count=(SELECT count(*) FROM repository_facts WHERE snapshot_id=$1) WHERE id=$1").bind(id).execute(&h.admin).await.unwrap();
        build(&h, &owner, &base, "repository", Some(snapshot)).await;
    }
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Union limits"}),
    )
    .await;
    let manifest=manifest(&h,&owner,&base,&env["id"],"Large exact pair",json!([{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]},{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]}])).await;
    combined_build(&h, &owner, &base, &manifest["id"]).await;
    let scope = json!({"kind":"combined","manifest_revision_id":manifest["id"],"selection":{"repository_ids":[a,b],"environment_id":env["id"]}});
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":scope})
        )
        .await
        .1["code"],
        "graph_scope_too_large"
    );
    let mut narrow = scope.clone();
    narrow["selection"]["repository_ids"] = json!([a]);
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":narrow})
        )
        .await["total_nodes"],
        2602
    );
    // The owned fault fixture adds descriptor padding, leaving physical graph
    // identities unchanged. Each input fits; their union exceeds the byte bound.
    sqlx::query("UPDATE graph_generations SET descriptor=descriptor||jsonb_build_object('fixture_padding',repeat('x',33*1024*1024)) WHERE brain_id=$1 AND kind='repository' AND state='ready'").bind(brain).execute(&h.admin).await.unwrap();
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":scope})
        )
        .await
        .1["code"],
        "graph_read_input_too_large"
    );
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":narrow})
        )
        .await["total_nodes"],
        2602
    );
    sqlx::query("UPDATE repository_snapshots SET fact_count=100001 WHERE id=$1")
        .bind(sa.as_str().unwrap().parse::<Uuid>().unwrap())
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/rebuild"),
            Some(&owner),
            json!({"kind":"combined","manifest_revision_id":manifest["id"]})
        )
        .await
        .1["code"],
        "graph_input_too_large"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_combined_discovery_rotates_when_only_one_queue_slot_is_available() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (a, sa, fa) = publish(
        &h,
        &owner,
        &base,
        "example.test/a/core",
        'a',
        json!([directory(json!([]))]),
    )
    .await;
    let (b, sb, _) = publish(
        &h,
        &owner,
        &base,
        "example.test/b/core",
        'b',
        json!([directory(json!([]))]),
    )
    .await;
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Discovery fairness"}),
    )
    .await;
    manifest(&h,&owner,&base,&env["id"],"Fair exact pair",json!([{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]},{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]}])).await;
    let mut claim = proposal(&fa[0]["id"], "Fairness assertion", "present");
    claim["content"]["selection"]["repository_ids"] = json!([a]);
    claim["content"]["supports"][0]["kind"] = json!("repository_fact");
    claim["content"]["supports"][0]["line_from"] = Value::Null;
    claim["content"]["supports"][0]["line_to"] = Value::Null;
    ok(&h, "POST", &format!("{base}/claims"), &owner, claim).await;
    let actor: Uuid = sqlx::query_scalar(
        "UPDATE brains SET graph_discovery_cursor=0 WHERE id=$1 RETURNING owner_id",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    let blockers:Vec<Uuid>=sqlx::query_scalar("INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane,not_before)
      SELECT gen_random_uuid(),$1,$2,j.audit_id,$1,'brain.refresh','interactive',clock_timestamp()+interval '1 day'
      FROM (SELECT audit_id FROM jobs WHERE brain_id=$1 LIMIT 1) j CROSS JOIN generate_series(1,499-(SELECT count(*) FROM jobs WHERE brain_id=$1 AND state IN ('queued','running'))) n RETURNING id")
        .bind(brain).bind(actor).fetch_all(&h.admin).await.unwrap();
    assert!(blockers.len() >= 3);
    for (pass, blocker) in blockers.iter().take(3).enumerate() {
        assert_eq!(
            graph::maintain_brain(&h.state, brain, actor).await.ok(),
            Some(1)
        );
        if pass < 2 {
            sqlx::query("UPDATE jobs SET state='cancelled' WHERE id=$1")
                .bind(blocker)
                .execute(&h.admin)
                .await
                .unwrap();
        }
    }
    let kinds: Vec<String> =
        sqlx::query_scalar("SELECT kind FROM graph_generations WHERE brain_id=$1 ORDER BY kind")
            .bind(brain)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    assert_eq!(kinds, vec!["combined", "knowledge", "repository"]);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_combined_aliases_exact_revisions_ambiguity_and_revoked_jobs() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (a, sa, fa) = publish(
        &h,
        &owner,
        &base,
        "example.test/a/core",
        'a',
        json!([
            module("module.alias", "alias.test/b/core", 'b'),
            module("module.next", "example.test/b/core", 'c'),
            pinned_directory("module.missing", "example.test/b/core", 'b', "missing"),
            pinned_directory(
                "module.duplicate",
                "example.test/b/core",
                'b',
                "modules/duplicate"
            ),
            pinned_directory("module.good", "example.test/b/core", 'b', "modules/good")
        ]),
    )
    .await;
    let targets = json!([
        directory(json!([])),
        subdirectory("modules/good", json!([])),
        subdirectory("modules/duplicate", json!([])),
        subdirectory("modules/duplicate", json!([]))
    ]);
    let (b, sb, fb) = publish(
        &h,
        &owner,
        &base,
        "example.test/b/core",
        'b',
        targets.clone(),
    )
    .await;
    let (_, sc, fc) = publish(&h, &owner, &base, "example.test/b/core", 'c', targets).await;
    for snapshot in [&sa, &sb, &sc] {
        build(&h, &owner, &base, "repository", Some(snapshot)).await;
    }
    let prod = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Production"}),
    )
    .await;
    let dev = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Development"}),
    )
    .await;
    let entries = json!([{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]},{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]}]);
    let full = manifest(
        &h,
        &owner,
        &base,
        &prod["id"],
        "Production exact",
        entries.clone(),
    )
    .await;
    let original = combined_build(&h, &owner, &base, &full["id"]).await;
    let scope = json!({"kind":"combined","manifest_revision_id":full["id"],"selection":{"repository_ids":[a,b],"environment_id":prod["id"]}});
    let path = json!({"scope":scope,"start":entity(&fa,"module.alias"),"end":entity(&fb,"."),"direction":"outgoing","max_hops":1});
    let before = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        path.clone(),
    )
    .await;
    assert_eq!(before["status"], "no_path_within_bound");
    for code in [
        "target_origin_unregistered",
        "target_revision_mismatch",
        "target_module_unavailable",
        "ambiguous_target_module",
    ] {
        assert!(
            before["view"]["link_issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["code"] == code),
            "{code}: {before}"
        );
    }
    let good = json!({"scope":scope,"start":entity(&fa,"module.good"),"end":entity(&fb,"modules/good"),"direction":"outgoing","max_hops":1});
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            good.clone()
        )
        .await["status"],
        "path"
    );
    let stale = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"combined","manifest_revision_id":full["id"]}),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!(
            "{base}/workspace/repositories/{}/origins",
            b.as_str().unwrap()
        ),
        &owner,
        json!({"origin":"ssh://git@alias.test/b/core.git"}),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/path"),
            Some(&owner),
            path.clone()
        )
        .await
        .1["code"],
        "graph_projection_missing"
    );
    let epoch =
        ok(&h, "GET", &format!("{base}/graph"), &owner, Value::Null).await["link_epoch"].clone();
    assert_ne!(epoch, original["input_epoch"]);
    let mut recovered = Value::Null;
    for _ in 0..30 {
        let _ = graph::run_once(&h.state).await;
        let _ = worker::run_once(&h.state, "heavy").await.unwrap();
        let status = ok(&h, "GET", &format!("{base}/graph"), &owner, Value::Null).await;
        if let Some(g) =
            status["generations"].as_array().unwrap().iter().find(|g| {
                g["kind"] == "combined" && g["state"] == "ready" && g["input_epoch"] == epoch
            })
        {
            recovered = g.clone();
            break;
        }
    }
    assert!(recovered.is_object());
    let stale_state: String =
        sqlx::query_scalar("SELECT error_code FROM graph_generations WHERE id=$1")
            .bind(stale["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(stale_state, "graph_input_changed");
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path.clone()
        )
        .await["status"],
        "path"
    );
    let mut next_entries = entries.clone();
    next_entries[1]["snapshot_id"] = sc.clone();
    next_entries[1]["revision"] = json!("c".repeat(40));
    let next = manifest(
        &h,
        &owner,
        &base,
        &dev["id"],
        "Development exact",
        next_entries,
    )
    .await;
    let next_generation = combined_build(&h, &owner, &base, &next["id"]).await;
    assert_ne!(
        recovered["input_snapshot_ids"],
        next_generation["input_snapshot_ids"]
    );
    let mut next_path = path.clone();
    next_path["scope"]["manifest_revision_id"] = next["id"].clone();
    next_path["scope"]["selection"]["environment_id"] = dev["id"].clone();
    next_path["end"] = json!(entity(&fc, "."));
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            next_path.clone()
        )
        .await["status"],
        "no_path_within_bound"
    );
    next_path["start"] = json!(entity(&fa, "module.next"));
    assert_eq!(
        ok(&h, "POST", &format!("{base}/graph/path"), &owner, next_path).await["status"],
        "path"
    );
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path.clone()
        )
        .await["view"]["generation"]["id"],
        recovered["id"],
        "A different exact set cannot supersede production"
    );
    let mut incomplete = entries;
    incomplete[1]["snapshot_id"] = Value::Null;
    let missing = manifest(
        &h,
        &owner,
        &base,
        &dev["id"],
        "Missing publication",
        incomplete,
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/rebuild"),
            Some(&owner),
            json!({"kind":"combined","manifest_revision_id":missing["id"]})
        )
        .await
        .1["code"],
        "graph_input_unavailable"
    );
    let (writer_id, writer) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        json!({"role":"writer"}),
    )
    .await;
    let queued = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &writer,
        json!({"kind":"combined","manifest_revision_id":full["id"]}),
    )
    .await;
    let (_, token) = h.pair_device(&writer, "Combined reader").await;
    let task = scope::device(
        &h,
        &token,
        &format!("{base}/workspace/tasks"),
        json!({"label":"Combined exact scope","selection":scope["selection"]}),
    )
    .await;
    let operation = scope::device(
        &h,
        &token,
        &format!(
            "{base}/workspace/tasks/{}/operations",
            task["task"]["id"].as_str().unwrap()
        ),
        json!({"kind":"retrieval"}),
    )
    .await;
    let mut native = path.clone();
    native["scope"]["operation_id"] = operation["id"].clone();
    assert_eq!(
        scope::device(&h, &token, &format!("{base}/graph/path"), native.clone()).await["status"],
        "path"
    );
    let mut widened = native.clone();
    widened["scope"]["selection"]["environment_id"] = dev["id"].clone();
    assert!(
        h.bearer("POST", &format!("{base}/graph/path"), &token, widened)
            .await
            .0
            .is_client_error()
    );
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{writer_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        h.bearer("POST", &format!("{base}/graph/path"), &token, native)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    for _ in 0..30 {
        if !worker::run_once(&h.state, "heavy").await.unwrap() {
            break;
        }
    }
    let cancelled: String = sqlx::query_scalar("SELECT state FROM jobs WHERE id=$1")
        .bind(queued["job_id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(cancelled, "cancelled");
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path.clone()
        )
        .await["status"],
        "path"
    );
    ok(
        &h,
        "POST",
        &format!("{base}/jobs/{}/retry", queued["job_id"].as_str().unwrap()),
        &owner,
        Value::Null,
    )
    .await;
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    assert_eq!(
        ok(&h, "POST", &format!("{base}/graph/path"), &owner, path).await["view"]["generation"]["id"],
        queued["id"]
    );
    // A different linker label cannot satisfy current ready/discovery lookup.
    sqlx::query(
        "UPDATE graph_generations SET adapter='legacy-link-fixture' WHERE brain_id=$1 AND id=$2",
    )
    .bind(brain)
    .bind(queued["id"].as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":scope})
        )
        .await
        .1["code"],
        "graph_projection_missing"
    );
    assert_eq!(graph::run_once(&h.state).await.ok(), Some(1));
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":scope})
        )
        .await["generation"]["adapter"],
        "terraform-git-module-1"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_combined_config_paths_correction_and_erasure_preserve_eligible_controls() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let mut longer = pinned_directory("module.long", "example.test/b/core", 'b', "modules/core");
    longer["file"] = json!("alternate.tf");
    let (a,sa,fa)=publish(&h,&owner,&base,"example.test/a/core",'a',json!([
        {"id":"entry","kind":"symbol","name":"entry","file":"main.tf","line":1,"relations":[declares("module.short"),declares("detour")]},
        pinned_directory("module.short","example.test/b/core",'b',"modules/core"),
        {"id":"detour","kind":"symbol","name":"detour","file":"alternate.tf","line":1,"relations":[declares("module.long")]},longer,
        {"id":"other","kind":"symbol","name":"module.outside","file":"private.tf","line":1,"props":{"hcl_block":"module","external":true}}
    ])).await;
    let (b,sb,fb)=publish(&h,&owner,&base,"example.test/b/core",'b',json!([
        subdirectory("modules/core",json!([declares("destination")])),
        {"id":"destination","kind":"symbol","name":"destination","file":"modules/core/main.tf","line":1},
        {"id":"prefix","kind":"symbol","name":"prefix decoy","file":"modules/core-extra/main.tf","line":1}
    ])).await;
    build(&h, &owner, &base, "repository", Some(&sa)).await;
    let gb = build(&h, &owner, &base, "repository", Some(&sb)).await;
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Scoped configuration"}),
    )
    .await;
    let entries = json!([{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]},{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]}]);
    let full = manifest(
        &h,
        &owner,
        &base,
        &env["id"],
        "Whole snapshots",
        entries.clone(),
    )
    .await;
    let first = combined_build(&h, &owner, &base, &full["id"]).await;
    let scope = json!({"kind":"combined","manifest_revision_id":full["id"],"selection":{"repository_ids":[a,b],"environment_id":env["id"]}});
    let request = json!({"scope":scope,"start":entity(&fa,"entry"),"end":entity(&fb,"destination"),"direction":"outgoing","max_hops":8});
    let baseline = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        request.clone(),
    )
    .await;
    assert_eq!(baseline["edges"].as_array().unwrap().len(), 3);
    let mut selected_entries = entries.clone();
    selected_entries[0]["config_paths"] = json!(["main.tf"]);
    selected_entries[1]["config_paths"] = json!(["modules/core"]);
    let selected = manifest(
        &h,
        &owner,
        &base,
        &env["id"],
        "Selected files",
        selected_entries.clone(),
    )
    .await;
    let mut selected_request = request.clone();
    selected_request["scope"]["manifest_revision_id"] = selected["id"].clone();
    let path = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        selected_request.clone(),
    )
    .await;
    assert_eq!(path["edges"].as_array().unwrap().len(), 3);
    assert_eq!(path["view"]["total_nodes"], 4);
    assert_eq!(
        path["view"]["link_issues_total"], 0,
        "Excluded source diagnostics stay outside the response"
    );
    assert_eq!(
        path["view"]["generation"]["id"], first["id"],
        "Config selectors qualify the shared generation"
    );
    // Exact recall shares this scope gate, including literal directory boundaries.
    for (name, facts, expected) in [
        ("destination", &fb, true),
        ("prefix decoy", &fb, false),
        ("module.outside", &fa, false),
    ] {
        let fact = entity(facts, name).split_once(':').unwrap().1.to_string();
        let recall=ok(&h,"POST",&format!("{base}/recall"),&owner,json!({"exact":{"kind":"repository_fact","id":fact},"selection":scope["selection"],"manifest_revision_id":selected["id"],"channels":["exact"]})).await;
        assert_eq!(
            recall["context"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["id"] == fact),
            expected,
            "{name}: {recall}"
        );
    }
    selected_entries[1]["config_paths"] = json!(["modules/c%"]);
    let literal = manifest(
        &h,
        &owner,
        &base,
        &env["id"],
        "Literal percent",
        selected_entries,
    )
    .await;
    let mut literal_scope = scope.clone();
    literal_scope["manifest_revision_id"] = literal["id"].clone();
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/view"),
            &owner,
            json!({"scope":literal_scope})
        )
        .await["total_nodes"],
        2
    );
    let short_id = fa
        .iter()
        .find(|f| f["record"]["name"] == "module.short")
        .unwrap()["id"]
        .clone();
    let mut claim = proposal(&short_id, "Combined short route", "rejected declaration");
    claim["content"]["selection"] = scope["selection"].clone();
    claim["content"]["supports"][0]["kind"] = json!("repository_fact");
    claim["content"]["supports"][0]["line_from"] = Value::Null;
    claim["content"]["supports"][0]["line_to"] = Value::Null;
    let claim = ok(&h, "POST", &format!("{base}/claims"), &owner, claim).await;
    ok(&h,"POST",&format!("{base}/claims/{}/review",claim["claim_id"].as_str().unwrap()),&owner,json!({"base_revision":claim["id"],"action":"reject","reason":"Reject the short cross-repository contribution."})).await;
    assert_eq!(cypher(&h,"MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation}]->() RETURN count(r)",json!({"brain":brain,"generation":first["id"]})).await,json!([[2]]));
    let longer = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        request.clone(),
    )
    .await;
    assert_eq!(
        longer["edges"].as_array().unwrap().len(),
        4,
        "The eligible alternative is selected before shortest-path choice"
    );
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            selected_request.clone()
        )
        .await["status"],
        "no_path_within_bound"
    );
    let rebuilt = combined_build(&h, &owner, &base, &full["id"]).await;
    assert_ne!(rebuilt["id"], first["id"]);
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            request.clone()
        )
        .await["edges"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let target = json!({"kind":"snapshot","id":sa});
    let preview = ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;
    assert_ne!(
        h.call("POST", &format!("{base}/graph/path"), Some(&owner), request)
            .await
            .0,
        StatusCode::OK
    );
    let status = ok(&h, "GET", &format!("{base}/graph"), &owner, Value::Null).await;
    assert_eq!(
        status["generations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == rebuilt["id"])
            .unwrap()["state"],
        "removed"
    );
    recollect_server::privacy_journal::maintain(&h.state)
        .await
        .unwrap();
    assert_eq!(cypher(&h,"MATCH ()-[r:RECOLLECT_GRAPH_EDGE {brain:$brain,generation:$generation}]->() RETURN count(r)",json!({"brain":brain,"generation":rebuilt["id"]})).await,json!([[0]]));
    let survivor=ok(&h,"POST",&format!("{base}/graph/path"),&owner,json!({"scope":{"kind":"repository","snapshot_id":sb,"selection":{"repository_ids":[b]}},"start":entity(&fb,"modules/core"),"end":entity(&fb,"destination"),"direction":"outgoing","max_hops":1})).await;
    assert_eq!(survivor["status"], "path");
    assert_eq!(survivor["view"]["generation"]["id"], gb["id"]);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_combined_exact_inputs_reuse_native_paths_and_keep_unresolved_links_visible() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (a,sa,fa)=publish(&h,&owner,&base,"example.test/a/core",'a',json!([
        directory(json!([{"kind":"declares","target":"module.bridge","target_id":"module.bridge"}])),
        module("module.bridge","example.test/b/core",'b'),
        {"id":"unknown","kind":"symbol","name":"module.unknown","file":"main.tf","line":1,"props":{"hcl_block":"module","external":true,"module_source":"registry.example.test/example/module"}}
    ])).await;
    let (b,sb,fb)=publish(&h,&owner,&base,"example.test/b/core",'b',json!([
        directory(json!([{"kind":"declares","target":"destination","target_id":"destination"}])),
        {"id":"destination","kind":"symbol","name":"destination","file":"main.tf","line":1}
    ])).await;
    let ga = build(&h, &owner, &base, "repository", Some(&sa)).await;
    let gb = build(&h, &owner, &base, "repository", Some(&sb)).await;
    let env = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Production"}),
    )
    .await;
    let entries = json!([{"repository_id":a,"snapshot_id":sa,"revision":"a".repeat(40),"config_paths":[]},{"repository_id":b,"snapshot_id":sb,"revision":"b".repeat(40),"config_paths":[]}]);
    let selected = manifest(&h, &owner, &base, &env["id"], "Pinned", entries.clone()).await;
    let generation = combined_build(&h, &owner, &base, &selected["id"]).await;
    assert_eq!(generation["node_count"], 2);
    assert_eq!(generation["edge_count"], 1);
    assert_eq!(generation["unresolved"], 1);
    let scope = json!({"kind":"combined","manifest_revision_id":selected["id"],"selection":{"repository_ids":[a,b],"area_ids":[],"environment_id":env["id"]}});
    let request = json!({"scope":scope,"start":entity(&fa,"."),"end":entity(&fb,"destination"),"direction":"outgoing","max_hops":6});
    let path = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        request.clone(),
    )
    .await;
    assert_eq!(path["status"], "path");
    assert_eq!(path["edges"].as_array().unwrap().len(), 3);
    assert_eq!(path["edges"][1]["family"], "cross_repository");
    assert_eq!(path["edges"][1]["relation"], "terraform_module");
    assert_eq!(path["view"]["link_issues_total"], 1);
    assert_eq!(
        path["view"]["link_issues"][0]["code"],
        "source_not_verified"
    );
    let inputs = path["view"]["inputs"].as_array().unwrap();
    assert!(inputs.iter().any(|g| g["id"] == ga["id"]));
    assert!(inputs.iter().any(|g| g["id"] == gb["id"]));
    // Another environment's identical snapshots reuse both base graphs and the
    // cross-link generation; the response still records its selected manifest.
    let development = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Development"}),
    )
    .await;
    let same = manifest(
        &h,
        &owner,
        &base,
        &development["id"],
        "Same exact inputs",
        entries,
    )
    .await;
    assert_eq!(graph::run_once(&h.state).await.ok(), Some(0));
    let mut other = request.clone();
    other["scope"]["selection"]["environment_id"] = development["id"].clone();
    other["scope"]["manifest_revision_id"] = same["id"].clone();
    let reused = ok(&h, "POST", &format!("{base}/graph/path"), &owner, other).await;
    assert_eq!(reused["view"]["generation"]["id"], generation["id"]);
    assert_eq!(reused["view"]["scope"]["manifest_revision_id"], same["id"]);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM graph_generations WHERE brain_id=$1 AND kind='combined'",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let mut narrowed = request.clone();
    narrowed["scope"]["selection"]["repository_ids"] = json!([a]);
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/path"),
            Some(&owner),
            narrowed
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut mismatch = request;
    mismatch["scope"]["selection"]["environment_id"] = development["id"].clone();
    assert!(
        h.call(
            "POST",
            &format!("{base}/graph/path"),
            Some(&owner),
            mismatch
        )
        .await
        .0
        .is_client_error()
    );
    assert_eq!(
        ok(
            &h,
            "GET",
            &format!("{base}/models/usage"),
            &owner,
            Value::Null
        )
        .await["total"],
        0
    );
    h.finish().await;
}
