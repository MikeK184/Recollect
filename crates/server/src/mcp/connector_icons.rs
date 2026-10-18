//! Optional identity decoration. No icon request participates in credentials,
//! execution authority or connection success.
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageFormat, ImageReader, Limits};
use recollect_mcp_runtime::discovery::resolve_public;
use std::{io::Cursor, time::Duration};

const LIMIT: usize = 256 * 1024;
const PREFIX: &str = "data:image/png;base64,";

fn normalized(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() > LIMIT {
        return None;
    }
    let format = image::guess_format(bytes).ok()?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) {
        return None;
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(512);
    limits.max_image_height = Some(512);
    limits.max_alloc = Some(4 * 1024 * 1024);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let pixels = reader.decode().ok()?.thumbnail(256, 256).to_rgba8();
    let mut output = Cursor::new(Vec::new());
    pixels.write_to(&mut output, ImageFormat::Png).ok()?;
    let output = output.into_inner();
    (output.len() <= LIMIT).then_some(output)
}

pub(super) fn valid_cached(value: &str) -> bool {
    let Some(encoded) = value.strip_prefix(PREFIX) else {
        return false;
    };
    if encoded.len() > LIMIT.div_ceil(3) * 4 {
        return false;
    }
    let Ok(bytes) = STANDARD.decode(encoded) else {
        return false;
    };
    normalized(&bytes).is_some_and(|png| png == bytes)
}

fn permitted_url(target: &reqwest::Url, source: &str) -> Option<reqwest::Url> {
    let url = target.join(source).ok()?;
    if target.scheme() != "https"
        || url.scheme() != "https"
        || url.origin() != target.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
        || source.len() > 2048
    {
        return None;
    }
    Some(url)
}

async fn candidate(target: &reqwest::Url, source: &str) -> Option<Vec<u8>> {
    if source.starts_with("data:") {
        let (kind, encoded) = source.split_once(',')?;
        if !matches!(
            kind,
            "data:image/png;base64"
                | "data:image/jpeg;base64"
                | "data:image/jpg;base64"
                | "data:image/webp;base64"
        ) || encoded.len() > LIMIT.div_ceil(3) * 4
        {
            return None;
        }
        let bytes = STANDARD.decode(encoded).ok()?;
        let format = image::guess_format(&bytes).ok()?;
        if !matches!(
            (kind, format),
            ("data:image/png;base64", ImageFormat::Png)
                | (
                    "data:image/jpeg;base64" | "data:image/jpg;base64",
                    ImageFormat::Jpeg
                )
                | ("data:image/webp;base64", ImageFormat::WebP)
        ) {
            return None;
        }
        return normalized(&bytes);
    }
    let url = permitted_url(target, source)?;
    let host = url.host_str()?;
    let addresses = resolve_public(host, url.port_or_known_default()?).await?;
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(3))
        .resolve_to_addrs(host, &addresses)
        .build()
        .ok()?;
    let mut response = http
        .get(url)
        .header(reqwest::header::ACCEPT, "image/png, image/jpeg, image/webp")
        .send()
        .await
        .ok()?;
    if !response.status().is_success()
        || response.content_length().is_some_and(|n| n > LIMIT as u64)
    {
        return None;
    }
    let kind = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)?
        .to_str()
        .ok()?
        .split(';')
        .next()?
        .trim()
        .to_owned();
    if !matches!(
        kind.as_str(),
        "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
    ) {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > LIMIT {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    let format = image::guess_format(&bytes).ok()?;
    if !matches!(
        (kind.as_str(), format),
        ("image/png", ImageFormat::Png)
            | ("image/jpeg" | "image/jpg", ImageFormat::Jpeg)
            | ("image/webp", ImageFormat::WebP)
    ) {
        return None;
    }
    normalized(&bytes)
}

pub(super) async fn inspect(target: &str, sources: &[String]) -> Option<String> {
    let target = reqwest::Url::parse(target).ok()?;
    tokio::time::timeout(Duration::from_secs(6), async {
        for source in sources.iter().take(3) {
            if let Some(bytes) = candidate(&target, source).await {
                return Some(format!("{PREFIX}{}", STANDARD.encode(bytes)));
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use recollect_mcp_runtime::discovery::is_public_address as public;
    #[test]
    fn private_reserved_and_mapped_addresses_never_fetch() {
        for value in [
            "0.0.0.0",
            "10.2.3.4",
            "127.0.0.1",
            "100.64.2.3",
            "169.254.169.254",
            "172.31.1.1",
            "192.168.1.1",
            "192.0.2.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.1.1.1",
            "::1",
            "::ffff:8.8.8.8",
            "fe80::1",
            "fc00::1",
            "2001:db8::1",
            "2002:808:808::1",
            "3fff::1",
        ] {
            assert!(!public(value.parse().unwrap()), "{value}");
        }
        for value in [
            "8.8.8.8",
            "1.1.1.1",
            "2001:4860:4860::8888",
            "2606:4700:4700::1111",
        ] {
            assert!(public(value.parse().unwrap()), "{value}");
        }
    }
    #[test]
    fn targets_have_exact_public_https_origin_and_no_routing_values() {
        let target = reqwest::Url::parse("https://example.com/mcp").unwrap();
        assert!(permitted_url(&target, "/icon.png").is_some());
        for src in [
            "http://example.com/icon.png",
            "https://elsewhere.example/icon.png",
            "https://user:pass@example.com/icon.png",
            "/icon.png?token=secret",
            "/icon.png#secret",
            "file:///icon.png",
        ] {
            assert!(permitted_url(&target, src).is_none(), "{src}");
        }
    }
    #[test]
    fn raster_normalization_strips_metadata_and_rejects_active_content() {
        let mut source = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(512, 256, image::Rgba([20, 80, 50, 128]))
            .write_to(&mut source, ImageFormat::Png)
            .unwrap();
        let png = normalized(source.get_ref()).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 256);
        assert!(valid_cached(&format!("{PREFIX}{}", STANDARD.encode(png))));
        assert!(normalized(b"<svg onload='run()'/>").is_none());
        assert!(normalized(&vec![0; LIMIT + 1]).is_none());
        let mut oversized = Cursor::new(Vec::new());
        image::RgbaImage::new(513, 1)
            .write_to(&mut oversized, ImageFormat::Png)
            .unwrap();
        assert!(normalized(oversized.get_ref()).is_none());
    }
    #[tokio::test]
    async fn rejected_icons_do_not_require_or_make_a_network_request() {
        assert!(
            inspect(
                "http://127.0.0.1:1/mcp",
                &[
                    "http://127.0.0.1:1/image.png".into(),
                    "data:image/svg+xml;base64,PHN2Zy8+".into()
                ]
            )
            .await
            .is_none()
        );
    }
}
