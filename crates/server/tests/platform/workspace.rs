use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn workspace_catalogue_private_paths_and_concurrent_immutable_task_scopes() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (writer_id, writer) = h.fixture_member().await;
    let (reader_id, reader) = h.fixture_member().await;
    let brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Workspace proof"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let base = format!("/api/brains/{brain}");
    let workspace = format!("{base}/workspace");
    for (id, role) in [(writer_id, "writer"), (reader_id, "reader")] {
        assert_eq!(
            h.call(
                "PUT",
                &format!("{base}/grants/{id}"),
                Some(&owner),
                json!({"role":role})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let area = h
        .call(
            "POST",
            &format!("{base}/evidence/groups"),
            Some(&owner),
            json!({"kind":"area","name":"Vault"}),
        )
        .await
        .1["id"]
        .clone();
    let environment = h
        .call(
            "POST",
            &format!("{base}/evidence/groups"),
            Some(&owner),
            json!({"kind":"environment","name":"Production"}),
        )
        .await
        .1["id"]
        .clone();
    let (device, token) = h.pair_device(&writer, "Workspace fixture").await;
    let (_, other_token) = h.pair_device(&writer, "Other checkout").await;
    let mut refresh = json!({"workspace_root":"/fixture/customer","complete":true,"notes":[],"checkouts":[
        {"local_path":"/fixture/customer/infra","origin":"https://member:discard@example.test/Team/infra.git","branch":"main","head":null,"dirty":true,"status":"available"},
        {"local_path":"/fixture/customer/fork","origin":"git@example.test:Team/fork.git","branch":null,"head":null,"dirty":null,"status":"available"}
    ]});
    assert_eq!(
        h.call(
            "POST",
            &format!("{workspace}/checkouts"),
            Some(&writer),
            refresh.clone()
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let result = h
        .bearer(
            "POST",
            &format!("{workspace}/checkouts"),
            &token,
            refresh.clone(),
        )
        .await;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    let registration = result.1["workspace"]["id"].clone();
    let catalogue = h.bearer("GET", &workspace, &token, Value::Null).await.1;
    assert_eq!(catalogue["repositories"].as_array().unwrap().len(), 2);
    assert_eq!(catalogue["checkouts"].as_array().unwrap().len(), 2);
    assert!(!catalogue.to_string().contains("discard"));
    let repo = catalogue["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["canonical_origin"] == "example.test/Team/infra")
        .unwrap()["id"]
        .clone();
    let fork = catalogue["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["canonical_origin"] == "example.test/Team/fork")
        .unwrap()["id"]
        .clone();
    assert_ne!(repo, fork);
    let private = h.call("GET", &workspace, Some(&owner), Value::Null).await.1;
    assert_eq!(private["repositories"].as_array().unwrap().len(), 2);
    assert!(private["workspaces"].as_array().unwrap().is_empty());
    assert!(private["checkouts"].as_array().unwrap().is_empty());
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{workspace}?workspace_id={}",
                registration.as_str().unwrap()
            ),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    refresh["checkouts"][0]["origin"] = json!("git@example.test:Team/infra.git");
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{workspace}/checkouts"),
            &other_token,
            refresh.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("GET", &workspace, &token, Value::Null).await.1["repositories"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    refresh["checkouts"] = json!([refresh["checkouts"][0].clone()]);
    refresh["complete"] = json!(false);
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{workspace}/checkouts"),
            &token,
            refresh.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    let own_url = format!(
        "{workspace}?workspace_id={}",
        registration.as_str().unwrap()
    );
    assert!(
        h.bearer("GET", &own_url, &token, Value::Null).await.1["checkouts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["present"] == true)
    );
    refresh["complete"] = json!(true);
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{workspace}/checkouts"),
            &token,
            refresh.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("GET", &own_url, &token, Value::Null).await.1["checkouts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["present"] == false)
            .count(),
        1
    );
    let alias_url = format!(
        "{workspace}/repositories/{}/origins",
        repo.as_str().unwrap()
    );
    assert_eq!(
        h.bearer(
            "POST",
            &alias_url,
            &token,
            json!({"origin":"ssh://git@example.test/Moved/infra.git"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "POST",
            &alias_url,
            Some(&owner),
            json!({"origin":"ssh://git@example.test/Moved/infra.git"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "POST",
            &alias_url,
            Some(&owner),
            json!({"origin":"https://example.test/Team/fork.git"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    refresh["checkouts"][0]["origin"] = json!("example.test/Moved/infra");
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{workspace}/checkouts"),
            &token,
            refresh.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("GET", &own_url, &token, Value::Null).await.1["checkouts"][0]["repository_id"],
        repo
    );
    let selection = json!({"repository_ids":[repo],"area_ids":[area],"environment_id":environment});
    let create =
        json!({"label":"Parent investigation","workspace_id":registration,"selection":selection});
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{workspace}/tasks"),
            &other_token,
            create.clone()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let result = h
        .bearer("POST", &format!("{workspace}/tasks"), &token, create)
        .await;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(result.1["handoff"]["fresh_context_required"], true);
    assert_eq!(result.1["handoff"]["retrieval_available"], true);
    let parent = result.1["task"]["id"].as_str().unwrap().to_owned();
    let parent_scope = result.1["task"]["scope"]["id"].clone();
    let parent_url = format!("{workspace}/tasks/{parent}");
    let first = h
        .bearer(
            "POST",
            &format!("{parent_url}/operations"),
            &token,
            json!({"kind":"context"}),
        )
        .await;
    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(first.1["device_id"], device.to_string());
    assert_eq!(first.1["scope"]["id"], parent_scope);
    let operation = first.1["id"].as_str().unwrap().to_owned();
    let mut children = vec![];
    for label in ["Agent A", "Agent B"] {
        let result = h
            .bearer(
                "POST",
                &format!("{workspace}/tasks"),
                &token,
                json!({"label":label,"parent_task_id":parent}),
            )
            .await;
        assert_eq!(result.0, StatusCode::OK);
        assert_eq!(result.1["task"]["scope"]["selection"], selection);
        children.push(result.1["task"].clone());
    }
    let path_a = format!(
        "{workspace}/tasks/{}/scope",
        children[0]["id"].as_str().unwrap()
    );
    let path_b = format!(
        "{workspace}/tasks/{}/scope",
        children[1]["id"].as_str().unwrap()
    );
    let empty = json!({"repository_ids":[],"area_ids":[],"environment_id":null});
    let only_fork = json!({"repository_ids":[fork],"area_ids":[],"environment_id":null});
    let (a, b) = tokio::join!(
        h.bearer(
            "PUT",
            &path_a,
            &token,
            json!({"base_scope":children[0]["scope"]["id"],"selection":empty})
        ),
        h.bearer(
            "PUT",
            &path_b,
            &token,
            json!({"base_scope":children[1]["scope"]["id"],"selection":only_fork})
        )
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(b.0, StatusCode::OK);
    assert_ne!(a.1["task"]["scope"]["id"], b.1["task"]["scope"]["id"]);
    assert_eq!(
        h.bearer("GET", &parent_url, &token, Value::Null).await.1["task"]["scope"]["id"],
        parent_scope
    );
    assert_eq!(
        h.bearer(
            "PUT",
            &path_a,
            &token,
            json!({"base_scope":children[0]["scope"]["id"],"selection":selection})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.call("GET", &parent_url, Some(&owner), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{workspace}/operations/{operation}"),
            Some(&reader),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let foreign = h
        .call(
            "POST",
            "/api/brains",
            Some(&reader),
            json!({"name":"Foreign workspace"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{foreign}/workspace/tasks"),
            Some(&reader),
            json!({"label":"Foreign reference","selection":selection})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let permitted = h
        .call(
            "POST",
            &format!("{workspace}/tasks"),
            Some(&reader),
            json!({"label":"Reader task"}),
        )
        .await;
    assert_eq!(permitted.0, StatusCode::OK);
    let reader_task = permitted.1["task"]["id"].as_str().unwrap();
    assert_eq!(
        h.call(
            "POST",
            &format!("{workspace}/tasks/{reader_task}/operations"),
            Some(&reader),
            json!({"kind":"write"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{workspace}/tasks/{reader_task}/operations"),
            Some(&reader),
            json!({"kind":"context"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{base}/evidence/groups/{}", area.as_str().unwrap()),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{parent_url}/operations"),
            &token,
            json!({"kind":"context"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let prior = h
        .bearer(
            "GET",
            &format!("{workspace}/operations/{operation}"),
            &token,
            Value::Null,
        )
        .await
        .1;
    assert_eq!(prior["scope"]["id"], parent_scope);
    assert_eq!(prior["scope"]["selection"], selection);
    assert_eq!(prior["scope_valid"], false);
    let change = h
        .bearer(
            "PUT",
            &format!("{parent_url}/scope"),
            &token,
            json!({"base_scope":parent_scope,"selection":empty}),
        )
        .await;
    assert_eq!(change.0, StatusCode::OK);
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{parent_url}/operations"),
            &token,
            json!({"kind":"context"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("POST", &format!("{parent_url}/close"), &token, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{parent_url}/operations"),
            &token,
            json!({"kind":"context"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let child_path = format!(
        "{workspace}/tasks/{}/operations",
        children[0]["id"].as_str().unwrap()
    );
    assert_eq!(
        h.bearer("POST", &child_path, &token, json!({"kind":"context"}))
            .await
            .0,
        StatusCode::OK
    );
    let mut direct = db::actor_tx(&h.state.pool, reader_id)
        .await
        .ok()
        .expect("reader transaction");
    let leaked: i64 = sqlx::query_scalar("SELECT count(*) FROM checkout_registrations")
        .fetch_one(&mut *direct)
        .await
        .unwrap();
    assert_eq!(leaked, 0);
    let other_tasks: i64 =
        sqlx::query_scalar("SELECT count(*) FROM workspace_tasks WHERE account_id=$1")
            .bind(writer_id)
            .fetch_one(&mut *direct)
            .await
            .unwrap();
    assert_eq!(other_tasks, 0);
    direct.rollback().await.unwrap();
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{device}"),
            Some(&writer),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.bearer("GET", &workspace, &token, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.bearer("POST", &child_path, &token, json!({"kind":"context"}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{base}/grants/{writer_id}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call("GET", &parent_url, Some(&writer), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call("PATCH", &base, Some(&owner), json!({"archived":true}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{workspace}/tasks"),
            Some(&owner),
            json!({"label":"Archived task"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    h.finish().await;
}
