use super::*;
use std::time::Duration;

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_publication_and_final_read_crossing_retention_deadline_refuse_scores() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = super::recovery_tests::setup(&h, &owner).await;
    let snapshot: Uuid = scope["snapshot_id"].as_str().unwrap().parse().unwrap();
    let retention = ok(&h, "GET", &format!("{base}/retention"), &owner, Value::Null).await;
    let mut policy = retention["policy"].clone();
    policy["repository_days"] = json!(1);
    ok(
        &h,
        "PUT",
        &format!("{base}/retention"),
        &owner,
        json!({"base_change":retention["change_id"],"policy":policy}),
    )
    .await;
    sqlx::raw_sql("CREATE FUNCTION analytics_publication_pause() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.state='ready' THEN PERFORM pg_sleep(6.2); END IF; RETURN NEW; END $$;
        CREATE TRIGGER analytics_publication_pause BEFORE UPDATE OF state ON analytics_reports FOR EACH ROW EXECUTE FUNCTION analytics_publication_pause();")
        .execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1").bind(snapshot).execute(&h.admin).await.unwrap();
    let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    tokio::time::timeout(Duration::from_secs(15), worker::run_once(&h.state, "heavy"))
        .await
        .expect("Publication must keep polling work across a lease heartbeat")
        .unwrap();
    let failed = read(&h, &owner, &base, &r["id"]).await;
    assert_eq!(failed["report"]["state"], "cancelled");
    assert_eq!(failed["report"]["error_code"], "analytics_inputs_changed");
    assert!(failed["report"]["published_at"].is_null());
    assert_eq!(failed["rows"], json!([]));
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mutation_audit WHERE brain_id=$1 AND action='graph.analysis.publish'",
    )
    .bind(brain)
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(
        count, 0,
        "Crossing a deadline must roll back both the numerical result and its publish audit"
    );
    sqlx::query("DROP TRIGGER analytics_publication_pause ON analytics_reports")
        .execute(&h.admin)
        .await
        .unwrap();

    sqlx::query("UPDATE repository_snapshots SET created_at=clock_timestamp()-interval '1 day'+interval '3 seconds' WHERE id=$1").bind(snapshot).execute(&h.admin).await.unwrap();
    let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
    completed(&h, &owner, &base, &r).await;
    // Delay only the last metadata lock in this isolated fixture database, after
    // full input qualification and hydration. The final clock gate must still run.
    sqlx::raw_sql("ALTER FUNCTION recollect_analytics_read_final(uuid,uuid) RENAME TO fixture_read_final;
        CREATE FUNCTION recollect_analytics_read_final(b uuid,rid uuid) RETURNS jsonb LANGUAGE plpgsql AS $$
        DECLARE result jsonb; BEGIN result:=fixture_read_final(b,rid); PERFORM pg_sleep(4); RETURN result; END $$;
        REVOKE ALL ON FUNCTION recollect_analytics_read_final(uuid,uuid) FROM PUBLIC;
        GRANT EXECUTE ON FUNCTION recollect_analytics_read_final(uuid,uuid) TO recollect_app;")
        .execute(&h.admin).await.unwrap();
    let crossed = h
        .call(
            "POST",
            &format!("{base}/graph/analytics/{}/view", r["id"].as_str().unwrap()),
            Some(&owner),
            json!({}),
        )
        .await;
    assert_eq!(crossed.0, StatusCode::CONFLICT);
    assert_eq!(crossed.1["code"], "analytics_inputs_changed");
    sqlx::raw_sql("DROP FUNCTION recollect_analytics_read_final(uuid,uuid); ALTER FUNCTION fixture_read_final(uuid,uuid) RENAME TO recollect_analytics_read_final")
        .execute(&h.admin).await.unwrap();
    let stale = read(&h, &owner, &base, &r["id"]).await;
    assert_eq!(stale["report"]["state"], "stale");
    assert_eq!(stale["rows"], json!([]));
    h.finish().await;
}

async fn blocked_by(h: &Harness, blocker: i32, waiter: Option<i32>) -> i32 {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity WHERE datname=current_database()
                 AND ($2::int IS NULL OR pid=$2) AND $1=ANY(pg_blocking_pids(pid)) LIMIT 1",
            )
            .bind(blocker)
            .bind(waiter)
            .fetch_optional(&h.admin)
            .await
            .unwrap();
            if let Some(pid) = pid {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("The exact fixture connection must reach its database barrier")
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL and Neo4j/GDS"]
async fn graph_analytics_reader_final_gate_serializes_observed_staleness() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base, scope) = super::recovery_tests::setup(&h, &owner).await;
    let (member, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{member}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let (_, foreign_base) = super::super::base(&h, &owner).await;
    sqlx::raw_sql(
        "ALTER FUNCTION recollect_analytics_read_final(uuid,uuid) RENAME TO fixture_read_final",
    )
    .execute(&h.admin)
    .await
    .unwrap();

    for stale_first in [true, false] {
        // The wrapper pauses only this report, before or after its real final
        // lock. All input qualification/hydration and ordinary reader RLS run.
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION recollect_analytics_read_final(b uuid,rid uuid) RETURNS jsonb
            LANGUAGE sql AS $$ SELECT fixture_read_final(b,rid) $$;
            REVOKE ALL ON FUNCTION recollect_analytics_read_final(uuid,uuid) FROM PUBLIC;
            GRANT EXECUTE ON FUNCTION recollect_analytics_read_final(uuid,uuid) TO recollect_app;")
            .execute(&h.admin).await.unwrap();
        let r = queue(&h, &owner, &base, &scope, "pagerank", "outgoing").await;
        completed(&h, &owner, &base, &r).await;
        let positive = read(&h, &reader, &base, &r["id"]).await;
        assert_eq!(positive["report"]["state"], "ready");
        assert_eq!(positive["rows"].as_array().unwrap().len(), 4);
        assert_eq!(
            h.call(
                "POST",
                &format!(
                    "{foreign_base}/graph/analytics/{}/view",
                    r["id"].as_str().unwrap()
                ),
                Some(&reader),
                json!({})
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
        let id: Uuid = r["id"].as_str().unwrap().parse().unwrap();
        let pause = format!(
            "IF rid='{id}'::uuid THEN PERFORM pg_advisory_xact_lock(73241990::bigint); END IF;"
        );
        let gate = "result:=fixture_read_final(b,rid);";
        let body = if stale_first {
            format!("{pause} {gate}")
        } else {
            format!("{gate} {pause}")
        };
        sqlx::raw_sql(&format!("CREATE OR REPLACE FUNCTION recollect_analytics_read_final(b uuid,rid uuid) RETURNS jsonb
            LANGUAGE plpgsql AS $$ DECLARE result jsonb; BEGIN {body} RETURN result; END $$;"))
            .execute(&h.admin).await.unwrap();
        let epoch: i64 = sqlx::query_scalar("SELECT analytics_epoch FROM brains WHERE id=$1")
            .bind(brain)
            .fetch_one(&h.admin)
            .await
            .unwrap();
        let mut barrier = h.admin.acquire().await.unwrap();
        sqlx::query("SELECT pg_advisory_lock(73241990::bigint)")
            .execute(&mut *barrier)
            .await
            .unwrap();
        let barrier_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *barrier)
            .await
            .unwrap();

        let invalidate = async {
            let read_pid = blocked_by(&h, barrier_pid, None).await;
            let mut observer = db::actor_tx(&h.state.pool, member)
                .await
                .unwrap_or_else(|e| panic!("{}", e.1));
            db::require_role(&mut observer, brain, false)
                .await
                .unwrap_or_else(|e| panic!("{}", e.1));
            let observer_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *observer)
                .await
                .unwrap();
            let update = async {
                sqlx::query("SELECT recollect_analytics_observed_stale($1,$2)")
                    .bind(brain)
                    .bind(id)
                    .execute(&mut *observer)
                    .await
                    .unwrap();
                observer.commit().await.unwrap();
            };
            if stale_first {
                update.await;
                sqlx::query("SELECT pg_advisory_unlock(73241990::bigint)")
                    .execute(&mut *barrier)
                    .await
                    .unwrap();
            } else {
                let release = async {
                    // FOR KEY SHARE or an unlocked final SELECT would fail this:
                    // the non-key stale update must wait on the active reader.
                    blocked_by(&h, read_pid, Some(observer_pid)).await;
                    sqlx::query("SELECT pg_advisory_unlock(73241990::bigint)")
                        .execute(&mut *barrier)
                        .await
                        .unwrap();
                };
                tokio::join!(update, release);
            }
        };
        let (result, ()) = tokio::join!(read(&h, &reader, &base, &r["id"]), invalidate);
        if stale_first {
            assert_eq!(result["report"]["state"], "stale");
            assert_eq!(result["rows"], json!([]));
            assert_eq!(result["total"], 0);
        } else {
            assert_eq!(result["report"]["state"], "ready");
            assert_eq!(result["total"], 4);
        }
        let saved: (String, bool, i64) = sqlx::query_as("SELECT r.state,r.scores IS NULL,b.analytics_epoch FROM analytics_reports r JOIN brains b ON b.id=r.brain_id WHERE r.id=$1")
            .bind(id).fetch_one(&h.admin).await.unwrap();
        assert_eq!(saved, ("stale".into(), true, epoch));
    }
    sqlx::raw_sql("DROP FUNCTION recollect_analytics_read_final(uuid,uuid); ALTER FUNCTION fixture_read_final(uuid,uuid) RENAME TO recollect_analytics_read_final")
        .execute(&h.admin).await.unwrap();
    h.finish().await;
}
