//! Hashed assets never receive the SPA document fallback. An open tab may
//! request an obsolete chunk after a deploy; reloading must fetch current HTML.
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn files(dir: &str) -> Router {
    Router::new()
        .nest_service("/assets", ServeDir::new(format!("{dir}/assets")))
        .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(format!("{dir}/index.html"))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::Request,
        http::{StatusCode, header},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn spa_document_loads_and_missing_assets_do_not_return_html() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.cache")
            .join(format!("web-files-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(dir.join("assets")).await.unwrap();
        tokio::fs::write(dir.join("index.html"), "<!doctype html>Current build")
            .await
            .unwrap();
        tokio::fs::write(dir.join("assets/current.js"), "export const ready = true;")
            .await
            .unwrap();
        let app = files(dir.to_str().unwrap());
        for path in ["/", "/brains/example/explore?view=repositories"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert!(
                String::from_utf8_lossy(&response.into_body().collect().await.unwrap().to_bytes())
                    .contains("Current build")
            );
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/assets/old-chunk.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .is_none_or(|v| v != "text/html")
        );
        assert!(
            !String::from_utf8_lossy(&response.into_body().collect().await.unwrap().to_bytes())
                .contains("Current build")
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/current.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
}
