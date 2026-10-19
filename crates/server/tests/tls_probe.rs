//! `recollect-server tls-probe` against a local HTTPS listener signed by a throwaway CA.
//!
//! Linux only: the root store there is the system file or directory, which
//! `SSL_CERT_FILE` replaces. macOS and Windows verify through the operating
//! system and ignore it, so the stage classification is unit-tested there
//! instead (`src/tls_probe.rs`). The CA and leaf come from the `openssl` command
//! line, as in the repository's other HTTPS fixtures.
#![cfg(target_os = "linux")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        crypto::aws_lc_rs,
        pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
    },
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("recollect-tls-probe-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn openssl(directory: &Path, arguments: &[&str]) {
    let output = Command::new("openssl")
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("the openssl command line is required for this fixture");
    assert!(
        output.status.success(),
        "openssl {} failed",
        arguments.first().unwrap_or(&"")
    );
}

/// Writes `<name>-ca.pem` and, when `leaf` is set, a localhost server certificate
/// and key signed by it.
fn authority(directory: &Path, name: &str, leaf: bool) {
    let (ca, ca_key) = (format!("{name}-ca.pem"), format!("{name}-ca.key"));
    openssl(
        directory,
        &[
            "req",
            "-x509",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:prime256v1",
            "-nodes",
            "-days",
            "1",
            "-subj",
            &format!("/CN=Recollect tls-probe {name} CA"),
            "-addext",
            "basicConstraints=critical,CA:TRUE,pathlen:0",
            "-addext",
            "keyUsage=critical,keyCertSign,cRLSign",
            "-keyout",
            &ca_key,
            "-out",
            &ca,
        ],
    );
    if !leaf {
        return;
    }
    openssl(
        directory,
        &[
            "req",
            "-new",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:prime256v1",
            "-nodes",
            "-subj",
            "/CN=localhost",
            "-keyout",
            "server.key",
            "-out",
            "server.csr",
        ],
    );
    fs::write(
        directory.join("extensions.cnf"),
        "subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\n",
    )
    .unwrap();
    openssl(
        directory,
        &[
            "x509",
            "-req",
            "-in",
            "server.csr",
            "-CA",
            &ca,
            "-CAkey",
            &ca_key,
            "-CAcreateserial",
            "-days",
            "1",
            "-extfile",
            "extensions.cnf",
            "-out",
            "server.pem",
        ],
    );
}

/// Serves an unauthenticated `401` over TLS on a loopback port.
async fn serve(directory: &Path) -> u16 {
    let chain = CertificateDer::pem_file_iter(directory.join("server.pem"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let key = PrivateKeyDer::from_pem_file(directory.join("server.key")).unwrap();
    let config = ServerConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                // A client that rejects the certificate abandons the handshake.
                let Ok(mut stream) = acceptor.accept(stream).await else {
                    return;
                };
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request).await;
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
                let _ = stream.shutdown().await;
            });
        }
    });
    port
}

/// Runs the real binary with an otherwise empty environment: the probe needs no
/// configuration, database or credential.
async fn probe(url: String, certificates: Option<&Path>) -> (Option<i32>, String) {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_recollect-server"));
    command
        .args(["tls-probe", &url])
        .env_clear()
        .kill_on_drop(true);
    if let Some(path) = certificates {
        command.env("SSL_CERT_FILE", path);
    }
    let output = command.output().await.unwrap();
    (
        output.status.code(),
        String::from_utf8(output.stdout).unwrap().trim().to_owned(),
    )
}

#[tokio::test]
async fn the_probe_trusts_exactly_the_roots_the_system_store_names() {
    let scratch = Scratch::new();
    authority(&scratch.0, "trusted", true);
    authority(&scratch.0, "other", false);
    let url = format!("https://127.0.0.1:{}/v1/models", serve(&scratch.0).await);

    let trusted = scratch.0.join("trusted-ca.pem");
    assert_eq!(
        probe(url.clone(), Some(&trusted)).await,
        (
            Some(0),
            r#"{"tls":"ok","stage":"http","status":401}"#.to_owned()
        ),
        "a 401 over a chain to the selected root proves TLS"
    );

    let other = scratch.0.join("other-ca.pem");
    assert_eq!(
        probe(url.clone(), Some(&other)).await,
        (
            Some(1),
            r#"{"tls":"failed","stage":"handshake","status":null}"#.to_owned()
        ),
        "a different root rejects the same server at the handshake"
    );

    assert_eq!(
        probe(url.clone(), Some(Path::new("/dev/null"))).await,
        (
            Some(1),
            r#"{"tls":"failed","stage":"roots","status":null}"#.to_owned()
        ),
        "an empty root store fails when the client is built"
    );

    // Without the override the system store decides, and it cannot know this CA.
    let (code, line) = probe(url, None).await;
    assert_eq!(code, Some(1));
    assert!(
        line == r#"{"tls":"failed","stage":"handshake","status":null}"#
            || line == r#"{"tls":"failed","stage":"roots","status":null}"#,
        "{line}"
    );
}
