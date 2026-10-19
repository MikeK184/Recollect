//! Request-surface behaviour: address-keyed throttles, pairing re-key
//! confirmation and idempotent replay after the saved command's inputs changed.
use super::*;
use axum::extract::ConnectInfo;
use std::net::SocketAddr;

impl Harness {
    async fn at_address(
        &self,
        address: &str,
        method: &str,
        path: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(
            format!("{address}:4000").parse::<SocketAddr>().unwrap(),
        ));
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn login_throttle_is_per_address_so_one_client_cannot_lock_out_the_owner() {
    let h = Harness::new().await;
    let wrong = json!({"username":h.state.config.owner_username,"password":"not the password"});
    for _ in 0..10 {
        assert_eq!(
            h.at_address("203.0.113.9", "POST", "/api/auth/login", wrong.clone())
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        h.at_address("203.0.113.9", "POST", "/api/auth/login", wrong.clone())
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    // The attacker's failures do not throttle the owner at another address.
    let right =
        json!({"username":h.state.config.owner_username,"password":h.state.config.owner_password});
    assert_eq!(
        h.at_address("198.51.100.4", "POST", "/api/auth/login", right.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.at_address("203.0.113.9", "POST", "/api/auth/login", right)
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn anonymous_pairing_start_is_throttled_per_address() {
    let h = Harness::new().await;
    for _ in 0..30 {
        assert_eq!(
            h.at_address(
                "203.0.113.9",
                "POST",
                "/api/devices/pairings",
                json!({"name":"Flood"})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (status, body) = h
        .at_address(
            "203.0.113.9",
            "POST",
            "/api/devices/pairings",
            json!({"name":"Flood"}),
        )
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], "throttled");
    assert_eq!(
        h.at_address(
            "198.51.100.4",
            "POST",
            "/api/devices/pairings",
            json!({"name":"Calm"})
        )
        .await
        .0,
        StatusCode::OK
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn pairing_a_matching_name_names_the_device_and_needs_confirmation_to_replace_it() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (device, old_token) = h.pair_device(&owner, "Laptop").await;
    let (_, start, _) = h
        .call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"laptop"}),
        )
        .await;
    let code = start["user_code"].as_str().unwrap();
    let (_, other, _) = h
        .call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"Phone"}),
        )
        .await;
    let view = |code: String| {
        let h = &h;
        let owner = &owner;
        async move {
            h.call(
                "GET",
                &format!("/api/devices/pairings/{code}"),
                Some(owner),
                Value::Null,
            )
            .await
            .1
        }
    };
    assert_eq!(view(code.to_owned()).await["reuses_device"], "Laptop");
    assert!(view(other["user_code"].as_str().unwrap().to_owned()).await["reuses_device"].is_null());

    let approve = format!("/api/devices/pairings/{code}/approve");
    let (status, body, _) = h
        .call("POST", &approve, Some(&owner), json!({"approve":true}))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "pairing_reuse_unconfirmed");
    let token: Uuid = sqlx::query_scalar("SELECT token FROM devices WHERE id=$1")
        .bind(device)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        token.to_string(),
        old_token,
        "unconfirmed approval keeps the token"
    );
    // Declining never needs confirmation.
    assert_eq!(
        h.call(
            "POST",
            &format!(
                "/api/devices/pairings/{}/approve",
                other["user_code"].as_str().unwrap()
            ),
            Some(&owner),
            json!({"approve":false})
        )
        .await
        .0,
        StatusCode::OK
    );

    let (status, body, _) = h
        .call(
            "POST",
            &approve,
            Some(&owner),
            json!({"approve":true,"confirm":true}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["reuses_device"], "Laptop");
    let token: Uuid = sqlx::query_scalar("SELECT token FROM devices WHERE id=$1")
        .bind(device)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_ne!(token.to_string(), old_token);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn replayed_manifest_create_returns_the_saved_result_after_its_snapshot_expired() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Replay"}),
        )
        .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain_id}");
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let (repo, snapshot) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,'example.test/team/replay',$3)")
        .bind(repo).bind(brain_id).bind(owner_id).execute(&h.admin).await.unwrap();
    sqlx::query("INSERT INTO repository_snapshots(id,brain_id,repository_id,revision,adapter,adapter_build,extractor_version,settings,coverage,file_count,fact_count,retained_file_count,created_by) VALUES($1,$2,$3,$4,'fixture','fixture','fixture','{}','{}',1,0,0,$5)")
        .bind(snapshot).bind(brain_id).bind(repo).bind("a".repeat(40)).bind(owner_id).execute(&h.admin).await.unwrap();
    let (_, environment, _) = h
        .call(
            "POST",
            &format!("{base}/evidence/groups"),
            Some(&owner),
            json!({"kind":"environment","name":"Production"}),
        )
        .await;
    let input = json!({"name":"Selection","environment_id":environment["id"],"kind":"committed","entries":[{"repository_id":repo,"revision":"a".repeat(40),"snapshot_id":snapshot,"config_paths":[]}],"notes":"","base_revision":null,"operation_id":null});
    let path = format!("{base}/revision-manifests");
    let (status, saved, _) = h
        .keyed(
            "POST",
            &path,
            Some(&owner),
            input.clone(),
            Some("replay-manifest-1"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    sqlx::query("UPDATE repository_snapshots SET privacy_state='expired' WHERE id=$1")
        .bind(snapshot)
        .execute(&h.admin)
        .await
        .unwrap();
    // The selected snapshot is no longer active, yet the retry gets the answer
    // it was already given; a new command with a new key is refused.
    let (status, replayed, _) = h
        .keyed(
            "POST",
            &path,
            Some(&owner),
            input.clone(),
            Some("replay-manifest-1"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{replayed}");
    assert_eq!(replayed["id"], saved["id"]);
    assert_eq!(
        h.keyed(
            "POST",
            &path,
            Some(&owner),
            input,
            Some("replay-manifest-2")
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    h.finish().await;
}
