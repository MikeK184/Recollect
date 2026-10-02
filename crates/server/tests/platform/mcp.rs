use super::*;
use recollect_protocol::McpDefinitionManifest;
use recollect_server::mcp::definitions;
#[path = "mcp/host_tools.rs"]
mod host_tools;
#[path = "mcp/plugin_session.rs"]
mod plugin_session;
#[path = "mcp/runtime.rs"]
mod runtime;
#[path = "mcp/tools.rs"]
pub(crate) mod tools;
#[path = "mcp/tools_policy.rs"]
mod tools_policy;

fn manifest() -> McpDefinitionManifest {
    serde_json::from_value(json!({
        "key":"fixture","name":"Approved fixture","description":"Synthetic cached catalogue",
        "transport":"stdio","command":"/usr/bin/false","arguments":[],
        "placements":["central","local","private"],"credential_aliases":["fixture-read"],
        "configuration_schema":{"type":"object","properties":{"region":{"type":"string","enum":["test"]}},"required":["region"],"additionalProperties":false},
        "tools":[{"name":"inspect","description":"Read synthetic fixture","inputSchema":{"type":"object","properties":{"name":{"type":"string"}},"additionalProperties":false},"outputSchema":{"type":"array","items":{"type":"string"}},"annotations":{"readOnlyHint":true}}]
    })).unwrap()
}
fn connection_body(environment: Option<Uuid>) -> Value {
    json!({"name":"Fixture connection","description":"Synthetic target","definition_key":"fixture","target":"fixture-target","placement":"central","runner_reference":null,"credential_alias":"fixture-read","environment_id":environment,"configuration":{"region":"test"},"enabled":true,"base_revision":null})
}
fn profile_body(connection: &Value, environment: Option<Uuid>) -> Value {
    json!({"name":"Fixture profile","description":"Configured only","environment_id":environment,"connection_ids":[connection["summary"]["id"]],"enabled":true,"base_revision":null})
}
fn rights(use_profile: bool, manage: bool, share: bool) -> Value {
    json!({"use_profile":use_profile,"manage":manage,"share":share})
}
async fn ok(h: &Harness, login: &Login, method: &str, path: &str, body: Value) -> Value {
    let (status, result, _) = h.call(method, path, Some(login), body).await;
    assert_eq!(status, StatusCode::OK, "{path}: {result}");
    result
}
async fn setup() -> (Harness, Login, Uuid, Value, Value) {
    let h = Harness::new().await;
    let owner = h.login().await;
    definitions::import_manifest(&h.admin, manifest())
        .await
        .unwrap();
    let brain = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"MCP fixture"}),
    )
    .await;
    let brain: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    let base = format!("/api/brains/{brain}/mcp");
    let connection = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/connections"),
        connection_body(None),
    )
    .await;
    let profile = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/profiles"),
        profile_body(&connection, None),
    )
    .await;
    (h, owner, brain, connection, profile)
}
async fn username(h: &Harness, id: Uuid) -> String {
    sqlx::query_scalar("SELECT username FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}
async fn reader(h: &Harness, owner: &Login, brain: Uuid, id: Uuid) {
    ok(
        h,
        owner,
        "PUT",
        &format!("/api/brains/{brain}/grants/{id}"),
        json!({"role":"reader"}),
    )
    .await;
}
async fn grant(
    h: &Harness,
    owner: &Login,
    brain: Uuid,
    profile: &Value,
    name: &str,
    powers: Value,
) -> Value {
    ok(
        h,
        owner,
        "PUT",
        &format!(
            "/api/brains/{brain}/mcp/profiles/{}/grants",
            profile["profile"]["id"].as_str().unwrap()
        ),
        json!({"username":name,"group_name":null,"rights":powers}),
    )
    .await
}
fn discover_body(profile: &Value, environment: Option<Uuid>) -> Value {
    json!({"profile_id":profile["profile"]["id"],"environment_id":environment,"operation_id":null,"offset":0})
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_independent_profile_powers_admin_no_use_and_foreign_rls() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp");
    let id = profile["profile"]["id"].as_str().unwrap();
    assert_eq!(profile["profile"]["rights"], rights(false, true, true));
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&owner),
            discover_body(&profile, None)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (user, user_login) = h.fixture_member().await;
    reader(&h, &owner, brain, user).await;
    let name = username(&h, user).await;
    let empty = ok(&h, &user_login, "GET", &base, Value::Null).await;
    assert_eq!(empty["profiles"], json!([]));
    assert_eq!(empty["connections"], json!([]));
    // Managing a profile neither authorizes execution nor grants sharing.
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(false, true, false),
    )
    .await;
    let mut edit = profile_body(&connection, None);
    edit["name"] = json!("Managed profile");
    edit["base_revision"] = profile["profile"]["revision"].clone();
    let profile = ok(
        &h,
        &user_login,
        "PUT",
        &format!("{base}/profiles/{id}"),
        edit.clone(),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&user_login),
            discover_body(&profile, None)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/profiles/{id}/grants"),
            Some(&user_login),
            json!({"username":name,"rights":rights(true,true,true)})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call(
            "GET",
            &format!(
                "{base}/connections/{}",
                connection["summary"]["id"].as_str().unwrap()
            ),
            Some(&user_login),
            Value::Null
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(true, false, false),
    )
    .await;
    let discovered = ok(
        &h,
        &user_login,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    assert_eq!(discovered["total"], 1);
    assert_eq!(discovered["execution_state"], "not_connected");
    assert_eq!(
        discovered["tools"][0]["tool"]["outputSchema"]["type"],
        "array"
    );
    let body = discovered.to_string();
    for excluded in [
        "/usr/bin/false",
        "fixture-target",
        "fixture-read",
        "runner_reference",
        "configuration",
    ] {
        assert!(
            !body.contains(excluded),
            "Discovery must omit configuration routing"
        );
    }
    edit["base_revision"] = profile["profile"]["revision"].clone();
    assert_eq!(
        h.call(
            "PUT",
            &format!("{base}/profiles/{id}"),
            Some(&user_login),
            edit
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(false, false, true),
    )
    .await;
    let share_only = ok(
        &h,
        &user_login,
        "GET",
        &format!("{base}/profiles/{id}"),
        Value::Null,
    )
    .await;
    assert_eq!(share_only["profile"]["rights"], rights(false, false, true));
    let changed = grant(
        &h,
        &user_login,
        brain,
        &profile,
        &name,
        rights(true, false, false),
    )
    .await;
    assert_eq!(changed["rights"], rights(true, false, false));
    ok(
        &h,
        &user_login,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    // No knowledge role survives removal just because a profile grant remains.
    ok(
        &h,
        &owner,
        "DELETE",
        &format!("/api/brains/{brain}/grants/{user}"),
        Value::Null,
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&user_login),
            discover_body(&profile, None)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut tx = db::actor_tx(&h.state.pool, user)
        .await
        .unwrap_or_else(|e| panic!("{}", e.2));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM mcp_profiles WHERE brain_id=$1")
        .bind(brain)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let can_use: bool = sqlx::query_scalar("SELECT recollect_mcp_can($1,'use')")
        .bind(id.parse::<Uuid>().unwrap())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(!can_use);
    tx.commit().await.unwrap();
    // The permitted owner control still has management access and can explicitly grant use.
    let owner_name = h.state.config.owner_username.clone();
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &owner_name,
        rights(true, false, false),
    )
    .await;
    ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_schema_approval_cold_discovery_pagination_and_reconfiguration() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp");
    let owner_name = h.state.config.owner_username.clone();
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &owner_name,
        rights(true, false, false),
    )
    .await;
    let marker = std::path::PathBuf::from(format!(".cache/{}-mcp-started", h.database));
    let executable = std::path::PathBuf::from(format!(".cache/{}-mcp-sentinel", h.database));
    std::fs::create_dir_all(".cache").unwrap();
    std::fs::write(
        &executable,
        format!(
            "#!/bin/sh\nprintf started > '{}'\n",
            std::env::current_dir().unwrap().join(&marker).display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut def = manifest();
    def.command = Some(
        std::fs::canonicalize(&executable)
            .unwrap()
            .to_string_lossy()
            .into(),
    );
    let tool = def.tools[0].clone();
    def.tools = (0..23)
        .map(|i| {
            let mut t = tool.clone();
            t.name = format!("inspect_{i:02}");
            t
        })
        .collect();
    def.tools[0].name = "inspect".into();
    definitions::import_manifest(&h.admin, def.clone())
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut remote = manifest();
    remote.key = "remote-fixture".into();
    remote.transport = "streamable_http".into();
    remote.command = None;
    definitions::import_manifest(&h.admin, remote.clone())
        .await
        .unwrap();
    let mut body = connection_body(None);
    body["name"] = json!("Remote fixture");
    body["definition_key"] = json!("remote-fixture");
    body["target"] = json!(format!("http://{address}/mcp"));
    let remote_connection = ok(&h, &owner, "POST", &format!("{base}/connections"), body).await;
    let mut edit = profile_body(&connection, None);
    edit["base_revision"] = profile["profile"]["revision"].clone();
    edit["connection_ids"] = json!([
        connection["summary"]["id"],
        remote_connection["summary"]["id"]
    ]);
    let profile = ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "{base}/profiles/{}",
            profile["profile"]["id"].as_str().unwrap()
        ),
        edit,
    )
    .await;
    let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let first = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    assert_eq!(first["total"], 24);
    assert_eq!(first["tools"].as_array().unwrap().len(), 20);
    assert_eq!(first["next_offset"], 20);
    let repeat = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    assert_eq!(first, repeat);
    let mut page = discover_body(&profile, None);
    page["offset"] = json!(20);
    let second = ok(&h, &owner, "POST", &format!("{base}/discover"), page).await;
    assert_eq!(second["tools"].as_array().unwrap().len(), 4);
    let colliding: Vec<_> = first["tools"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["tools"].as_array().unwrap())
        .filter(|t| t["tool"]["name"] == "inspect")
        .map(|t| t["connection_id"].clone())
        .collect();
    assert_eq!(colliding.len(), 2);
    assert_ne!(colliding[0], colliding[1]);
    assert!(!marker.exists());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(80), listener.accept())
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        jobs
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM model_requests")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        0
    );
    // No remote/file schema fetches or credential-bearing configuration are accepted.
    let mut bad = def.clone();
    bad.tools[0].input_schema = json!({"type":"object","$ref":format!("http://{address}/schema")});
    assert!(definitions::import_manifest(&h.admin, bad).await.is_err());
    let mut bad = def.clone();
    bad.configuration_schema = json!({"type":"invalid","additionalProperties":false});
    assert!(definitions::import_manifest(&h.admin, bad).await.is_err());
    let mut body = connection_body(None);
    body["name"] = json!("Denied inline secret");
    body["configuration"]["token"] = json!("synthetic-not-a-credential");
    assert_eq!(
        h.call("POST", &format!("{base}/connections"), Some(&owner), body)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let mut override_args = discover_body(&profile, None);
    override_args["command"] = json!("unapproved");
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&owner),
            override_args
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    def.configuration_schema["properties"]["region"]["enum"] = json!(["replacement"]);
    definitions::import_manifest(&h.admin, def).await.unwrap();
    let partial = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    assert_eq!(partial["total"], 1);
    assert_eq!(
        partial["unavailable_connections"][0]["reason"],
        "configuration_invalid"
    );
    let mut stop = connection_body(None);
    stop["enabled"] = json!(false);
    stop["base_revision"] = connection["summary"]["revision"].clone();
    let paused = ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "{base}/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        stop.clone(),
    )
    .await;
    assert_eq!(paused["summary"]["availability"], "disabled");
    let mut invalid_reenable = stop.clone();
    invalid_reenable["enabled"] = json!(true);
    invalid_reenable["base_revision"] = paused["summary"]["revision"].clone();
    assert_eq!(
        h.call(
            "PUT",
            &format!(
                "{base}/connections/{}",
                connection["summary"]["id"].as_str().unwrap()
            ),
            Some(&owner),
            invalid_reenable.clone()
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    definitions::disable(&h.admin, "remote-fixture")
        .await
        .unwrap();
    let none = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    assert_eq!(none["total"], 0);
    assert_eq!(none["unavailable_connections"].as_array().unwrap().len(), 2);
    definitions::import_manifest(&h.admin, manifest())
        .await
        .unwrap();
    ok(
        &h,
        &owner,
        "PUT",
        &format!(
            "{base}/connections/{}",
            connection["summary"]["id"].as_str().unwrap()
        ),
        invalid_reenable,
    )
    .await;
    let audit: i64 =
        sqlx::query_scalar("SELECT count(*) FROM mutation_audit WHERE action='mcp.definition'")
            .fetch_one(&h.admin)
            .await
            .unwrap();
    definitions::import_manifest(&h.admin, manifest())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mutation_audit WHERE action='mcp.definition'"
        )
        .fetch_one(&h.admin)
        .await
        .unwrap(),
        audit
    );
    assert_eq!(
        ok(
            &h,
            &owner,
            "POST",
            &format!("{base}/discover"),
            discover_body(&profile, None)
        )
        .await["total"],
        1
    );
    std::fs::remove_file(executable).unwrap();
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_environment_operation_device_and_archived_boundaries() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp");
    let env = ok(
        &h,
        &owner,
        "POST",
        &format!("/api/brains/{brain}/evidence/groups"),
        json!({"kind":"environment","name":"Production fixture"}),
    )
    .await;
    let env: Uuid = env["id"].as_str().unwrap().parse().unwrap();
    let mut body = connection_body(Some(env));
    body["name"] = json!("Environment target");
    let scoped = ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/connections"),
        body.clone(),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/profiles"),
            Some(&owner),
            profile_body(&scoped, None)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut pbody = profile_body(&scoped, Some(env));
    pbody["name"] = json!("Production tools");
    let scoped_profile = ok(&h, &owner, "POST", &format!("{base}/profiles"), pbody).await;
    let owner_name = h.state.config.owner_username.clone();
    grant(
        &h,
        &owner,
        brain,
        &scoped_profile,
        &owner_name,
        rights(true, false, false),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&owner),
            discover_body(&scoped_profile, None)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    ok(
        &h,
        &owner,
        "POST",
        &format!("{base}/discover"),
        discover_body(&scoped_profile, Some(env)),
    )
    .await;
    assert_eq!(
        h.call(
            "DELETE",
            &format!("/api/brains/{brain}/evidence/groups/{env}"),
            Some(&owner),
            Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut change = connection_body(Some(env));
    change["base_revision"] = connection["summary"]["revision"].clone();
    assert_eq!(
        h.call(
            "PUT",
            &format!(
                "{base}/connections/{}",
                connection["summary"]["id"].as_str().unwrap()
            ),
            Some(&owner),
            change
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (first, token) = h.pair_device(&owner, "MCP caller").await;
    let (_, other_token) = h.pair_device(&owner, "Other MCP caller").await;
    let (status,task)=h.bearer("POST",&format!("/api/brains/{brain}/workspace/tasks"),&token,json!({"label":"MCP immutable task","selection":{"repository_ids":[],"area_ids":[],"environment_id":env}})).await;
    assert_eq!(status, StatusCode::OK, "{task}");
    let task_id = task["task"]["id"].as_str().unwrap();
    let (status, operation) = h
        .bearer(
            "POST",
            &format!("/api/brains/{brain}/workspace/tasks/{task_id}/operations"),
            &token,
            json!({"kind":"tool"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let mut discover = discover_body(&scoped_profile, Some(env));
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/discover"),
            &token,
            discover.clone()
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    discover["operation_id"] = operation["id"].clone();
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/discover"),
            &token,
            discover.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/discover"),
            &other_token,
            discover.clone()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (status,_)=h.bearer("PUT",&format!("/api/brains/{brain}/workspace/tasks/{task_id}/scope"),&token,json!({"base_scope":task["task"]["scope"]["id"],"selection":{"repository_ids":[],"area_ids":[],"environment_id":null}})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/discover"),
            &token,
            discover.clone()
        )
        .await
        .0,
        StatusCode::OK,
        "Old operations retain their original environment"
    );
    assert_eq!(
        h.bearer(
            "POST",
            &format!("{base}/connections"),
            &token,
            connection_body(None)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, _, _) = h
        .call(
            "DELETE",
            &format!("/api/devices/{first}"),
            Some(&owner),
            Value::Null,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        h.bearer("POST", &format!("{base}/discover"), &token, discover)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    // Known foreign profile IDs remain unavailable under an independently accessible Brain.
    let another = ok(
        &h,
        &owner,
        "POST",
        "/api/brains",
        json!({"name":"Other MCP Brain"}),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!(
                "/api/brains/{}/mcp/discover",
                another["id"].as_str().unwrap()
            ),
            Some(&owner),
            discover_body(&profile, None)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let _ = ok(
        &h,
        &owner,
        "PATCH",
        &format!("/api/brains/{brain}"),
        json!({"archived":true}),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&owner),
            discover_body(&scoped_profile, Some(env))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    ok(&h, &owner, "GET", &base, Value::Null).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_group_expiry_direct_overlap_and_last_share_revocation() {
    let (mut h, owner, brain, _, profile) = setup().await;
    let issuer = "https://identity.example.invalid";
    let mut config = (*h.state.config).clone();
    config.oidc = Some(recollect_server::config::OidcConfig {
        issuer: issuer.into(),
        client_id: "synthetic-catalogue".into(),
        client_secret: Uuid::new_v4().to_string(),
        scopes: vec![],
    });
    h.state.config = std::sync::Arc::new(config);
    h.router = app(h.state.clone());
    let (user, login) = h.fixture_member().await;
    reader(&h, &owner, brain, user).await;
    let name = username(&h, user).await;
    sqlx::query("UPDATE accounts SET auth_kind='oidc',oidc_issuer=$2,oidc_subject=$3,oidc_groups=ARRAY['ops'],membership_until=now()+interval '5 minutes' WHERE id=$1")
        .bind(user).bind(issuer).bind(Uuid::new_v4().to_string()).execute(&h.admin).await.unwrap();
    let base = format!("/api/brains/{brain}/mcp");
    let path = format!(
        "{base}/profiles/{}",
        profile["profile"]["id"].as_str().unwrap()
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(false, true, false),
    )
    .await;
    let change = ok(
        &h,
        &owner,
        "PUT",
        &format!("{path}/grants"),
        json!({"group_name":"ops","rights":rights(true,false,false)}),
    )
    .await;
    let group_id = change["profile"]["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["group_name"] == "ops")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        ok(&h, &login, "GET", &path, Value::Null).await["profile"]["rights"],
        rights(true, true, false)
    );
    ok(
        &h,
        &login,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    let removed = ok(
        &h,
        &owner,
        "DELETE",
        &format!("{path}/grants/{group_id}"),
        Value::Null,
    )
    .await;
    let member = removed["profile"]["effective_members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["account_id"] == user.to_string())
        .unwrap();
    assert_eq!(member["rights"], rights(false, true, false));
    assert_eq!(member["direct_rights"], rights(false, true, false));
    ok(
        &h,
        &owner,
        "PUT",
        &format!("{path}/grants"),
        json!({"group_name":"ops","rights":rights(true,false,false)}),
    )
    .await;
    sqlx::query("UPDATE accounts SET membership_until=now()-interval '1 second' WHERE id=$1")
        .bind(user)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&login),
            discover_body(&profile, None)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        ok(&h, &login, "GET", &path, Value::Null).await["profile"]["rights"],
        rights(false, true, false)
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(true, true, false),
    )
    .await;
    ok(
        &h,
        &login,
        "POST",
        &format!("{base}/discover"),
        discover_body(&profile, None),
    )
    .await;
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(false, true, false),
    )
    .await;
    sqlx::query("UPDATE accounts SET membership_until=now()+interval '5 minutes',oidc_issuer='https://other.example.invalid' WHERE id=$1").bind(user).execute(&h.admin).await.unwrap();
    assert_eq!(
        ok(&h, &login, "GET", &path, Value::Null).await["profile"]["rights"],
        rights(false, true, false)
    );
    sqlx::query("UPDATE accounts SET oidc_issuer=$2,oidc_groups='{}' WHERE id=$1")
        .bind(user)
        .bind(issuer)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        ok(&h, &login, "GET", &path, Value::Null).await["profile"]["rights"],
        rights(false, true, false)
    );
    // A delegated sharer can remove their last right without rolling the mutation back.
    let changed = grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(false, false, true),
    )
    .await;
    let direct_id = changed["profile"]["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["account_id"] == user.to_string())
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let final_rights = ok(
        &h,
        &login,
        "DELETE",
        &format!("{path}/grants/{direct_id}"),
        Value::Null,
    )
    .await;
    assert_eq!(final_rights["rights"], rights(false, false, false));
    assert!(final_rights["profile"].is_null());
    assert_eq!(
        h.call("GET", &path, Some(&login), Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    grant(
        &h,
        &owner,
        brain,
        &profile,
        &name,
        rights(true, false, false),
    )
    .await;
    ok(
        &h,
        &owner,
        "PATCH",
        &format!("/api/team/accounts/{user}"),
        json!({"enabled":false}),
    )
    .await;
    assert_eq!(
        h.call(
            "POST",
            &format!("{base}/discover"),
            Some(&login),
            discover_body(&profile, None)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    ok(&h, &owner, "GET", &base, Value::Null).await;
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn mcp_concurrent_creation_stale_edits_capacity_and_atomic_audit() {
    let (h, owner, brain, connection, profile) = setup().await;
    let base = format!("/api/brains/{brain}/mcp");
    let mut create = connection_body(None);
    create["name"] = json!("Concurrent connection");
    let path = format!("{base}/connections");
    let (left, right) = tokio::join!(
        h.keyed(
            "POST",
            &path,
            Some(&owner),
            create.clone(),
            Some("mcp-concurrent")
        ),
        h.keyed(
            "POST",
            &path,
            Some(&owner),
            create.clone(),
            Some("mcp-concurrent")
        )
    );
    assert_eq!(left.0, StatusCode::OK, "{}", left.1);
    assert_eq!(right.0, StatusCode::OK);
    assert_eq!(left.1, right.1);
    let id = left.1["summary"]["id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .unwrap();
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE action='mcp.connection' AND target_id=$1",
    )
    .bind(id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(audit, 1);
    let mut a = create.clone();
    a["base_revision"] = left.1["summary"]["revision"].clone();
    a["name"] = json!("Editor A");
    let mut b = a.clone();
    b["name"] = json!("Editor B");
    let path = format!("{base}/connections/{id}");
    let (a, b) = tokio::join!(
        h.call("PUT", &path, Some(&owner), a),
        h.call("PUT", &path, Some(&owner), b)
    );
    assert!(
        (a.0 == StatusCode::OK && b.0 == StatusCode::CONFLICT)
            || (b.0 == StatusCode::OK && a.0 == StatusCode::CONFLICT)
    );
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE action='mcp.connection' AND target_id=$1",
    )
    .bind(id)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(audit, 2);
    create["name"] = json!("Changed replay");
    assert_eq!(
        h.keyed(
            "POST",
            &format!("{base}/connections"),
            Some(&owner),
            create,
            Some("mcp-concurrent")
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // Real API capacity checks with distinct persisted records, preserving updates below the same cap.
    sqlx::query("INSERT INTO mcp_connections(id,brain_id,name,description,definition_key,target,placement,configuration,revision,created_by) SELECT gen_random_uuid(),brain_id,'capacity-'||n,'','fixture','fixture-target','central',configuration,gen_random_uuid(),created_by FROM mcp_connections CROSS JOIN generate_series(1,98) n WHERE id=$1")
        .bind(connection["summary"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    let mut body = connection_body(None);
    body["name"] = json!("Over connection capacity");
    assert_eq!(
        h.call("POST", &format!("{base}/connections"), Some(&owner), body)
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    sqlx::query("INSERT INTO mcp_profiles(id,brain_id,name,revision,created_by) SELECT gen_random_uuid(),brain_id,'capacity-'||n,gen_random_uuid(),created_by FROM mcp_profiles CROSS JOIN generate_series(1,99) n WHERE id=$1")
        .bind(profile["profile"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    let mut body = profile_body(&connection, None);
    body["name"] = json!("Over profile capacity");
    assert_eq!(
        h.call("POST", &format!("{base}/profiles"), Some(&owner), body)
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    sqlx::query("INSERT INTO mcp_profile_grants(id,brain_id,profile_id,issuer,group_name,can_use,can_manage,can_share) SELECT gen_random_uuid(),brain_id,id,'https://identity.example.invalid','capacity-'||n,true,false,false FROM mcp_profiles CROSS JOIN generate_series(1,100) n WHERE id=$1")
        .bind(profile["profile"]["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&h.admin).await.unwrap();
    assert_eq!(
        h.call(
            "PUT",
            &format!(
                "{base}/profiles/{}/grants",
                profile["profile"]["id"].as_str().unwrap()
            ),
            Some(&owner),
            json!({"username":h.state.config.owner_username,"rights":rights(true,false,false)})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    sqlx::query("INSERT INTO mcp_definitions(key,id,manifest,approved_by) SELECT 'capacity-'||n,gen_random_uuid(),jsonb_set(manifest,'{key}',to_jsonb('capacity-'||n)),approved_by FROM mcp_definitions CROSS JOIN generate_series(1,99) n WHERE key='fixture'").execute(&h.admin).await.unwrap();
    let mut over = manifest();
    over.key = "overflow".into();
    assert!(definitions::import_manifest(&h.admin, over).await.is_err());
    let mut existing = manifest();
    existing.description = "Updated at capacity".into();
    definitions::import_manifest(&h.admin, existing)
        .await
        .unwrap();
    assert_eq!(
        ok(&h, &owner, "GET", &base, Value::Null).await["profiles"]
            .as_array()
            .unwrap()
            .len(),
        100
    );
    h.finish().await;
}

#[path = "mcp/plugin_hosts.rs"]
mod plugin_hosts;
