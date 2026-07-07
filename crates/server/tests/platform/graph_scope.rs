use super::*;

pub(super) async fn device(h: &Harness, token: &str, path: &str, input: Value) -> Value {
    let (status, body) = h.bearer("POST", path, token, input).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_exact_manifests_native_operations_and_rls_keep_brain_boundaries() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, old, old_facts) = materialized(&h, &owner, &base, 'a', None).await;
    build(&h, &owner, &base, "repository", Some(&old)).await;
    let (_, new, new_facts) = materialized(&h, &owner, &base, 'b', Some(repo.clone())).await;
    build(&h, &owner, &base, "repository", Some(&new)).await;
    let environment = ok(
        &h,
        "POST",
        &format!("{base}/evidence/groups"),
        &owner,
        json!({"kind":"environment","name":"Synthetic production"}),
    )
    .await;
    let manifest=ok(&h,"POST",&format!("{base}/revision-manifests"),&owner,json!({"name":"Exact synthetic production","environment_id":environment["id"],"kind":"committed","entries":[{"repository_id":repo,"snapshot_id":old,"revision":"a".repeat(40),"config_paths":[]}],"notes":"","base_revision":null})).await;
    let selection =
        json!({"repository_ids":[repo],"area_ids":[],"environment_id":environment["id"]});
    let scope =
        json!({"kind":"repository","selection":selection,"manifest_revision_id":manifest["id"]});
    let path = ok(
        &h,
        "POST",
        &format!("{base}/graph/path"),
        &owner,
        path_input(&scope, &old_facts),
    )
    .await;
    assert_eq!(path["view"]["scope"]["snapshot_id"], old);
    assert_eq!(path["edges"].as_array().unwrap().len(), 2);
    let mut mismatch = scope.clone();
    mismatch["snapshot_id"] = new.clone();
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":mismatch})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let direct =
        json!({"kind":"repository","snapshot_id":old,"selection":{"repository_ids":[repo]}});
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path_input(&direct, &old_facts)
        )
        .await["status"],
        "path"
    );
    let latest = json!({"kind":"repository","selection":{"repository_ids":[repo]}});
    assert_eq!(
        ok(
            &h,
            "POST",
            &format!("{base}/graph/path"),
            &owner,
            path_input(&latest, &new_facts)
        )
        .await["view"]["scope"]["snapshot_id"],
        new
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
    let (_, token) = h.pair_device(&writer, "Native graph proof").await;
    let task = device(
        &h,
        &token,
        &format!("{base}/workspace/tasks"),
        json!({"label":"Native graph scope","selection":selection}),
    )
    .await;
    let task_url = format!(
        "{base}/workspace/tasks/{}",
        task["task"]["id"].as_str().unwrap()
    );
    let operation = device(
        &h,
        &token,
        &format!("{task_url}/operations"),
        json!({"kind":"retrieval"}),
    )
    .await;
    let mut native = scope.clone();
    native["operation_id"] = operation["id"].clone();
    assert_eq!(
        device(
            &h,
            &token,
            &format!("{base}/graph/path"),
            path_input(&native, &old_facts)
        )
        .await["status"],
        "path"
    );
    let missing_operation = h
        .bearer(
            "POST",
            &format!("{base}/graph/view"),
            &token,
            json!({"scope":scope}),
        )
        .await;
    assert!(missing_operation.0.is_client_error());
    let mut widened = native.clone();
    widened["selection"]["environment_id"] = Value::Null;
    assert!(
        h.bearer(
            "POST",
            &format!("{base}/graph/view"),
            &token,
            json!({"scope":widened})
        )
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
        h.bearer(
            "POST",
            &format!("{base}/graph/path"),
            &token,
            path_input(&native, &old_facts)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut foreign = recollect_server::db::actor_tx(&h.state.pool, writer_id)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM graph_generations WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *foreign)
            .await
            .unwrap(),
        0
    );
    foreign.rollback().await.unwrap();
    let mut fact_scope = recollect_server::db::actor_tx(&h.state.pool, writer_id)
        .await
        .unwrap_or_else(|e| panic!("{}", e.1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM repository_facts WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&mut *fact_scope)
            .await
            .unwrap(),
        0
    );
    fact_scope.rollback().await.unwrap();
    let other = ok(
        &h,
        "POST",
        "/api/brains",
        &owner,
        json!({"name":"Foreign synthetic graph"}),
    )
    .await;
    let foreign_base = format!("/api/brains/{}", other["id"].as_str().unwrap());
    let foreign_rebuild = h
        .call(
            "POST",
            &format!("{foreign_base}/graph/rebuild"),
            Some(&owner),
            json!({"kind":"repository","snapshot_id":old}),
        )
        .await;
    assert_eq!(foreign_rebuild.0, StatusCode::NOT_FOUND);
    assert_eq!(
        h.call("GET", &format!("{base}/graph"), Some(&writer), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    cleanup(&h, brain).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j"]
async fn graph_large_inputs_and_reads_refuse_explicitly_without_truncation() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let (repo, snapshot, _) = materialized(&h, &owner, &base, 'f', None).await;
    let id: Uuid = snapshot.as_str().unwrap().parse().unwrap();
    // Expand only this owned canonical fixture to exercise admission separately
    // from extractor/upload bounds. No external checkout is read or modified.
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('id','bulk-'||n,'kind','function','name','bulk_'||n,'file','src/lib.rs','line',1) FROM generate_series(7,5000) n")
        .bind(brain).bind(id).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET fact_count=5001 WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    let generation = build(&h, &owner, &base, "repository", Some(&snapshot)).await;
    assert_eq!(generation["node_count"], 5001);
    let selection =
        json!({"kind":"repository","snapshot_id":snapshot,"selection":{"repository_ids":[repo]}});
    let too_large = h
        .call(
            "POST",
            &format!("{base}/graph/view"),
            Some(&owner),
            json!({"scope":selection}),
        )
        .await;
    assert_eq!(too_large.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(too_large.1["code"], "graph_scope_too_large");
    sqlx::query("INSERT INTO repository_facts(id,brain_id,snapshot_id,ordinal,record) SELECT gen_random_uuid(),$1,$2,n,jsonb_build_object('id','bulk-'||n,'kind','function','name','bulk_'||n) FROM generate_series(5001,100000) n")
        .bind(brain).bind(id).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET fact_count=100001 WHERE id=$1")
        .bind(id)
        .execute(&h.admin)
        .await
        .unwrap();
    let input = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":snapshot}),
    )
    .await;
    assert!(worker::run_once(&h.state, "heavy").await.unwrap());
    let status = ok(&h, "GET", &format!("{base}/graph"), &owner, Value::Null).await;
    let failed = status["generations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == input["id"])
        .unwrap();
    assert_eq!(failed["state"], "cancelled");
    assert_eq!(failed["error_code"], "graph_input_too_large");
    assert_eq!(
        cypher(
            &h,
            "MATCH (g:RecollectGraphGeneration {brain:$brain,id:$id}) RETURN count(g)",
            json!({"brain":brain,"id":input["id"]})
        )
        .await,
        json!([[0]])
    );
    cleanup(&h, brain).await;
    h.finish().await;
}
