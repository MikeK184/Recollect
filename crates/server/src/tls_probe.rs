//! `recollect-server tls-probe [URL]`: proves the server's outbound HTTPS trust
//! path without configuration, a database or a credential.
//!
//! The probe builds the same client as [`crate::AppState`] ([`crate::http_client`])
//! and requests one HTTPS URL (an unauthenticated OpenAI model list by default).
//! Any HTTP response, including `401`, proves that name resolution, TCP, the TLS
//! handshake and certificate verification against the system trust store (and
//! `SSL_CERT_FILE` / `SSL_CERT_DIR` on Linux) all worked. It prints one JSON line
//! with no URL, header, body or error text and exits 0 on success, 1 on failure.
use serde::Serialize;
use std::{error::Error, io};

pub const DEFAULT_URL: &str = "https://api.openai.com/v1/models";

/// The furthest stage reached: a failure names the stage that did not complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// The client (and so the root store) could not be built.
    Roots,
    /// Name resolution, TCP connection, or no response before the timeout.
    Connect,
    /// The TLS handshake or certificate verification was rejected.
    Handshake,
    /// TLS was established; an HTTP response arrived or the exchange failed after it.
    Http,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Ok,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    pub tls: Outcome,
    pub stage: Stage,
    pub status: Option<u16>,
}

impl Report {
    pub fn ok(status: u16) -> Self {
        Self {
            tls: Outcome::Ok,
            stage: Stage::Http,
            status: Some(status),
        }
    }
    pub fn failed(stage: Stage) -> Self {
        Self {
            tls: Outcome::Failed,
            stage,
            status: None,
        }
    }
    pub fn succeeded(&self) -> bool {
        self.tls == Outcome::Ok
    }
    /// The single JSON line the command prints.
    pub fn line(&self) -> String {
        serde_json::to_string(self).expect("a probe report always serializes")
    }
}

/// Text that a TLS failure leaves in the error chain. rustls reports through
/// `io::Error`, so its error type is not visible to a downcast.
const TLS_MARKERS: [&str; 6] = [
    "certificate",
    "tls",
    "handshake",
    "alert",
    "rustls",
    "corrupt message",
];

/// Separates a TLS failure from an ordinary connection failure in the chain of
/// a reqwest connect error.
pub fn classify_connect(error: &(dyn Error + 'static)) -> Stage {
    let mut next = Some(error);
    while let Some(current) = next {
        let text = current.to_string().to_ascii_lowercase();
        let io_kind = current.downcast_ref::<io::Error>().map(io::Error::kind);
        if TLS_MARKERS.iter().any(|marker| text.contains(marker))
            || matches!(
                io_kind,
                Some(io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof)
            )
        {
            return Stage::Handshake;
        }
        next = current.source();
    }
    Stage::Connect
}

fn stage_of(error: &reqwest::Error) -> Stage {
    if error.is_connect() {
        classify_connect(error)
    } else if error.is_timeout() {
        // No response arrived in time, so nothing past connecting is proven.
        Stage::Connect
    } else {
        Stage::Http
    }
}

/// Accepts only an HTTPS URL with a host; anything else proves nothing about TLS.
pub fn target(argument: &str) -> Option<reqwest::Url> {
    reqwest::Url::parse(argument)
        .ok()
        .filter(|url| url.scheme() == "https" && url.host_str().is_some())
}

pub async fn probe(url: reqwest::Url) -> Report {
    let Ok(client) = crate::http_client().build() else {
        return Report::failed(Stage::Roots);
    };
    match client.get(url).send().await {
        Ok(response) => Report::ok(response.status().as_u16()),
        Err(error) => Report::failed(stage_of(&error)),
    }
}

/// Runs the command and returns its exit status: 0 ok, 1 failed, 2 unusable URL.
pub async fn command(argument: Option<String>) -> i32 {
    let Some(url) = target(argument.as_deref().unwrap_or(DEFAULT_URL)) else {
        eprintln!("tls-probe expects an https:// URL");
        return 2;
    };
    let report = probe(url).await;
    println!("{}", report.line());
    i32::from(!report.succeeded())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct Layer(&'static str, Option<Box<dyn Error + 'static>>);
    impl fmt::Display for Layer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }
    impl Error for Layer {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            self.1.as_deref()
        }
    }
    fn chain(top: &'static str, inner: impl Error + 'static) -> Layer {
        Layer(top, Some(Box::new(inner)))
    }

    #[test]
    fn report_is_one_json_line_without_text_from_the_peer() {
        assert_eq!(
            Report::ok(401).line(),
            r#"{"tls":"ok","stage":"http","status":401}"#
        );
        for (stage, name) in [
            (Stage::Roots, "roots"),
            (Stage::Connect, "connect"),
            (Stage::Handshake, "handshake"),
            (Stage::Http, "http"),
        ] {
            let report = Report::failed(stage);
            assert_eq!(
                report.line(),
                format!(r#"{{"tls":"failed","stage":"{name}","status":null}}"#)
            );
            assert!(!report.succeeded());
        }
        assert!(Report::ok(200).succeeded());
    }

    #[test]
    fn tls_failures_are_separated_from_connection_failures() {
        let refused = chain(
            "client error (Connect)",
            io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused"),
        );
        assert_eq!(classify_connect(&refused), Stage::Connect);
        let lookup = chain(
            "client error (Connect)",
            Layer(
                "dns error",
                Some(Box::new(Layer("failed to lookup address", None))),
            ),
        );
        assert_eq!(classify_connect(&lookup), Stage::Connect);
        let reset = chain(
            "client error (Connect)",
            io::Error::new(io::ErrorKind::ConnectionReset, "Connection reset by peer"),
        );
        assert_eq!(classify_connect(&reset), Stage::Connect);
        for text in [
            "invalid peer certificate: UnknownIssuer",
            "received fatal alert: HandshakeFailure",
            "tls handshake eof",
            "received corrupt message of type InvalidContentType",
        ] {
            let rejected = chain("client error (Connect)", io::Error::other(text));
            assert_eq!(classify_connect(&rejected), Stage::Handshake, "{text}");
        }
        let eof = chain(
            "client error (Connect)",
            io::Error::new(io::ErrorKind::UnexpectedEof, "peer closed"),
        );
        assert_eq!(classify_connect(&eof), Stage::Handshake);
    }

    #[test]
    fn only_an_https_url_with_a_host_is_probed() {
        assert!(target(DEFAULT_URL).is_some());
        assert!(target("https://127.0.0.1:8443/health/ready").is_some());
        for refused in [
            "http://example.com/",
            "ftp://example.com/",
            "example.com",
            "https://",
            "",
        ] {
            assert!(target(refused).is_none(), "{refused}");
        }
    }

    /// A real refused connection, through the shared client, on any platform.
    #[tokio::test]
    async fn a_refused_connection_fails_at_connect() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let report = probe(target(&format!("https://127.0.0.1:{port}/")).unwrap()).await;
        assert_eq!(report, Report::failed(Stage::Connect));
    }

    /// A peer that answers the handshake with plaintext fails at the handshake,
    /// not at connect and not at the HTTP layer.
    #[tokio::test]
    async fn a_peer_that_does_not_speak_tls_fails_at_the_handshake() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut hello = [0u8; 512];
                let _ = stream.read(&mut hello).await;
                let _ = stream
                    .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                    .await;
            }
        });
        let report = probe(target(&format!("https://127.0.0.1:{port}/")).unwrap()).await;
        assert_eq!(report, Report::failed(Stage::Handshake));
    }
}
