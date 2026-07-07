use super::*;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_scoped_history_and_expected_context_snapshot() {
    let (h, _owner, writer, _, base, version) = setup().await;
    let (_, token) = h.pair_device(&writer, "Scoped memory bridge").await;
    let (_, other_token) = h.pair_device(&writer, "Other bridge").await;
    let mut selections = Vec::new();
    for name in ["Current task", "Other task"] {
        let area = ok(
            &h,
            "POST",
            &format!("{base}/evidence/groups"),
            &writer,
            json!({"name":name,"kind":"area"}),
        )
        .await;
        selections.push(json!({"repository_ids":[],"area_ids":[area["id"]],"environment_id":null}));
    }
    let mut input = proposal(&version, "Scoped observation", "OTHER_SCOPE_VALUE");
    input["content"]["selection"] = selections[1].clone();
    let old = ok(
        &h,
        "POST",
        &format!("{base}/claims"),
        &writer,
        input.clone(),
    )
    .await;
    let accepted = reviewed(&h, &base, &writer, &old, action(&old, "accept")).await;
    let mut correction = action(&accepted, "correct");
    input["content"]["value"] = json!("CURRENT_SCOPE_VALUE");
    input["content"]["selection"] = selections[0].clone();
    correction["content"] = input["content"].clone();
    correction["reason"] = json!("OTHER_SCOPE_REASON: reviewed transition between applicability.");
    let current = reviewed(&h, &base, &writer, &accepted, correction).await;
    let task_path = format!("{base}/workspace/tasks");
    let (status, started) = h.bearer("POST", &task_path, &token,
        json!({"label":"Scoped MCP proof","selection":selections[0],"parent_task_id":null,"workspace_id":null})).await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let task = format!("{task_path}/{}", started["task"]["id"].as_str().unwrap());
    let operations = format!("{task}/operations");
    let (status, bound) = h
        .bearer(
            "POST",
            &operations,
            &token,
            json!({"kind":"retrieval","expected_scope":started["task"]["scope"]["id"]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{bound}");
    let operation = bound["id"].as_str().unwrap();
    let claim = url(&base, &current);
    let (status, detail) = h
        .bearer(
            "GET",
            &format!("{claim}?operation_id={operation}"),
            &token,
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(
        detail["selected"]["revision"]["content"]["value"],
        "CURRENT_SCOPE_VALUE"
    );
    assert_eq!(detail["history"].as_array().unwrap().len(), 1);
    assert_eq!(detail["total"], 1);
    assert!(!detail.to_string().contains("OTHER_SCOPE_VALUE"));
    let (status, history) = h
        .bearer(
            "GET",
            &format!("{claim}/review?operation_id={operation}"),
            &token,
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_eq!(history["total"], 0);
    assert_eq!(history["decisions"], json!([]));
    assert_eq!(history["rules"], json!([]));
    assert!(!history.to_string().contains("OTHER_SCOPE"));
    let browser = ok(&h, "GET", &format!("{claim}/review"), &writer, Value::Null).await;
    assert_eq!(
        browser["total"], 2,
        "browser retains its authorized review surface"
    );
    assert!(browser.to_string().contains("OTHER_SCOPE_REASON"));
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{claim}?operation_id={operation}"),
            &other_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let old_history = h
        .bearer(
            "GET",
            &format!(
                "{claim}?operation_id={operation}&knowledge_at={}",
                old["recorded_at"].as_str().unwrap()
            ),
            &token,
            Value::Null,
        )
        .await;
    assert_eq!(old_history.0, StatusCode::NOT_FOUND);
    let (status, changed) = h
        .bearer(
            "PUT",
            &format!("{task}/scope"),
            &token,
            json!({"base_scope":started["task"]["scope"]["id"],"selection":selections[1]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let stale = h
        .bearer(
            "POST",
            &operations,
            &token,
            json!({"kind":"retrieval","expected_scope":started["task"]["scope"]["id"]}),
        )
        .await;
    assert_eq!(stale.0, StatusCode::CONFLICT, "{stale:?}");
    let (status, fresh) = h
        .bearer(
            "POST",
            &operations,
            &token,
            json!({"kind":"retrieval","expected_scope":changed["task"]["scope"]["id"]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{fresh}");
    assert_eq!(fresh["scope"]["selection"], selections[1]);
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{claim}?operation_id={}", fresh["id"].as_str().unwrap()),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Old operations keep their own scope after the default moves.
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{claim}?operation_id={operation}"),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, page) = h
        .bearer(
            "GET",
            &format!(
                "{base}/claims?operation_id={}",
                fresh["id"].as_str().unwrap()
            ),
            &token,
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["items"], json!([]));
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{claim}/review"),
            &token,
            action(&current, "reject")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, implicit) = h
        .bearer("POST", &operations, &token, json!({"kind":"write"}))
        .await;
    assert_eq!(
        implicit["scope"]["id"], changed["task"]["scope"]["id"],
        "legacy callers can omit expected_scope"
    );
    assert_eq!(
        h.bearer(
            "GET",
            &format!("{claim}?operation_id={}", implicit["id"].as_str().unwrap()),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    h.finish().await;
}
