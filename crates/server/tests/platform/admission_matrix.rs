//! Golden status matrix of Brain admission (slice `platform-brain-admission`).
//!
//! For one representative read or write endpoint per admission family, every
//! principal over both transports meets an active, an archived and a deleted
//! Brain, and the exact (HTTP status, error code) pair is asserted from a data
//! table. The table was recorded against the helpers the slice replaces and
//! stays identical through the helper move (W1). The behaviour-unification step
//! (W3) flips its cells explicitly in this table, one reviewable edit.
//!
//! Set `ADMISSION_MATRIX_RECORD=1` to print the observed table instead of
//! asserting it.
use super::*;
use std::collections::BTreeMap;

/// (HTTP status, error code); the code is `-` when the response has none.
type Cell = (u16, &'static str);
const ARCHIVED: usize = 1;
const DELETED: usize = 2;
const STATES: [&str; 3] = ["active", "archived", "deleted"];
const PRINCIPALS: [&str; 11] = [
    "admin/browser",
    "admin/device",
    "writer/browser",
    "writer/device",
    "reader/browser",
    "reader/device",
    "outsider/browser",
    "outsider/device",
    "disabled/browser",
    "disabled/device",
    "revoked/device",
];
const ENDPOINTS: [&str; 11] = [
    "mcp_read",
    "mcp_write",
    "coordination_read",
    "coordination_write",
    "skills_read",
    "skills_write",
    "workspace_write",
    "evidence_write",
    "publication_write",
    "require_role_read",
    "require_writer_write",
];

struct Caller {
    login: Option<Login>,
    token: Option<String>,
}

struct Matrix {
    callers: Vec<Caller>,
    /// Per Brain state: the coordination session each device principal owns.
    sessions: [BTreeMap<usize, Uuid>; 3],
    brains: [Uuid; 3],
}

fn request(endpoint: &str, brain: Uuid, session: Uuid, n: usize) -> (&'static str, String, Value) {
    let base = format!("/api/brains/{brain}");
    let skill = json!({"name":format!("matrix-skill-{n}"),"description":"Read scoped documentation.","kind":"skill","environment_id":null,"profile_ids":[],"tags":["matrix"],"enabled":true,"expected_version":null,
        "files":[{"path":"SKILL.md","content":format!("---\nname: matrix-skill-{n}\ndescription: >-\n  Read scoped documentation.\n---\nRead the notes.\n")}]});
    match endpoint {
        "mcp_read" => ("GET", format!("{base}/mcp"), Value::Null),
        "mcp_write" => (
            "POST",
            format!("{base}/mcp/profiles"),
            json!({"name":format!("Matrix profile {n}"),"connection_ids":[]}),
        ),
        "coordination_read" => ("GET", format!("{base}/coordination/sessions"), Value::Null),
        "coordination_write" => (
            "POST",
            format!("{base}/coordination/sessions/{session}/close"),
            Value::Null,
        ),
        "skills_read" => ("GET", format!("{base}/skills"), Value::Null),
        "skills_write" => ("POST", format!("{base}/skills"), skill),
        "workspace_write" => (
            "POST",
            format!("{base}/workspace/tasks"),
            json!({"label":format!("Matrix task {n}")}),
        ),
        "evidence_write" => (
            "POST",
            format!("{base}/evidence/groups"),
            json!({"kind":"collection","name":format!("Matrix group {n}")}),
        ),
        "publication_write" => (
            "PUT",
            format!("{base}/repositories/policy"),
            json!({"allow_file_content":false}),
        ),
        "require_role_read" => ("GET", format!("{base}/workspace/repositories"), Value::Null),
        // Admission is the first database step; an admitted caller later meets
        // the unknown source, so 403 and 409 are the cells that prove admission.
        "require_writer_write" => (
            "POST",
            format!("{base}/learning"),
            json!({"source_version_id":Uuid::new_v4(),"selection":{"repository_ids":[],"area_ids":[],"environment_id":null},"manifest_revision_id":null,"operation_id":null}),
        ),
        other => panic!("unknown endpoint {other}"),
    }
}

impl Matrix {
    async fn send(
        &self,
        h: &Harness,
        who: usize,
        state: usize,
        endpoint: &str,
        n: usize,
    ) -> (u16, String) {
        let session = self.sessions[state]
            .get(&who)
            .copied()
            .unwrap_or_else(Uuid::new_v4);
        let (method, path, body) = request(endpoint, self.brains[state], session, n);
        let caller = &self.callers[who];
        let (status, body) = match (&caller.login, &caller.token) {
            (Some(login), _) => {
                let (status, body, _) = h.call(method, &path, Some(login), body).await;
                (status, body)
            }
            (None, Some(token)) => h.bearer(method, &path, token, body).await,
            _ => unreachable!("every caller has a transport"),
        };
        (
            status.as_u16(),
            body["code"].as_str().unwrap_or("-").to_owned(),
        )
    }
}

async fn json_ok(h: &Harness, login: &Login, method: &str, path: &str, body: Value) -> Value {
    let (status, value, _) = h.call(method, path, Some(login), body).await;
    assert_eq!(status, StatusCode::OK, "{method} {path}: {value}");
    value
}

async fn announce(h: &Harness, base: &str, token: &str, repo: Uuid, path: &str) -> Uuid {
    let (status, task) = h
        .bearer(
            "POST",
            &format!("{base}/workspace/tasks"),
            token,
            json!({"label":"Matrix coordination","selection":{"repository_ids":[repo],"environment_id":null}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{task}");
    let task = &task["task"];
    let id = Uuid::new_v4();
    let (status, body) = h
        .bearer(
            "POST",
            &format!("{base}/coordination/sessions"),
            token,
            json!({"id":id,"task_id":task["id"],"expected_scope":task["scope"]["id"],"host_kind":"codex","label":"Matrix agent","goal":"Exercise Brain admission","topics":["admission"],"intents":[{"repository_id":repo,"path":path,"mode":"read","phase":"planned"}]}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    id
}

async fn build(h: &Harness) -> Matrix {
    let owner = h.login().await;
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM accounts WHERE installation_owner")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    let (_, admin_token) = h.pair_device(&owner, "matrix-admin").await;
    let (revoked_device, revoked_token) = h.pair_device(&owner, "matrix-revoked").await;
    let mut members = vec![];
    for name in ["writer", "reader", "outsider", "disabled"] {
        let (id, login) = h.fixture_member().await;
        let (_, token) = h.pair_device(&login, &format!("matrix-{name}")).await;
        members.push((name, id, login, token));
    }
    let mut brains = [Uuid::nil(); 3];
    let mut sessions: [BTreeMap<usize, Uuid>; 3] = Default::default();
    for (state, name) in STATES.into_iter().enumerate() {
        let created = json_ok(
            h,
            &owner,
            "POST",
            "/api/brains",
            json!({"name":format!("Matrix {name} Brain")}),
        )
        .await;
        let brain: Uuid = created["id"].as_str().unwrap().parse().unwrap();
        brains[state] = brain;
        let base = format!("/api/brains/{brain}");
        for (member, id, _, _) in &members {
            let role = match *member {
                "writer" => "writer",
                "reader" => "reader",
                "disabled" => "admin",
                _ => continue,
            };
            json_ok(
                h,
                &owner,
                "PUT",
                &format!("{base}/grants/{id}"),
                json!({"role":role}),
            )
            .await;
        }
        if state == DELETED {
            continue;
        }
        let repo = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repositories(id,brain_id,canonical_origin,created_by) VALUES($1,$2,$3,$4)",
        )
        .bind(repo)
        .bind(brain)
        .bind(format!("example.test/matrix/{state}"))
        .bind(owner_id)
        .execute(&h.admin)
        .await
        .unwrap();
        // One live coordination session per member device, announced while open.
        let device_principals = [
            (1usize, &admin_token),
            (3, &members[0].3),
            (5, &members[1].3),
        ];
        for (index, token) in device_principals {
            let id = announce(h, &base, token, repo, &format!("matrix/{index}.txt")).await;
            sessions[state].insert(index, id);
        }
    }
    json_ok(
        h,
        &owner,
        "PATCH",
        &format!("/api/brains/{}", brains[ARCHIVED]),
        json!({"archived":true}),
    )
    .await;
    // Delete the third Brain through the real confirmed erasure flow.
    let doomed = brains[DELETED];
    let preview = json_ok(
        h,
        &owner,
        "POST",
        &format!("/api/brains/{doomed}/deletions/preview"),
        Value::Null,
    )
    .await;
    json_ok(
        h,
        &owner,
        "DELETE",
        &format!("/api/brains/{doomed}"),
        json!({"closure":preview["closure"],"confirmation":format!("Matrix {} Brain",STATES[DELETED])}),
    )
    .await;
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM brains WHERE id=$1")
        .bind(doomed)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert_eq!(remaining, 0, "A deleted Brain has no row");
    // The last two principals lose their standing only after every fixture exists.
    sqlx::query("UPDATE accounts SET enabled=false WHERE id=$1")
        .bind(members[3].1)
        .execute(&h.admin)
        .await
        .unwrap();
    sqlx::query("UPDATE devices SET revoked_at=now() WHERE id=$1")
        .bind(revoked_device)
        .execute(&h.admin)
        .await
        .unwrap();
    let [writer, reader, outsider, disabled] = <[_; 4]>::try_from(members).ok().unwrap();
    let callers = vec![
        Caller {
            login: Some(owner),
            token: None,
        },
        Caller {
            login: None,
            token: Some(admin_token),
        },
        Caller {
            login: Some(writer.2),
            token: None,
        },
        Caller {
            login: None,
            token: Some(writer.3),
        },
        Caller {
            login: Some(reader.2),
            token: None,
        },
        Caller {
            login: None,
            token: Some(reader.3),
        },
        Caller {
            login: Some(outsider.2),
            token: None,
        },
        Caller {
            login: None,
            token: Some(outsider.3),
        },
        Caller {
            login: Some(disabled.2),
            token: None,
        },
        Caller {
            login: None,
            token: Some(disabled.3),
        },
        Caller {
            login: None,
            token: Some(revoked_token),
        },
    ];
    Matrix {
        callers,
        sessions,
        brains,
    }
}

/// Recorded against the helpers this slice replaces, one row per endpoint and
/// principal, one cell per Brain state in the order active, archived, deleted.
#[rustfmt::skip]
const GOLDEN: &[(&str, [[Cell; 3]; 11])] = &[
    ("mcp_read", [
        [(200, "-"), (200, "-"), (404, "not_found")], // admin/browser
        [(200, "-"), (200, "-"), (404, "not_found")], // admin/device
        [(200, "-"), (200, "-"), (404, "not_found")], // writer/browser
        [(200, "-"), (200, "-"), (404, "not_found")], // writer/device
        [(200, "-"), (200, "-"), (404, "not_found")], // reader/browser
        [(200, "-"), (200, "-"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("mcp_write", [
        [(200, "-"), (409, "mcp_configuration_conflict"), (404, "not_found")], // admin/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // admin/device
        [(403, "forbidden"), (409, "mcp_configuration_conflict"), (404, "not_found")], // writer/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // writer/device
        [(403, "forbidden"), (409, "mcp_configuration_conflict"), (404, "not_found")], // reader/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("coordination_read", [
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // admin/browser
        [(200, "-"), (409, "coordination_conflict"), (404, "not_found")], // admin/device
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // writer/browser
        [(200, "-"), (409, "coordination_conflict"), (404, "not_found")], // writer/device
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // reader/browser
        [(200, "-"), (409, "coordination_conflict"), (404, "not_found")], // reader/device
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("coordination_write", [
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // admin/browser
        [(200, "-"), (409, "coordination_conflict"), (404, "not_found")], // admin/device
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // writer/browser
        [(200, "-"), (409, "coordination_conflict"), (404, "not_found")], // writer/device
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // reader/browser
        [(200, "-"), (409, "coordination_conflict"), (404, "not_found")], // reader/device
        [(403, "forbidden"), (403, "forbidden"), (403, "forbidden")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("skills_read", [
        [(200, "-"), (200, "-"), (404, "not_found")], // admin/browser
        [(403, "forbidden"), (409, "brain_archived"), (404, "not_found")], // admin/device
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // writer/browser
        [(403, "forbidden"), (409, "brain_archived"), (404, "not_found")], // writer/device
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // reader/browser
        [(403, "forbidden"), (409, "brain_archived"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("skills_write", [
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // admin/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // admin/device
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // writer/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // writer/device
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(403, "browser_required"), (403, "browser_required"), (403, "browser_required")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("workspace_write", [
        [(200, "-"), (409, "workspace_conflict"), (404, "not_found")], // admin/browser
        [(200, "-"), (409, "workspace_conflict"), (404, "not_found")], // admin/device
        [(200, "-"), (409, "workspace_conflict"), (404, "not_found")], // writer/browser
        [(200, "-"), (409, "workspace_conflict"), (404, "not_found")], // writer/device
        [(200, "-"), (409, "workspace_conflict"), (404, "not_found")], // reader/browser
        [(200, "-"), (409, "workspace_conflict"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("evidence_write", [
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // admin/browser
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // admin/device
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // writer/browser
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // writer/device
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/browser
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("publication_write", [
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // admin/browser
        [(200, "-"), (409, "brain_archived"), (404, "not_found")], // admin/device
        [(403, "forbidden"), (409, "brain_archived"), (404, "not_found")], // writer/browser
        [(403, "forbidden"), (409, "brain_archived"), (404, "not_found")], // writer/device
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/browser
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("require_role_read", [
        [(200, "-"), (200, "-"), (404, "not_found")], // admin/browser
        [(200, "-"), (200, "-"), (404, "not_found")], // admin/device
        [(200, "-"), (200, "-"), (404, "not_found")], // writer/browser
        [(200, "-"), (200, "-"), (404, "not_found")], // writer/device
        [(200, "-"), (200, "-"), (404, "not_found")], // reader/browser
        [(200, "-"), (200, "-"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
    ("require_writer_write", [
        [(404, "not_found"), (409, "brain_archived"), (404, "not_found")], // admin/browser
        [(404, "not_found"), (409, "brain_archived"), (404, "not_found")], // admin/device
        [(404, "not_found"), (409, "brain_archived"), (404, "not_found")], // writer/browser
        [(404, "not_found"), (409, "brain_archived"), (404, "not_found")], // writer/device
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/browser
        [(403, "forbidden"), (403, "forbidden"), (404, "not_found")], // reader/device
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/browser
        [(404, "not_found"), (404, "not_found"), (404, "not_found")], // outsider/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/browser
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // disabled/device
        [(401, "unauthorized"), (401, "unauthorized"), (401, "unauthorized")], // revoked/device
    ]),
];

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn admission_matrix_status_and_code_by_principal_transport_and_brain_state() {
    let h = Harness::new().await;
    let matrix = build(&h).await;
    let mut observed: Vec<(&str, [[Cell; 3]; 11])> = vec![];
    let mut n = 0;
    for endpoint in ENDPOINTS {
        let mut rows = [[(0u16, ""); 3]; 11];
        for (who, row) in rows.iter_mut().enumerate() {
            for (state, cell) in row.iter_mut().enumerate() {
                n += 1;
                let (status, code) = matrix.send(&h, who, state, endpoint, n).await;
                // The table owns `'static` codes; leaking a short string is fine in a test.
                *cell = (status, Box::leak(code.into_boxed_str()));
            }
        }
        observed.push((endpoint, rows));
    }
    if std::env::var_os("ADMISSION_MATRIX_RECORD").is_some() {
        eprintln!("ADMISSION_MATRIX_BEGIN");
        for (endpoint, rows) in &observed {
            eprintln!("    (\"{endpoint}\", [");
            for (who, cells) in rows.iter().enumerate() {
                let cells: Vec<String> = cells
                    .iter()
                    .map(|(status, code)| format!("({status}, \"{code}\")"))
                    .collect();
                eprintln!("        [{}], // {}", cells.join(", "), PRINCIPALS[who]);
            }
            eprintln!("    ]),");
        }
        eprintln!("ADMISSION_MATRIX_END");
        h.finish().await;
        return;
    }
    assert_eq!(GOLDEN.len(), ENDPOINTS.len(), "Record the golden table");
    let mut mismatches = vec![];
    for ((endpoint, expected), (_, actual)) in GOLDEN.iter().zip(&observed) {
        for who in 0..PRINCIPALS.len() {
            for state in 0..STATES.len() {
                if expected[who][state] != actual[who][state] {
                    mismatches.push(format!(
                        "{endpoint} {} on a {} Brain: expected {:?}, observed {:?}",
                        PRINCIPALS[who], STATES[state], expected[who][state], actual[who][state],
                    ));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    h.finish().await;
}
