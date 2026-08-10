//! These opt-in proofs use owned real Vault/Proxy/PostgreSQL fixtures. They never
//! consult VAULT_TOKEN or VAULT_ADDR, and never revoke tokens or secret leases.
use super::*;
use crate::{
    CancellationToken, Connected,
    manager::{InstanceKey, RuntimeManager, StartSpec},
};
use serde_json::{Value, json};
use sqlx::{
    Connection,
    postgres::{PgConnectOptions, PgConnection, PgSslMode},
};
use std::time::{Duration, Instant};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture() -> PathBuf {
    repository().join("target/debug/examples/mcp-fixture")
}
fn directory() -> PathBuf {
    let path = repository()
        .join(".cache/vault-adapter-proof")
        .join(Uuid::new_v4().to_string());
    std::fs::create_dir_all(&path).unwrap();
    path.canonicalize().unwrap()
}
struct Local {
    root: PathBuf,
    address: String,
    token: String,
}
impl Local {
    fn new() -> Self {
        let root = PathBuf::from(
            std::env::var_os("RECOLLECT_TEST_VAULT_LOCAL")
                .expect("Select an owned local fixture explicitly"),
        );
        let boundary: Value =
            serde_json::from_slice(&std::fs::read(root.join("boundary.json")).unwrap()).unwrap();
        assert_eq!(boundary["owner"], "recollect-local-vault-proof");
        let address = boundary["address"].as_str().unwrap().to_owned();
        assert!(address.starts_with("http://127.0.0.1:"));
        let token = std::fs::read_to_string(root.join("server-token")).unwrap();
        Self {
            root,
            address,
            token,
        }
    }
    async fn admin(&self, path: &str, body: Option<Value>) -> Value {
        assert!(!path.contains("revoke") && !path.contains("rotate"));
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!("{}/v1/{path}", self.address);
        let request = match body {
            Some(body) => client.post(url).json(&body),
            None => client.get(url),
        };
        let response = request
            .header("x-vault-token", &self.token)
            .send()
            .await
            .unwrap_or_else(|_| panic!("owned Vault request failed"));
        assert!(response.status().is_success(), "owned Vault request denied");
        let bytes = response.bytes().await.unwrap();
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| panic!("owned Vault response invalid"))
        }
    }
    async fn rotate(&self) {
        let generation = Uuid::new_v4();
        self.admin("fixture-kv/data/fixture", Some(json!({"data":{"username":format!("fixture-user-{generation}"),"password":format!("fixture-password-{generation}")}}))).await;
    }
    async fn resolver(&self, leased: bool) -> (CredentialResolver, Uuid) {
        references(
            &self.root.join("vault.sock"),
            if leased {
                "fixture-db/creds/runner"
            } else {
                "fixture-kv/data/fixture"
            },
            leased,
        )
        .await
    }
}
async fn references(
    socket: &std::path::Path,
    path: &str,
    leased: bool,
) -> (CredentialResolver, Uuid) {
    let connection = Uuid::new_v4();
    let file = directory().join("references.json");
    let binding = json!({"vault_sources":{"fixture":{"socket":socket,"path":path,"kind":if leased {"leased"}else{"kv_v2"}}},
        "bindings":[{"connection_id":connection,"runner_reference":"central","alias":"fixture","environment":{
            "FIXTURE_USER":{"provider":"vault","source":"fixture","field":"username"},
            "FIXTURE_CREDENTIAL":{"provider":"vault","source":"fixture","field":"password"}}}]});
    tokio::fs::write(&file, serde_json::to_vec(&binding).unwrap())
        .await
        .unwrap();
    (CredentialResolver::new(Some(file)), connection)
}
fn definition() -> recollect_protocol::McpDefinitionManifest {
    serde_json::from_value(json!({"key":"vault-fixture","name":"Vault fixture","description":"Owned proof","command":fixture(),"arguments":["serve"],"transport":"stdio","placements":["central"],"credential_aliases":["fixture"],
        "configuration_schema":{"type":"object","properties":{"marker":{"type":"string"}},"additionalProperties":false},
        "tools":(["inspect","slow"].map(|name|json!({"name":name,"description":"Synthetic operation","inputSchema":{"type":"object","properties":{"millis":{"type":"integer"}},"additionalProperties":false}})))})).unwrap()
}
fn key(secret: &ResolvedCredentials, connection: Uuid) -> InstanceKey {
    InstanceKey {
        brain_id: Uuid::new_v4(),
        profile_id: Uuid::new_v4(),
        connection_id: connection,
        actor_id: Uuid::new_v4(),
        device_id: None,
        client_session_id: Uuid::new_v4(),
        runner_epoch: Uuid::new_v4(),
        connection_revision: Uuid::new_v4(),
        definition_revision: "fixture".into(),
        credential_generation: secret.generation,
    }
}
async fn db_accepts(secret: &ResolvedCredentials, database: &Value) -> bool {
    let options = PgConnectOptions::new()
        .host(database["address"].as_str().unwrap())
        .port(database["port"].as_u64().unwrap() as u16)
        .database("postgres")
        .username(&secret.environment["FIXTURE_USER"])
        .password(&secret.environment["FIXTURE_CREDENTIAL"])
        .ssl_mode(PgSslMode::Disable);
    if let Ok(Ok(connection)) =
        tokio::time::timeout(Duration::from_secs(3), PgConnection::connect_with(&options)).await
    {
        connection.close().await.unwrap();
        true
    } else {
        false
    }
}

#[tokio::test]
async fn vault_is_optional_and_unused_sources_do_not_affect_other_bindings() {
    let resolver = CredentialResolver::new(Some(directory().join("missing.json")));
    assert_eq!(
        resolver
            .resolve(Uuid::new_v4(), None, "central")
            .await
            .unwrap()
            .generation,
        Uuid::nil()
    );
    let (resolver, connection) = references(
        &directory().join("not-running.sock"),
        "missing/data/fixture",
        false,
    )
    .await;
    let file = resolver.path.as_ref().unwrap();
    let mut configuration: Value =
        serde_json::from_slice(&tokio::fs::read(file).await.unwrap()).unwrap();
    // An unused Vault socket must not affect a connection with an environment
    // reference. PATH is read only as a non-secret test value, never a destination.
    configuration["bindings"][0]["environment"] =
        json!({"FIXTURE_CREDENTIAL":{"provider":"environment","name":"PATH"}});
    tokio::fs::write(file, serde_json::to_vec(&configuration).unwrap())
        .await
        .unwrap();
    assert!(
        resolver
            .resolve(connection, Some("fixture"), "central")
            .await
            .is_ok()
    );
    configuration["bindings"][0]["environment"]["FIXTURE_CREDENTIAL"]["name"] =
        json!("VAULT_TOKEN");
    tokio::fs::write(file, serde_json::to_vec(&configuration).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        resolver
            .resolve(connection, Some("fixture"), "central")
            .await,
        Err(RuntimeError("credential_reference_invalid"))
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn vault_proxy_failures_and_nonrenewable_leases_never_use_stale_credentials() {
    use axum::{
        Router,
        body::Body,
        http::{Method, StatusCode, Uri},
        response::Response,
        routing::any,
    };
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::AtomicUsize;
    let root = directory();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.join("vault.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mode = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let renewals = Arc::new(AtomicUsize::new(0));
    let current = mode.clone();
    let total = requests.clone();
    let renewed = renewals.clone();
    let app=Router::new().fallback(any(move |method:Method, uri:Uri| {
        let mode=current.load(Ordering::SeqCst); total.fetch_add(1,Ordering::SeqCst); let renewed=renewed.clone();
        async move {
            if method==Method::POST {
                assert_eq!(uri.path(),"/v1/sys/leases/renew","no other control path is permitted");
                renewed.fetch_add(1,Ordering::SeqCst);
                return Response::builder().status(StatusCode::FORBIDDEN).body(Body::from("suppressed test provider diagnostic")).unwrap();
            }
            if mode==2 {return Response::builder().status(StatusCode::FOUND).header("location","http://127.0.0.1:1/never-contact").body(Body::empty()).unwrap();}
            if mode==3 {return Response::new(Body::from("x".repeat(262145)));}
            if mode==5 {tokio::time::sleep(Duration::from_secs(4)).await;}
            if mode==8 {return Response::builder().status(StatusCode::SERVICE_UNAVAILABLE).body(Body::empty()).unwrap();}
            let fields=if mode==4 {json!({"username":42,"password":"synthetic-password"})} else {json!({"username":"synthetic-user","password":"synthetic-password"})};
            let value=if uri.path().contains("/kv/") {json!({"data":{"data":fields}})} else {json!({"data":fields,"lease_id":"synthetic/owned/lease","lease_duration":if mode==6 {86401}else{2},"renewable":mode==1})};
            Response::new(Body::from(serde_json::to_vec(&value).unwrap()))
        }
    }));
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let (resolver, connection) = references(&socket, "fixture/creds/test", true).await;
    let nonrenewable = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    let nonrenewable_operation = nonrenewable.operation(|| async {
        panic!("a nonrenewable lease cannot request authorization to renew")
    });
    mode.store(1, Ordering::SeqCst);
    let denied = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    let denied_operation = denied.operation(|| async { Ok(()) });
    tokio::time::timeout(Duration::from_secs(4), async {
        tokio::join!(nonrenewable.expired(), denied.expired());
    })
    .await
    .unwrap();
    assert!(!nonrenewable.valid() && !denied.valid());
    assert_eq!(renewals.load(Ordering::SeqCst), 1);
    assert!(!denied.leases[0].reusable());
    drop(nonrenewable_operation);
    drop(denied_operation);
    mode.store(0, Ordering::SeqCst);
    let (static_resolver, static_connection) =
        references(&socket, "fixture/kv/data/test", false).await;
    static_resolver
        .resolve(static_connection, Some("fixture"), "central")
        .await
        .unwrap();
    for (mode_value, expected) in [
        (2, "credential_vault_unavailable"),
        (3, "credential_vault_response_invalid"),
        (4, "credential_value_invalid"),
        (5, "credential_vault_unavailable"),
        (8, "credential_vault_unavailable"),
    ] {
        mode.store(mode_value, Ordering::SeqCst);
        let before = requests.load(Ordering::SeqCst);
        let result = static_resolver
            .resolve(static_connection, Some("fixture"), "central")
            .await;
        assert!(matches!(result,Err(RuntimeError(code)) if code==expected));
        assert_eq!(
            requests.load(Ordering::SeqCst),
            before + 1,
            "no retry, redirect or stale fallback"
        );
    }
    mode.store(6, Ordering::SeqCst);
    assert!(matches!(
        resolver
            .resolve(connection, Some("fixture"), "central")
            .await,
        Err(RuntimeError("credential_lease_invalid"))
    ));
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[ignore = "Requires explicitly selected owned local Vault/Proxy and built mcp-fixture"]
async fn vault_live_static_rotation_preserves_active_calls_and_redacts_coherent_fields() {
    let local = Local::new();
    local.rotate().await;
    let (resolver, connection) = local.resolver(false).await;
    let one = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    let same = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert_eq!(one.generation, same.generation);
    assert!(
        one.environment["FIXTURE_USER"].strip_prefix("fixture-user-")
            == one.environment["FIXTURE_CREDENTIAL"].strip_prefix("fixture-password-")
    );
    let manager = RuntimeManager::default();
    let definition = definition();
    let binary = fixture();
    let marker = directory().join("calls.txt");
    let configuration = json!({"marker":marker});
    let spec = StartSpec {
        definition: &definition,
        target: "vault-fixture",
        configuration: &configuration,
        supervisor: &binary,
    };
    let mut key = key(&one, connection);
    let old = manager
        .acquire(key.clone(), spec, one, |_| async { Ok(()) })
        .await
        .unwrap();
    let old_id = old.instance_id();
    let active = tokio::spawn(async move {
        old.call(
            "slow",
            json!({"millis":2000}),
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if tokio::fs::read_to_string(&marker)
                .await
                .unwrap_or_default()
                .contains("call slow")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    local.rotate().await;
    let new = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert_ne!(new.generation, key.credential_generation);
    key.credential_generation = new.generation;
    let new = manager
        .acquire(key, spec, new, |_| async { Ok(()) })
        .await
        .unwrap();
    assert_ne!(new.instance_id(), old_id);
    assert!(
        manager
            .statuses(Instant::now())
            .iter()
            .any(|s| s.id == old_id && s.state == "draining" && s.active_calls == 1)
    );
    assert!(manager.maintain(Instant::now()).await.is_empty());
    let fresh = new
        .call(
            "inspect",
            json!({}),
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await;
    for result in [active.await.unwrap(), fresh] {
        assert_eq!(result.state, "succeeded");
        let content = &result.result.unwrap()["structuredContent"];
        assert_eq!(content["echo"], "[redacted]");
        assert_eq!(content["kv_pair_coherent"], true);
        assert_eq!(content["root_token_leaked"], false);
    }
    manager.drain_all();
    assert!(
        manager
            .maintain(Instant::now())
            .await
            .into_iter()
            .all(|(_, r)| r.is_ok())
    );
}

#[tokio::test]
#[ignore = "Requires explicitly selected owned local Vault/Proxy/PostgreSQL and built mcp-fixture"]
async fn vault_live_dynamic_renewal_extends_same_credentials_and_denied_renewal_expires() {
    let local = Local::new();
    let boundary: Value =
        serde_json::from_slice(&std::fs::read(local.root.join("boundary.json")).unwrap()).unwrap();
    let database = boundary["database"].clone();
    let (resolver, connection) = local.resolver(true).await;
    let secret = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    let generation = secret.generation;
    let initial_deadline = secret.leases[0].deadline();
    assert!(
        db_accepts(&secret, &database).await,
        "issued credentials must authenticate to the real database"
    );
    let checks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = checks.clone();
    let operation = secret.operation(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        async { Ok(()) }
    });
    let connected = Connected::open(
        &definition(),
        "vault-fixture",
        &json!({}),
        secret.clone(),
        &fixture(),
    )
    .await
    .unwrap();
    let result = connected
        .call(
            "slow",
            json!({"millis":10000}),
            Duration::from_secs(16),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.state, "succeeded");
    assert!(checks.load(Ordering::SeqCst) >= 2);
    assert_eq!(generation, secret.generation);
    assert!(secret.leases[0].deadline() > initial_deadline);
    assert!(
        db_accepts(&secret, &database).await,
        "same database credentials remain valid beyond their original TTL"
    );
    connected.close().await.unwrap();
    let current_deadline = secret.leases[0].deadline();
    drop(operation);
    assert!(!secret.valid());
    tokio::time::sleep_until(current_deadline + Duration::from_secs(1)).await;
    assert!(
        !db_accepts(&secret, &database).await,
        "dropping operation stops renewal; natural TTL expiry rejects credentials"
    );
    let denied = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert_ne!(
        generation, denied.generation,
        "leases are not shared between operations"
    );
    let initial_deadline = denied.leases[0].deadline();
    let denied_operation = denied.operation(|| async { Err(RuntimeError("mcp_use_required")) });
    let connected = Connected::open(
        &definition(),
        "vault-fixture",
        &json!({}),
        denied.clone(),
        &fixture(),
    )
    .await
    .unwrap();
    let result = connected
        .call(
            "slow",
            json!({"millis":10000}),
            Duration::from_secs(16),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.state, "unknown");
    assert_eq!(result.code, "credential_expired");
    assert_eq!(
        denied.leases[0].deadline(),
        initial_deadline,
        "denied renewal never extends the lease"
    );
    assert!(!db_accepts(&denied, &database).await);
    connected.close().await.unwrap();
    drop(denied_operation);
}

#[tokio::test]
#[ignore = "Requires explicitly selected user-authorized Enterprise Proxy socket; read-only"]
async fn vault_live_enterprise_proxy_reads_coherent_kv_without_admin_token() {
    let root = PathBuf::from(
        std::env::var_os("RECOLLECT_TEST_VAULT_ENTERPRISE")
            .expect("Select the authorized namespace fixture"),
    );
    let proof: Value =
        serde_json::from_slice(&std::fs::read(root.join("verification.json")).unwrap()).unwrap();
    assert_eq!(proof["runtime_has_root_token"], false);
    let (resolver, connection) =
        references(&root.join("vault.sock"), "mcp-test-kv/data/fixture", false).await;
    let secret = resolver
        .resolve(connection, Some("fixture"), "central")
        .await
        .unwrap();
    assert!(
        secret.environment["FIXTURE_USER"].strip_prefix("fixture-user-")
            == secret.environment["FIXTURE_CREDENTIAL"].strip_prefix("fixture-password-")
    );
    let connected = Connected::open(
        &definition(),
        "enterprise-kv-fixture",
        &json!({}),
        secret,
        &fixture(),
    )
    .await
    .unwrap();
    let result = connected
        .call(
            "inspect",
            json!({}),
            Duration::from_secs(5),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.state, "succeeded");
    let content = &result.result.unwrap()["structuredContent"];
    assert_eq!(content["echo"], "[redacted]");
    assert_eq!(content["kv_pair_coherent"], true);
    assert_eq!(content["root_token_leaked"], false);
    connected.close().await.unwrap();
}
