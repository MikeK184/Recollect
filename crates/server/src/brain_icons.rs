use crate::{
    AppState,
    auth::Auth,
    brains, db,
    error::{Error, Result},
};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use image::{ImageFormat, ImageReader, Limits};
use recollect_protocol::Brain;
use std::io::Cursor;
use uuid::Uuid;

pub const MAX_BYTES: usize = 512 * 1024;

/// Decode only PNG with strict dimensions, then strip all input metadata by
/// re-encoding pixels. SVG/ICO conversion is confined to browser image context.
fn normalize(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::invalid("Upload a PNG icon no larger than 512 KiB."));
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(1024);
    limits.max_image_height = Some(1024);
    limits.max_alloc = Some(8 * 1024 * 1024);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| Error::invalid("The icon is not a valid PNG within 1024 × 1024 pixels."))?;
    let mut output = Cursor::new(Vec::new());
    image
        .thumbnail(256, 256)
        .to_rgba8()
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| Error::invalid("The icon could not be encoded."))?;
    let output = output.into_inner();
    if output.len() > MAX_BYTES {
        return Err(Error::invalid("The icon is too large."));
    }
    Ok(output)
}

#[utoipa::path(get,path="/api/brains/{id}/icon",operation_id="getBrainIcon",params(("id"=Uuid,Path)),responses((status=200,body=String,content_type="image/png"),(status=404,body=recollect_protocol::ApiError)))]
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, false).await?;
    let png: Option<Vec<u8>> = sqlx::query_scalar("SELECT icon_png FROM brains WHERE id=$1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let png = png.ok_or_else(Error::missing)?;
    tx.commit().await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        png,
    )
        .into_response())
}

#[utoipa::path(put,path="/api/brains/{id}/icon",operation_id="putBrainIcon",params(("id"=Uuid,Path)),request_body(content=String,content_type="image/png"),responses((status=200,body=Brain),(status=400,body=recollect_protocol::ApiError),(status=403,body=recollect_protocol::ApiError),(status=413,body=recollect_protocol::ApiError)))]
pub async fn put(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<Brain>> {
    auth.require_browser()?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some("image/png")
    {
        return Err(Error::invalid("The icon must be a PNG image."));
    }
    let png = normalize(&bytes)?;
    let changed = sqlx::query("UPDATE brains SET icon_png=$2,icon_revision=gen_random_uuid(),updated_at=now(),change_id=gen_random_uuid() WHERE id=$1 AND icon_png IS DISTINCT FROM $2").bind(id).bind(png).execute(&mut *tx).await?.rows_affected();
    if changed > 0 {
        db::audit(
            &mut tx,
            auth.user.id,
            id,
            "brain.icon.replace",
            id,
            "updated",
        )
        .await?;
    }
    // No image bytes enter durable command receipts or audit payloads.
    tx.commit().await?;
    brains::get(State(state), auth, Path(id)).await
}

#[utoipa::path(delete,path="/api/brains/{id}/icon",operation_id="removeBrainIcon",params(("id"=Uuid,Path)),responses((status=200,body=Brain),(status=403,body=recollect_protocol::ApiError),(status=404,body=recollect_protocol::ApiError)))]
pub async fn remove(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Brain>> {
    auth.require_browser()?;
    let mut tx = auth.tx(&state.pool).await?;
    db::require_role(&mut tx, id, true).await?;
    let changed = sqlx::query("UPDATE brains SET icon_png=NULL,icon_revision=NULL,updated_at=now(),change_id=gen_random_uuid() WHERE id=$1 AND icon_png IS NOT NULL").bind(id).execute(&mut *tx).await?.rows_affected();
    if changed > 0 {
        db::audit(
            &mut tx,
            auth.user.id,
            id,
            "brain.icon.remove",
            id,
            "removed",
        )
        .await?;
    }
    tx.commit().await?;
    brains::get(State(state), auth, Path(id)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_png_is_bounded_and_preserves_transparency() {
        let mut source = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(512, 256, image::Rgba([30, 80, 65, 128]))
            .write_to(&mut source, ImageFormat::Png)
            .unwrap();
        let normalized =
            normalize(source.get_ref()).unwrap_or_else(|_| panic!("valid PNG rejected"));
        let decoded = image::load_from_memory(&normalized).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (256, 128));
        assert_eq!(decoded.get_pixel(0, 0).0[3], 128);
        assert_eq!(
            normalize(&normalized).unwrap_or_else(|_| panic!("canonical PNG rejected")),
            normalized
        );
    }
    #[test]
    fn rejects_wrong_format_malformed_and_large_dimensions() {
        assert!(normalize(b"<svg onload='evil()'/>").is_err());
        assert!(normalize(b"\x89PNG\r\n\x1a\ninvalid").is_err());
        assert!(normalize(&vec![0; MAX_BYTES + 1]).is_err());
        let mut source = Cursor::new(Vec::new());
        image::RgbaImage::new(1025, 1)
            .write_to(&mut source, ImageFormat::Png)
            .unwrap();
        assert!(normalize(source.get_ref()).is_err());
    }
}
