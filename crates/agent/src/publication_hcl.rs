//! Bounded syntax evidence supplements Enola; it does not replace extraction or
//! execute Terraform. Only the already-permitted committed bytes reach this code.
use recollect_protocol::{ExtractedFile, ModuleSourceWitness, git_module_source};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ops::ControlFlow,
    time::{Duration, Instant},
};
use tree_sitter::{Node, ParseOptions, Parser};

pub(crate) const PARSER: &str = "tree-sitter-0.27.0/hcl-1.1.0";

#[derive(Default)]
pub(crate) struct FileEvidence {
    state: &'static str,
    modules: Vec<Module>,
}
impl FileEvidence {
    pub(crate) fn module_count(&self) -> usize {
        self.modules.len()
    }
}
struct Module {
    name: String,
    earliest_line: u32,
    witness: ModuleSourceWitness,
}
pub(crate) fn directory(path: &str) -> &str {
    path.rsplit_once('/')
        .map_or(".", |(directory, _)| directory)
}
pub(crate) fn is_hcl(path: &str) -> bool {
    path.ends_with(".tf") || path.ends_with(".hcl")
}
fn children(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    (0..node.named_child_count()).filter_map(move |i| node.named_child(i as u32))
}
fn literal(node: Node<'_>, text: &str) -> Option<String> {
    if node.kind() != "string_lit" {
        return None;
    }
    let raw = text.get(node.byte_range())?;
    // These are HCL template escapes, not JSON escapes. They cannot identify a
    // supported literal repository destination without another evaluation rule.
    if raw.contains("$${") || raw.contains("%%{") {
        return None;
    }
    serde_json::from_str(raw).ok()
}

#[cfg(test)]
fn inspect(path: &str, text: &str) -> FileEvidence {
    inspect_limited(path, text, 100_000)
}
pub(crate) fn inspect_limited(path: &str, text: &str, limit: usize) -> FileEvidence {
    inspect_with_budget(path, text, Duration::from_secs(1), limit)
}
fn inspect_with_budget(path: &str, text: &str, budget: Duration, limit: usize) -> FileEvidence {
    let mut output = FileEvidence {
        state: "source_parse_error",
        ..Default::default()
    };
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_hcl::LANGUAGE.into())
        .is_err()
    {
        output.state = "source_parser_unavailable";
        return output;
    }
    let began = Instant::now();
    let mut progress = |_: &tree_sitter::ParseState| {
        if began.elapsed() >= budget {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let bytes = text.as_bytes();
    let tree = parser.parse_with_options(
        &mut |offset, _| bytes.get(offset..).unwrap_or_default(),
        None,
        Some(ParseOptions::new().progress_callback(&mut progress)),
    );
    let Some(tree) = tree else {
        output.state = "source_parse_timeout";
        return output;
    };
    if began.elapsed() >= budget {
        output.state = "source_parse_timeout";
        return output;
    }
    if tree.root_node().has_error() {
        return output;
    }
    let Some(body) = children(tree.root_node()).find(|n| n.kind() == "body") else {
        output.state = if children(tree.root_node()).all(|n| n.kind() == "comment") {
            "parsed"
        } else {
            "source_not_native_hcl"
        };
        return output;
    };
    output.state = "parsed";
    // Build once; counting all preceding newlines per module is quadratic.
    let mut line_starts = vec![0];
    line_starts.extend(
        bytes
            .iter()
            .enumerate()
            .filter_map(|(i, b)| (*b == b'\n').then_some(i + 1)),
    );
    for block in children(body).filter(|n| n.kind() == "block") {
        if began.elapsed() >= budget {
            output.state = "source_parse_timeout";
            output.modules.clear();
            return output;
        }
        let parts: Vec<_> = children(block).filter(|n| n.kind() != "comment").collect();
        if parts
            .first()
            .is_none_or(|n| n.kind() != "identifier" || text.get(n.byte_range()) != Some("module"))
        {
            continue;
        }
        let labels: Vec<_> = parts
            .iter()
            .skip(1)
            .take_while(|n| n.kind() != "block_start")
            .filter(|n| matches!(n.kind(), "string_lit" | "identifier"))
            .collect();
        let [label] = labels.as_slice() else {
            continue;
        };
        let label = if label.kind() == "identifier" {
            text.get(label.byte_range()).map(str::to_owned)
        } else {
            literal(**label, text)
        };
        let Some(label) = label.filter(|v| !v.is_empty() && v.len() <= 1000) else {
            continue;
        };
        let name = if directory(path) == "." {
            format!("module.{label}")
        } else {
            format!("{}.module.{label}", directory(path))
        };
        let from = block.start_position().row as u32 + 1;
        let prefix = &text[..block.start_byte()];
        let earliest_line = prefix
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_whitespace())
            .map_or(1, |(offset, _)| {
                line_starts.partition_point(|start| *start <= offset) as u32 + 1
            });
        let mut witness = ModuleSourceWitness {
            parser: PARSER.into(),
            state: "source_missing".into(),
            line_from: from,
            line_to: block.end_position().row as u32 + 1,
            target: None,
        };
        if let Some(body) = parts.iter().find(|n| n.kind() == "body") {
            let attributes: Vec<_> = children(*body)
                .filter(|n| n.kind() == "attribute")
                .filter(|n| {
                    children(*n)
                        .next()
                        .is_some_and(|n| text.get(n.byte_range()) == Some("source"))
                })
                .collect();
            witness.state = match attributes.len() {
                0 => "source_missing",
                1 => "source_expression",
                _ => "ambiguous_source_attribute",
            }
            .into();
            if let [attribute] = attributes.as_slice() {
                let value = children(*attribute)
                    .find(|n| n.kind() == "expression")
                    .and_then(|n| children(n).find(|c| c.kind() != "comment"))
                    .filter(|n| n.kind() == "literal_value")
                    .and_then(|n| children(n).next())
                    .and_then(|n| literal(n, text));
                if let Some(source) = value {
                    match git_module_source(&source) {
                        Ok(target) => {
                            witness.state = "verified_literal".into();
                            witness.target = Some(target);
                        }
                        Err(reason) => witness.state = reason.into(),
                    }
                }
            }
        }
        output.modules.push(Module {
            name,
            earliest_line,
            witness,
        });
        if output.modules.len() > limit {
            // Keep only limit+1 records so the publication rejects its aggregate
            // count; do not continue walking a file that cannot be admitted.
            output.state = "source_module_limit";
            return output;
        }
    }
    if began.elapsed() >= budget {
        output.state = "source_parse_timeout";
        output.modules.clear();
    }
    output
}

pub(crate) fn attach(
    facts: &mut [Value],
    files: &[ExtractedFile],
    scans: &BTreeMap<String, FileEvidence>,
) {
    let mut unavailable = BTreeMap::<String, &str>::new();
    let mut labels = BTreeMap::<String, usize>::new();
    let mut source_facts = BTreeMap::<(String, String), usize>::new();
    let mut declarations = BTreeMap::<(&str, &str), Vec<&Module>>::new();
    let directories: std::collections::BTreeSet<_> = scans.keys().map(|p| directory(p)).collect();
    for file in files {
        let name = file.path.rsplit('/').next().unwrap_or("");
        let reason = if name == "override.tf"
            || name.ends_with("_override.tf")
            || name == "override.tf.json"
            || name.ends_with("_override.tf.json")
        {
            Some("module_override_unsupported")
        } else if name.ends_with(".tf.json") {
            Some("terraform_json_unexamined")
        } else if is_hcl(&file.path) {
            Some(
                scans
                    .get(&file.path)
                    .map_or("source_file_unavailable", |s| s.state),
            )
            .filter(|s| *s != "parsed")
        } else {
            None
        };
        if let Some(reason) = reason {
            unavailable.insert(directory(&file.path).into(), reason);
        }
    }
    for (path, scan) in scans {
        for module in &scan.modules {
            *labels.entry(module.name.clone()).or_default() += 1;
            declarations
                .entry((path, &module.name))
                .or_default()
                .push(module);
        }
    }
    for fact in facts.iter() {
        if source_fact(fact) {
            let key = (
                fact["file"].as_str().unwrap_or("").into(),
                fact["name"].as_str().unwrap_or("").into(),
            );
            *source_facts.entry(key).or_default() += 1;
        }
    }
    for fact in facts {
        // An upstream extractor cannot pre-populate our parsed-evidence fields.
        if let Some(props) = fact["props"].as_object_mut() {
            props.remove("recollect_module_source");
            props.remove("recollect_hcl_directory");
        }
        let path = fact["file"].as_str().unwrap_or("").to_owned();
        let name = fact["name"].as_str().unwrap_or("").to_owned();
        if source_fact(fact) {
            let matches = declarations
                .get(&(path.as_str(), name.as_str()))
                .map(Vec::as_slice)
                .unwrap_or_default();
            let mut witness = ModuleSourceWitness {
                parser: PARSER.into(),
                state: "unmatched_declaration".into(),
                line_from: 0,
                line_to: 0,
                target: None,
            };
            if let [module] = matches {
                witness = module.witness.clone();
                let location = fact["line"].as_u64();
                let reason = unavailable
                    .get(directory(&path))
                    .copied()
                    .or_else(|| {
                        (labels.get(&name) != Some(&1)).then_some("ambiguous_module_declaration")
                    })
                    .or_else(|| {
                        (source_facts.get(&(path.clone(), name.clone())) != Some(&1))
                            .then_some("ambiguous_source_fact")
                    })
                    .or_else(|| {
                        location
                            .is_none_or(|l| {
                                l < module.earliest_line as u64 || l > witness.line_from as u64
                            })
                            .then_some("unmatched_source_location")
                    });
                if let Some(reason) = reason {
                    witness.state = reason.into();
                    witness.target = None;
                }
            } else if let Some(reason) = unavailable.get(directory(&path)) {
                witness.state = (*reason).into();
            }
            fact["props"]["recollect_module_source"] = json!(witness);
        } else if fact["kind"] == "module" && fact["props"]["language"] == "hcl" && name == path {
            let state = unavailable.get(&path).copied().unwrap_or_else(|| {
                if directories.contains(path.as_str()) {
                    "parsed"
                } else {
                    "source_file_unavailable"
                }
            });
            fact["props"]["recollect_hcl_directory"] =
                json!({"parser":PARSER,"state":state,"directory":path});
        }
    }
}
fn source_fact(fact: &Value) -> bool {
    fact["kind"] == "symbol" && fact["props"]["hcl_block"] == "module"
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(path: &str) -> ExtractedFile {
        ExtractedFile {
            path: path.into(),
            object_id: "a".repeat(40),
            mode: "100644".into(),
            size: Some(100),
            status: "materialized".into(),
            extraction: "facts_emitted".into(),
            content: None,
        }
    }
    fn fact(path: &str, name: &str, line: u32) -> Value {
        json!({"kind":"symbol","name":name,"file":path,"line":line,"props":{"hcl_block":"module"}})
    }
    #[test]
    fn syntax_witness_ignores_decoys_and_preserves_enola_whitespace_location() {
        let text = format!(
            r#"

module "dns" {{
  /* source = "git::https://decoy.test/fake?ref=main" */
  note = <<EOT
source = "git::https://decoy.test/fake?ref=main"
EOT
  source = "git::https://example.test/team/dns.git//modules/dns?ref={}"
}}
module "dynamic" {{ source = var.module_source }}
module "local" {{ source = "./local" }}
"#,
            "1".repeat(40)
        );
        let scan = inspect("main.tf", &text);
        assert_eq!(scan.state, "parsed");
        assert_eq!(scan.modules.len(), 3);
        assert_eq!(
            scan.modules[0].witness.target.as_ref().unwrap().origin,
            "example.test/team/dns"
        );
        let mut facts = vec![
            fact("main.tf", "module.dns", 1),
            fact("main.tf", "module.dynamic", 10),
            fact("main.tf", "module.local", 11),
            json!({"kind":"module","name":".","file":".","props":{"language":"hcl"}}),
        ];
        attach(
            &mut facts,
            &[file("main.tf")],
            &BTreeMap::from([("main.tf".into(), scan)]),
        );
        assert_eq!(
            facts[0]["props"]["recollect_module_source"]["state"],
            "verified_literal"
        );
        assert_eq!(facts[0]["line"], 1);
        assert_eq!(facts[0]["props"]["recollect_module_source"]["line_from"], 3);
        assert_eq!(
            facts[1]["props"]["recollect_module_source"]["state"],
            "source_expression"
        );
        assert_eq!(
            facts[2]["props"]["recollect_module_source"]["state"],
            "local_module"
        );
        assert_eq!(
            facts[3]["props"]["recollect_hcl_directory"]["state"],
            "parsed"
        );
    }
    #[test]
    fn ambiguous_overridden_unexamined_and_cancelled_sources_never_link() {
        let text = format!(
            "module \"dns\" {{ source = \"git::https://example.test/team/dns.git?ref={}\" }}\n",
            "1".repeat(40)
        );
        let scans = BTreeMap::from([
            ("main.tf".into(), inspect("main.tf", &text)),
            ("duplicate.tf".into(), inspect("duplicate.tf", &text)),
        ]);
        let mut facts = vec![
            fact("main.tf", "module.dns", 1),
            fact("duplicate.tf", "module.dns", 1),
        ];
        attach(&mut facts, &[file("main.tf"), file("duplicate.tf")], &scans);
        assert!(
            facts
                .iter()
                .all(|f| f["props"]["recollect_module_source"]["state"]
                    == "ambiguous_module_declaration")
        );
        for (extra, reason) in [
            ("override.tf", "module_override_unsupported"),
            ("config.tf.json", "terraform_json_unexamined"),
        ] {
            let mut facts = vec![fact("main.tf", "module.dns", 1)];
            attach(
                &mut facts,
                &[file("main.tf"), file(extra)],
                &BTreeMap::from([("main.tf".into(), inspect("main.tf", &text))]),
            );
            assert_eq!(
                facts[0]["props"]["recollect_module_source"]["state"],
                reason
            );
            assert!(facts[0]["props"]["recollect_module_source"]["target"].is_null());
        }
        assert_eq!(
            inspect("main.tf", "module \"dns\" {").state,
            "source_parse_error"
        );
        let nested = format!("a={}0{}", "[".repeat(10000), "]".repeat(10000));
        assert_eq!(
            inspect_with_budget("main.tf", &nested, Duration::ZERO, 100_000).state,
            "source_parse_timeout"
        );
        let many = (0..2000)
            .map(|n| format!("module \"m{n}\" {{ source = \"./local\" }}\n"))
            .collect::<String>();
        let limited = inspect_limited("main.tf", &many, 1);
        assert_eq!(limited.state, "source_module_limit");
        assert_eq!(limited.module_count(), 2);
        let parsed = inspect("main.tf", &many);
        assert_eq!(parsed.state, "parsed");
        assert_eq!(parsed.modules.last().unwrap().witness.line_from, 2000);
        assert_eq!(parsed.modules.last().unwrap().earliest_line, 2000);
    }
}
