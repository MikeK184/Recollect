use crate::{
    db,
    error::{Error, Result},
};
use axum::http::{HeaderMap, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub fn key(headers: &HeaderMap) -> Result<Option<String>> {
    headers
        .get("idempotency-key")
        .map(|value| {
            let value = value
                .to_str()
                .map_err(|_| Error::invalid("Idempotency-Key must be printable ASCII."))?;
            if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_graphic())
            {
                return Err(Error::invalid(
                    "Idempotency-Key must contain 1 to 128 printable non-space ASCII characters.",
                ));
            }
            Ok(value.to_string())
        })
        .transpose()
}

pub async fn reserve<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    key: Option<&str>,
    operation: &str,
    input: Value,
) -> Result<Option<T>> {
    let Some(key) = key else {
        return Ok(None);
    };
    sqlx::query("SELECT recollect_forget_expired_receipt($1)")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO command_receipts(actor_id,key,operation,input) VALUES (recollect_actor(),$1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(key).bind(operation).bind(&input).execute(&mut **tx).await?;
    type Receipt = (String, Value, Option<Uuid>, Option<Value>, bool);
    let row: Receipt = sqlx::query_as("SELECT operation,input,brain_id,response,invalidated FROM command_receipts WHERE actor_id=recollect_actor() AND key=$1 FOR UPDATE")
        .bind(key).fetch_optional(&mut **tx).await?.ok_or_else(Error::missing)?;
    if row.4 {
        return Err(Error(
            StatusCode::GONE,
            "command_invalidated",
            "This saved command was removed by retention or erasure. Inspect current state before starting a new request.",
        ));
    }
    if row.0 != operation || row.1 != input {
        return Err(Error(
            StatusCode::CONFLICT,
            "idempotency_conflict",
            "This request key was already used with different input.",
        ));
    }
    if let Some(response) = row.3 {
        if let Some(brain) = row.2 {
            let role = db::require_role(tx, brain, false).await?;
            if !matches!(role.as_str(), "writer" | "admin") {
                return Err(Error::forbidden());
            }
        }
        return serde_json::from_value(response).map(Some).map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "receipt_unreadable",
                "The saved command result could not be read.",
            )
        });
    }
    Ok(None)
}

/// Bind a reserved receipt to its Brain before an operation that removes the
/// Brain. The existing apply chain invalidates receipts by brain_id (key kept,
/// input and response emptied) while the Brain row still exists; a post-deletion
/// write would violate the receipt row-level security policy, which requires a
/// live role for a non-null brain_id.
pub async fn bind_brain(
    tx: &mut Transaction<'_, Postgres>,
    key: Option<&str>,
    brain: Uuid,
) -> Result<()> {
    if let Some(key) = key {
        sqlx::query(
            "UPDATE command_receipts SET brain_id=$2 WHERE actor_id=recollect_actor() AND key=$1",
        )
        .bind(key)
        .bind(brain)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn finish<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    key: Option<&str>,
    brain: Uuid,
    response: &T,
) -> Result<()> {
    if let Some(key) = key {
        let response = serde_json::to_value(response).map_err(|_| {
            Error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "receipt_unwritable",
                "The command result could not be saved.",
            )
        })?;
        sqlx::query("UPDATE command_receipts SET brain_id=$2,response=$3 WHERE actor_id=recollect_actor() AND key=$1")
            .bind(key).bind(brain).bind(response).execute(&mut **tx).await?;
    }
    Ok(())
}
