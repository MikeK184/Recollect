use recollect_mcp_runtime::agent_transport;
use uuid::Uuid;

#[tokio::test]
#[ignore = "Owned TLS fixture; run scripts/test-installation-local.sh"]
async fn explicit_tls_roots_preserve_native_mcp_verification() {
    let endpoint = std::env::var("RECOLLECT_TEST_TLS_URL").expect("Use the owned TLS fixture");
    assert!(
        endpoint.starts_with("https://localhost:") || endpoint.starts_with("https://127.0.0.1:")
    );
    let expected = std::env::var("RECOLLECT_TEST_TLS_EXPECTED").unwrap() == "accepted";
    let result = agent_transport::connect(&endpoint, Uuid::new_v4(), Uuid::new_v4()).await;
    assert_eq!(
        result.is_ok(),
        expected,
        "TLS admission differed from the fixture expectation"
    );
    if let Ok(service) = result {
        assert!(service.peer().list_all_tools().await.unwrap().is_empty());
        service.cancel().await.unwrap();
    }
}
