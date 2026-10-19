//! The canonical support predicate is a membership-gated SECURITY DEFINER
//! function (migration 068) over the support closure of migration 071. These
//! proofs keep it equal to the previous invoker-rights body, retained here as a
//! reference function, and keep the definer from silently bypassing a future
//! restrictive read policy. The only intended difference from the previous body
//! is the learning basis boundary: the edges autonomous::basis writes from a
//! published claim to its offered reconciliation inputs are provenance and
//! erasure dependencies, not support or deadline dependencies; only a handover
//! contribution is. The reference encodes exactly that exception, over its own
//! edge formulation, and nothing else.
use super::*;
use sqlx::{Postgres, Transaction};

const REFERENCE: &str = "recollect_fixture_old_memory_supported";
const DEADLINE_REFERENCE: &str = "recollect_fixture_old_deadline";
const FULL_PRE_DIGEST: &str = "recollect_fixture_full_pre_digest";

/// The previous support and deadline bodies (migrations 043 and 046) under test
/// names. They are invoker-rights functions, so called as the application role
/// they are evaluated through every row-level security policy, exactly as before.
///
/// The reference closure is the previous exact-contribution and current-head
/// closure with one exception, written here independently of the migration: an
/// edge is followed only when its parent revision carries a handover section,
/// which is what procedures::append keys its inserts on. Every other edge is
/// a learning basis edge (a published claim linked to an offered input) and is
/// not followed. `recollect_fixture_full_pre_digest` is the unmodified 046 body
/// over the unchanged full closure; it is the control that shows what the
/// previous semantics did with those edges.
pub(crate) async fn install_reference(h: &Harness) {
    const CLOSURE: &str = "recollect_fixture_support_dependencies";
    const PRE_DIGEST: &str = "recollect_fixture_old_pre_digest";
    let exact = include_str!("../../migrations/046_exact_memory_deadlines.sql");
    let pre_digest = exact
        .split("CREATE OR REPLACE FUNCTION recollect_pre_digest_supported")
        .nth(1)
        .and_then(|rest| rest.split("$$;").next())
        .expect("Migration 046 defines the previous pre-digest support body");
    let deadline = exact
        .split("CREATE OR REPLACE FUNCTION recollect_memory_deadline")
        .nth(1)
        .and_then(|rest| rest.split("$$;").next())
        .expect("Migration 046 defines the previous deadline body");
    let migration = include_str!("../../migrations/043_support_predicate_short_circuit.sql");
    let body = migration
        .split("CREATE OR REPLACE FUNCTION recollect_memory_supported")
        .nth(1)
        .expect("Migration 043 defines the previous support body");
    let supported = body
        .replace("recollect_pre_digest_supported(", &format!("{PRE_DIGEST}("))
        .replace("recollect_memory_dependencies(", &format!("{CLOSURE}("));
    sqlx::raw_sql(&format!(
        "CREATE OR REPLACE FUNCTION {CLOSURE}(b uuid,target uuid) RETURNS TABLE(revision_id uuid)
         LANGUAGE sql STABLE SET search_path=public,pg_temp AS $$
          WITH RECURSIVE edges(parent,child) AS (
            SELECT c.revision_id,c.input_revision_id FROM claim_contributions c
              JOIN claim_revisions p ON p.brain_id=c.brain_id AND p.id=c.revision_id
              WHERE c.brain_id=b AND jsonb_typeof(p.revision#>'{{content,handover}}')='object'
            UNION SELECT c.revision_id,h.current_revision FROM claim_contributions c
              JOIN claim_revisions p ON p.brain_id=c.brain_id AND p.id=c.revision_id
              JOIN claim_revisions i ON i.brain_id=c.brain_id AND i.id=c.input_revision_id
              JOIN claims h ON h.brain_id=i.brain_id AND h.id=i.claim_id
              WHERE c.brain_id=b AND jsonb_typeof(p.revision#>'{{content,handover}}')='object'
          ), dependencies(id) AS (
            SELECT target UNION SELECT e.child FROM edges e JOIN dependencies d ON d.id=e.parent
          ) SELECT id FROM dependencies
         $$;
         CREATE OR REPLACE FUNCTION {PRE_DIGEST}{filtered}$$;
         CREATE OR REPLACE FUNCTION {FULL_PRE_DIGEST}{pre_digest}$$;
         CREATE OR REPLACE FUNCTION {DEADLINE_REFERENCE}{deadline_filtered}$$;
         CREATE OR REPLACE FUNCTION {REFERENCE}{supported}
         GRANT EXECUTE ON FUNCTION {CLOSURE}(uuid,uuid),{PRE_DIGEST}(uuid,uuid),{FULL_PRE_DIGEST}(uuid,uuid),
           {DEADLINE_REFERENCE}(uuid,uuid),{REFERENCE}(uuid,uuid) TO recollect_app;",
        filtered = pre_digest.replace("recollect_memory_dependencies(", &format!("{CLOSURE}(")),
        deadline_filtered = deadline.replace("recollect_memory_dependencies(", &format!("{CLOSURE}(")),
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

/// The owner-rights connection has no actor and no row-level security applies
/// to it, as for the privacy closure that calls the predicate: the scalar function
/// must keep answering unfiltered, like the previous body and the unchanged core.
async fn exempt_verdicts(
    h: &Harness,
    brain: Uuid,
    targets: &[Uuid],
) -> std::collections::BTreeMap<Uuid, bool> {
    let rows: Vec<(Uuid, bool, bool, bool)> = sqlx::query_as(&format!(
        "SELECT t.id,recollect_memory_supported($1,t.id),{REFERENCE}($1,t.id),recollect_memory_supported_core($1,t.id) FROM unnest($2::uuid[]) t(id)"
    ))
    .bind(brain)
    .bind(targets)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    let mut verdicts = std::collections::BTreeMap::new();
    for (id, scalar, old, core) in rows {
        assert!(
            scalar == old && core == old,
            "Owner-rights support changed for {id}: scalar {scalar}, core {core}, previous {old}"
        );
        verdicts.insert(id, old);
    }
    verdicts
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
        let members = verdicts(h, *brain, Some(owner), &targets).await;
        assert_eq!(
            exempt_verdicts(h, *brain, &targets).await,
            members,
            "An owner-rights caller and a member see the same support"
        );
        for (id, supported) in members {
            result.insert((*brain, id), supported);
        }
        for (id, supported) in verdicts(h, *brain, None, &targets).await {
            assert!(!supported, "A non-member never sees support for {id}");
        }
    }
    result
}

/// The previous body's verdict for one revision as the Brain owner, with the
/// scalar, batch, owner-rights and non-member results asserted equal to it first.
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
    assert_eq!(
        exempt_verdicts(h, brain, &targets).await,
        members,
        "An owner-rights caller and a member see the same support"
    );
    for (id, supported) in verdicts(h, brain, None, &targets).await {
        assert!(!supported, "A non-member never sees support for {id}");
    }
    members[&revision]
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
           WHERE n.nspname='public' AND p.proname IN ('recollect_memory_supported_core','recollect_memory_supported_many')
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

// Learning basis boundary (migration 071). A claim published by learning
// reconciliation is linked by claim_contributions edges to every reconciliation
// input the run offered (the revision it replaced and the other offered
// revisions), as autonomous::basis does. These cases pin those edges to
// provenance and erasure only and leave every genuine dependency, the
// contributions of a handover, as they were.

/// A source with its identity and one version, as returned by the API.
async fn source_of(h: &Harness, owner: &Login, base: &str, title: &str) -> Value {
    review::ok(
        h,
        "POST",
        &format!("{base}/sources"),
        owner,
        json!({"title":title,"media_type":"text/plain","content":"Synthetic values for support proof.\n","retain_content":true,"source_uri":null,"group_ids":[],"observed_at":null}),
    )
    .await
}

/// Replace `prior` (PUT with its base revision) citing `version`.
async fn replace(
    h: &Harness,
    owner: &Login,
    base: &str,
    prior: &Value,
    version: &Value,
    value: &str,
) -> Value {
    let mut input = review::proposal(version, "Supersession subject", value);
    input["base_revision"] = prior["id"].clone();
    review::ok(
        h,
        "PUT",
        &format!("{base}/claims/{}", prior["claim_id"].as_str().unwrap()),
        owner,
        input,
    )
    .await
}

/// The edge autonomous::basis writes: a published claim is linked to each
/// reconciliation input the run offered, whether the revision it replaced or
/// another claim's revision.
async fn link(h: &Harness, brain: Uuid, revision: &Value, input: &Value) {
    sqlx::query(
        "INSERT INTO claim_contributions(brain_id,revision_id,input_revision_id) VALUES($1,$2,$3)",
    )
    .bind(brain)
    .bind(id(revision))
    .bind(id(input))
    .execute(&h.admin)
    .await
    .unwrap();
}

/// A succeeded `supported` assessment, the machine-maintained route to support
/// that learning writes at publication. An unreviewed revision needs it, and an
/// unreviewed revision's expired source is what makes the previous closure fail.
async fn assess(h: &Harness, brain: Uuid, revision: &Value) {
    let inserted = sqlx::query(
        "INSERT INTO memory_support_assessments(id,brain_id,revision_id,policy_id,verifier_version,actor_id,state,disposition,reason,finished_at)
         SELECT gen_random_uuid(),$1,$2,p.policy_id,'source-support-3',$3,'succeeded','supported','Synthetic assessed support',clock_timestamp()
         FROM model_policy_heads p WHERE p.brain_id=$1",
    )
    .bind(brain)
    .bind(id(revision))
    .bind(owner_of(h, brain).await)
    .execute(&h.admin)
    .await
    .unwrap();
    assert_eq!(inserted.rows_affected(), 1, "The Brain has a model policy");
}

/// Document retention of one day, with `versions` created well before that.
async fn expire(h: &Harness, owner: &Login, base: &str, versions: &[&Value]) {
    let retention = review::ok(h, "GET", &format!("{base}/retention"), owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["document_days"] = json!(1);
    review::ok(
        h,
        "PUT",
        &format!("{base}/retention"),
        owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    for version in versions {
        sqlx::query("UPDATE source_versions SET created_at=clock_timestamp()-interval '400 days' WHERE id=$1")
            .bind(version["version"]["id"].as_str().unwrap().parse::<Uuid>().unwrap())
            .execute(&h.admin)
            .await
            .unwrap();
    }
}

/// Deadline of every revision from the migration function and from the previous
/// body over the reference closure, as the Brain owner through the application
/// role, asserted equal. Returns the deadlines.
type Deadline = Option<chrono::DateTime<chrono::Utc>>;

async fn deadlines(h: &Harness, brain: Uuid) -> std::collections::BTreeMap<Uuid, Deadline> {
    install_reference(h).await;
    let mut tx = app_tx(h, Some(owner_of(h, brain).await)).await;
    let rows: Vec<(Uuid, Deadline, Deadline)> = sqlx::query_as(&format!(
        "SELECT r.id,recollect_memory_deadline($1,r.id),{DEADLINE_REFERENCE}($1,r.id)
             FROM claim_revisions r WHERE r.brain_id=$1 ORDER BY r.id"
    ))
    .bind(brain)
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    rows.into_iter()
        .map(|(id, new, old)| {
            assert_eq!(new, old, "Deadline of {id} differs from the previous body");
            (id, new)
        })
        .collect()
}

/// The retention deadline of one source version, as its own evidence sets it.
async fn source_deadline(h: &Harness, brain: Uuid, source: &Value) -> Deadline {
    sqlx::query_scalar(
        "SELECT recollect_retention_deadline(v.brain_id,v.retention_class,v.created_at)
         FROM source_versions v WHERE v.brain_id=$1 AND v.id=$2",
    )
    .bind(brain)
    .bind(
        source["version"]["id"]
            .as_str()
            .unwrap()
            .parse::<Uuid>()
            .unwrap(),
    )
    .fetch_one(&h.admin)
    .await
    .unwrap()
}

async fn pre_digest(h: &Harness, brain: Uuid, revision: &Value, function: &str) -> bool {
    let mut tx = app_tx(h, Some(owner_of(h, brain).await)).await;
    let verdict = sqlx::query_scalar(&format!("SELECT {function}($1,$2)"))
        .bind(brain)
        .bind(id(revision))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    verdict
}

async fn dependencies(h: &Harness, brain: Uuid, revision: &Value, function: &str) -> Vec<Uuid> {
    sqlx::query_scalar(&format!(
        "SELECT revision_id FROM {function}($1,$2) ORDER BY revision_id"
    ))
    .bind(brain)
    .bind(id(revision))
    .fetch_all(&h.admin)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn a_replacement_keeps_its_support_and_deadline_after_the_replaced_source_expires() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = new_brain(&h, &owner, "Supersession boundary").await;
    crate::models::allow(&h, &owner, &base, |_| {}).await;
    let old = source_of(&h, &owner, &base, "Superseded raw evidence").await;
    let fresh = source_of(&h, &owner, &base, "Replacement evidence").await;
    let predecessor = propose(
        &h,
        &owner,
        &base,
        &old["version"]["id"],
        "Supersession subject",
        "8080",
    )
    .await;
    let replacement = replace(
        &h,
        &owner,
        &base,
        &predecessor,
        &fresh["version"]["id"],
        "9090",
    )
    .await;
    assert_eq!(replacement["claim_id"], predecessor["claim_id"]);
    link(&h, brain, &replacement, &predecessor).await;
    assess(&h, brain, &predecessor).await;
    assess(&h, brain, &replacement).await;

    // Before expiry both are supported through every form of the predicate.
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(verdict[&(brain, id(&predecessor))] && verdict[&(brain, id(&replacement))]);

    expire(&h, &owner, &base, &[&old]).await;
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(
        !verdict[&(brain, id(&predecessor))],
        "The replaced revision's own expired source still withdraws it"
    );
    assert!(
        verdict[&(brain, id(&replacement))],
        "The replacement does not inherit the replaced revision's source expiry"
    );
    // The previous semantics, kept as a control: the full closure reaches the
    // expired predecessor and withdraws the replacement.
    assert!(!pre_digest(&h, brain, &replacement, FULL_PRE_DIGEST).await);
    assert!(pre_digest(&h, brain, &replacement, "recollect_pre_digest_supported").await);

    // The full contribution graph is kept for erasure and the audit; only the
    // support closure drops the basis edge and the predecessor.
    assert!(
        dependencies(&h, brain, &replacement, "recollect_memory_dependencies")
            .await
            .contains(&id(&predecessor))
    );
    assert!(
        dependencies(
            &h,
            brain,
            &replacement,
            "recollect_memory_exact_dependencies"
        )
        .await
        .contains(&id(&predecessor))
    );
    assert_eq!(
        dependencies(
            &h,
            brain,
            &replacement,
            "recollect_memory_support_dependencies"
        )
        .await,
        vec![id(&replacement)]
    );

    // The replacement keeps its own deadline: its claim retention (until erased)
    // and its own fresh source, never the expired source of the predecessor.
    let all = deadlines(&h, brain).await;
    let own = source_deadline(&h, brain, &fresh).await;
    assert!(own.unwrap() > chrono::Utc::now());
    assert_eq!(all[&id(&replacement)], own);
    assert!(
        all[&id(&predecessor)].unwrap() < chrono::Utc::now(),
        "The predecessor keeps its own expired deadline"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn erasing_the_replaced_revisions_source_still_erases_the_replacement() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = new_brain(&h, &owner, "Supersession erasure").await;
    crate::models::allow(&h, &owner, &base, |_| {}).await;
    let old = source_of(&h, &owner, &base, "Superseded raw evidence").await;
    let fresh = source_of(&h, &owner, &base, "Replacement evidence").await;
    let predecessor = propose(
        &h,
        &owner,
        &base,
        &old["version"]["id"],
        "Supersession subject",
        "8080",
    )
    .await;
    let replacement = replace(
        &h,
        &owner,
        &base,
        &predecessor,
        &fresh["version"]["id"],
        "9090",
    )
    .await;
    link(&h, brain, &replacement, &predecessor).await;
    assess(&h, brain, &predecessor).await;
    assess(&h, brain, &replacement).await;
    // Independent control: its own source and no edge to the erased one. It must
    // not share the replacement's source: erasure fences the other sources of a
    // revision derived from an erased input against further model use.
    let unrelated = source_of(&h, &owner, &base, "Unrelated evidence").await;
    let control = propose(
        &h,
        &owner,
        &base,
        &unrelated["version"]["id"],
        "Independent subject",
        "7070",
    )
    .await;
    assess(&h, brain, &control).await;
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(verdict[&(brain, id(&replacement))] && verdict[&(brain, id(&control))]);

    let target = json!({"kind":"source","id":old["id"]});
    let preview = review::ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    review::ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;

    let states: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,privacy_state FROM claim_revisions WHERE brain_id=$1 ORDER BY id",
    )
    .bind(brain)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    let state = |revision: &Value| {
        states
            .iter()
            .find(|(candidate, _)| *candidate == id(revision))
            .map(|(_, state)| state.as_str())
            .unwrap()
    };
    assert_eq!(state(&predecessor), "erased");
    assert_eq!(
        state(&replacement),
        "erased",
        "The replacement stays an erasure dependent of the revision it replaced"
    );
    assert_eq!(
        state(&control),
        "active",
        "An independent revision survives"
    );
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(!verdict[&(brain, id(&replacement))] && verdict[&(brain, id(&control))]);
    h.finish().await;
}

/// What the run `autonomous::basis` records for one reconciliation: each
/// published claim is linked to every input the run offered.
async fn basis(h: &Harness, brain: Uuid, published: &[&Value], offered: &[&Value]) {
    for revision in published {
        for input in offered {
            link(h, brain, revision, input).await;
        }
    }
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn published_claims_keep_their_support_and_deadline_when_a_different_offered_inputs_source_expires()
 {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = new_brain(&h, &owner, "Learning basis boundary").await;
    crate::models::allow(&h, &owner, &base, |_| {}).await;
    let expiring = source_of(&h, &owner, &base, "Offered input evidence").await;
    let prior = source_of(&h, &owner, &base, "Replaced evidence").await;
    let fresh = source_of(&h, &owner, &base, "Published evidence").await;
    // One reconciliation run was offered revisions of two different claims from
    // the same session: one is replaced, the other is only offered.
    let offered = propose(
        &h,
        &owner,
        &base,
        &expiring["version"]["id"],
        "Offered subject",
        "1",
    )
    .await;
    let predecessor = propose(
        &h,
        &owner,
        &base,
        &prior["version"]["id"],
        "Supersession subject",
        "2",
    )
    .await;
    let replacement = replace(
        &h,
        &owner,
        &base,
        &predecessor,
        &fresh["version"]["id"],
        "3",
    )
    .await;
    let learned = propose(
        &h,
        &owner,
        &base,
        &fresh["version"]["id"],
        "Learned subject",
        "4",
    )
    .await;
    for revision in [&offered, &predecessor, &replacement, &learned] {
        assess(&h, brain, revision).await;
    }
    basis(
        &h,
        brain,
        &[&replacement, &learned],
        &[&predecessor, &offered],
    )
    .await;
    let verdict = assert_equivalent(&h, &[brain]).await;
    for revision in [&offered, &predecessor, &replacement, &learned] {
        assert!(verdict[&(brain, id(revision))], "{revision}");
    }

    // Only the other offered input's raw source expires.
    expire(&h, &owner, &base, &[&expiring]).await;
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(
        !verdict[&(brain, id(&offered))],
        "The offered input's own expired source still withdraws it"
    );
    assert!(verdict[&(brain, id(&predecessor))]);
    for (name, revision) in [("replacement", &replacement), ("new claim", &learned)] {
        assert!(
            verdict[&(brain, id(revision))],
            "The {name} does not inherit a different offered input's source expiry"
        );
        // The previous semantics, kept as a control: the full closure reaches
        // the offered input and withdraws the published claim.
        assert!(!pre_digest(&h, brain, revision, FULL_PRE_DIGEST).await);
        assert!(pre_digest(&h, brain, revision, "recollect_pre_digest_supported").await);
        assert_eq!(
            dependencies(&h, brain, revision, "recollect_memory_support_dependencies").await,
            vec![id(revision)],
            "The {name}'s support closure is the revision alone"
        );
        // Every offered input stays an erasure and audit dependency.
        let full = dependencies(&h, brain, revision, "recollect_memory_dependencies").await;
        let exact = dependencies(&h, brain, revision, "recollect_memory_exact_dependencies").await;
        for input in [&offered, &predecessor] {
            assert!(
                full.contains(&id(input)) && exact.contains(&id(input)),
                "{name}"
            );
        }
    }

    // Each published claim keeps its own deadline: its claim retention (until
    // erased) and its own fresh source, never the offered input's expired source.
    let all = deadlines(&h, brain).await;
    let own = source_deadline(&h, brain, &fresh).await;
    assert!(own.unwrap() > chrono::Utc::now());
    assert_eq!(all[&id(&replacement)], own);
    assert_eq!(all[&id(&learned)], own);
    assert!(
        all[&id(&offered)].unwrap() < chrono::Utc::now(),
        "The offered input keeps its own expired deadline"
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn erasing_a_different_offered_inputs_source_still_erases_the_published_claim() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = new_brain(&h, &owner, "Offered input erasure").await;
    crate::models::allow(&h, &owner, &base, |_| {}).await;
    let erased = source_of(&h, &owner, &base, "Offered input evidence").await;
    let fresh = source_of(&h, &owner, &base, "Published evidence").await;
    let unrelated = source_of(&h, &owner, &base, "Unrelated evidence").await;
    let offered = propose(
        &h,
        &owner,
        &base,
        &erased["version"]["id"],
        "Offered subject",
        "1",
    )
    .await;
    let learned = propose(
        &h,
        &owner,
        &base,
        &fresh["version"]["id"],
        "Learned subject",
        "2",
    )
    .await;
    let control = propose(
        &h,
        &owner,
        &base,
        &unrelated["version"]["id"],
        "Independent subject",
        "3",
    )
    .await;
    for revision in [&offered, &learned, &control] {
        assess(&h, brain, revision).await;
    }
    basis(&h, brain, &[&learned], &[&offered]).await;
    let verdict = assert_equivalent(&h, &[brain]).await;
    for revision in [&offered, &learned, &control] {
        assert!(verdict[&(brain, id(revision))], "{revision}");
    }

    let target = json!({"kind":"source","id":erased["id"]});
    let preview = review::ok(
        &h,
        "POST",
        &format!("{base}/erasures/preview"),
        &owner,
        target.clone(),
    )
    .await;
    review::ok(
        &h,
        "POST",
        &format!("{base}/erasures"),
        &owner,
        json!({"target":target,"eligibility_epoch":preview["eligibility_epoch"]}),
    )
    .await;

    let states: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,privacy_state FROM claim_revisions WHERE brain_id=$1 ORDER BY id",
    )
    .bind(brain)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    let state = |revision: &Value| {
        states
            .iter()
            .find(|(candidate, _)| *candidate == id(revision))
            .map(|(_, state)| state.as_str())
            .unwrap()
    };
    assert_eq!(state(&offered), "erased");
    assert_eq!(
        state(&learned),
        "erased",
        "A published claim stays an erasure dependent of every offered input"
    );
    assert_eq!(
        state(&control),
        "active",
        "An independent revision survives"
    );
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(!verdict[&(brain, id(&learned))] && verdict[&(brain, id(&control))]);
    h.finish().await;
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn handover_contributions_still_withdraw_their_dependants_when_their_source_expires() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = new_brain(&h, &owner, "Handover dependency").await;
    crate::models::allow(&h, &owner, &base, |_| {}).await;
    let expiring = source_of(&h, &owner, &base, "Expiring evidence").await;
    let fresh = source_of(&h, &owner, &base, "Fresh evidence").await;
    // A handover contribution is evidence the handover stands on.
    let contributor = propose(
        &h,
        &owner,
        &base,
        &expiring["version"]["id"],
        "Contributor",
        "1",
    )
    .await;
    assess(&h, brain, &contributor).await;
    // The handover preserves its contributor's exact evidence, so it cites the
    // same source; it is reviewed, which tolerates that source's expiry for its
    // own evidence but not for what it depends on.
    let summary = review_claim(
        &h,
        &owner,
        &base,
        &handover(
            &h,
            &owner,
            &base,
            &expiring["version"]["id"],
            "Handover over contributor",
            &[&contributor],
        )
        .await,
        "accept",
    )
    .await;
    // A learned claim offered the contributor is a different thing: its basis
    // edge is provenance, so the contributor's expiry does not reach it.
    let learned = propose(
        &h,
        &owner,
        &base,
        &fresh["version"]["id"],
        "Learned beside contributor",
        "2",
    )
    .await;
    assess(&h, brain, &learned).await;
    basis(&h, brain, &[&learned], &[&contributor]).await;

    // The classification the migration relies on, on rows the product wrote:
    // an edge's parent carries a handover section exactly when its kind is
    // handover, and the parent of the handover edge is the handover.
    let mismatched: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM claim_contributions c
         JOIN claim_revisions p ON p.brain_id=c.brain_id AND p.id=c.revision_id
         WHERE c.brain_id=$1 AND (p.revision#>>'{content,kind}'='handover')
           <> coalesce(jsonb_typeof(p.revision#>'{content,handover}')='object',false)",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(mismatched, 0);
    let procedure_claims: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT p.claim_id FROM claim_contributions c
         JOIN claim_revisions p ON p.brain_id=c.brain_id AND p.id=c.revision_id
         WHERE c.brain_id=$1 AND p.revision#>>'{content,kind}'='handover'",
    )
    .bind(brain)
    .fetch_all(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        procedure_claims,
        vec![
            summary["claim_id"]
                .as_str()
                .unwrap()
                .parse::<Uuid>()
                .unwrap()
        ],
        "Only the handover's revisions have procedure contribution edges"
    );
    let verdict = assert_equivalent(&h, &[brain]).await;
    for revision in [&contributor, &summary, &learned] {
        assert!(verdict[&(brain, id(revision))], "{revision}");
    }
    assert!(
        deadlines(&h, brain).await.values().all(Option::is_none),
        "Document retention is until erased by default"
    );

    expire(&h, &owner, &base, &[&expiring]).await;
    let verdict = assert_equivalent(&h, &[brain]).await;
    assert!(!verdict[&(brain, id(&contributor))]);
    assert!(
        !verdict[&(brain, id(&summary))],
        "A handover whose contributor's source expired is withdrawn"
    );
    assert!(
        verdict[&(brain, id(&learned))],
        "A claim learned beside the contributor does not inherit its expiry"
    );
    let after = deadlines(&h, brain).await;
    assert!(
        after[&id(&summary)].unwrap() < chrono::Utc::now(),
        "The handover's deadline still includes its contributor's expired source"
    );
    assert_eq!(
        dependencies(&h, brain, &summary, "recollect_memory_support_dependencies").await,
        dependencies(&h, brain, &summary, "recollect_memory_dependencies").await,
        "A handover's closure drops no contribution"
    );
    h.finish().await;
}
