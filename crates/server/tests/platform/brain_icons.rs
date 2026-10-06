use super::*;

fn png(color: [u8; 4]) -> Vec<u8> {
    let mut output = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(512, 256, image::Rgba(color))
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    output.into_inner()
}
async fn icon_call(
    h: &Harness,
    method: &str,
    path: &str,
    login: Option<&Login>,
    csrf: bool,
    bytes: Vec<u8>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "image/png");
    if let Some(login) = login {
        request = request.header("cookie", &login.cookie);
        if csrf {
            request = request.header("x-csrf-token", &login.csrf);
        }
    }
    h.router
        .clone()
        .oneshot(request.body(Body::from(bytes)).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "Requires repository-owned PostgreSQL"]
async fn brain_icon_authority_normalization_and_deletion() {
    let h = Harness::new().await;
    let owner = h.login().await;
    let (member_id, member) = h.fixture_member().await;
    let (_, brain, _) = h
        .call(
            "POST",
            "/api/brains",
            Some(&owner),
            json!({"name":"Icon proof"}),
        )
        .await;
    let id = Uuid::parse_str(brain["id"].as_str().unwrap()).unwrap();
    let path = format!("/api/brains/{id}/icon");
    let first = png([43, 103, 85, 120]);
    assert_eq!(
        icon_call(&h, "PUT", &path, Some(&owner), false, first.clone())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        icon_call(&h, "PUT", &path, Some(&member), true, first.clone())
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let response = icon_call(&h, "PUT", &path, Some(&owner), true, first.clone()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let meta: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert!(meta["icon_revision"].is_string());
    assert!(meta.get("icon_png").is_none());
    let get = icon_call(&h, "GET", &path, Some(&owner), false, vec![]).await;
    assert_eq!(get.status(), StatusCode::OK);
    assert_eq!(get.headers()["cache-control"], "no-store");
    assert_eq!(get.headers()["x-content-type-options"], "nosniff");
    assert_eq!(get.headers()["content-type"], "image/png");
    let stored = get.into_body().collect().await.unwrap().to_bytes();
    let decoded = image::load_from_memory(&stored).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (256, 128));
    assert_eq!(decoded.get_pixel(0, 0).0[3], 120);
    assert_eq!(
        icon_call(&h, "GET", &path, None, false, vec![])
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        icon_call(&h, "GET", &path, Some(&member), false, vec![])
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("INSERT INTO brain_grants(brain_id,account_id,role) VALUES($1,$2,'reader')")
        .bind(id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        icon_call(&h, "GET", &path, Some(&member), false, vec![])
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        icon_call(&h, "PUT", &path, Some(&member), true, first)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        h.call("DELETE", &path, Some(&member), Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        icon_call(&h, "PUT", &path, Some(&owner), true, b"<svg/>".to_vec())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        icon_call(&h, "PUT", &path, Some(&owner), true, vec![0; 524289])
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let second = icon_call(&h, "PUT", &path, Some(&owner), true, png([0, 0, 255, 255])).await;
    let second: Value =
        serde_json::from_slice(&second.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_ne!(meta["icon_revision"], second["icon_revision"]);
    for _ in 0..2 {
        assert_eq!(
            h.call("DELETE", &path, Some(&owner), Value::Null).await.0,
            StatusCode::OK
        );
    }
    assert_eq!(
        icon_call(&h, "GET", &path, Some(&owner), false, vec![])
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        icon_call(&h, "PUT", &path, Some(&owner), true, png([255, 0, 0, 255]))
            .await
            .status(),
        StatusCode::OK
    );
    // No image-bearing receipt: only the ordinary Brain create command can exist.
    let image_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM command_receipts WHERE operation LIKE 'brain.icon%'",
    )
    .fetch_one(&h.admin)
    .await
    .unwrap();
    assert_eq!(image_receipts, 0);
    sqlx::query("DELETE FROM brain_grants WHERE brain_id=$1 AND account_id=$2")
        .bind(id)
        .bind(member_id)
        .execute(&h.admin)
        .await
        .unwrap();
    assert_eq!(
        icon_call(&h, "GET", &path, Some(&member), false, vec![])
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let (_, preview, _) = h
        .call(
            "POST",
            &format!("/api/brains/{id}/deletions/preview"),
            Some(&owner),
            json!({}),
        )
        .await;
    let (status, response, _) = h
        .call(
            "DELETE",
            &format!("/api/brains/{id}"),
            Some(&owner),
            json!({"closure":preview["closure"],"confirmation":"Icon proof"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM brains WHERE id=$1)")
        .bind(id)
        .fetch_one(&h.admin)
        .await
        .unwrap();
    assert!(!exists);
    assert_eq!(
        icon_call(&h, "GET", &path, Some(&owner), false, vec![])
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    // Simulate the pre-deletion Brain row in a restored database, then run the
    // actual retained-journal SQL replay. Roll back this simulation afterwards.
    let entry: Value = sqlx::query_scalar("SELECT to_jsonb(p)||jsonb_build_object('installation_id',i.id) FROM privacy_requests p CROSS JOIN privacy_installation i WHERE p.brain_id=$1").bind(id).fetch_one(&h.admin).await.unwrap();
    let actor = Uuid::parse_str(brain["owner_id"].as_str().unwrap()).unwrap();
    let mut restore = h.admin.begin().await.unwrap();
    sqlx::query("DELETE FROM brain_tombstones WHERE brain_id=$1")
        .bind(id)
        .execute(&mut *restore)
        .await
        .unwrap();
    sqlx::query("DELETE FROM brain_deletions WHERE brain_id=$1")
        .bind(id)
        .execute(&mut *restore)
        .await
        .unwrap();
    sqlx::query("DELETE FROM privacy_requests WHERE brain_id=$1")
        .bind(id)
        .execute(&mut *restore)
        .await
        .unwrap();
    sqlx::query("INSERT INTO brains(id,owner_id,name,icon_png,icon_revision) VALUES($1,$2,'Restored icon',$3,gen_random_uuid())").bind(id).bind(actor).bind(png([255,0,0,255])).execute(&mut *restore).await.unwrap();
    sqlx::query("SELECT recollect_privacy_replay($1)")
        .bind(entry)
        .execute(&mut *restore)
        .await
        .unwrap();
    let restored_icon: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM brains WHERE id=$1)")
        .bind(id)
        .fetch_one(&mut *restore)
        .await
        .unwrap();
    assert!(!restored_icon);
    restore.rollback().await.unwrap();
    h.finish().await;
}
