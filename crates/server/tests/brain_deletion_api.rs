//! Brain deletion HTTP layer integration test (ADR 0016, contract
//! docs/contracts/platform-brain-deletion.md). It runs the real API against a
//! disposable repository-owned database: the authorization matrix, preview
//! counts versus actuals, the three rejection paths before any write,
//! unknown-versus-forbidden for a principal that lost access, read and recall
//! denial across consumers after commit, idempotent replay and the companion
//! fence. No provider calls are made.
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use recollect_server::{AppState, app, config::Config, db};
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

impl Harness {
    async fn new() -> Self {
        let mut config = Config::from_env()
            .expect("Use ./scripts/test-platform.sh with repository-owned databases");
        let admin_url = std::env::var("DATABASE_ADMIN_URL").expect("DATABASE_ADMIN_URL required");
        let root = db::pool(&admin_url).await.unwrap();
        let database = format!("recollect_test_brain_del_api_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {database}"))
            .execute(&root)
            .await
            .unwrap();
        eprintln!("Disposable Brain deletion API test database: {database}");
        sqlx::query(&format!("COMMENT ON DATABASE {database} IS 'Recollect disposable Brain deletion API test created by crates/server/tests/brain_deletion_api.rs'"))
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
    ) -> (StatusCode, Value) {
        self.keyed(method, path, login, body, None).await
    }
    async fn keyed(
        &self,
        method: &str,
        path: &str,
        login: Option<&Login>,
        body: Value,
        key: Option<&str>,
    ) -> (StatusCode, Value) {
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
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn login(&self) -> Login {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"username":self.state.config.owner_username,"password":self.state.config.owner_password}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        Login {
            cookie: cookie.split(';').next().unwrap().into(),
            csrf: body["csrf_token"].as_str().unwrap().into(),
        }
    }
    /// Open a browser session for an account the fixture seeded directly.
    async fn session_for(&self, account: Uuid) -> Login {
        let token = Uuid::new_v4();
        let csrf = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO sessions (token,account_id,csrf_token,expires_at) VALUES ($1,$2,$3,now()+interval '1 hour')",
        )
        .bind(token)
        .bind(account)
        .bind(csrf)
        .execute(&self.admin)
        .await
        .unwrap();
        Login {
            cookie: format!("recollect_session={token}"),
            csrf: csrf.to_string(),
        }
    }
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
        let (_, start) = self
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
        let (_, poll) = self
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
    async fn finish(self) {
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

struct Fixture {
    a: Uuid,
    b: Uuid,
    member: Uuid,
    dev: Uuid,
    s1: Uuid,
    v1: Uuid,
    v2: Uuid,
    c2: Uuid,
    cl1: Uuid,
    r1: Uuid,
    r2: Uuid,
    d1: Uuid,
    ar1: Uuid,
    rep1: Uuid,
    snap1: Uuid,
    art1: Uuid,
    man1: Uuid,
    mrev1: Uuid,
    env1: Uuid,
    col1: Uuid,
    area1: Uuid,
    ws1: Uuid,
    t1: Uuid,
    ss1: Uuid,
    cb1: Uuid,
    ce1: Uuid,
    conn1: Uuid,
    prof1: Uuid,
    mc1: Uuid,
    mp: Uuid,
    sp1: Uuid,
    mreq1: Uuid,
    seed_req: Uuid,
    jh: Uuid,
    jl: Uuid,
    ja: Uuid,
    jg: Uuid,
    jw: Uuid,
    sb: Uuid,
    vb: Uuid,
    clb: Uuid,
    rb: Uuid,
}

impl Fixture {
    fn new() -> Self {
        let u = || Uuid::new_v4();
        Self {
            a: u(),
            b: u(),
            member: u(),
            dev: u(),
            s1: u(),
            v1: u(),
            v2: u(),
            c2: u(),
            cl1: u(),
            r1: u(),
            r2: u(),
            d1: u(),
            ar1: u(),
            rep1: u(),
            snap1: u(),
            art1: u(),
            man1: u(),
            mrev1: u(),
            env1: u(),
            col1: u(),
            area1: u(),
            ws1: u(),
            t1: u(),
            ss1: u(),
            cb1: u(),
            ce1: u(),
            conn1: u(),
            prof1: u(),
            mc1: u(),
            mp: u(),
            sp1: u(),
            mreq1: u(),
            seed_req: u(),
            jh: u(),
            jl: u(),
            ja: u(),
            jg: u(),
            jw: u(),
            sb: u(),
            vb: u(),
            clb: u(),
            rb: u(),
        }
    }
}

/// Seeds the shared Brain-deletion fixture (rich Brain A "Alpha", control
/// Brain B "Beta", a reader member and its device) through the same script the
/// SQL-level test uses, so both layers see identical content.
async fn seed(admin: &PgPool, owner: Uuid, f: &Fixture) {
    let mut sql = std::fs::read_to_string(seed_script_path()).unwrap();
    for (token, id) in [
        ("__R1Q__", f.r1),
        ("__R2Q__", f.r2),
        ("__SNAP1Q__", f.snap1),
    ] {
        sql = sql.replace(token, &format!("'{id}'"));
    }
    for (token, id) in [
        ("__A__", f.a),
        ("__B__", f.b),
        ("__OWNER__", owner),
        ("__MEMBER__", f.member),
        ("__DEV__", f.dev),
        ("__S1__", f.s1),
        ("__V1__", f.v1),
        ("__V2__", f.v2),
        ("__C2__", f.c2),
        ("__CL1__", f.cl1),
        ("__R1__", f.r1),
        ("__R2__", f.r2),
        ("__D1__", f.d1),
        ("__AR1__", f.ar1),
        ("__REP1__", f.rep1),
        ("__SNAP1__", f.snap1),
        ("__ART1__", f.art1),
        ("__MAN1__", f.man1),
        ("__MREV1__", f.mrev1),
        ("__ENV1__", f.env1),
        ("__COL1__", f.col1),
        ("__AREA1__", f.area1),
        ("__WS1__", f.ws1),
        ("__T1__", f.t1),
        ("__SS1__", f.ss1),
        ("__CB1__", f.cb1),
        ("__CE1__", f.ce1),
        ("__CONN1__", f.conn1),
        ("__PROF1__", f.prof1),
        ("__MC1__", f.mc1),
        ("__MP__", f.mp),
        ("__SP1__", f.sp1),
        ("__MREQ1__", f.mreq1),
        ("__SEEDREQ__", f.seed_req),
        ("__JH__", f.jh),
        ("__JL__", f.jl),
        ("__JA__", f.ja),
        ("__JG__", f.jg),
        ("__JW__", f.jw),
        ("__SB__", f.sb),
        ("__VB__", f.vb),
        ("__CLB__", f.clb),
        ("__RB__", f.rb),
    ] {
        sql = sql.replace(token, &format!("'{id}'"));
    }
    for (token, marker) in [
        ("__MT_TITLE__", "brain-del-api-marker-title"),
        ("__MT_URI__", "https://marker.example/api/doc"),
        ("__MT_PATH__", "/tmp/brain-del-api-marker-path/file.txt"),
        ("__MT_CLAIM__", "brain-del-api-marker-claim-value"),
        ("__MT_CHUNK__", "brain-del-api-marker-chunk-content"),
        ("__MT_RULE__", "brain-del-api-marker-rule-text"),
        ("__MT_RECEIPT__", "brain-del-api-marker-receipt-payload"),
        ("__MT_MANIFEST__", "brain-del-api-marker-manifest-name"),
    ] {
        sql = sql.replace(token, marker);
    }
    assert!(!sql.contains("__"), "unreplaced seed token remains");
    let mut tx = admin.begin().await.unwrap();
    sqlx::raw_sql(&sql).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

fn seed_script_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("brain_deletion_seed.sql")
}

async fn count(admin: &PgPool, table: &str, brain: Uuid) -> i64 {
    // Relation names come from this test's own fixed list.
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE brain_id=$1"))
        .bind(brain)
        .fetch_one(admin)
        .await
        .unwrap()
}

const COUNT_KEYS: &[&str] = &[
    "sources",
    "source_versions",
    "excerpts",
    "chunks",
    "claims",
    "claim_revisions",
    "review_decisions",
    "rejected_rules",
    "snapshots",
    "facts",
    "manifests",
    "manifest_revisions",
    "environments",
    "collections",
    "areas",
    "memberships",
    "repositories",
    "workspaces",
    "checkouts",
    "tasks",
    "capture_bindings",
    "capture_events",
    "capture_reports",
    "connections",
    "profiles",
    "profile_grants",
    "calls",
    "private_runners",
    "semantic_entries",
    "semantic_profiles",
    "learning_runs",
    "handover_runs",
    "graph_generations",
    "analytics_reports",
    "members",
    "group_members",
    "policies",
    "artifacts",
];

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn brain_deletion_api_authorization_and_pre_write_rejections() {
    let h = Harness::new().await;
    let owner_account: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let owner = h.login().await;
    let f = Fixture::new();
    seed(&h.admin, owner_account, &f).await;
    let member = h.session_for(f.member).await;

    // --- Authorization matrix on the live Brain. ---
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{}/deletions/preview", f.a),
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    // A reader is denied on the live Brain: forbidden, not unknown.
    for (method, path) in [
        ("POST", &format!("/api/brains/{}/deletions/preview", f.a)),
        ("DELETE", &format!("/api/brains/{}", f.a)),
    ] {
        let (status, body) = h
            .call(
                method,
                path,
                Some(&member),
                json!({"closure":0,"confirmation":"Alpha"}),
            )
            .await;
        assert_eq!(
            (status, body["code"].as_str()),
            (StatusCode::FORBIDDEN, Some("forbidden")),
            "{method} {path} for a reader"
        );
    }
    // Device tokens cannot initiate deletion at all.
    for (method, path) in [
        ("POST", &format!("/api/brains/{}/deletions/preview", f.a)),
        ("DELETE", &format!("/api/brains/{}", f.a)),
    ] {
        let (status, body) = h
            .bearer(
                method,
                path,
                &f.dev.to_string(),
                json!({"closure":0,"confirmation":"Alpha"}),
            )
            .await;
        assert_eq!(
            (status, body["code"].as_str()),
            (StatusCode::FORBIDDEN, Some("browser_required")),
            "{method} {path} for a device"
        );
    }
    // Unknown Brain: unknown, not forbidden, even for the owner.
    let ghost = Uuid::new_v4();
    assert_eq!(
        h.call(
            "POST",
            &format!("/api/brains/{}/deletions/preview", ghost),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/brains/{ghost}"),
            Some(&owner),
            json!({"closure":0,"confirmation":"Ghost"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // --- Preview: content-free, counter-bearing, counts match actuals. ---
    let (status, preview) = h
        .call(
            "POST",
            &format!("/api/brains/{}/deletions/preview", f.a),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["brain_id"], f.a.to_string());
    assert_eq!(preview["name"], "Alpha");
    assert_eq!(preview["archived"], false);
    assert_eq!(preview["repository_content_allowed"], false);
    assert_eq!(preview["backup_days"], 7);
    assert_eq!(preview["pending_work"], 5);
    let closure = preview["closure"].as_i64().expect("preview counter");
    assert!(closure > 0);
    let counts = &preview["counts"];
    for key in COUNT_KEYS {
        assert!(
            counts.get(*key).and_then(Value::as_i64).is_some(),
            "preview counts miss {key}"
        );
    }
    assert_eq!(counts["sources"], count(&h.admin, "sources", f.a).await);
    assert_eq!(
        counts["source_versions"],
        count(&h.admin, "source_versions", f.a).await
    );
    assert_eq!(counts["claims"], count(&h.admin, "claims", f.a).await);
    assert_eq!(
        counts["claim_revisions"],
        count(&h.admin, "claim_revisions", f.a).await
    );
    assert_eq!(
        counts["snapshots"],
        count(&h.admin, "repository_snapshots", f.a).await
    );
    assert_eq!(
        counts["repositories"],
        count(&h.admin, "repositories", f.a).await
    );
    assert_eq!(
        counts["workspaces"],
        count(&h.admin, "workspace_registrations", f.a).await
    );
    assert_eq!(
        counts["tasks"],
        count(&h.admin, "workspace_tasks", f.a).await
    );
    assert_eq!(
        counts["connections"],
        count(&h.admin, "mcp_connections", f.a).await
    );
    assert_eq!(
        counts["profiles"],
        count(&h.admin, "mcp_profiles", f.a).await
    );
    assert_eq!(
        counts["members"],
        count(&h.admin, "brain_grants", f.a).await
    );
    assert_eq!(
        counts["group_members"],
        count(&h.admin, "brain_group_grants", f.a).await
    );
    assert_eq!(counts["policies"], 3);
    // No controlled content leaks through the preview.
    let rendered = preview.to_string();
    for marker in [
        "brain-del-api-marker-title",
        "brain-del-api-marker-claim-value",
        "brain-del-api-marker-chunk-content",
        "https://marker.example/api/doc",
        "/tmp/brain-del-api-marker-path/file.txt",
    ] {
        assert!(!rendered.contains(marker), "preview leaks {marker}");
    }

    // --- The three rejection paths, each before any write. ---
    let requests_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM privacy_requests WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&h.admin)
            .await
            .unwrap();

    // 1. Wrong confirmation: 400 with the distinct actionable code.
    let (status, body) = h
        .call(
            "DELETE",
            &format!("/api/brains/{}", f.a),
            Some(&owner),
            json!({"closure":closure,"confirmation":"Beta"}),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (
            StatusCode::BAD_REQUEST,
            Some("brain_deletion_confirmation_mismatch")
        )
    );

    // 2. Stale counter with the right confirmation: 409 requiring a fresh preview.
    let (status, body) = h
        .call(
            "DELETE",
            &format!("/api/brains/{}", f.a),
            Some(&owner),
            json!({"closure":closure + 1,"confirmation":"Alpha"}),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::CONFLICT, Some("brain_deletion_preview_changed"))
    );

    // 3. Unknown Brain: 404 no-op (also proven above for the owner).
    let (status, body) = h
        .call(
            "DELETE",
            &format!("/api/brains/{ghost}"),
            Some(&owner),
            json!({"closure":0,"confirmation":"Ghost"}),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("not_found"))
    );

    // None of the rejections wrote anything.
    let requests_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM privacy_requests WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(requests_before, requests_after);
    let brain_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM brains WHERE id=$1")
        .bind(f.a)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(brain_rows, 1);
    let tombstones: i64 =
        sqlx::query_scalar("SELECT count(*) FROM brain_tombstones WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(tombstones, 0);

    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run ./scripts/test-platform.sh"]
async fn brain_deletion_api_full_flow_denial_and_fence() {
    let h = Harness::new().await;
    let owner_account: Uuid =
        sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    let owner = h.login().await;
    let f = Fixture::new();
    seed(&h.admin, owner_account, &f).await;
    let member = h.session_for(f.member).await;

    // --- The accepted deletion. ---
    let (_, preview) = h
        .call(
            "POST",
            &format!("/api/brains/{}/deletions/preview", f.a),
            Some(&owner),
            Value::Null,
        )
        .await;
    let closure = preview["closure"].as_i64().unwrap();
    let (status, deleted) = h
        .keyed(
            "DELETE",
            &format!("/api/brains/{}", f.a),
            Some(&owner),
            json!({"closure":closure,"confirmation":"Alpha"}),
            Some("brain-delete-key-1"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let rid: Uuid = deleted["request"]["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(deleted["request"]["brain_id"], f.a.to_string());
    assert_eq!(deleted["request"]["actor_id"], owner_account.to_string());
    assert_eq!(deleted["request"]["closure"], closure);
    assert_eq!(deleted["request"]["disposition"], "deleted");
    assert_eq!(deleted["request"]["state"], "pending");
    assert_eq!(deleted["request"]["journaled"], false);
    assert_eq!(deleted["request"]["pending_artifacts"], 1);
    assert_eq!(deleted["request"]["acknowledged_devices"], 0);
    assert_eq!(deleted["backup_days"], 7);

    // The Brain row is gone; the control Brain is untouched.
    let brain_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM brains WHERE id=$1")
        .bind(f.a)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(brain_rows, 0);
    let (status, beta) = h
        .call(
            "GET",
            &format!("/api/brains/{}", f.b),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(beta["name"], "Beta");

    // --- Idempotent replay through the existing key protocol. ---
    // The committed deletion invalidated its receipt while the Brain row still
    // existed, so the apply chain emptied it and the tombstone RLS now hides it
    // from the app role: a keyed retry is unknown, exactly like any other
    // request for an absent Brain.
    let (status, body) = h
        .keyed(
            "DELETE",
            &format!("/api/brains/{}", f.a),
            Some(&owner),
            json!({"closure":closure,"confirmation":"Alpha"}),
            Some("brain-delete-key-1"),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("not_found"))
    );
    // The receipt survived content-free: key and Brain kept, input and response
    // emptied, invalidated.
    let receipt: (Uuid, serde_json::Value, Option<serde_json::Value>, bool) =
        sqlx::query_as(
            "SELECT brain_id,input,response,invalidated FROM command_receipts WHERE actor_id=$1 AND key=$2",
        )
        .bind(owner_account)
        .bind("brain-delete-key-1")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(receipt.0, f.a);
    assert_eq!(receipt.1, json!({}));
    assert!(receipt.2.is_none());
    assert!(receipt.3);
    // A fresh attempt on the already-absent Brain is a 404 no-op.
    let (status, body) = h
        .call(
            "DELETE",
            &format!("/api/brains/{}", f.a),
            Some(&owner),
            json!({"closure":closure,"confirmation":"Alpha"}),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("not_found"))
    );

    // --- Status: visible to the initiator and the installation owner only. ---
    let (status, row) = h
        .call(
            "GET",
            &format!("/api/brains/{}/deletions/{rid}", f.a),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(row["id"], rid.to_string());
    assert_eq!(row["brain_id"], f.a.to_string());
    assert_eq!(row["actor_id"], owner_account.to_string());
    assert_eq!(row["closure"], closure);
    assert_eq!(row["disposition"], "deleted");
    assert_eq!(row["state"], "pending");
    assert_eq!(row["acknowledged_devices"], 0);
    // A principal that lost access cannot distinguish deleted from absent.
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{}/deletions/{rid}", f.a),
            Some(&member),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // An unknown request identity is unknown for everyone.
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{}/deletions/{}", f.a, Uuid::new_v4()),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // --- Read and recall denial across consumers (owner's own view). ---
    let list = h
        .call("GET", "/api/brains", Some(&owner), Value::Null)
        .await
        .1;
    assert!(
        !list
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["id"] == f.a.to_string()),
        "deleted Brain still listed"
    );
    for (method, path, body) in [
        ("GET", &format!("/api/brains/{}", f.a), Value::Null),
        (
            "POST",
            &format!("/api/brains/{}/recall", f.a),
            json!({"query":"anything"}),
        ),
        (
            "POST",
            &format!("/api/brains/{}/answer-requests", f.a),
            // A question with a non-stopword term passes input validation so the
            // assertion reaches the Brain authority check.
            json!({"request_id":Uuid::new_v4().to_string(),"question":"describe the marker"}),
        ),
        ("GET", &format!("/api/brains/{}/semantic", f.a), Value::Null),
        ("GET", &format!("/api/brains/{}/graph", f.a), Value::Null),
        ("GET", &format!("/api/brains/{}/evidence", f.a), Value::Null),
        (
            "GET",
            &format!("/api/brains/{}/workspace", f.a),
            Value::Null,
        ),
        ("GET", &format!("/api/brains/{}/claims", f.a), Value::Null),
        ("GET", &format!("/api/brains/{}/jobs", f.a), Value::Null),
        ("GET", &format!("/api/brains/{}/audit", f.a), Value::Null),
        ("GET", &format!("/api/brains/{}/mcp", f.a), Value::Null),
    ] {
        let (status, err) = h.call(method, path, Some(&owner), body).await;
        assert_eq!(
            (status, err["code"].as_str()),
            (StatusCode::NOT_FOUND, Some("not_found")),
            "{method} {path} must be unknown after deletion"
        );
    }
    // The access-lost principal gets unknown, never forbidden.
    for (method, path) in [
        ("GET", &format!("/api/brains/{}", f.a)),
        ("POST", &format!("/api/brains/{}/deletions/preview", f.a)),
        ("DELETE", &format!("/api/brains/{}", f.a)),
    ] {
        let (status, err) = h
            .call(
                method,
                path,
                Some(&member),
                json!({"closure":closure,"confirmation":"Alpha"}),
            )
            .await;
        assert_eq!(
            (status, err["code"].as_str()),
            (StatusCode::NOT_FOUND, Some("not_found")),
            "{method} {path} for the access-lost principal"
        );
    }

    // --- Companion fence: device-only, visibility-driven. ---
    let (_, owner_device) = h.pair_device(&owner, "Fence laptop").await;
    let (status, fence) = h
        .bearer(
            "GET",
            &format!("/api/brains/{}/deletions/fence", f.a),
            &owner_device,
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fence["brain_id"], f.a.to_string());
    assert_eq!(fence["deletion_id"], rid.to_string());
    assert!(fence["sequence"].as_i64().is_some());
    // The member's device cannot see the tombstone: unknown, not forbidden.
    assert_eq!(
        h.bearer(
            "GET",
            &format!("/api/brains/{}/deletions/fence", f.a),
            &f.dev.to_string(),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Browser callers cannot fetch the fence.
    assert_eq!(
        h.call(
            "GET",
            &format!("/api/brains/{}/deletions/fence", f.a),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // A live Brain has no fence.
    assert_eq!(
        h.bearer(
            "GET",
            &format!("/api/brains/{}/deletions/fence", f.b),
            &owner_device,
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    // Retained rows are content-free.
    let retained: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(target::text,'')||' '||coalesce(manifest::text,'') FROM privacy_requests WHERE brain_id=$1",
    )
    .bind(f.a)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    let rendered = retained.join(" ");
    for marker in [
        "brain-del-api-marker-title",
        "brain-del-api-marker-claim-value",
        "brain-del-api-marker-chunk-content",
        "https://marker.example/api/doc",
        "/tmp/brain-del-api-marker-path/file.txt",
    ] {
        assert!(
            !rendered.contains(marker),
            "retained journal leaks {marker}"
        );
    }

    h.finish().await;
}
