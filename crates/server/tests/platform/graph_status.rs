//! Metadata-only exact-generation tracking on actual authenticated API reads.
//! Projection states are deliberately controlled storage fixtures, not native
//! projection readiness or semantic quality proof. Status reads invoke no work
//! or model; repository setup separately uses the canonical materializer.
use super::*;

async fn queued(h: &Harness, owner: &Login, base: &str) -> Value {
    ok(
        h,
        "POST",
        &format!("{base}/graph/rebuild"),
        owner,
        json!({"kind":"knowledge","snapshot_id":null,"manifest_revision_id":null}),
    )
    .await
}
fn tracking(base: &str, ids: &[Uuid], offset: i64) -> String {
    format!(
        "{base}/graph?offset={offset}&tracked_generation_ids={}",
        ids.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[tokio::test]
#[ignore = "Owned PostgreSQL and local Neo4j cleanup; authenticated status metadata only, zero model calls"]
async fn graph_status_tracks_old_generations_outside_inventory_with_exact_read_authority_and_bounds()
 {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (brain, base) = base(&h, &owner).await;
    let old = queued(&h, &owner, &base).await;
    let replacement = queued(&h, &owner, &base).await;
    assert_eq!(old["input_epoch"], replacement["input_epoch"]);
    let old_id: Uuid = old["id"].as_str().unwrap().parse().unwrap();
    let replacement_id: Uuid = replacement["id"].as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE graph_generations SET state=CASE WHEN id=$1 THEN 'superseded' ELSE 'ready' END,descriptor=jsonb_build_object('private_source_label','must-not-leak') WHERE id=ANY($2)")
        .bind(old_id).bind(vec![old_id,replacement_id]).execute(&h.admin).await.unwrap();
    for _ in 0..21 {
        queued(&h, &owner, &base).await;
    }
    let (_, other_base) = super::base(&h, &owner).await;
    let foreign = queued(&h, &owner, &other_base).await;
    let foreign_id: Uuid = foreign["id"].as_str().unwrap().parse().unwrap();
    let missing = Uuid::new_v4();
    let ids = [old_id, replacement_id, missing, foreign_id];
    let inventory = ok(&h, "GET", &format!("{base}/graph"), &owner, Value::Null).await;
    assert_eq!(inventory["total"], 23);
    assert_eq!(inventory["generations"].as_array().unwrap().len(), 20);
    assert!(
        inventory["tracked_generations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        inventory["missing_tracked_generation_ids"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        inventory["generations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["id"] != old["id"] && g["id"] != replacement["id"])
    );
    let tracked = ok(&h, "GET", &tracking(&base, &ids, 0), &owner, Value::Null).await;
    for field in [
        "generations",
        "jobs",
        "total",
        "offset",
        "memory_epoch",
        "link_epoch",
        "organization_epoch",
    ] {
        assert_eq!(
            tracked[field], inventory[field],
            "Tracking must preserve ordinary inventory field {field}"
        );
    }
    assert_eq!(tracked["tracked_generations"].as_array().unwrap().len(), 2);
    assert_eq!(tracked["tracked_generations"][0]["id"], old["id"]);
    assert_eq!(tracked["tracked_generations"][0]["state"], "superseded");
    assert_eq!(tracked["tracked_generations"][1]["id"], replacement["id"]);
    assert_eq!(tracked["tracked_generations"][1]["state"], "ready");
    assert_eq!(
        tracked["missing_tracked_generation_ids"],
        json!([missing, foreign_id])
    );
    assert!(!tracked.to_string().contains("must-not-leak"));
    assert!(
        tracked["tracked_generations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g.get("descriptor").is_none() && g.get("descriptor_bytes").is_none())
    );
    let page = ok(
        &h,
        "GET",
        &format!("{base}/graph?offset=20"),
        &owner,
        Value::Null,
    )
    .await;
    let tracked_page = ok(&h, "GET", &tracking(&base, &ids, 20), &owner, Value::Null).await;
    assert_eq!(page["generations"], tracked_page["generations"]);
    assert_eq!(tracked_page["generations"].as_array().unwrap().len(), 3);
    assert_eq!(tracked_page["offset"], 20);
    assert_eq!(tracked_page["total"], 23);
    assert_eq!(
        tracked_page["tracked_generations"],
        tracked["tracked_generations"]
    );

    let (reader_id, reader) = h.fixture_member().await;
    ok(
        &h,
        "PUT",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        json!({"role":"reader"}),
    )
    .await;
    let (_, token) = h
        .pair_device(&reader, "Exact tracked generation metadata")
        .await;
    let reader_view = ok(&h, "GET", &tracking(&base, &ids, 0), &reader, Value::Null).await;
    assert_eq!(reader_view, tracked);
    let paired = h
        .bearer("GET", &tracking(&base, &ids, 0), &token, Value::Null)
        .await;
    assert_eq!(paired.0, StatusCode::OK);
    assert_eq!(paired.1, tracked);
    for inaccessible in [
        format!("{other_base}/graph?tracked_generation_ids={foreign_id}"),
        format!(
            "/api/brains/{}/graph?tracked_generation_ids={old_id}",
            Uuid::new_v4()
        ),
    ] {
        assert_eq!(
            h.call("GET", &inaccessible, Some(&reader), Value::Null)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            h.bearer("GET", &inaccessible, &token, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    let mut rls = db::actor_tx(&h.state.pool, reader_id)
        .await
        .unwrap_or_else(|error| panic!("{}", error.1));
    let visible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM graph_generations WHERE id=ANY($1)")
            .bind(ids.to_vec())
            .fetch_one(&mut *rls)
            .await
            .unwrap();
    assert_eq!(
        visible, 2,
        "The exact foreign ID remains invisible under ordinary read RLS"
    );
    rls.rollback().await.unwrap();

    let maximum: Vec<Uuid> = (0..102).map(|_| Uuid::new_v4()).collect();
    let max_view = ok(
        &h,
        "GET",
        &tracking(&base, &maximum, 0),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(max_view["missing_tracked_generation_ids"], json!(maximum));
    assert!(
        max_view["tracked_generations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(max_view["total"], 23);
    let compact103 = (0..103)
        .map(|_| Uuid::new_v4().simple().to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        compact103.len() < 3774,
        "Count overflow is independently below the byte cap"
    );
    let bad = vec![
        String::new(),
        "not-a-uuid".into(),
        Uuid::nil().to_string(),
        format!("{old_id},{old_id}"),
        format!("{old_id},"),
        format!(",{old_id}"),
        compact103,
        "a".repeat(3775),
        format!("{old_id},%20{replacement_id}"),
    ];
    for value in bad {
        assert_eq!(
            h.call(
                "GET",
                &format!("{base}/graph?tracked_generation_ids={value}"),
                Some(&owner),
                Value::Null
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        h.call("GET", &tracking(&base, &ids, -1), Some(&owner), Value::Null)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let jobs_before: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&h.admin)
        .await
        .unwrap();
    ok(&h, "GET", &tracking(&base, &ids, 0), &owner, Value::Null).await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs")
            .fetch_one(&h.admin)
            .await
            .unwrap(),
        jobs_before,
        "Status metadata never queues work"
    );
    ok(
        &h,
        "DELETE",
        &format!("{base}/grants/{reader_id}"),
        &owner,
        Value::Null,
    )
    .await;
    assert_eq!(
        h.bearer("GET", &tracking(&base, &ids, 0), &token, Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.call("GET", &tracking(&base, &ids, 0), Some(&reader), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        brain.to_string(),
        tracked["tracked_generations"][0]["brain_id"]
            .as_str()
            .unwrap()
    );
    h.finish().await;
}

#[tokio::test]
#[ignore = "Owned PostgreSQL and local Neo4j cleanup; canonical repository import, metadata-only status read"]
async fn graph_status_tracked_unavailable_repository_input_is_synthetically_removed_without_content_or_mutation()
 {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (_, base) = base(&h, &owner).await;
    let (_, snapshot, _) = Box::pin(materialized(&h, &owner, &base, 'a', None)).await;
    let generation = ok(
        &h,
        "POST",
        &format!("{base}/graph/rebuild"),
        &owner,
        json!({"kind":"repository","snapshot_id":snapshot}),
    )
    .await;
    let id: Uuid = generation["id"].as_str().unwrap().parse().unwrap();
    let snapshot: Uuid = snapshot.as_str().unwrap().parse().unwrap();
    sqlx::query("UPDATE graph_generations SET descriptor=jsonb_build_object('source_content','never-return-this-source') WHERE id=$1").bind(id).execute(&h.admin).await.unwrap();
    sqlx::query("UPDATE repository_snapshots SET privacy_state='expired' WHERE id=$1")
        .bind(snapshot)
        .execute(&h.admin)
        .await
        .unwrap();
    let result = ok(&h, "GET", &tracking(&base, &[id], 0), &owner, Value::Null).await;
    let tracked = &result["tracked_generations"][0];
    assert_eq!(tracked["id"], generation["id"]);
    assert_eq!(tracked["state"], "removed");
    assert_eq!(tracked["error_code"], "graph_input_unavailable");
    assert_eq!(
        result["generations"][0], *tracked,
        "Inventory and tracked rows use identical unavailable-input semantics"
    );
    assert!(
        result["missing_tracked_generation_ids"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!result.to_string().contains("never-return-this-source"));
    let stored: (String, Option<String>) =
        sqlx::query_as("SELECT state,error_code FROM graph_generations WHERE id=$1")
            .bind(id)
            .fetch_one(&h.admin)
            .await
            .unwrap();
    assert_eq!(
        stored,
        ("queued".into(), None),
        "Metadata read must not mutate the retained generation"
    );
    h.finish().await;
}
