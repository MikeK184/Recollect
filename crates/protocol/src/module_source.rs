//! A declared Git module destination. Parsing the HCL expression is the
//! companion's responsibility; this shared boundary never resolves remote refs.
use crate::{canonical_origin, git_object_id, repository_path};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GitModuleSource {
    pub origin: String,
    pub revision: String,
    pub directory: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSourceWitness {
    pub parser: String,
    pub state: String,
    pub line_from: u32,
    pub line_to: u32,
    pub target: Option<GitModuleSource>,
}

pub fn git_module_source(source: &str) -> Result<GitModuleSource, &'static str> {
    if source.starts_with("./") || source.starts_with("../") {
        return Err("local_module");
    }
    if source.len() > 2000
        || source.chars().any(|c| c.is_whitespace() || c.is_control())
        || source.contains(['#', '\\', '$', '{', '}'])
    {
        return Err("unsupported_source_literal");
    }
    let source = if let Some(source) = source.strip_prefix("git::") {
        if !source.starts_with("https://") && !source.starts_with("ssh://") {
            return Err("unsupported_source_transport");
        }
        source.to_owned()
    } else if source.starts_with("github.com/") || source.starts_with("bitbucket.org/") {
        format!("https://{source}")
    } else {
        return Err("unsupported_source_transport");
    };
    let (address, query) = source.split_once('?').ok_or("unresolved_source_ref")?;
    let parameters: Vec<_> = url::form_urlencoded::parse(query.as_bytes()).collect();
    if parameters.len() != 1 || parameters[0].0 != "ref" || query.contains('?') {
        return Err("unsupported_source_options");
    }
    let revision = parameters[0].1.as_ref();
    if !git_object_id(revision) {
        return Err("unresolved_source_ref");
    }
    let start = address.find("://").ok_or("unsupported_source_transport")? + 3;
    let (address, directory) = match address[start..].find("//") {
        Some(offset) => {
            let offset = start + offset;
            (&address[..offset], &address[offset + 2..])
        }
        None => (address, "."),
    };
    if address.contains('%')
        || directory.contains('%')
        || (directory != "." && !repository_path(directory))
    {
        return Err("unsupported_source_path");
    }
    let url = url::Url::parse(address).map_err(|_| "unsupported_source_path")?;
    if url.password().is_some() || (url.scheme() != "ssh" && !url.username().is_empty()) {
        return Err("credential_source_excluded");
    }
    let origin = canonical_origin(address).map_err(|_| "unsupported_source_path")?;
    Ok(GitModuleSource {
        origin,
        revision: revision.into(),
        directory: directory.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_module_references_preserve_exact_identity_and_reject_guessing() {
        let revision = "0123456789abcdef0123456789abcdef01234567";
        let target = git_module_source(&format!(
            "git::https://EXAMPLE.test/team/module.git//modules/dns?ref={revision}"
        ))
        .unwrap();
        assert_eq!(target.origin, "example.test/team/module");
        assert_eq!(target.directory, "modules/dns");
        assert_eq!(target.revision, revision);
        assert_eq!(
            git_module_source(&format!(
                "git::ssh://git@example.test/team/module.git//modules/dns?ref={revision}"
            ))
            .unwrap(),
            target
        );
        assert_eq!(
            git_module_source(&format!("github.com/team/module?ref={revision}"))
                .unwrap()
                .origin,
            "github.com/team/module"
        );
        for (source, reason) in [
            ("./modules/dns".into(), "local_module"),
            (
                "hashicorp/consul/aws".into(),
                "unsupported_source_transport",
            ),
            (
                "git::https://example.test/team/module.git".into(),
                "unresolved_source_ref",
            ),
            (
                "git::https://example.test/team/module.git?ref=main".into(),
                "unresolved_source_ref",
            ),
            (
                format!("git::https://example.test/team/module.git?ref={revision}&ref={revision}"),
                "unsupported_source_options",
            ),
            (
                format!("git::https://example.test/team/module.git?ref={revision}&depth=1"),
                "unsupported_source_options",
            ),
            (
                format!("git::https://example.test/team/module.git//../dns?ref={revision}"),
                "unsupported_source_path",
            ),
            (
                format!("git::https://example.test/team/module.git//modules%2Fdns?ref={revision}"),
                "unsupported_source_path",
            ),
            (
                format!("git::https://example.test/team/module.git//?ref={revision}"),
                "unsupported_source_path",
            ),
            (
                format!("git::https://synthetic@example.test/team/module.git?ref={revision}"),
                "credential_source_excluded",
            ),
        ] {
            assert_eq!(git_module_source(&source).unwrap_err(), reason);
        }
    }
}
