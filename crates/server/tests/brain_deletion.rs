//! Brain-wide deletion integration test (ADR 0016, contract
//! docs/contracts/platform-brain-deletion.md). It runs the exposed SQL
//! functions directly against a disposable repository-owned database: no HTTP
//! layer, no provider calls. It proves full closure of a rich Brain by one
//! delete, that other Brains are untouched, that retained rows are
//! content-free, counter agreement, distinct rejection codes before any write,
//! tombstone RLS, idempotent repeat, and that per-record erasure is unchanged.
use recollect_server::db;
use serde_json::Value;
use sqlx::{Error as SqlxError, PgPool};
use std::collections::BTreeMap;
use uuid::Uuid;

// Content markers seeded into Brain A. None of them may appear in any row that
// survives the deletion (tombstones, journal, receipts, audit).
const MT_TITLE: &str = "brain-del-marker-title-9f21";
const MT_URI: &str = "https://marker.example/alpha/doc";
const MT_PATH: &str = "/tmp/brain-del-marker-path/file.txt";
const MT_CLAIM: &str = "brain-del-marker-claim-value";
const MT_CHUNK: &str = "brain-del-marker-chunk-content";
const MT_RULE: &str = "brain-del-marker-rule-text";
const MT_RECEIPT: &str = "brain-del-marker-receipt-payload";
const MT_MANIFEST: &str = "brain-del-marker-manifest-name";
const MARKERS: &[&str] = &[
    MT_TITLE,
    MT_URI,
    MT_PATH,
    MT_CLAIM,
    MT_CHUNK,
    MT_RULE,
    MT_RECEIPT,
    MT_MANIFEST,
];

struct Env {
    root: PgPool,
    admin: PgPool,
    app: PgPool,
    database: String,
}

impl Env {
    async fn new() -> (Self, Uuid) {
        let admin_url = std::env::var("DATABASE_ADMIN_URL").expect("DATABASE_ADMIN_URL required");
        let app_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let root = db::pool(&admin_url).await.unwrap();
        let database = format!("recollect_test_brain_del_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {database}"))
            .execute(&root)
            .await
            .unwrap();
        eprintln!("Disposable Brain deletion test database: {database}");
        sqlx::query(&format!("COMMENT ON DATABASE {database} IS 'Recollect disposable Brain deletion test created by crates/server/tests/brain_deletion.rs'"))
            .execute(&root)
            .await
            .unwrap();
        let mut url = reqwest::Url::parse(&admin_url).unwrap();
        url.set_path(&database);
        let admin = db::pool(url.as_str()).await.unwrap();
        // Applying every migration here also re-proves the fresh 001..029 path
        // on each run; 029 is last, so this is simultaneously the 028->029
        // upgrade proof.
        db::migrate(&admin).await.unwrap();
        let owner = db::bootstrap(&admin, "owner").await.unwrap();
        let mut url = reqwest::Url::parse(&app_url).unwrap();
        url.set_path(&database);
        let app = db::pool(url.as_str()).await.unwrap();
        (
            Self {
                root,
                admin,
                app,
                database,
            },
            owner,
        )
    }
    async fn finish(self) {
        // These identities come only from this harness's UUID-owned database.
        // Failed fixtures do not reach finish and remain available for diagnosis.
        self.admin.close().await;
        self.app.close().await;
        sqlx::query(&format!("DROP DATABASE {} WITH (FORCE)", self.database))
            .execute(&self.root)
            .await
            .unwrap();
        self.root.close().await;
    }
}

fn code(e: &SqlxError) -> Option<String> {
    match e {
        SqlxError::Database(d) => d.code().map(|c| c.to_string()),
        _ => None,
    }
}

struct Fixture {
    a: Uuid,
    b: Uuid,
    member: Uuid,
    dev: Uuid,
    // Brain A content
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
    // Brain B content (control + per-record regression)
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

/// Seeds a rich Brain A (every dependent class the closure must close) and a
/// small control Brain B. All identities are fresh UUIDs owned by this test.
async fn seed(admin: &PgPool, owner: Uuid, f: &Fixture) {
    let mut sql = std::fs::read_to_string(seed_script_path()).unwrap();
    // Variants for values embedded inside SQL string concatenation.
    for (token, id) in [
        ("__R1Q__", f.r1),
        ("__R2Q__", f.r2),
        ("__SNAP1Q__", f.snap1),
    ] {
        sql = sql.replace(token, &format!("'{id}'"));
    }
    // UUID placeholders are quoted: a bare uuid is not a valid SQL literal.
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
        ("__MT_TITLE__", MT_TITLE),
        ("__MT_URI__", MT_URI),
        ("__MT_PATH__", MT_PATH),
        ("__MT_CLAIM__", MT_CLAIM),
        ("__MT_CHUNK__", MT_CHUNK),
        ("__MT_RULE__", MT_RULE),
        ("__MT_RECEIPT__", MT_RECEIPT),
        ("__MT_MANIFEST__", MT_MANIFEST),
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

/// Row counts for one Brain across every relation in the 029 dependent list.
async fn inventory(pool: &PgPool, brain: Uuid) -> BTreeMap<String, i64> {
    let rels: Vec<String> =
        sqlx::query_scalar("SELECT relation FROM brain_deletion_dependents ORDER BY ordinal")
            .fetch_all(pool)
            .await
            .unwrap();
    let mut out = BTreeMap::new();
    for r in &rels {
        // Relation names come from the migration-owned catalog table.
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {r} WHERE brain_id=$1"))
            .bind(brain)
            .fetch_one(pool)
            .await
            .unwrap();
        out.insert(r.clone(), n);
    }
    out
}

/// Every text-bearing column of the rows that must survive the deletion.
async fn retained_text(env: &Env, brain: Uuid) -> String {
    let mut out = String::new();
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(target::text,'')||' '||coalesce(manifest::text,'')
         FROM privacy_requests WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_all(&env.admin)
    .await
    .unwrap();
    out.push_str(&rows.join(" "));
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(id::text,'')||' '||coalesce(brain_id::text,'')||' '
              ||coalesce(actor_id::text,'')||' '||coalesce(disposition,'')
         FROM brain_deletions WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_all(&env.admin)
    .await
    .unwrap();
    out.push_str(&rows.join(" "));
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(brain_id::text,'')||' '||coalesce(deletion_id::text,'')
         FROM brain_tombstones WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_all(&env.admin)
    .await
    .unwrap();
    out.push_str(&rows.join(" "));
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(input::text,'')||' '||coalesce(response::text,'')
         FROM command_receipts WHERE brain_id=$1",
    )
    .bind(brain)
    .fetch_all(&env.admin)
    .await
    .unwrap();
    out.push_str(&rows.join(" "));
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(action,'')||' '||coalesce(target_id::text,'')||' '||coalesce(disposition,'')
         FROM mutation_audit WHERE brain_id IS NULL AND action='brain.delete'",
    )
    .fetch_all(&env.admin)
    .await
    .unwrap();
    out.push_str(&rows.join(" "));
    out
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run with DATABASE_ADMIN_URL and DATABASE_URL"]
async fn brain_deletion_full_closure() {
    let (env, owner) = Env::new().await;
    let f = Fixture::new();
    seed(&env.admin, owner, &f).await;

    let before_a = inventory(&env.admin, f.a).await;
    let before_b = inventory(&env.admin, f.b).await;
    // The fixture really is rich: every dependent class has at least one row.
    assert!(
        before_a.values().all(|&n| n >= 1),
        "fixture gap: {before_a:?}"
    );

    // --- Preview: admin-only, browser-only, content-free, counter-bearing. ---
    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let preview: Option<Value> = sqlx::query_scalar("SELECT recollect_brain_preview($1)")
        .bind(f.a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    let preview = preview.expect("owner preview must be visible");
    assert_eq!(preview["brain_id"], f.a.to_string());
    assert_eq!(preview["name"], "Alpha");
    assert_eq!(preview["archived"], false);
    assert_eq!(preview["repository_content_allowed"], false);
    assert_eq!(preview["backup_days"], 7);
    // Five seeded jobs are queued, so all of them are fenced pending work.
    assert_eq!(preview["pending_work"], 5);
    let counts = &preview["counts"];
    assert_eq!(counts["sources"], 1);
    assert_eq!(counts["claims"], 1);
    assert_eq!(counts["snapshots"], 1);
    assert_eq!(counts["repositories"], 1);
    assert_eq!(counts["workspaces"], 1);
    assert_eq!(counts["tasks"], 1);
    assert_eq!(counts["connections"], 1);
    assert_eq!(counts["profiles"], 1);
    assert_eq!(counts["members"], 1);
    assert_eq!(counts["group_members"], 1);
    assert_eq!(counts["policies"], 3);
    let closure: i64 = preview["closure"].as_i64().unwrap();
    let counter: i64 = {
        let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
        let c: Option<i64> = sqlx::query_scalar("SELECT recollect_brain_counter($1)")
            .bind(f.a)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        drop(tx);
        c.unwrap()
    };
    assert_eq!(
        closure, counter,
        "preview counter must equal the closure hash"
    );
    // No controlled content leaks through the preview.
    let rendered = preview.to_string();
    for m in MARKERS {
        assert!(!rendered.contains(m), "preview leaks marker {m}");
    }
    // Non-admins and device callers get NULL (unknown, not forbidden).
    let mut tx = db::actor_tx(&env.app, f.member).await.ok().unwrap();
    let member_preview: Option<Value> = sqlx::query_scalar("SELECT recollect_brain_preview($1)")
        .bind(f.a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    assert!(member_preview.is_none());
    let mut tx = db::device_tx(&env.app, f.member, Some(f.dev))
        .await
        .ok()
        .unwrap();
    let device_preview: Option<Value> = sqlx::query_scalar("SELECT recollect_brain_preview($1)")
        .bind(f.a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    assert!(device_preview.is_none());

    // --- Rejections: distinct codes, and zero writes before the checks. ---
    let receipt_before: (String, Option<String>, bool) = sqlx::query_as(
        "SELECT input::text, response::text, invalidated FROM command_receipts WHERE brain_id=$1",
    )
    .bind(f.a)
    .fetch_one(&env.admin)
    .await
    .unwrap();
    assert!(!receipt_before.2);

    // Wrong confirmation: 22023.
    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let err = sqlx::query_scalar::<_, Option<Uuid>>("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(f.a)
        .bind(closure)
        .bind("Wrong Name")
        .bind(Uuid::new_v4())
        .fetch_one(&mut *tx)
        .await
        .unwrap_err();
    drop(tx);
    assert_eq!(
        code(&err).as_deref(),
        Some("22023"),
        "wrong confirmation code: {err}"
    );

    // Stale counter with the right confirmation: 40001.
    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let err = sqlx::query_scalar::<_, Option<Uuid>>("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(f.a)
        .bind(closure + 1)
        .bind("Alpha")
        .bind(Uuid::new_v4())
        .fetch_one(&mut *tx)
        .await
        .unwrap_err();
    drop(tx);
    assert_eq!(
        code(&err).as_deref(),
        Some("40001"),
        "stale counter code: {err}"
    );

    // Non-admin: insufficient privilege.
    let mut tx = db::actor_tx(&env.app, f.member).await.ok().unwrap();
    let err = sqlx::query_scalar::<_, Option<Uuid>>("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(f.a)
        .bind(closure)
        .bind("Alpha")
        .bind(Uuid::new_v4())
        .fetch_one(&mut *tx)
        .await
        .unwrap_err();
    drop(tx);
    assert_eq!(
        code(&err).as_deref(),
        Some("42501"),
        "non-admin must be denied: {err}"
    );

    // Device token: insufficient privilege.
    let mut tx = db::device_tx(&env.app, f.member, Some(f.dev))
        .await
        .ok()
        .unwrap();
    let err = sqlx::query_scalar::<_, Option<Uuid>>("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(f.a)
        .bind(closure)
        .bind("Alpha")
        .bind(Uuid::new_v4())
        .fetch_one(&mut *tx)
        .await
        .unwrap_err();
    drop(tx);
    assert_eq!(
        code(&err).as_deref(),
        Some("42501"),
        "device caller must be denied: {err}"
    );

    // None of the rejections wrote anything.
    let requests: i64 =
        sqlx::query_scalar("SELECT count(*) FROM privacy_requests WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&env.admin)
            .await
            .unwrap();
    // Only the seeded dormant request remains; no rejection wrote a journal row.
    assert_eq!(requests, 1, "rejections must not create journal entries");
    let brain_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM brains WHERE id=$1")
        .bind(f.a)
        .fetch_one(&env.admin)
        .await
        .unwrap();
    assert_eq!(brain_rows, 1);
    let after_reject = inventory(&env.admin, f.a).await;
    assert_eq!(
        before_a, after_reject,
        "rejections must not mutate dependents"
    );
    let receipt_after: (String, Option<String>, bool) = sqlx::query_as(
        "SELECT input::text, response::text, invalidated FROM command_receipts WHERE brain_id=$1",
    )
    .bind(f.a)
    .fetch_one(&env.admin)
    .await
    .unwrap();
    assert_eq!(
        receipt_before, receipt_after,
        "rejections must not invalidate receipts"
    );

    // --- The accepted deletion. ---
    let rid = Uuid::new_v4();
    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let deleted: Option<Uuid> = sqlx::query_scalar("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(f.a)
        .bind(closure)
        .bind("Alpha")
        .bind(rid)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    // Dropping a sqlx transaction rolls it back; the deletion must commit.
    tx.commit().await.unwrap();
    assert_eq!(deleted, Some(rid));

    // The Brain row is gone; the control Brain is untouched.
    let brain_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM brains WHERE id=$1")
        .bind(f.a)
        .fetch_one(&env.admin)
        .await
        .unwrap();
    assert_eq!(brain_rows, 0);
    let after_a = inventory(&env.admin, f.a).await;
    assert!(
        after_a.values().all(|&n| n == 0),
        "Brain A not fully closed: {after_a:?}"
    );
    let after_b = inventory(&env.admin, f.b).await;
    assert_eq!(before_b, after_b, "control Brain B must be untouched");

    // Counter agreement: the tombstone stores the honored preview counter.
    let tomb: (Uuid, Uuid, i64) =
        sqlx::query_as("SELECT id, actor_id, closure FROM brain_deletions WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&env.admin)
            .await
            .unwrap();
    assert_eq!(tomb.0, rid);
    assert_eq!(tomb.1, owner);
    assert_eq!(
        tomb.2, closure,
        "tombstone counter must equal the preview counter"
    );
    let stones: i64 = sqlx::query_scalar("SELECT count(*) FROM brain_tombstones WHERE brain_id=$1")
        .bind(f.a)
        .fetch_one(&env.admin)
        .await
        .unwrap();
    assert_eq!(stones, 1);

    // The journal row is retained and the Brain-scoped audit trail survives.
    let request: (String, bool, i64) = sqlx::query_as(
        "SELECT state, journaled, (SELECT count(*) FROM privacy_artifacts a WHERE a.request_id=p.id AND a.removed_at IS NULL)
         FROM privacy_requests p WHERE id=$1",
    )
    .bind(rid)
    .fetch_one(&env.admin)
    .await
    .unwrap();
    assert_eq!(request.0, "pending");
    assert!(!request.1);
    assert_eq!(
        request.2, 1,
        "the seeded artifact stays pending physical cleanup"
    );
    let audit_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE action='brain.delete' AND target_id=$1",
    )
    .bind(f.a)
    .fetch_one(&env.admin)
    .await
    .unwrap();
    assert_eq!(audit_rows, 1);

    // Receipts are invalidated, not deleted: key kept, content emptied.
    let receipt: (String, Option<String>, bool) = sqlx::query_as(
        "SELECT input::text, response::text, invalidated FROM command_receipts WHERE brain_id=$1",
    )
    .bind(f.a)
    .fetch_one(&env.admin)
    .await
    .unwrap();
    assert_eq!(receipt.0, "{}");
    assert!(receipt.1.is_none());
    assert!(receipt.2);

    // Retained rows are content-free.
    let retained = retained_text(&env, f.a).await;
    for m in MARKERS {
        assert!(!retained.contains(m), "retained row leaks marker {m}");
    }

    // --- Tombstone visibility: RLS hides it from access-lost principals. ---
    let mut tx = db::actor_tx(&env.app, f.member).await.ok().unwrap();
    let member_deleted: bool = sqlx::query_scalar("SELECT recollect_brain_deleted($1)")
        .bind(f.a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let member_deletions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM brain_deletions WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let member_stones: i64 =
        sqlx::query_scalar("SELECT count(*) FROM brain_tombstones WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let member_status: Option<Value> = sqlx::query_scalar("SELECT recollect_brain_deletion($1,$2)")
        .bind(f.a)
        .bind(rid)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    assert!(
        !member_deleted,
        "access-lost principal must not see the tombstone"
    );
    assert_eq!(member_deletions, 0);
    assert_eq!(member_stones, 0);
    assert!(member_status.is_none());

    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let owner_deleted: bool = sqlx::query_scalar("SELECT recollect_brain_deleted($1)")
        .bind(f.a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let owner_deletions: i64 =
        sqlx::query_scalar("SELECT count(*) FROM brain_deletions WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    let status: Option<Value> = sqlx::query_scalar("SELECT recollect_brain_deletion($1,$2)")
        .bind(f.a)
        .bind(rid)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    drop(tx);
    assert!(owner_deleted);
    assert_eq!(owner_deletions, 1);
    let status = status.expect("owner must see the deletion status");
    assert_eq!(status["id"], rid.to_string());
    assert_eq!(status["brain_id"], f.a.to_string());
    assert_eq!(status["actor_id"], owner.to_string());
    assert_eq!(status["closure"], closure);
    assert_eq!(status["disposition"], "deleted");
    assert_eq!(status["state"], "pending");
    assert_eq!(status["journaled"], false);
    assert_eq!(status["pending_artifacts"], 1);
    assert_eq!(status["acknowledged_devices"], 0);

    // --- Repeat: an already-absent Brain returns NULL and writes nothing. ---
    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let repeat: Option<Uuid> = sqlx::query_scalar("SELECT recollect_brain_delete($1,$2,$3,$4)")
        .bind(f.a)
        .bind(closure)
        .bind("Alpha")
        .bind(Uuid::new_v4())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        repeat.is_none(),
        "repeat deletion must report already-absent"
    );
    let requests: i64 =
        sqlx::query_scalar("SELECT count(*) FROM privacy_requests WHERE brain_id=$1")
            .bind(f.a)
            .fetch_one(&env.admin)
            .await
            .unwrap();
    // The seeded dormant request plus the single deletion journal entry.
    assert_eq!(
        requests, 2,
        "repeat deletion must not create a second journal entry"
    );

    // --- Per-record erasure is unchanged for the other Brain. ---
    let epoch: i64 = sqlx::query_scalar(
        "SELECT coalesce((SELECT epoch FROM memory_epochs WHERE brain_id=$1),0)",
    )
    .bind(f.b)
    .fetch_one(&env.admin)
    .await
    .unwrap();
    let erase_rid = Uuid::new_v4();
    let mut tx = db::actor_tx(&env.app, owner).await.ok().unwrap();
    let erased: Option<Uuid> = sqlx::query_scalar("SELECT recollect_erase($1,'source',$2,$3,$4)")
        .bind(f.b)
        .bind(f.sb)
        .bind(epoch)
        .bind(erase_rid)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(erased, Some(erase_rid));
    let redacted: (String, String, Option<String>) =
        sqlx::query_as("SELECT title, privacy_state, source_uri FROM source_versions WHERE id=$1")
            .bind(f.vb)
            .fetch_one(&env.admin)
            .await
            .unwrap();
    assert_eq!(redacted.0, "Erased source");
    assert_eq!(redacted.1, "erased");
    assert!(redacted.2.is_none());
    let b_chunks: i64 =
        sqlx::query_scalar("SELECT count(*) FROM source_chunks WHERE version_id=$1")
            .bind(f.vb)
            .fetch_one(&env.admin)
            .await
            .unwrap();
    assert_eq!(b_chunks, 0);
    let b_epoch: i64 = sqlx::query_scalar("SELECT epoch FROM memory_epochs WHERE brain_id=$1")
        .bind(f.b)
        .fetch_one(&env.admin)
        .await
        .unwrap();
    assert_eq!(b_epoch, epoch + 1);

    env.finish().await;
}

/// Catalog invariant referenced by the 029 migration comment: every public
/// table holding a brain_id column is either in the dependent list or an
/// intentionally retained relation, and every listed relation is a plain table
/// with a brain_id column. A later migration that adds a Brain-dependent
/// table without extending the list fails here.
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL; run with DATABASE_ADMIN_URL and DATABASE_URL"]
async fn brain_deletion_catalog_invariant() {
    let (env, _owner) = Env::new().await;
    let relations: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.relname, c.relkind::text FROM pg_class c
         JOIN pg_namespace n ON n.oid=c.relnamespace
         WHERE n.nspname='public' AND c.relkind IN ('r','m','v')
           AND EXISTS (SELECT 1 FROM information_schema.columns ic
                       WHERE ic.table_schema='public' AND ic.table_name=c.relname
                         AND ic.column_name='brain_id')
         ORDER BY 1",
    )
    .fetch_all(&env.admin)
    .await
    .unwrap();
    let listed: Vec<String> = sqlx::query_scalar("SELECT relation FROM brain_deletion_dependents")
        .fetch_all(&env.admin)
        .await
        .unwrap();
    let listed_set: std::collections::BTreeSet<&str> = listed.iter().map(|s| s.as_str()).collect();
    // Retained on purpose: the Brain row itself, the journal and its cleanup
    // state, Brain-free fence/acknowledgement records, invalidated receipts,
    // and the 029 tombstone metadata.
    let excluded = [
        "brains",
        "privacy_requests",
        "privacy_artifacts",
        "privacy_publication_fences",
        "privacy_device_positions",
        "command_receipts",
        "brain_deletions",
        "brain_tombstones",
    ];
    for (name, kind) in &relations {
        if kind != "r" {
            // Views and materialized views hold no rows; they may be unlisted.
            continue;
        }
        assert!(
            listed_set.contains(name.as_str()) || excluded.contains(&name.as_str()),
            "table {name} holds brain_id but is neither listed in 029 nor retained"
        );
    }
    for name in &listed {
        let row: Option<(String, bool)> = sqlx::query_as(
            "SELECT c.relkind::text, EXISTS(SELECT 1 FROM information_schema.columns ic
                WHERE ic.table_schema='public' AND ic.table_name=c.relname
                  AND ic.column_name='brain_id')
             FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
             WHERE n.nspname='public' AND c.relname=$1",
        )
        .bind(name)
        .fetch_optional(&env.admin)
        .await
        .unwrap();
        let (kind, has_column) = row.unwrap_or_else(|| panic!("listed relation {name} is missing"));
        assert_eq!(kind, "r", "listed relation {name} is not a plain table");
        assert!(
            has_column,
            "listed relation {name} lost its brain_id column"
        );
    }
    env.finish().await;
}
