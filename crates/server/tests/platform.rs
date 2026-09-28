use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use recollect_server::{AppState, app, config::Config, db, health};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

struct Harness {
    router: Router,
    state: AppState,
    admin: PgPool,
    root: PgPool,
    database: String,
}
struct Login {
    cookie: String,
    csrf: String,
}

#[path = "platform/brain_admission.rs"]
mod brain_admission;
#[path = "platform/managed.rs"]
mod managed;

#[path = "platform/graph.rs"]
mod graph;
#[path = "platform/mcp.rs"]
mod mcp;
#[path = "platform/memory.rs"]
mod memory;
#[path = "platform/models.rs"]
mod models;
#[path = "platform/operations.rs"]
mod operations;
#[path = "platform/publication.rs"]
mod publication;
#[path = "platform/recovery.rs"]
mod recovery;
#[path = "platform/retention.rs"]
mod retention;
#[path = "platform/retrieval.rs"]
mod retrieval;
#[path = "platform/review.rs"]
mod review;
#[path = "platform/workspace.rs"]
mod workspace;

fn session_from(body: &Value, cookie: &str) -> Login {
    Login {
        cookie: cookie.split(';').next().unwrap().into(),
        csrf: body["csrf_token"].as_str().unwrap().into(),
    }
}

async fn oidc_authorization(h: &Harness) -> (String, String, String) {
    let response = h
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/oidc/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let mut location = response.headers()["location"].to_str().unwrap().to_owned();
    assert!(
        location.starts_with("http://127.0.0.1:5556/"),
        "Provider discovery must succeed"
    );
    let state = reqwest::Url::parse(&location)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .to_string();
    for _ in 0..10 {
        if location.starts_with(&h.state.config.public_origin) {
            return (location, cookie, state);
        }
        let url = reqwest::Url::parse(&location).unwrap();
        let response = h.state.http.get(url.clone()).send().await.unwrap();
        assert!(
            response.status().is_redirection(),
            "Synthetic provider must return authorization code through redirects"
        );
        location = url
            .join(response.headers()["location"].to_str().unwrap())
            .unwrap()
            .to_string();
    }
    panic!("Provider exceeded redirect limit")
}
async fn oidc_callback(h: &Harness, url: &str, cookie: &str) -> (bool, String) {
    let url = reqwest::Url::parse(url).unwrap();
    let uri = format!("{}?{}", url.path(), url.query().unwrap_or(""));
    let response = h
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let success = response.headers()["location"] == "/";
    let cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("recollect_session="))
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    (success, cookie)
}

#[tokio::test]
#[ignore = "Requires isolated Dex; run ./scripts/test-oidc.sh"]
async fn oidc_live_identity_membership_expiry_and_callback_replay() {
    use recollect_server::{oidc, worker};
    let h = Harness::new().await;
    assert!(h.state.config.oidc.is_some());
    let owner = h.login().await;
    let flow = oidc_authorization(&h).await;
    assert!(
        !oidc_callback(&h, &flow.0, &flow.1).await.0,
        "OIDC is not public signup"
    );
    let provision = h
        .call(
            "POST",
            "/api/team/oidc-accounts",
            Some(&owner),
            json!({"username":"organization-member","subject":"Cg0wLTM4NS0yODA4OS0wEgRtb2Nr"}),
        )
        .await;
    assert_eq!(provision.0, StatusCode::OK);
    let member: Uuid = provision.1["id"].as_str().unwrap().parse().unwrap();
    let (_, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Organization shared work"}),
        )
        .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let path = format!("/api/brains/{brain_id}");
    let mapping = h
        .call(
            "PUT",
            &format!("{path}/group-grants"),
            Some(&owner),
            json!({"group_name":"authors","role":"admin"}),
        )
        .await;
    assert_eq!(mapping.0, StatusCode::OK);
    let flow = oidc_authorization(&h).await;
    assert!(!oidc_callback(&h, &flow.0, "recollect_oidc=wrong").await.0);
    let (success, cookie) = oidc_callback(&h, &flow.0, &flow.1).await;
    assert!(
        success,
        "Known issuer/subject must authenticate through signed code exchange"
    );
    assert!(
        !oidc_callback(&h, &flow.0, &flow.1).await.0,
        "Callback state is single-use"
    );
    let mut member_session = Login {
        cookie,
        csrf: String::new(),
    };
    let me = h
        .call("GET", "/api/auth/me", Some(&member_session), Value::Null)
        .await;
    assert_eq!(me.0, StatusCode::OK);
    member_session.csrf = me.1["csrf_token"].as_str().unwrap().into();
    let (oidc_device, oidc_device_token) = h
        .pair_device(&member_session, "Organization companion")
        .await;
    assert_eq!(
        h.bearer("GET", &path, &oidc_device_token, Value::Null)
            .await
            .1["role"],
        "admin"
    );
    let ttl: i64 = sqlx::query_scalar(
        "SELECT extract(epoch FROM (expires_at-now()))::bigint FROM sessions WHERE account_id=$1",
    )
    .bind(member)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert!((1..=300).contains(&ttl));
    assert_eq!(
        h.call("GET", &path, Some(&member_session), Value::Null)
            .await
            .1["role"],
        "admin"
    );
    let unrelated = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Unrelated private work"}),
        )
        .await
        .1["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{unrelated}"),
            Some(&member_session),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let access = h
        .call("GET", &format!("{path}/access"), Some(&owner), Value::Null)
        .await
        .1;
    let mapped = access["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["account"]["id"] == member.to_string())
        .unwrap();
    assert_eq!(mapped["groups"], json!(["authors"]));
    // Direct and inherited permissions remain independent when one is removed.
    assert_eq!(
        h.call(
            "PUT",
            &format!("{path}/grants/{member}"),
            Some(&owner),
            json!({"role":"reader"})
        )
        .await
        .1["effective_role"],
        "admin"
    );
    let mapping_path = format!("{path}/group-grants/{}", mapping.1["id"].as_str().unwrap());
    assert_eq!(
        h.call("DELETE", &mapping_path, Some(&owner), Value::Null)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.call("GET", &path, Some(&member_session), Value::Null)
            .await
            .1["role"],
        "reader"
    );
    assert_eq!(
        h.call(
            "PUT",
            &format!("{path}/group-grants"),
            Some(&owner),
            json!({"group_name":"authors","role":"admin"})
        )
        .await
        .0,
        StatusCode::OK
    );
    // A signed ID token with the wrong bound nonce is rejected by the real callback.
    let wrong_nonce = oidc_authorization(&h).await;
    sqlx::query("UPDATE oidc_flows SET nonce=$2 WHERE state=$1")
        .bind(&wrong_nonce.2)
        .bind(Uuid::new_v4().to_string())
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(!oidc_callback(&h, &wrong_nonce.0, &wrong_nonce.1).await.0);
    let expired = oidc_authorization(&h).await;
    sqlx::query("UPDATE oidc_flows SET expires_at=now()-interval '1 second' WHERE state=$1")
        .bind(&expired.2)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(!oidc_callback(&h, &expired.0, &expired.1).await.0);
    // Missing, malformed and Entra-overage claims are never interpreted as complete membership.
    for data in [
        json!({}),
        json!({"groups":["authors",42]}),
        json!({"groups":["authors"],"hasgroups":true}),
        json!({"groups":["authors"],"_claim_names":{"groups":"src"}}),
    ] {
        let claims: oidc::ExtraClaims = serde_json::from_value(data).unwrap();
        assert!(oidc::complete_groups(&claims).is_empty());
    }
    assert_eq!(
        h.call(
            "PATCH",
            &path,
            Some(&member_session),
            json!({"description":"Queued by organization identity"})
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE jobs SET state='cancelled' WHERE actor_id<>$1")
        .bind(member)
        .execute(&h.admin)
        .await
        .unwrap();
    let job = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    // Advance persisted deadlines to exercise the five-minute boundary without sleeping.
    sqlx::query("UPDATE accounts SET membership_until=now()-interval '1 second' WHERE id=$1")
        .bind(member)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.bearer("GET", "/api/auth/me", &oidc_device_token, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call("GET", &path, Some(&member_session), Value::Null)
            .await
            .1["role"],
        "reader"
    );
    assert_eq!(
        worker::execute(&h.state, &job).await,
        Err(worker::Failure::Revoked)
    );
    h.call(
        "DELETE",
        &format!("{path}/grants/{member}"),
        Some(&owner),
        Value::Null,
    )
    .await;
    assert_eq!(
        h.call("GET", &path, Some(&member_session), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE sessions SET expires_at=now()-interval '1 second' WHERE account_id=$1")
        .bind(member)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call("GET", "/api/auth/me", Some(&member_session), Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    // Browser reauthentication refreshes the device's identity snapshot; it never restores revocation.
    let fresh = oidc_authorization(&h).await;
    assert!(oidc_callback(&h, &fresh.0, &fresh.1).await.0);
    assert_eq!(
        h.bearer("GET", &path, &oidc_device_token, Value::Null)
            .await
            .1["role"],
        "admin"
    );
    assert_eq!(
        h.bearer(
            "POST",
            "/api/devices/revoke-self",
            &oidc_device_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let fresh = oidc_authorization(&h).await;
    assert!(oidc_callback(&h, &fresh.0, &fresh.1).await.0);
    assert_eq!(
        h.bearer("GET", "/api/auth/me", &oidc_device_token, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT revoked_at IS NOT NULL FROM devices WHERE id=$1")
            .bind(oidc_device)
            .fetch_one(&h.admin)
            .await
            .unwrap()
    );
    // Unreachable organization sign-in does not break the separate local-owner path.
    let mut offline = (*h.state.config).clone();
    offline.oidc.as_mut().unwrap().issuer = "http://127.0.0.1:1".into();
    let offline = app(AppState::new(h.state.pool.clone(), offline).unwrap());
    let unavailable = offline
        .oneshot(
            Request::builder()
                .uri("/api/auth/oidc/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        unavailable.headers()["location"],
        "/?auth_error=organization_unavailable"
    );
    assert_eq!(
        h.call("GET", "/api/team", Some(&owner), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn invited_team_recovery_effective_grants_and_ownership() {
    use recollect_server::worker;
    let h = Harness::new().await;
    let owner = h.login().await;
    let owner_id: Uuid = h
        .call("GET", "/api/auth/me", Some(&owner), Value::Null)
        .await
        .1["user"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let (_, invitation, _) = h
        .call(
            "POST",
            "/api/team/invitations",
            Some(&owner),
            json!({"username":"teammate"}),
        )
        .await;
    assert!(invitation["token"].is_string());
    assert_eq!(
        h.call(
            "POST",
            "/api/team/invitations",
            Some(&owner),
            json!({"username":"teammate"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let password = Uuid::new_v4().to_string();
    let input = json!({"token":invitation["token"],"password":password});
    let (a, b) = tokio::join!(
        h.call("POST", "/api/auth/enroll", None, input.clone()),
        h.call("POST", "/api/auth/enroll", None, input)
    );
    assert!(
        (a.0 == StatusCode::OK && b.0 == StatusCode::GONE)
            || (b.0 == StatusCode::OK && a.0 == StatusCode::GONE)
    );
    let (_, body, cookie) = if a.0 == StatusCode::OK { a } else { b };
    let teammate = session_from(&body, &cookie);
    let member: Uuid = body["user"]["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"teammate","password":password})
        )
        .await
        .0,
        StatusCode::OK
    );
    // Missing credentials are a visible dependency failure, without activating another account.
    let saved = std::fs::read(&h.state.config.credential_file).unwrap();
    std::fs::write(&h.state.config.credential_file, b"invalid-json").unwrap();
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"teammate","password":password})
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    std::fs::write(&h.state.config.credential_file, saved).unwrap();
    assert_eq!(
        h.call("GET", "/api/team", Some(&teammate), Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/team/invitations",
            Some(&teammate),
            json!({"username":"intruder"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Team work"}),
        )
        .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let path = format!("/api/brains/{brain_id}");
    assert_eq!(
        h.call("GET", &path, Some(&teammate), Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{path}/grants"),
            Some(&owner),
            json!({"username":"teammate","role":"writer"})
        )
        .await
        .1["effective_role"],
        "writer"
    );
    assert_eq!(
        h.call("GET", &path, Some(&teammate), Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{path}/access"),
            Some(&teammate),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "PUT",
            &format!("{path}/grants/{member}"),
            Some(&owner),
            json!({"role":"admin"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "POST",
            &format!("{path}/owner"),
            Some(&teammate),
            json!({"account_id":member})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Removing a direct grant cannot remove independent ownership.
    h.call(
        "PUT",
        &format!("{path}/grants/{owner_id}"),
        Some(&owner),
        json!({"role":"reader"}),
    )
    .await;
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{path}/grants/{owner_id}"),
            Some(&owner),
            Value::Null
        )
        .await
        .1["effective_role"],
        "admin"
    );
    let transfer = h
        .call(
            "POST",
            &format!("{path}/owner"),
            Some(&owner),
            json!({"account_id":member}),
        )
        .await;
    assert_eq!(transfer.0, StatusCode::OK);
    assert!(transfer.1["effective_role"].is_null());
    assert_eq!(
        h.call("GET", &path, Some(&owner), Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "DELETE",
            &format!("{path}/grants/{member}"),
            Some(&teammate),
            Value::Null
        )
        .await
        .1["effective_role"],
        "admin"
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("{path}/access"),
            Some(&teammate),
            Value::Null
        )
        .await
        .1["members"][0]["owner"],
        true
    );
    // A member's queued mutation cannot publish after the installation owner disables it.
    assert_eq!(
        h.call(
            "PATCH",
            &path,
            Some(&teammate),
            json!({"name":"Member contribution"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let job = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    let pending = if job.actor_id == member {
        job
    } else {
        worker::fail(&h.state.pool, &job, worker::Failure::Revoked)
            .await
            .unwrap();
        worker::claim(&h.state.pool, "interactive")
            .await
            .unwrap()
            .unwrap()
    };
    assert_eq!(pending.actor_id, member);
    {
        let active = db::actor_tx(&h.state.pool, member)
            .await
            .unwrap_or_else(|_| panic!("Enabled member should authorize the transaction"));
        let disable_path = format!("/api/team/accounts/{member}");
        let disable = h.call(
            "PATCH",
            &disable_path,
            Some(&owner),
            json!({"enabled":false}),
        );
        tokio::pin!(disable);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), &mut disable)
                .await
                .is_err(),
            "Disable must wait for the already-authorized transaction"
        );
        active.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(2), &mut disable)
                .await
                .unwrap()
                .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        worker::execute(&h.state, &pending).await,
        Err(worker::Failure::Revoked)
    );
    assert_eq!(
        h.call("GET", &path, Some(&teammate), Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"teammate","password":password})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/team/accounts/{owner_id}"),
            Some(&owner),
            json!({"enabled":false})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    h.call(
        "PATCH",
        &format!("/api/team/accounts/{member}"),
        Some(&owner),
        json!({"enabled":true}),
    )
    .await;
    assert_eq!(
        h.call("GET", "/api/auth/me", Some(&teammate), Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let reset = h
        .call(
            "POST",
            &format!("/api/team/accounts/{member}/reset"),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(reset.0, StatusCode::OK);
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"teammate","password":password})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let new_password = Uuid::new_v4().to_string();
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/enroll",
            None,
            json!({"token":reset.1["token"],"password":new_password})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"teammate","password":password})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"teammate","password":new_password})
        )
        .await
        .0,
        StatusCode::OK
    );
    let stale = h
        .call(
            "POST",
            "/api/team/invitations",
            Some(&owner),
            json!({"username":"expired"}),
        )
        .await
        .1;
    sqlx::query("UPDATE invitations SET expires_at=now()-interval '1 second' WHERE id=$1")
        .bind(stale["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/enroll",
            None,
            json!({"token":stale["token"],"password":new_password})
        )
        .await
        .0,
        StatusCode::GONE
    );
    let revoked = h
        .call(
            "POST",
            "/api/team/invitations",
            Some(&owner),
            json!({"username":"revoked"}),
        )
        .await
        .1;
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/team/invitations/{}", revoked["id"].as_str().unwrap()),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/enroll",
            None,
            json!({"token":revoked["token"],"password":new_password})
        )
        .await
        .0,
        StatusCode::GONE
    );
    let list = h
        .call("GET", "/api/team", Some(&owner), Value::Null)
        .await
        .1;
    assert!(
        !list
            .to_string()
            .contains(invitation["token"].as_str().unwrap())
    );
    assert!(!list.to_string().contains(&password));
    assert!(
        h.call("GET", "/api/team/audit", Some(&owner), Value::Null)
            .await
            .1
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["action"] == "account.reset")
    );
    h.finish().await;
}

impl Harness {
    async fn new() -> Self {
        let mut config = Config::from_env()
            .expect("Use ./scripts/test-platform.sh with repository-owned databases");
        let admin_url = std::env::var("DATABASE_ADMIN_URL").expect("DATABASE_ADMIN_URL required");
        let root = db::pool(&admin_url).await.unwrap();
        let database = format!("recollect_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {database}"))
            .execute(&root)
            .await
            .unwrap();
        eprintln!("Disposable Recollect test database: {database}");
        sqlx::query(&format!("COMMENT ON DATABASE {database} IS 'Recollect disposable integration test created by crates/server/tests/platform.rs'"))
            .execute(&root).await.unwrap();
        let mut url = reqwest::Url::parse(&admin_url).unwrap();
        url.set_path(&database);
        let admin = db::pool(url.as_str()).await.unwrap();
        db::migrate(&admin).await.unwrap();
        db::bootstrap(&admin, &config.owner_username).await.unwrap();
        let mut url = reqwest::Url::parse(&config.database_url).unwrap();
        url.set_path(&database);
        config.database_url = url.to_string();
        let test_state = std::env::var_os("RECOLLECT_TEST_STATE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| ".cache".into());
        config.credential_file = test_state
            .join(format!("{database}-credentials.json"))
            .to_string_lossy()
            .into_owned();
        config.erasure_journal = test_state
            .join(format!("{database}-erasure-journal"))
            .to_string_lossy()
            .into_owned();
        config.artifact_dir = test_state
            .join(format!("{database}-artifacts"))
            .to_string_lossy()
            .into_owned();
        let pool = db::pool(&config.database_url).await.unwrap();
        recollect_server::privacy_journal::initialize(&admin, &config)
            .await
            .unwrap();
        let state = AppState::new(pool, config).unwrap();
        Self {
            router: app(state.clone()),
            state,
            admin,
            root,
            database,
        }
    }
    async fn call(
        &self,
        method: &str,
        path: &str,
        login: Option<&Login>,
        body: Value,
    ) -> (StatusCode, Value, String) {
        self.keyed(method, path, login, body, None).await
    }
    async fn keyed(
        &self,
        method: &str,
        path: &str,
        login: Option<&Login>,
        body: Value,
        key: Option<&str>,
    ) -> (StatusCode, Value, String) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json");
        if let Some(key) = key {
            request = request.header("idempotency-key", key);
        }
        if let Some(login) = login {
            request = request
                .header("cookie", &login.cookie)
                .header("x-csrf-token", &login.csrf);
        }
        let response = self
            .router
            .clone()
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
            cookie,
        )
    }
    async fn login(&self) -> Login {
        let (status,body,cookie) = self.call("POST","/api/auth/login",None,json!({"username":self.state.config.owner_username,"password":self.state.config.owner_password})).await;
        assert_eq!(status, StatusCode::OK);
        assert!(cookie.contains("HttpOnly; SameSite=Strict"));
        Login {
            cookie: cookie.split(';').next().unwrap().into(),
            csrf: body["csrf_token"].as_str().unwrap().into(),
        }
    }
    async fn fixture_member(&self) -> (Uuid, Login) {
        let id = Uuid::new_v4();
        let token = Uuid::new_v4();
        let csrf = Uuid::new_v4();
        sqlx::query("INSERT INTO accounts (id,username) VALUES ($1,$2)")
            .bind(id)
            .bind(format!("member-{id}"))
            .execute(&self.admin)
            .await
            .unwrap();
        sqlx::query("INSERT INTO sessions (token,account_id,csrf_token,expires_at) VALUES ($1,$2,$3,now()+interval '1 hour')").bind(token).bind(id).bind(csrf).execute(&self.admin).await.unwrap();
        (
            id,
            Login {
                cookie: format!("recollect_session={token}"),
                csrf: csrf.to_string(),
            },
        )
    }
    async fn finish(self) {
        recollect_server::graph::analytics::reconcile(&self.state.config, &self.admin)
            .await
            .unwrap();
        // These identities come only from this harness's UUID-owned database.
        // Failed fixtures do not reach finish and remain available for diagnosis.
        let brains: Vec<Uuid> = sqlx::query_scalar(
            "SELECT brain_id FROM graph_generations UNION SELECT brain_id FROM privacy_requests",
        )
        .fetch_all(&self.admin)
        .await
        .unwrap();
        for brain in brains {
            graph::cleanup(&self, brain).await;
        }
        let _ = std::fs::remove_file(&self.state.config.credential_file);
        let _ = std::fs::remove_dir_all(&self.state.config.artifact_dir);
        let _ = std::fs::remove_dir_all(&self.state.config.erasure_journal);
        self.state.pool.close().await;
        self.admin.close().await;
        sqlx::query(&format!("DROP DATABASE {} WITH (FORCE)", self.database))
            .execute(&self.root)
            .await
            .unwrap();
        self.root.close().await;
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn owner_brain_isolation_audit_and_recovery() {
    let h = Harness::new().await;
    db::migrate(&h.admin).await.unwrap();
    let owner = db::bootstrap(&h.admin, &h.state.config.owner_username)
        .await
        .unwrap();
    assert!(db::bootstrap(&h.admin, "renamed-owner").await.is_err());
    let owners: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(owners, 1);
    assert_eq!(
        h.call("GET", "/api/brains", None, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/auth/login",
            None,
            json!({"username":"owner","password":Uuid::new_v4().to_string()})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let external = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("origin", "https://outside.invalid")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        h.router.clone().oneshot(external).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let login = h.login().await;
    assert_eq!(
        h.call("GET", "/api/auth/me", Some(&login), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call("GET", "/api/not-a-route", Some(&login), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call("POST", "/api/brains", Some(&login), json!({"name":"  "}))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let no_csrf = Login {
        cookie: login.cookie.clone(),
        csrf: String::new(),
    };
    assert_eq!(
        h.call(
            "POST",
            "/api/brains",
            Some(&no_csrf),
            json!({"name":"denied"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"  Owner Brain  ","description":"Allowed evidence"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(brain["name"], "Owner Brain");
    let id = brain["id"].as_str().unwrap();
    let brain_id = Uuid::parse_str(id).unwrap();
    let (member_id, member) = h.fixture_member().await;
    let (status, other, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&member),
            json!({"name":"Other Brain canary"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let other_id = other["id"].as_str().unwrap();
    let (_, visible, _) = h
        .call("GET", "/api/brains", Some(&login), Value::Null)
        .await;
    assert_eq!(visible.as_array().unwrap().len(), 1);
    assert_eq!(visible[0]["id"], id);
    for suffix in ["", "/audit"] {
        assert_eq!(
            h.call(
                "GET",
                &format!("/api/brains/{other_id}{suffix}"),
                Some(&login),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            h.call(
                "GET",
                &format!("/api/brains/{id}{suffix}"),
                Some(&member),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{other_id}"),
            Some(&login),
            json!({"name":"forbidden"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{other_id}"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );

    sqlx::query("INSERT INTO brain_grants (brain_id,account_id,role) VALUES ($1,$2,'reader')")
        .bind(brain_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{id}"),
            Some(&member),
            json!({"name":"reader write"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}/audit"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, updated, _) = h
        .call(
            "PATCH",
            &format!("/api/brains/{id}"),
            Some(&login),
            json!({"name":"Updated Brain","archived":true}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["archived"], true);
    let persisted: String = sqlx::query_scalar("SELECT name FROM brains WHERE id=$1")
        .bind(brain_id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(persisted, "Updated Brain");
    let (_, events, _) = h
        .call(
            "GET",
            &format!("/api/brains/{id}/audit"),
            Some(&login),
            Value::Null,
        )
        .await;
    assert_eq!(events.as_array().unwrap().len(), 2);
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{id}"),
            Some(&login),
            json!({"archived":false})
        )
        .await
        .1["archived"],
        false
    );
    sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
        .bind(brain_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{other_id}"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );

    // Prove RLS with the actual non-owner app connection, outside HTTP filtering.
    let privileged: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&h.state.pool)
    .await
    .unwrap();
    assert!(!privileged);
    let unbound: i64 = sqlx::query_scalar("SELECT count(*) FROM brains")
        .fetch_one(&h.state.pool)
        .await
        .unwrap();
    assert_eq!(unbound, 0);
    let mut tx = db::actor_tx(&h.state.pool, owner).await.ok().unwrap();
    let scoped: i64 = sqlx::query_scalar("SELECT count(*) FROM brains")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(scoped, 1);
    tx.commit().await.unwrap();

    // Inject an audit failure and use the production create handler. Its canonical insert must roll back.
    sqlx::query("ALTER TABLE mutation_audit ADD CONSTRAINT test_audit_failure CHECK (action <> 'brain.create') NOT VALID").execute(&h.admin).await.unwrap();
    assert_eq!(
        h.call(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"Must roll back"})
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let rolled_back: i64 =
        sqlx::query_scalar("SELECT count(*) FROM brains WHERE name='Must roll back'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(rolled_back, 0);
    sqlx::query("ALTER TABLE mutation_audit DROP CONSTRAINT test_audit_failure")
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"After rollback"})
        )
        .await
        .0,
        StatusCode::OK
    );

    // Session persists across router restart, expires, revokes and recovers through canonical state.
    let restarted = app(AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap());
    let request = Request::builder()
        .uri(format!("/api/brains/{id}"))
        .header("cookie", &login.cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        restarted.oneshot(request).await.unwrap().status(),
        StatusCode::OK
    );
    db::recover_owner(&h.admin).await.unwrap();
    assert_eq!(
        h.call("GET", "/api/auth/me", Some(&login), Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let renewed = h.login().await;
    assert_eq!(
        h.call("POST", "/api/auth/logout", Some(&renewed), Value::Null)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.call("GET", "/api/auth/me", Some(&renewed), Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let expiring = h.login().await;
    sqlx::query("UPDATE sessions SET expires_at=now()-interval '1 second' WHERE account_id=$1")
        .bind(owner)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call("GET", "/api/auth/me", Some(&expiring), Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires real PostgreSQL/pgvector and Neo4j/GDS; run ./scripts/test-platform.sh"]
async fn dependencies_report_real_success_and_failure() {
    let h = Harness::new().await;
    let status = health::check(&h.state).await;
    assert!(
        status.ready,
        "Both repository-owned databases, pgvector and GDS must answer real queries"
    );
    assert!(status.dependencies.iter().all(|s| s.connected));
    let mut config = (*h.state.config).clone();
    config.neo4j_url = "http://127.0.0.1:1".into();
    let degraded = AppState::new(h.state.pool.clone(), config).unwrap();
    let status = health::check(&degraded).await;
    assert!(!status.ready);
    assert!(status.dependencies[0].connected);
    assert!(!status.dependencies[1].connected);
    let response = app(degraded)
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn durable_command_replay_atomicity_capacity_and_freshness() {
    use recollect_server::worker;
    let h = Harness::new().await;
    let login = h.login().await;
    let key = Uuid::new_v4().to_string();
    let body = json!({"name":"Durable Brain"});
    let (a, b) = tokio::join!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&login),
            body.clone(),
            Some(&key)
        ),
        h.keyed("POST", "/api/brains", Some(&login), body, Some(&key))
    );
    assert_eq!(a.0, StatusCode::OK);
    assert_eq!(b.0, StatusCode::OK);
    assert_eq!(a.1, b.1);
    let id = a.1["id"].as_str().unwrap();
    let brain = Uuid::parse_str(id).unwrap();
    let (mutations, jobs, receipts): (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM mutation_audit WHERE action='brain.create'),(SELECT count(*) FROM jobs),(SELECT count(*) FROM command_receipts)").fetch_one(&h.admin).await.unwrap();
    assert_eq!((mutations, jobs, receipts), (1, 1, 1));
    assert_eq!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"Different"}),
            Some(&key)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"Invalid key"}),
            Some(" ")
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    // An outbox write failure must roll back canonical state, audit and receipt.
    sqlx::query(
        "ALTER TABLE jobs ADD CONSTRAINT reject_enqueue CHECK (kind <> 'brain.refresh') NOT VALID",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    let failed_key = Uuid::new_v4().to_string();
    assert_eq!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"No partial state"}),
            Some(&failed_key)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let partial: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM brains),(SELECT count(*) FROM mutation_audit WHERE action='brain.create'),(SELECT count(*) FROM command_receipts)").fetch_one(&h.admin).await.unwrap();
    assert_eq!(partial, (1, 1, 1));
    sqlx::query("ALTER TABLE jobs DROP CONSTRAINT reject_enqueue")
        .execute(&h.admin)
        .await
        .unwrap();

    let original_job: Uuid = sqlx::query_scalar("SELECT id FROM jobs WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("INSERT INTO jobs(id,brain_id,actor_id,audit_id,target_id,kind,lane) SELECT gen_random_uuid(),brain_id,actor_id,audit_id,target_id,kind,lane FROM jobs CROSS JOIN generate_series(1,499) WHERE id=$1")
        .bind(original_job).execute(&h.admin).await.unwrap();
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{id}"),
            Some(&login),
            json!({"name":"Must wait"})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}"),
            Some(&login),
            Value::Null
        )
        .await
        .1["name"],
        "Durable Brain"
    );
    sqlx::query("DELETE FROM jobs WHERE id<>$1")
        .bind(original_job)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}/processing"),
            Some(&login),
            Value::Null
        )
        .await
        .1["state"],
        "queued"
    );
    // New AppState represents restart: all useful work is recovered from the DB.
    let restarted = AppState::new(h.state.pool.clone(), (*h.state.config).clone()).unwrap();
    assert!(worker::run_once(&restarted, "interactive").await.unwrap());
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}/processing"),
            Some(&login),
            Value::Null
        )
        .await
        .1["state"],
        "current"
    );
    let changed_key = Uuid::new_v4().to_string();
    assert_eq!(
        h.keyed(
            "PATCH",
            &format!("/api/brains/{id}"),
            Some(&login),
            json!({"name":"Second state"}),
            Some(&changed_key)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}/processing"),
            Some(&login),
            Value::Null
        )
        .await
        .1["state"],
        "queued"
    );
    let stale = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{id}"),
            Some(&login),
            json!({"name":"Latest state"})
        )
        .await
        .0,
        StatusCode::OK
    );
    worker::execute(&h.state, &stale).await.unwrap();
    let projected: String =
        sqlx::query_scalar("SELECT name FROM brain_directory WHERE brain_id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(projected, "Latest state");
    assert_eq!(
        worker::execute(&h.state, &stale).await,
        Err(worker::Failure::LostLease)
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}/processing"),
            Some(&login),
            Value::Null
        )
        .await
        .1["state"],
        "current"
    );

    // Another actor can use the same key independently, but cannot read this job/index/receipt.
    let (member_id, member) = h.fixture_member().await;
    assert_eq!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&member),
            json!({"name":"Member Brain"}),
            Some(&key)
        )
        .await
        .0,
        StatusCode::OK
    );
    for suffix in ["jobs", "processing"] {
        assert_eq!(
            h.call(
                "GET",
                &format!("/api/brains/{id}/{suffix}"),
                Some(&member),
                Value::Null
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    let mut tx = db::actor_tx(&h.state.pool, member_id).await.ok().unwrap();
    let counts: (i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM jobs WHERE brain_id=$1),(SELECT count(*) FROM brain_directory WHERE brain_id=$1),(SELECT count(*) FROM command_receipts WHERE brain_id=$1)").bind(brain).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
    tx.commit().await.unwrap();
    // An ownership transfer fixture makes a formerly authorized receipt inaccessible.
    sqlx::query("UPDATE brains SET owner_id=$2 WHERE id=$1")
        .bind(brain)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"Durable Brain"}),
            Some(&key)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{id}"),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    // Receipt expiry permits a fresh command even after access to its former Brain was revoked.
    sqlx::query("UPDATE command_receipts SET expires_at=now()-interval '1 second' WHERE brain_id=$1 AND key=$2")
        .bind(brain).bind(&key).execute(&h.admin).await.unwrap();
    assert_eq!(
        h.keyed(
            "POST",
            "/api/brains",
            Some(&login),
            json!({"name":"Fresh after expiry"}),
            Some(&key)
        )
        .await
        .0,
        StatusCode::OK
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn worker_limits_crash_leases_retry_cancel_and_revocation() {
    use recollect_server::worker::{self, Failure};
    let h = Harness::new().await;
    let login = h.login().await;
    for name in ["Lease A", "Lease B", "Lease C"] {
        assert_eq!(
            h.call("POST", "/api/brains", Some(&login), json!({"name":name}))
                .await
                .0,
            StatusCode::OK
        );
    }
    let (a, b, c) = tokio::join!(
        worker::claim(&h.state.pool, "interactive"),
        worker::claim(&h.state.pool, "interactive"),
        worker::claim(&h.state.pool, "interactive")
    );
    let claims: Vec<_> = [a.unwrap(), b.unwrap(), c.unwrap()]
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(claims.len(), 2);
    assert_ne!(claims[0].id, claims[1].id);
    let old = claims[0].clone();
    let other = claims[1].clone();
    assert!(worker::renew(&h.state.pool, &old).await.unwrap());
    // Simulate an interrupted worker after its durable claim, then reclaim its expired lease.
    sqlx::query("UPDATE jobs SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(old.id)
        .execute(&h.admin)
        .await
        .unwrap();
    let fresh = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fresh.id, old.id);
    assert_ne!(fresh.lease_token, old.lease_token);
    assert_eq!(
        worker::execute(&h.state, &old).await,
        Err(Failure::LostLease)
    );
    assert!(!worker::renew(&h.state.pool, &old).await.unwrap());
    assert!(
        !worker::fail(&h.state.pool, &old, Failure::Database)
            .await
            .unwrap()
    );
    // Exhausted crashed attempts become a visible failure rather than an eternal lease.
    sqlx::query(
        "UPDATE jobs SET attempts=max_attempts,lease_until=now()-interval '1 second' WHERE id=$1",
    )
    .bind(fresh.id)
    .execute(&h.admin)
    .await
    .unwrap();
    let third = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    let failed: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM jobs WHERE id=$1")
            .bind(fresh.id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(failed, ("failed".into(), Some("lease_expired".into())));
    worker::execute(&h.state, &third).await.unwrap();
    let cancel_path = format!("/api/brains/{}/jobs/{}/cancel", other.brain_id, other.id);
    assert_eq!(
        h.call("POST", &cancel_path, Some(&login), Value::Null)
            .await
            .1["state"],
        "cancelled"
    );
    assert_eq!(
        worker::execute(&h.state, &other).await,
        Err(Failure::LostLease)
    );
    assert_eq!(
        h.call("POST", &cancel_path, Some(&login), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    let retry_path = format!("/api/brains/{}/jobs/{}/retry", fresh.brain_id, fresh.id);
    let (member_id, member) = h.fixture_member().await;
    assert_eq!(
        h.call("POST", &retry_path, Some(&member), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query("INSERT INTO brain_grants(brain_id,account_id,role) VALUES ($1,$2,'reader')")
        .bind(fresh.brain_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{}/jobs", fresh.brain_id),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call("POST", &retry_path, Some(&member), Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call("POST", &retry_path, Some(&login), Value::Null)
            .await
            .1["state"],
        "queued"
    );
    assert_eq!(
        h.call("POST", &retry_path, Some(&login), Value::Null)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let retry = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retry.id, fresh.id);
    assert!(
        worker::fail(&h.state.pool, &retry, Failure::Database)
            .await
            .unwrap()
    );
    let scheduled: (String, i32, bool) =
        sqlx::query_as("SELECT state,attempts,not_before>clock_timestamp() FROM jobs WHERE id=$1")
            .bind(retry.id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(scheduled, ("queued".into(), 1, true));
    assert!(
        worker::claim(&h.state.pool, "interactive")
            .await
            .unwrap()
            .is_none()
    );
    sqlx::query("UPDATE jobs SET not_before=now() WHERE id=$1")
        .bind(retry.id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(worker::run_once(&h.state, "interactive").await.unwrap());

    // A queued admin contribution cannot publish once that grant is removed.
    sqlx::query("UPDATE brain_grants SET role='admin' WHERE brain_id=$1 AND account_id=$2")
        .bind(fresh.brain_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{}", fresh.brain_id),
            Some(&member),
            json!({"name":"Before revocation"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let revoked = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(revoked.actor_id, member_id);
    sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
        .bind(fresh.brain_id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        worker::execute(&h.state, &revoked).await,
        Err(Failure::Revoked)
    );
    assert!(
        worker::fail(&h.state.pool, &revoked, Failure::Revoked)
            .await
            .unwrap()
    );
    let projected: String =
        sqlx::query_scalar("SELECT name FROM brain_directory WHERE brain_id=$1")
            .bind(fresh.brain_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_ne!(projected, "Before revocation");
    let retry_revoked = format!("/api/brains/{}/jobs/{}/retry", revoked.brain_id, revoked.id);
    assert_eq!(
        h.call("POST", &retry_revoked, Some(&login), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert!(worker::run_once(&h.state, "interactive").await.unwrap());
    let projected: String =
        sqlx::query_scalar("SELECT name FROM brain_directory WHERE brain_id=$1")
            .bind(fresh.brain_id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(projected, "Before revocation");
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{}/jobs/{}/retry", third.brain_id, revoked.id),
            Some(&login),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Unsupported consumers fail honestly; ordinary completed jobs stay successful.
    assert_eq!(
        h.call(
            "PATCH",
            &format!("/api/brains/{}", fresh.brain_id),
            Some(&login),
            json!({"description":"Unsupported consumer fixture"})
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE jobs SET kind='unregistered.fixture' WHERE state='queued'")
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(worker::run_once(&h.state, "interactive").await.unwrap());
    let unsupported: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM jobs WHERE kind='unregistered.fixture'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        unsupported,
        ("failed".into(), Some("unsupported_kind".into()))
    );
    let success: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE state='succeeded'")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(success >= 2);
    let unknown: Uuid = sqlx::query_scalar("SELECT id FROM jobs WHERE kind='unregistered.fixture'")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET kind='brain.refresh' WHERE id=$1")
        .bind(unknown)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{}/jobs/{unknown}/retry", fresh.brain_id),
            Some(&login),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    for attempt in 1..=3 {
        let work = worker::claim(&h.state.pool, "interactive")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(work.id, unknown);
        assert!(
            worker::fail(&h.state.pool, &work, Failure::Database)
                .await
                .unwrap()
        );
        let state: (String, i32) = sqlx::query_as("SELECT state,attempts FROM jobs WHERE id=$1")
            .bind(unknown)
            .fetch_one(&h.admin)
            .await
            .unwrap();
        assert_eq!(
            state,
            (
                if attempt == 3 { "failed" } else { "queued" }.into(),
                attempt
            )
        );
        sqlx::query("UPDATE jobs SET not_before=now() WHERE id=$1")
            .bind(unknown)
            .execute(&h.admin)
            .await
            .unwrap();
    }
    // Cleanup expires only old operational records; canonical data and audit survive.
    let audit_before: i64 = sqlx::query_scalar("SELECT count(*) FROM mutation_audit")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET updated_at=now()-interval '8 days' WHERE state='succeeded'")
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(
        worker::claim(&h.state.pool, "interactive")
            .await
            .unwrap()
            .is_none()
    );
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE state='succeeded'")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    let audit_after: i64 = sqlx::query_scalar("SELECT count(*) FROM mutation_audit")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(audit_after, audit_before);
    let brains: i64 = sqlx::query_scalar("SELECT count(*) FROM brains")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(brains, 3);
    h.finish().await;
}

impl Harness {
    async fn bearer(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn pair_device(&self, login: &Login, name: &str) -> (Uuid, String) {
        let (_, start, _) = self
            .call("POST", "/api/devices/pairings", None, json!({"name":name}))
            .await;
        assert_eq!(
            self.call(
                "POST",
                &format!(
                    "/api/devices/pairings/{}/approve",
                    start["user_code"].as_str().unwrap()
                ),
                Some(login),
                json!({"approve":true})
            )
            .await
            .0,
            StatusCode::OK
        );
        let (_, poll, _) = self
            .call(
                "POST",
                "/api/devices/pairings/poll",
                None,
                json!({"device_code":start["device_code"]}),
            )
            .await;
        let token = poll["token"].as_str().unwrap().to_owned();
        assert_eq!(
            self.call(
                "POST",
                "/api/devices/pairings/finish",
                None,
                json!({"device_code":start["device_code"]})
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
        (
            poll["device"]["id"].as_str().unwrap().parse().unwrap(),
            token,
        )
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn device_pairing_delivery_revocation_and_worker_authority() {
    use recollect_server::worker;
    let h = Harness::new().await;
    let owner = h.login().await;
    let (foreign, foreign_login) = h.fixture_member().await;
    let (_, start, _) = h
        .call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"First laptop"}),
        )
        .await;
    let public = start["user_code"].as_str().unwrap();
    let private = json!({"device_code":start["device_code"]});
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/devices/pairings/{public}"),
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.call("POST", "/api/devices/pairings/poll", None, private.clone())
            .await
            .1["state"],
        "pending"
    );
    assert_eq!(
        h.call("POST", "/api/devices/pairings/poll", None, private.clone())
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    let approval = format!("/api/devices/pairings/{public}/approve");
    assert_eq!(
        h.call("POST", &approval, Some(&owner), json!({"approve":true}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        h.call("POST", &approval, Some(&owner), json!({"approve":true}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE device_pairings SET last_polled_at=NULL")
        .execute(&h.admin)
        .await
        .unwrap();
    let (_, poll, _) = h
        .call("POST", "/api/devices/pairings/poll", None, private.clone())
        .await;
    let token = poll["token"].as_str().unwrap().to_owned();
    let first: Uuid = poll["device"]["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        h.bearer("GET", "/api/auth/me", &token, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE device_pairings SET last_polled_at=NULL")
        .execute(&h.admin)
        .await
        .unwrap();
    assert!(
        h.call("POST", "/api/devices/pairings/poll", None, private.clone())
            .await
            .1["token"]
            == poll["token"]
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/devices/pairings/finish",
            None,
            private.clone()
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        h.call(
            "POST",
            "/api/devices/pairings/finish",
            None,
            private.clone()
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    h.call("POST", "/api/devices/pairings/cancel", None, private)
        .await;
    assert_eq!(
        h.bearer("GET", "/api/auth/me", &token, Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer("GET", "/api/team", &token, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.bearer("POST", &approval, &token, json!({"approve":true}))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let wrong = Request::builder()
        .uri("/api/brains")
        .header("cookie", &owner.cookie)
        .header("authorization", "Bearer invalid")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        h.router.clone().oneshot(wrong).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let private_brain = h
        .call(
            "POST",
            "/api/brains",
            Some(&foreign_login),
            json!({"name":"Foreign private evidence"}),
        )
        .await
        .1;
    assert_eq!(
        h.bearer(
            "GET",
            &format!("/api/brains/{}", private_brain["id"].as_str().unwrap()),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, brain) = h
        .bearer(
            "POST",
            "/api/brains",
            &token,
            json!({"name":"Device provenance"}),
        )
        .await;
    let brain_id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let audit_device: Uuid = sqlx::query_scalar(
        "SELECT device_id FROM mutation_audit WHERE brain_id=$1 AND action='brain.create'",
    )
    .bind(brain_id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(audit_device, first);
    let (second, second_token) = h.pair_device(&owner, "Second laptop").await;
    assert_eq!(
        h.bearer(
            "PATCH",
            &format!("/api/brains/{brain_id}"),
            &second_token,
            json!({"name":"Still permitted"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer(
            "DELETE",
            &format!("/api/devices/{second}"),
            &token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{first}"),
            Some(&foreign_login),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call("GET", "/api/devices", Some(&foreign_login), Value::Null)
            .await
            .1,
        json!([])
    );
    let listed = h
        .call("GET", "/api/devices", Some(&owner), Value::Null)
        .await
        .1
        .to_string();
    assert!(!listed.contains(&token));
    sqlx::query("UPDATE jobs SET state='cancelled' WHERE device_id IS NULL")
        .execute(&h.admin)
        .await
        .unwrap();
    let first_job = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first_job.device_id, Some(first));
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/devices/{first}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        worker::execute(&h.state, &first_job).await,
        Err(worker::Failure::Revoked)
    );
    worker::fail(&h.state.pool, &first_job, worker::Failure::Revoked)
        .await
        .unwrap();
    assert_eq!(
        h.bearer("GET", "/api/brains", &token, Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    let second_job = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second_job.device_id, Some(second));
    worker::execute(&h.state, &second_job).await.unwrap();
    assert_eq!(
        h.bearer("GET", "/api/brains", &second_token, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    // The approving browser becomes the retry submitter, without the revoked device.
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{brain_id}/jobs/{}/retry", first_job.id),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::OK
    );
    let retried = worker::claim(&h.state.pool, "interactive")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retried.device_id, None);
    worker::execute(&h.state, &retried).await.unwrap();
    // Revocation-only possession is allowed after authentication expiry, including disabled accounts.
    let (foreign_device, foreign_token) = h.pair_device(&foreign_login, "Foreign laptop").await;
    h.call(
        "PATCH",
        &format!("/api/team/accounts/{foreign}"),
        Some(&owner),
        json!({"enabled":false}),
    )
    .await;
    assert_eq!(
        h.bearer("GET", "/api/auth/me", &foreign_token, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.bearer(
            "POST",
            "/api/devices/revoke-self",
            &foreign_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM devices WHERE id=$1")
            .bind(foreign_device)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert!(revoked);
    assert_eq!(
        h.bearer(
            "POST",
            "/api/devices/revoke-self",
            &foreign_token,
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    for (decision, expire) in [(false, false), (true, true)] {
        let (_, pending, _) = h
            .call(
                "POST",
                "/api/devices/pairings",
                None,
                json!({"name":"Not enrolled"}),
            )
            .await;
        let code = pending["user_code"].as_str().unwrap();
        if expire {
            sqlx::query("UPDATE device_pairings SET expires_at=now()-interval '1 second' WHERE user_code=$1").bind(code).execute(&h.admin).await.unwrap();
        }
        let status = h
            .call(
                "POST",
                &format!("/api/devices/pairings/{code}/approve"),
                Some(&owner),
                json!({"approve":decision}),
            )
            .await
            .0;
        assert_eq!(
            status,
            if expire {
                StatusCode::GONE
            } else {
                StatusCode::OK
            }
        );
        assert_eq!(
            h.call(
                "POST",
                "/api/devices/pairings/poll",
                None,
                json!({"device_code":pending["device_code"]})
            )
            .await
            .0,
            StatusCode::GONE
        );
    }
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    sqlx::query("INSERT INTO devices(id,account_id,name,token) SELECT gen_random_uuid(),$1,'Capacity fixture',gen_random_uuid() FROM generate_series(1,19)").bind(owner_id).execute(&h.admin).await.unwrap();
    let (_, pending, _) = h
        .call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"Over capacity"}),
        )
        .await;
    assert_eq!(
        h.call(
            "POST",
            &format!(
                "/api/devices/pairings/{}/approve",
                pending["user_code"].as_str().unwrap()
            ),
            Some(&owner),
            json!({"approve":true})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    sqlx::query("INSERT INTO device_pairings(id,device_code,user_code,name) SELECT gen_random_uuid(),gen_random_uuid(),gen_random_uuid()::text,'Admission fixture' FROM generate_series(1,1000)").execute(&h.admin).await.unwrap();
    assert_eq!(
        h.call(
            "POST",
            "/api/devices/pairings",
            None,
            json!({"name":"Too many pending"})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    h.finish().await;
}

#[path = "platform/evidence.rs"]
mod evidence_tests;
