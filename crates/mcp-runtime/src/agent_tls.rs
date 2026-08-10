//! Explicit client-local trust for the Recollect endpoint, never a TLS bypass.
use crate::{Result, RuntimeError};
use std::{io::Read, path::PathBuf};

pub fn configured_path() -> Result<Option<PathBuf>> {
    std::env::var_os("RECOLLECT_CA_FILE")
        .map(|path| {
            let path = PathBuf::from(path)
                .canonicalize()
                .map_err(|_| RuntimeError("agent_ca_file_unavailable"))?;
            if !path.is_file() {
                return Err(RuntimeError("agent_ca_file_invalid"));
            }
            Ok(path)
        })
        .transpose()
}

pub fn configured_pem() -> Result<Option<Vec<u8>>> {
    let Some(path) = configured_path()? else {
        return Ok(None);
    };
    let file = std::fs::File::open(path).map_err(|_| RuntimeError("agent_ca_file_unavailable"))?;
    let mut pem = Vec::new();
    file.take(256 * 1024 + 1)
        .read_to_end(&mut pem)
        .map_err(|_| RuntimeError("agent_ca_file_unavailable"))?;
    if pem.is_empty() || pem.len() > 256 * 1024 {
        return Err(RuntimeError("agent_ca_file_invalid"));
    }
    Ok(Some(pem))
}
