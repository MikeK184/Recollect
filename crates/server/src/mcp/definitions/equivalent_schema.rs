//! Declaration-only compatibility for an audited subset of inspected schemas.
//! Direct approval and runtime schemas still use their existing validators.
use crate::error::{Error, Result};
use serde_json::Value;

const DRAFT7: &str = "http://json-schema.org/draft-07/schema#";
const DRAFT202012: &str = "https://json-schema.org/draft/2020-12/schema";

fn subset(schema: &Value, depth: usize, root: bool) -> bool {
    if depth > 24 {
        return false;
    }
    let Some(fields) = schema.as_object() else {
        return false;
    };
    fields.iter().all(|(key, value)| match key.as_str() {
        "$schema" => root && value.as_str() == Some(DRAFT7),
        "type" => value.as_str().is_some_and(|kind| {
            matches!(
                kind,
                "object" | "array" | "string" | "number" | "integer" | "boolean" | "null"
            )
        }),
        "properties" => value
            .as_object()
            .is_some_and(|properties| properties.values().all(|s| subset(s, depth + 2, false))),
        "items" => subset(value, depth + 1, false),
        "additionalProperties" => value.is_boolean(),
        "required" => value
            .as_array()
            .is_some_and(|names| names.iter().all(Value::is_string)),
        "description" => value.is_string(),
        "minLength" | "maxLength" => value.as_u64().is_some(),
        "minimum" => value.is_number(),
        _ => false,
    })
}

pub(super) fn normalize(schema: &mut Value) -> Result<()> {
    if schema.get("$schema").and_then(Value::as_str) != Some(DRAFT7) {
        return Ok(());
    }
    if serde_json::to_vec(schema).map_or(true, |bytes| bytes.len() > 32768)
        || !subset(schema, 0, true)
    {
        return Err(Error::invalid(
            "This Draft 7 schema uses unsupported keywords or exceeds inspection limits.",
        ));
    }
    // Every accepted assertion has the same meaning in both drafts. Preserve
    // all other bytes/values; the final candidate still receives strict validation.
    schema["$schema"] = Value::String(DRAFT202012.into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn exa_search() -> Value {
        json!({"$schema":DRAFT7,"type":"object","properties":{
            "query":{"type":"string","minLength":1,"description":"Search query"},
            "objective":{"type":"string","minLength":1,"maxLength":4096,"description":"Search goal"},
            "numResults":{"type":"number","description":"Result count"}
        },"required":["query","objective"],"additionalProperties":false})
    }

    fn exa_fetch() -> Value {
        json!({"$schema":DRAFT7,"type":"object","properties":{
            "urls":{"type":"array","items":{"type":"string"}},
            "maxCharacters":{"type":"number","minimum":1}
        },"required":["urls"],"additionalProperties":false})
    }

    #[test]
    fn inspected_exa_preserves_assertions_and_runtime_validation() {
        let cases = vec![
            json!({}),
            json!(null),
            json!([]),
            json!(42),
            json!({"unexpected":"x"}),
            json!({"query":"Rust","objective":"Find official documentation"}),
            json!({"query":"Rust"}),
            json!({"query":"","objective":"x"}),
            json!({"query":"x","objective":""}),
            json!({"query":"x","objective":"x".repeat(4096)}),
            json!({"query":"x","objective":"x".repeat(4097)}),
            json!({"query":"x","objective":"y","numResults":"10"}),
            json!({"query":"x","objective":"y","numResults":3.5}),
            json!({"urls":["https://example.com"]}),
            json!({"urls":"https://example.com"}),
            json!({"urls":[]}),
            json!({"urls":[1]}),
            json!({"urls":[],"maxCharacters":0}),
            json!({"urls":[],"maxCharacters":1}),
        ];
        for original in [exa_search(), exa_fetch()] {
            let mut normalized = original.clone();
            assert!(normalize(&mut normalized).is_ok());
            assert_eq!(normalized["$schema"], DRAFT202012);
            let mut restored = normalized.clone();
            restored["$schema"] = json!(DRAFT7);
            assert_eq!(restored, original);
            // The approval validator remains strict for raw Draft 7 input.
            assert!(super::super::validator(&original, true, 32768).is_err());
            let approved = super::super::validator(&normalized, true, 32768)
                .map_err(|e| e.2)
                .unwrap();
            let draft7 = jsonschema::options()
                .with_draft(jsonschema::Draft::Draft7)
                .offline()
                .build(&original)
                .unwrap();
            let runtime = jsonschema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .offline()
                .build(&original)
                .unwrap();
            for case in &cases {
                assert_eq!(approved.is_valid(case), draft7.is_valid(case));
                assert_eq!(approved.is_valid(case), runtime.is_valid(case));
            }
        }
        let mut output = json!({"$schema":DRAFT7,"type":"array","items":{"type":"string"}});
        assert!(normalize(&mut output).is_ok());
        assert!(super::super::validator(&output, false, 32768).is_ok());
        assert!(super::super::validator(&output, true, 32768).is_err());
    }

    #[test]
    fn inspected_draft7_rejects_unsupported_or_unsafe_forms() {
        for (key, value) in [
            ("$ref", json!("#/properties/query")),
            ("$ref", json!("https://example.com/schema")),
            ("$id", json!("https://example.com/schema")),
            ("x-mcp-header", json!("Authorization")),
            ("unevaluatedProperties", json!(false)),
            ("unknownAssertion", json!(true)),
            ("items", json!([{"type":"string"}])),
            ("additionalProperties", json!({"type":"string"})),
            ("minLength", json!(-1)),
            ("minimum", json!("1")),
        ] {
            let mut schema = exa_search();
            schema[key] = value;
            let original = schema.clone();
            assert!(normalize(&mut schema).is_err(), "{key}");
            assert_eq!(schema, original);
        }
        let mut nested = exa_search();
        nested["properties"]["query"]["$schema"] = json!(DRAFT7);
        assert!(normalize(&mut nested).is_err());
        let mut oversized = exa_search();
        oversized["description"] = json!("x".repeat(32768));
        assert!(normalize(&mut oversized).is_err());
        let mut deep = json!({"type":"string"});
        for _ in 0..25 {
            deep = json!({"type":"array","items":deep});
        }
        deep["$schema"] = json!(DRAFT7);
        assert!(normalize(&mut deep).is_err());
        let mut malformed = json!({"$schema":DRAFT7,"type":"object","required":["x","x"]});
        assert!(normalize(&mut malformed).is_ok());
        assert!(super::super::validator(&malformed, true, 32768).is_err());
    }

    #[test]
    fn canonical_and_other_declarations_stay_unchanged() {
        for original in [
            json!({"$schema":DRAFT202012,"type":"object","properties":{"name":{"$ref":"#/$defs/name"}},"$defs":{"name":{"type":"string"}}}),
            json!({"$schema":"http://json-schema.org/draft-04/schema#","type":"object"}),
            json!({"type":"object","properties":{"name":{"type":"string"}}}),
        ] {
            let mut schema = original.clone();
            assert!(normalize(&mut schema).is_ok());
            assert_eq!(schema, original);
        }
        let mut canonical = json!({"$schema":DRAFT202012,"type":"object","properties":{"name":{"$ref":"#/$defs/name"}},"$defs":{"name":{"type":"string"}}});
        assert!(normalize(&mut canonical).is_ok());
        assert!(super::super::validator(&canonical, true, 32768).is_ok());
    }
}
