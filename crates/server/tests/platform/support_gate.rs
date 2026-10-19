//! The canonical support predicate is a membership-gated SECURITY DEFINER
//! function (migration 068). These proofs keep it equal to the previous
//! invoker-rights body, which is retained here as a reference function, and
//! keep the definer from silently bypassing a future restrictive read policy.
use super::*;
use sqlx::{Postgres, Transaction};

const REFERENCE: &str = "recollect_fixture_old_memory_supported";

/// The previous `recollect_memory_supported` body (migration 043) under a test
/// name. It is an invoker-rights function, so called as the application role it
/// is evaluated through every row-level security policy, exactly as before.
pub(crate) async fn install_reference(h: &Harness) {
    let migration = include_str!("../../migrations/043_support_predicate_short_circuit.sql");
    let body = migration
        .split("CREATE OR REPLACE FUNCTION recollect_memory_supported")
        .nth(1)
        .expect("Migration 043 defines the previous support body");
    sqlx::raw_sql(&format!(
        "CREATE OR REPLACE FUNCTION {REFERENCE}{body}
         GRANT EXECUTE ON FUNCTION {REFERENCE}(uuid,uuid) TO recollect_app;"
    ))
    .execute(&h.admin)
    .await
    .unwrap();
}

/// A transaction that evaluates as the application role for `actor`, or for a
/// throwaway account that has no grant anywhere when `actor` is `None`. The
/// transaction is rolled back by the caller, so nothing it creates persists.
async fn app_tx(h: &Harness, actor: Option<Uuid>) -> Transaction<'_, Postgres> {
    let mut tx = h.admin.begin().await.unwrap();
    let actor = match actor {
        Some(actor) => actor,
        None => {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO accounts(id,username) VALUES($1,$2)")
                .bind(id)
                .bind(format!("support-gate-{id}"))
                .execute(&mut *tx)
                .await
                .unwrap();
            id
        }
    };
    sqlx::query("SELECT set_config('recollect.actor',$1,true)")
        .bind(actor.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SET LOCAL ROLE recollect_app")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx
}

async fn owner_of(h: &Harness, brain: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT owner_id FROM brains WHERE id=$1")
        .bind(brain)
        .fetch_one(&h.admin)
        .await
        .unwrap()
}

/// Verdict for every given target from the previous body, the new scalar and
/// the batch function, asserted equal, as `actor` through the application role.
/// Returns the verdicts so a caller can also assert the expected outcome.
async fn verdicts(
    h: &Harness,
    brain: Uuid,
    actor: Option<Uuid>,
    targets: &[Uuid],
) -> std::collections::BTreeMap<Uuid, bool> {
    let mut tx = app_tx(h, actor).await;
    let scalar: Vec<(Uuid, bool, bool)> = sqlx::query_as(&format!(
        "SELECT t.id,recollect_memory_supported($1,t.id),{REFERENCE}($1,t.id) FROM unnest($2::uuid[]) t(id)"
    ))
    .bind(brain)
    .bind(targets)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let mut expected = std::collections::BTreeMap::new();
    for (id, new, old) in scalar {
        assert_eq!(
            new, old,
            "Scalar support changed for revision {id} (actor {actor:?}, brain {brain})"
        );
        expected.insert(id, old);
    }
    // Duplicates and a NULL target must not change any verdict or add a row.
    let mut padded = targets.to_vec();
    padded.extend_from_slice(targets);
    let batch: Vec<(Uuid, bool)> = sqlx::query_as(
        "SELECT revision_id,supported FROM recollect_memory_supported_many($1,$2::uuid[] || ARRAY[NULL::uuid])",
    )
    .bind(brain)
    .bind(&padded)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        batch.len(),
        expected.len(),
        "The batch returns one row per distinct non-null target"
    );
    for (id, supported) in batch {
        assert_eq!(
            Some(&supported),
            expected.get(&id),
            "Batch support differs from the previous body for {id} (actor {actor:?})"
        );
    }
    tx.rollback().await.unwrap();
    expected
}

/// Every revision of every Brain, asked about in each Brain's context, so the
/// foreign-Brain revisions are exercised too. Members must get the previous
/// body's verdict and a non-member must get false from both.
pub(crate) async fn assert_equivalent(
    h: &Harness,
    brains: &[Uuid],
) -> std::collections::BTreeMap<(Uuid, Uuid), bool> {
    install_reference(h).await;
    let mut targets: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM claim_revisions ORDER BY id")
        .fetch_all(&h.admin)
        .await
        .unwrap();
    targets.push(Uuid::new_v4());
    let mut result = std::collections::BTreeMap::new();
    for brain in brains {
        let owner = owner_of(h, *brain).await;
        for (id, supported) in verdicts(h, *brain, Some(owner), &targets).await {
            result.insert((*brain, id), supported);
        }
        for (id, supported) in verdicts(h, *brain, None, &targets).await {
            assert!(!supported, "A non-member never sees support for {id}");
        }
    }
    result
}

/// The previous body's verdict for one revision as the Brain owner, with the
/// scalar, batch and non-member results asserted equal to it first. Existing
/// scenarios call this where they used to ask an unauthenticated connection.
pub(crate) async fn checked(h: &Harness, brain: Uuid, revision: Uuid) -> bool {
    install_reference(h).await;
    let all: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM claim_revisions WHERE brain_id=$1")
        .bind(brain)
        .fetch_all(&h.admin)
        .await
        .unwrap();
    let owner = owner_of(h, brain).await;
    let mut targets = all;
    if !targets.contains(&revision) {
        targets.push(revision);
    }
    let members = verdicts(h, brain, Some(owner), &targets).await;
    for (id, supported) in verdicts(h, brain, None, &targets).await {
        assert!(!supported, "A non-member never sees support for {id}");
    }
    members[&revision]
}

/// `recollect_memory_supported` is membership-gated, so a bare administrative
/// connection (which has no actor) sees false. Scenarios that inspect support
/// through `pool` ask as the owner of the revision's Brain instead.
pub(crate) async fn supported_as_owner(pool: &sqlx::PgPool, revision: Uuid) -> bool {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('recollect.actor',(SELECT b.owner_id::text FROM claim_revisions r JOIN brains b ON b.id=r.brain_id WHERE r.id=$1),true)")
        .bind(revision)
        .execute(&mut *tx)
        .await
        .unwrap();
    let verdict = sqlx::query_scalar(
        "SELECT recollect_memory_supported(brain_id,id) FROM claim_revisions WHERE id=$1",
    )
    .bind(revision)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    verdict
}

/// The owner-context verdict of every session digest claim of the Brain.
pub(crate) async fn digest_verdicts(pool: &sqlx::PgPool, brain: Uuid) -> Vec<bool> {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('recollect.actor',(SELECT owner_id::text FROM brains WHERE id=$1),true)",
    )
    .bind(brain)
    .execute(&mut *tx)
    .await
    .unwrap();
    let verdicts = sqlx::query_scalar("SELECT recollect_memory_supported(brain_id,revision_id) FROM session_digest_claims WHERE brain_id=$1 ORDER BY claim_id")
        .bind(brain)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    verdicts
}

/// Table, row-level security flag, then policy name, permissive flag, command and
/// normalized USING expression (all absent for a table without policies).
type PolicyRow = (
    String,
    bool,
    Option<String>,
    Option<bool>,
    Option<String>,
    Option<String>,
);

/// Problems with the read policies of every table the support predicate reads.
/// The tables are derived from the function bodies, so a table added to the
/// predicate tree later is checked without editing a list. Definer callees are
/// not descended into: they already read as the definer and are unchanged.
async fn policy_problems(h: &Harness) -> (Vec<String>, Vec<String>) {
    let rows: Vec<PolicyRow> = sqlx::query_as(
        "WITH RECURSIVE closure(oid) AS (
           SELECT p.oid FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
           WHERE n.nspname='public' AND p.proname IN ('recollect_memory_supported','recollect_memory_supported_many')
           UNION
           SELECT q.oid FROM closure c JOIN pg_proc cp ON cp.oid=c.oid
            JOIN pg_proc q ON q.pronamespace=cp.pronamespace AND q.proname LIKE 'recollect\\_%'
           WHERE NOT q.prosecdef AND cp.prosrc ~ ('\\m'||q.proname||'\\M')
         ), tables AS (
           SELECT c.oid,c.relname,c.relrowsecurity FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
           WHERE n.nspname='public' AND c.relkind IN ('r','p') AND EXISTS(
             SELECT 1 FROM closure f JOIN pg_proc p ON p.oid=f.oid WHERE p.prosrc ~ ('\\m'||c.relname||'\\M'))
         ) SELECT t.relname::text,t.relrowsecurity,q.polname::text,q.polpermissive,q.polcmd::text,
             regexp_replace(pg_get_expr(q.polqual,q.polrelid),'\\s+',' ','g')
           FROM tables t LEFT JOIN pg_policy q ON q.polrelid=t.oid ORDER BY t.relname,q.polname",
    )
    .fetch_all(&h.admin)
    .await
    .unwrap();
    // Reads are reduced to Brain membership by each of these forms. The second
    // and third are the Brain-set and bulk-qualification spellings of the same
    // visible set (migrations 055 and 057).
    let membership = [
        "(recollect_role(brain_id) IS NOT NULL)",
        "(brain_id IN ( SELECT brains.id FROM brains))",
        "CASE WHEN (current_setting('recollect.bulk_qualification'::text, true) = 'on'::text) THEN (brain_id IN ( SELECT brains.id FROM brains)) ELSE (recollect_role(brain_id) IS NOT NULL) END",
    ];
    let mut problems = vec![];
    let mut tables = std::collections::BTreeMap::<String, usize>::new();
    for (table, rls, name, permissive, command, qual) in &rows {
        let granting = tables.entry(table.clone()).or_default();
        if !*rls {
            problems.push(format!(
                "{table} is read without row-level security, so non-members could read it"
            ));
        }
        let (Some(name), Some(permissive), Some(command)) = (name, permissive, command) else {
            continue;
        };
        // 'r' is SELECT and '*' is ALL; INSERT, UPDATE and DELETE policies do not constrain reads.
        if command != "r" && command != "*" {
            continue;
        }
        if !*permissive {
            problems.push(format!(
                "{table}.{name} is a restrictive read policy that the definer would bypass"
            ));
        }
        if qual
            .as_deref()
            .map(str::trim)
            .is_some_and(|q| membership.contains(&q))
        {
            *granting += 1;
        }
    }
    for (table, granting) in &tables {
        if *granting == 0 {
            problems.push(format!(
                "{table} has no permissive Brain-membership read policy"
            ));
        }
    }
    // The Brain-set forms read `brains`, so its own read policy must be
    // membership too.
    let brains: Vec<(bool, Option<String>)> = sqlx::query_as(
        "SELECT q.polpermissive,regexp_replace(pg_get_expr(q.polqual,q.polrelid),'\\s+',' ','g')
         FROM pg_policy q WHERE q.polrelid='public.brains'::regclass AND q.polcmd IN ('r','*')",
    )
    .fetch_all(&h.admin)
    .await
    .unwrap();
    if brains.is_empty()
        || !brains.iter().all(|(permissive, qual)| {
            *permissive
                && qual.as_deref().map(str::trim) == Some("(recollect_role(id) IS NOT NULL)")
        })
    {
        problems.push(format!(
            "brains must be readable exactly by Brain members: {brains:?}"
        ));
    }
    (problems, tables.into_keys().collect())
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_gate_reads_only_tables_with_brain_membership_policies() {
    let h = Harness::new().await;
    let (problems, tables) = policy_problems(&h).await;
    assert!(problems.is_empty(), "{problems:#?}");
    // Discovery must not be vacuous: the core tables of the predicate are present.
    for expected in [
        "claim_revisions",
        "claim_contributions",
        "claims",
        "claim_supports",
        "memory_support_assessments",
        "model_policy_heads",
        "memory_decisions",
        "assertion_rules",
        "memory_rule_exceptions",
        "model_claim_fences",
        "model_input_fences",
        "source_versions",
        "repository_facts",
        "repository_snapshots",
        "repositories",
        "manifest_revisions",
        "evidence_groups",
        "retention_policies",
        "session_digest_claims",
        "session_digest_partitions",
        "session_digest_generations",
        "session_digest_pages",
    ] {
        assert!(
            tables.iter().any(|t| t == expected),
            "Policy discovery did not find {expected}: {tables:?}"
        );
    }
    // The guard itself must fail when a policy that the definer would bypass
    // appears. These changes exist only in this disposable database.
    sqlx::raw_sql(
        "CREATE POLICY support_gate_probe ON model_claim_fences AS RESTRICTIVE FOR SELECT
           USING(recollect_role(brain_id)='admin')",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    let (problems, _) = policy_problems(&h).await;
    assert!(
        problems
            .iter()
            .any(|p| p.contains("model_claim_fences") && p.contains("restrictive")),
        "A restrictive read policy must be reported: {problems:?}"
    );
    sqlx::raw_sql(
        "DROP POLICY support_gate_probe ON model_claim_fences;
         DROP POLICY memory_read ON claim_revisions;
         CREATE POLICY memory_read ON claim_revisions FOR SELECT USING(recollect_role(brain_id) IN ('writer','admin'))",
    )
    .execute(&h.admin)
    .await
    .unwrap();
    let (problems, _) = policy_problems(&h).await;
    assert!(
        problems
            .iter()
            .any(|p| p.contains("claim_revisions") && p.contains("no permissive")),
        "A narrower-than-membership read policy must be reported: {problems:?}"
    );
    h.finish().await;
}

async fn new_brain(h: &Harness, owner: &Login, name: &str) -> (Uuid, String) {
    let brain = review::ok(h, "POST", "/api/brains", owner, json!({"name":name})).await;
    let id: Uuid = brain["id"].as_str().unwrap().parse().unwrap();
    (id, format!("/api/brains/{id}"))
}

async fn evidence(h: &Harness, owner: &Login, base: &str, title: &str) -> Value {
    review::ok(
        h,
        "POST",
        &format!("{base}/sources"),
        owner,
        json!({"title":title,"media_type":"text/plain","content":"Synthetic values for support proof.\n","retain_content":true,"source_uri":null,"group_ids":[],"observed_at":null}),
    )
    .await["version"]["id"]
        .clone()
}

async fn propose(
    h: &Harness,
    owner: &Login,
    base: &str,
    version: &Value,
    subject: &str,
    value: &str,
) -> Value {
    review::ok(
        h,
        "POST",
        &format!("{base}/claims"),
        owner,
        review::proposal(version, subject, value),
    )
    .await
}

async fn review_claim(h: &Harness, owner: &Login, base: &str, r: &Value, action: &str) -> Value {
    review::ok(
        h,
        "POST",
        &format!("{base}/claims/{}/review", r["claim_id"].as_str().unwrap()),
        owner,
        json!({"base_revision":r["id"],"action":action,"reason":"Synthetic support equivalence proof","content":null,"revalidation_basis":null}),
    )
    .await["claims"][0]["revision"]
        .clone()
}

async fn handover(
    h: &Harness,
    owner: &Login,
    base: &str,
    version: &Value,
    subject: &str,
    inputs: &[&Value],
) -> Value {
    let mut input = review::proposal(version, subject, "Synthetic handover summary.");
    input["content"]["kind"] = json!("handover");
    input["content"]["handover"] = json!({
        "completed":["Recorded the synthetic declarations."],
        "next_steps":["Verify runtime separately."],
        "risks":["A declaration does not prove health."],
        "contributions":inputs.iter().map(|r| r["id"].clone()).collect::<Vec<_>>()
    });
    review::ok(h, "POST", &format!("{base}/claims"), owner, input).await
}

fn id(value: &Value) -> Uuid {
    value["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_gate_matches_the_previous_body_for_members_non_members_and_foreign_brains() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (a, base) = new_brain(&h, &owner, "Support gate A").await;
    let (b, other) = new_brain(&h, &owner, "Support gate B").await;
    let version = evidence(&h, &owner, &base, "Support gate evidence").await;
    let foreign_version = evidence(&h, &owner, &other, "Support gate foreign evidence").await;

    // Plain claims: reviewed, unreviewed, rejected and one blocked by a durable rule.
    let plain = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Plain", "1").await,
        "accept",
    )
    .await;
    let second = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Second", "2").await,
        "accept",
    )
    .await;
    let unreviewed = propose(&h, &owner, &base, &version, "Unreviewed", "3").await;
    let rejected = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Rejected", "4").await,
        "reject",
    )
    .await;
    let first = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Rule", "5").await,
        "accept",
    )
    .await;
    let blocked = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Rule", "5").await,
        "accept",
    )
    .await;
    review_claim(&h, &owner, &base, &first, "reject").await;
    let fenced = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Fenced", "6").await,
        "accept",
    )
    .await;
    let foreign = review_claim(
        &h,
        &owner,
        &other,
        &propose(&h, &owner, &other, &foreign_version, "Foreign", "7").await,
        "accept",
    )
    .await;
    let mut tx = h.admin.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO model_claim_fences(brain_id,revision_id,request_id) VALUES($1,$2,gen_random_uuid())")
        .bind(a)
        .bind(id(&fenced))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let verdict = assert_equivalent(&h, &[a, b]).await;
    for (revision, expected) in [
        (&plain, true),
        (&second, true),
        (&unreviewed, false),
        (&rejected, false),
        (&blocked, false),
        (&fenced, false),
    ] {
        assert_eq!(verdict[&(a, id(revision))], expected, "{revision}");
    }
    assert!(
        verdict[&(b, id(&foreign))],
        "The foreign Brain's own claim is supported there"
    );
    assert!(
        !verdict[&(a, id(&foreign))],
        "A revision of another Brain is never supported here"
    );
    assert!(!verdict[&(b, id(&plain))]);

    // Handovers: a supported root, an unreviewed root, a handover that reaches
    // another handover (never allowed as a non-root dependency) and exact cycles.
    let root = review_claim(
        &h,
        &owner,
        &base,
        &handover(
            &h,
            &owner,
            &base,
            &version,
            "Root handover",
            &[&plain, &second],
        )
        .await,
        "accept",
    )
    .await;
    let draft = handover(
        &h,
        &owner,
        &base,
        &version,
        "Unreviewed handover",
        &[&plain],
    )
    .await;
    let nested = review_claim(
        &h,
        &owner,
        &base,
        &handover(&h, &owner, &base, &version, "Nested handover", &[&plain]).await,
        "accept",
    )
    .await;
    let left = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Left", "8").await,
        "accept",
    )
    .await;
    let right = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &version, "Right", "9").await,
        "accept",
    )
    .await;
    sqlx::raw_sql("ALTER TABLE claim_contributions DISABLE TRIGGER contribution_acyclic")
        .execute(&h.admin)
        .await
        .unwrap();
    for (parent, input) in [(&nested, &root), (&left, &right), (&right, &left)] {
        sqlx::query("INSERT INTO claim_contributions(brain_id,revision_id,input_revision_id) VALUES($1,$2,$3)")
            .bind(a)
            .bind(id(parent))
            .bind(id(input))
            .execute(&h.admin)
            .await
            .unwrap();
    }
    sqlx::raw_sql("ALTER TABLE claim_contributions ENABLE TRIGGER contribution_acyclic")
        .execute(&h.admin)
        .await
        .unwrap();
    let verdict = assert_equivalent(&h, &[a, b]).await;
    for (revision, expected) in [
        (&root, true),
        (&draft, false),
        (&nested, false),
        (&left, false),
        (&right, false),
    ] {
        assert_eq!(verdict[&(a, id(revision))], expected, "{revision}");
    }

    // A rejected current head of an exact contributor withdraws the root.
    let head = review_claim(&h, &owner, &base, &second, "reject").await;
    let verdict = assert_equivalent(&h, &[a, b]).await;
    assert!(
        !verdict[&(a, id(&root))],
        "The unsupported current head withdraws the handover"
    );
    assert!(!verdict[&(a, id(&head))]);
    assert!(verdict[&(a, id(&plain))]);

    // Retention: a human-reviewed claim keeps its expired source, an unreviewed one does not.
    let old = evidence(&h, &owner, &base, "Support gate expiring evidence").await;
    let expired = review_claim(
        &h,
        &owner,
        &base,
        &propose(&h, &owner, &base, &old, "Expired reviewed", "10").await,
        "accept",
    )
    .await;
    let expiring = propose(&h, &owner, &base, &old, "Expired unreviewed", "11").await;
    let retention = review::ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["document_days"] = json!(1);
    review::ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::query(
        "UPDATE source_versions SET created_at=clock_timestamp()-interval '400 days' WHERE id=$1",
    )
    .bind(old.as_str().unwrap().parse::<Uuid>().unwrap())
    .execute(&h.admin)
    .await
    .unwrap();
    let verdict = assert_equivalent(&h, &[a, b]).await;
    assert!(
        verdict[&(a, id(&expired))],
        "Reviewed claims survive their source's natural expiry"
    );
    assert!(!verdict[&(a, id(&expiring))]);
    let supported = verdict.values().filter(|v| **v).count();
    assert!(
        supported >= 3 && supported < verdict.len(),
        "Both outcomes are exercised"
    );
    h.finish().await;
}

/// Per-revision cost as the real application role (row-level security applies
/// to the previous body). The numbers are printed for the record; the only
/// assertion is a generous bound so that load on the machine cannot flake it.
#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn support_gate_cost_per_revision_as_the_application_role() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = new_brain(&h, &owner, "Support gate cost").await;
    let version = evidence(&h, &owner, &base, "Support gate cost evidence").await;
    let mut accepted = vec![];
    for n in 0..30 {
        let claim = propose(
            &h,
            &owner,
            &base,
            &version,
            &format!("Cost subject {n}"),
            &n.to_string(),
        )
        .await;
        accepted.push(review_claim(&h, &owner, &base, &claim, "accept").await);
    }
    for (n, group) in accepted.chunks(10).enumerate() {
        let inputs: Vec<&Value> = group.iter().collect();
        review_claim(
            &h,
            &owner,
            &base,
            &handover(
                &h,
                &owner,
                &base,
                &version,
                &format!("Cost handover {n}"),
                &inputs,
            )
            .await,
            "accept",
        )
        .await;
    }
    install_reference(&h).await;
    let actor = owner_of(&h, brain).await;
    let targets: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM claim_revisions WHERE brain_id=$1 ORDER BY id")
            .bind(brain)
            .fetch_all(&h.admin)
            .await
            .unwrap();
    let mut tx = db::actor_tx(&h.state.pool, actor).await.ok().unwrap();
    let mut measure = async |label: &str, sql: String, batch: bool| {
        // One untimed pass warms plans and caches; the timed pass is the median of five.
        let mut samples = vec![];
        let mut supported = 0usize;
        for pass in 0..6 {
            let started = std::time::Instant::now();
            supported = if batch {
                let rows: Vec<(Uuid, bool)> = sqlx::query_as(&sql)
                    .bind(brain)
                    .bind(&targets)
                    .fetch_all(&mut *tx)
                    .await
                    .unwrap();
                rows.iter().filter(|(_, s)| *s).count()
            } else {
                let mut count = 0;
                for target in &targets {
                    let verdict: bool = sqlx::query_scalar(&sql)
                        .bind(brain)
                        .bind(target)
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();
                    count += usize::from(verdict);
                }
                count
            };
            if pass > 0 {
                samples.push(started.elapsed());
            }
        }
        samples.sort();
        let median = samples[samples.len() / 2];
        eprintln!(
            "support cost [{label}]: {} revisions, {supported} supported, {:.2} ms total, {:.3} ms per revision",
            targets.len(),
            median.as_secs_f64() * 1000.0,
            median.as_secs_f64() * 1000.0 / targets.len() as f64
        );
        (median, supported)
    };
    let (old, old_supported) =
        measure("previous body", format!("SELECT {REFERENCE}($1,$2)"), false).await;
    let (new, new_supported) = measure(
        "definer scalar",
        "SELECT recollect_memory_supported($1,$2)".into(),
        false,
    )
    .await;
    let (many, many_supported) = measure(
        "definer batch",
        "SELECT revision_id,supported FROM recollect_memory_supported_many($1,$2)".into(),
        true,
    )
    .await;
    assert!(old_supported > 0 && old_supported == new_supported && new_supported == many_supported);
    // Generous: the definer forms must not be slower than the previous body.
    assert!(new <= old * 2, "definer scalar {new:?} vs previous {old:?}");
    assert!(
        many <= old * 2,
        "definer batch {many:?} vs previous {old:?}"
    );
    tx.rollback().await.unwrap();
    h.finish().await;
}
