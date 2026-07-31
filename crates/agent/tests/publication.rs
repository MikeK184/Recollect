use chrono::Utc;
use recollect_agent::publication::*;
use recollect_protocol::*;
use std::path::Path;
use tokio::{fs, process::Command};
use uuid::Uuid;

async fn git(path: &Path, args: &[&str]) -> String {
    let result = Command::new("git")
        .current_dir(path)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Recollect fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_NAME", "Recollect fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .await
        .unwrap();
    assert!(result.status.success(), "Fixture Git command failed");
    String::from_utf8(result.stdout).unwrap().trim().into()
}
#[tokio::test]
async fn exact_committed_extraction_and_resumable_bundle() {
    let root = project_root()
        .join(".cache")
        .join(format!("publication-test-{}", Uuid::new_v4()));
    let checkout = root.join("checkout");
    fs::create_dir_all(checkout.join("src")).await.unwrap();
    for dir in ["web", "deploy", "modules/dns", ".recollect", "auth"] {
        fs::create_dir_all(checkout.join(dir)).await.unwrap();
    }
    git(&checkout, &["init", "-q"]).await;
    git(
        &checkout,
        &[
            "remote",
            "add",
            "origin",
            "https://example.test/team/fixture.git",
        ],
    )
    .await;
    let first_source = "pub fn welcome() { greet(); }\npub fn greet() {}\n";
    for (path, text) in [
        ("src/lib.rs", first_source),
        (
            "Cargo.toml",
            "[package]\nname = 'fixture'\nversion = '0.1.0'\n",
        ),
        (".gitattributes", "src/lib.rs export-ignore\n"),
        (
            "package.json",
            r#"{"name":"fixture","dependencies":{"express":"^5"}}"#,
        ),
        (
            "web/api.ts",
            "import express from 'express';\nconst app = express();\napp.get('/hello', (req, res) => res.send('hello'));\n",
        ),
        (
            "main.tf",
            "variable \"name\" { type = string }\nmodule \"dns\" { source = \"./modules/dns\" }\nresource \"terraform_data\" \"demo\" { input = var.name }\n\nmodule \"remote\" {\n  /* source = \"git::https://decoy.test/false.git?ref=main\" */\n  source = \"git::https://example.test/team/dns.git//modules/dns?ref=1111111111111111111111111111111111111111\"\n}\n",
        ),
        (
            "auth/credential.tf",
            "module \"excluded\" { source = \"git::https://synthetic-user@example.test/private.git?ref=main\" }\n",
        ),
        (
            "modules/dns/main.tf",
            "output \"name\" { value = \"fixture\" }\n",
        ),
        (
            "deploy/app.yaml",
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: fixture\n",
        ),
        (".env", "DO_NOT_CAPTURE=fixture\n"),
        (".recollect/private.txt", "excluded tool input\n"),
        ("terraform.tfstate", "excluded state\n"),
        ("secret.txt", "password = 'synthetic-test-only'\n"),
        ("binary.dat", "not-text\0fixture"),
        ("odd.unknown", "unsupported fixture\n"),
    ] {
        fs::write(checkout.join(path), text).await.unwrap();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink("../outside", checkout.join("outside-link")).unwrap();
    fs::write(
        checkout.join("oversized.txt"),
        "x".repeat(REPOSITORY_TEXT_MAX_BYTES + 1),
    )
    .await
    .unwrap();
    git(&checkout, &["add", "."]).await;
    git(&checkout, &["commit", "-qm", "First fixture revision"]).await;
    let initial = git(&checkout, &["rev-parse", "HEAD"]).await;
    git(
        &checkout,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{initial},vendor"),
        ],
    )
    .await;
    git(&checkout, &["commit", "-qm", "Record gitlink"]).await;
    let first = git(&checkout, &["rev-parse", "HEAD"]).await;
    fs::write(checkout.join("src/lib.rs"), "pub fn next_revision() {}\n")
        .await
        .unwrap();
    git(&checkout, &["add", "src/lib.rs"]).await;
    git(&checkout, &["commit", "-qm", "Second fixture revision"]).await;
    let second = git(&checkout, &["rev-parse", "HEAD"]).await;
    fs::write(checkout.join("src/lib.rs"), "dirty_only_marker\n")
        .await
        .unwrap();
    // A local replacement must not redefine bytes attributed to the original commit.
    git(&checkout, &["replace", &first, &second]).await;
    let index_before = fs::read(checkout.join(".git/index")).await.unwrap();
    let config_before = fs::read(checkout.join(".git/config")).await.unwrap();
    let now = Utc::now();
    let repository = Repository {
        id: Uuid::new_v4(),
        brain_id: Uuid::new_v4(),
        canonical_origin: "example.test/team/fixture".into(),
        origins: vec!["example.test/team/fixture".into()],
        created_at: now,
    };
    let task = Uuid::new_v4();
    let operation = OperationBinding {
        id: Uuid::new_v4(),
        brain_id: repository.brain_id,
        task_id: task,
        scope: ScopeSnapshot {
            id: Uuid::new_v4(),
            task_id: task,
            brain_id: repository.brain_id,
            selection: ScopeSelection {
                repository_ids: vec![repository.id],
                ..Default::default()
            },
            repositories: vec![],
            areas: vec![],
            environment: None,
            created_at: now,
        },
        scope_valid: true,
        kind: "capture".into(),
        actor_id: Uuid::new_v4(),
        device_id: Some(Uuid::new_v4()),
        created_at: now,
    };
    let stage = root.join("bundles");
    let binary = default_enola();
    assert!(
        binary.is_file(),
        "Run python3 scripts/setup-enola.py before the native integration fixture"
    );
    let options = |revision, allowed, retained| Preparation {
        checkout: &checkout,
        revision,
        repository: &repository,
        operation: &operation,
        retain_files: retained,
        allow_file_content: allowed,
        enola: &binary,
        configured_secrets: vec![],
        staging_root: &stage,
    };
    assert!(
        prepare(options(&first, false, vec!["src/lib.rs".into()]))
            .await
            .is_err()
    );
    let input = prepare(options(&first, true, vec!["src/lib.rs".into()]))
        .await
        .unwrap();
    assert_eq!(input.revision, first);
    assert!(input.dirty);
    let source = input.files.iter().find(|f| f.path == "src/lib.rs").unwrap();
    assert_eq!(source.content.as_deref(), Some(first_source));
    let facts = serde_json::to_string(&input.facts).unwrap();
    for marker in ["greet", "welcome", "express", "terraform_data"] {
        assert!(facts.contains(marker), "Missing supported fact");
    }
    assert!(!facts.contains("dirty_only_marker"));
    assert!(!facts.contains("next_revision"));
    let remote = input
        .facts
        .iter()
        .find(|f| f["name"] == "module.remote")
        .unwrap();
    let witness = &remote["props"]["recollect_module_source"];
    assert_eq!(witness["state"], "verified_literal");
    assert_eq!(witness["target"]["origin"], "example.test/team/dns");
    assert_eq!(witness["target"]["directory"], "modules/dns");
    assert_eq!(witness["line_from"], 5);
    assert_eq!(remote["line"], 4);
    let directory = input
        .facts
        .iter()
        .find(|f| f["kind"] == "module" && f["name"] == "modules/dns")
        .unwrap();
    assert_eq!(
        directory["props"]["recollect_hcl_directory"]["state"],
        "parsed"
    );
    assert!(
        input
            .files
            .iter()
            .filter(|f| f.path.ends_with(".tf"))
            .all(|f| f.content.is_none())
    );
    assert!(input.facts.iter().any(|f| {
        f.get("relations")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|r| r.iter().any(|r| r.get("target_id").is_none()))
    }));
    for (path, status) in [
        (".env", "excluded_path"),
        ("secret.txt", "excluded_secret"),
        ("auth/credential.tf", "excluded_secret"),
        ("binary.dat", "binary"),
        ("terraform.tfstate", "excluded_path"),
        ("oversized.txt", "oversized"),
        ("vendor", "gitlink"),
    ] {
        assert_eq!(
            input.files.iter().find(|f| f.path == path).unwrap().status,
            status
        );
    }
    #[cfg(unix)]
    assert_eq!(
        input
            .files
            .iter()
            .find(|f| f.path == "outside-link")
            .unwrap()
            .status,
        "symlink"
    );
    assert_eq!(
        input
            .files
            .iter()
            .find(|f| f.path == "deploy/app.yaml")
            .unwrap()
            .extraction,
        "no_facts"
    );
    assert!(input.receipt.get("repo_path").is_none());
    assert!(input.receipt.get("snapshot_id").is_none());
    let repeated = prepare(options(&first, true, vec!["src/lib.rs".into()]))
        .await
        .unwrap();
    assert_eq!(input.facts, repeated.facts);
    assert_eq!(input.insights, repeated.insights);
    assert_eq!(input.receipt, repeated.receipt);
    assert_eq!(input.files, repeated.files);
    let later = prepare(options("HEAD", false, vec![])).await.unwrap();
    assert_eq!(later.revision, second);
    assert!(
        serde_json::to_string(&later.facts)
            .unwrap()
            .contains("next_revision")
    );
    assert!(
        prepare(options(&first, true, vec![".env".into()]))
            .await
            .is_err()
    );
    assert!(prepare(options("--help", false, vec![])).await.is_err());
    let mut broken = options(&first, false, vec![]);
    broken.enola = Path::new("/usr/bin/false");
    assert!(prepare(broken).await.is_err());
    let bundle = PreparedPublication {
        endpoint: "http://127.0.0.1:8787".into(),
        device_id: operation.device_id.unwrap(),
        brain_id: repository.brain_id,
        repository_id: repository.id,
        input,
    };
    let saved = save_bundle(&stage, &bundle).await.unwrap();
    assert!(save_bundle(&stage, &bundle).await.is_err());
    let loaded = load_bundle(&stage, bundle.input.publication_id)
        .await
        .unwrap();
    assert_eq!(loaded.input.operation_id, operation.id);
    assert_eq!(loaded.input.facts, bundle.input.facts);
    assert!(
        !fs::read_to_string(&saved)
            .await
            .unwrap()
            .contains("synthetic-test-only")
    );
    remove_bundle(&stage, bundle.input.publication_id)
        .await
        .unwrap();
    assert!(!saved.exists());
    assert!(
        fs::read_dir(&stage)
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fs::read(checkout.join(".git/index")).await.unwrap(),
        index_before
    );
    assert_eq!(
        fs::read(checkout.join(".git/config")).await.unwrap(),
        config_before
    );
    assert_eq!(
        fs::read_to_string(checkout.join("src/lib.rs"))
            .await
            .unwrap(),
        "dirty_only_marker\n"
    );
    // Path rejection and missing promisor objects never fall back to working bytes or a fetch.
    fs::write(checkout.join("bad\nname.rs"), "pub fn unsafe_path() {}\n")
        .await
        .unwrap();
    git(&checkout, &["add", "bad\nname.rs"]).await;
    git(&checkout, &["commit", "-qm", "Invalid path fixture"]).await;
    assert!(prepare(options("HEAD", false, vec![])).await.is_err());
    git(&checkout, &["config", "remote.origin.promisor", "true"]).await;
    git(
        &checkout,
        &["config", "remote.origin.partialclonefilter", "blob:none"],
    )
    .await;
    let object = &bundle
        .input
        .files
        .iter()
        .find(|f| f.path == "src/lib.rs")
        .unwrap()
        .object_id;
    let object_path = checkout
        .join(".git/objects")
        .join(&object[..2])
        .join(&object[2..]);
    let committed_blob = fs::read(&object_path).await.unwrap();
    fs::remove_file(&object_path).await.unwrap();
    let start = std::time::Instant::now();
    assert!(prepare(options(&first, false, vec![])).await.is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
    fs::write(&object_path, committed_blob).await.unwrap();
    // Keep this owned fixture available for inspection.
    println!("Native committed extraction proof: {}", root.display());
}
