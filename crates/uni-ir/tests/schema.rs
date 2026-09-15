//! The schema is the normative definition of the IR, so it has to be true.
//!
//! `docs/specification.md` used to claim that `uni compile` validated its output
//! against `schemas/uni.schema.json`. Nothing did, and adding a general JSON
//! Schema engine would mean a dependency for one document. This does the smaller,
//! sharper thing instead: it walks the schema the way this repository actually
//! uses it (`type`, `required`, `enum`, `const`, `properties`, `items`) and
//! checks both directions, so a field added to the IR without the schema, or a
//! `required` key the compiler does not emit, fails here rather than being
//! discovered by a reader.

use std::path::PathBuf;

use uni_ir::{compile, to_json};

fn schema() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/uni.schema.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("schemas/uni.schema.json must be valid JSON")
}

fn ir_json(src: &str) -> serde_json::Value {
    let contract = uni_parser::parse(src).expect("fixture must parse");
    let ir = compile(&contract).expect("fixture must compile");
    serde_json::from_str(&to_json(&ir).unwrap()).unwrap()
}

/// Every contract shape the compiler can emit: a plain claim, an invariant, an
/// inline shell command, a REQUIRE, and an OPTIONAL claim.
fn fixtures() -> Vec<(&'static str, serde_json::Value)> {
    let head = "VERSION 0.1\nDOMAIN software\nINTENT t\nGOAL\n  n\n";
    let tail = "ACCEPT WHEN\n  required_claims == VERIFIED\n";
    vec![
        (
            "plain",
            ir_json(&format!(
                "{head}CLAIM a REQUIRED\n  ENSURE x\nVERIFY a\n  USING project.check\n{tail}"
            )),
        ),
        (
            "invariant, inline shell, optional claim",
            ir_json(&format!(
                "{head}CLAIM a OPTIONAL\n  ENSURE x\nINVARIANT b CRITICAL\n  ENSURE y\n\
                 VERIFY a\n  USING shell \"cargo test\"\nVERIFY b\n  USING project.check\n{tail}"
            )),
        ),
        (
            "require",
            ir_json(&format!(
                "{head}CLAIM a REQUIRED\n  ENSURE x\nVERIFY a\n  USING suite\n  \
                 REQUIRE behavior(\"checks\")\n{tail}"
            )),
        ),
        (
            "forbid",
            ir_json(&format!(
                "{head}FORBID\n  direct_write(\"ledger\")\nVERIFY forbid-1\n  USING project.check\n{tail}"
            )),
        ),
    ]
}

fn type_matches(value: &serde_json::Value, ty: &str) -> bool {
    match ty {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        other => panic!("the validator does not know the schema type '{other}'"),
    }
}

/// Check `value` against `schema`, and report every mismatch as a path-qualified
/// string so a failure names the field rather than the document.
fn check(value: &serde_json::Value, schema: &serde_json::Value, path: &str, out: &mut Vec<String>) {
    if let Some(ty) = schema.get("type") {
        let ok = match ty {
            serde_json::Value::String(t) => type_matches(value, t),
            serde_json::Value::Array(ts) => ts
                .iter()
                .any(|t| t.as_str().map(|t| type_matches(value, t)).unwrap_or(false)),
            _ => panic!("'type' must be a string or an array of strings"),
        };
        if !ok {
            out.push(format!(
                "{path}: wrong type (schema says {ty}, value is {value}"
            ));
        }
    }
    if let Some(k) = schema.get("const") {
        if value != k {
            out.push(format!(
                "{path}: const mismatch (schema says {k}, value is {value}"
            ));
        }
    }
    if let Some(allowed) = schema.get("enum").and_then(|e| e.as_array()) {
        if !allowed.contains(value) {
            out.push(format!("{path}: {value} is not one of {allowed:?}"));
        }
    }
    if let Some(serde_json::Value::Object(props)) = schema.get("properties") {
        if let Some(obj) = value.as_object() {
            // Forward: every required key must be present.
            if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                for key in required.iter().filter_map(|k| k.as_str()) {
                    if !obj.contains_key(key) {
                        out.push(format!("{path}: missing required key '{key}'"));
                    }
                }
            }
            // Backward: nothing the schema does not declare. A new IR field with
            // no schema entry is drift in the direction that matters, because
            // the schema is what a reader trusts.
            for (key, val) in obj {
                match props.get(key) {
                    Some(sub) => check(val, sub, &format!("{path}.{key}"), out),
                    None => out.push(format!(
                        "{path}: key '{key}' is emitted by the compiler but not declared in the schema"
                    )),
                }
            }
        }
    }
    if let Some(items) = schema.get("items") {
        if let Some(arr) = value.as_array() {
            for (i, item) in arr.iter().enumerate() {
                check(item, items, &format!("{path}[{i}]"), out);
            }
        }
    }
}

#[test]
fn every_emitted_ir_matches_the_schema_both_ways() {
    let schema = schema();
    for (name, ir) in fixtures() {
        let mut problems = Vec::new();
        check(&ir, &schema, "$", &mut problems);
        assert!(
            problems.is_empty(),
            "IR for '{name}' does not match schemas/uni.schema.json:\n  {}",
            problems.join("\n  ")
        );
    }
}

/// If the compiler grows a field, the schema has to grow with it. This is the
/// half a one-directional check would miss.
#[test]
fn the_schema_declares_every_top_level_ir_field() {
    let schema = schema();
    let declared: Vec<&str> = schema["properties"]
        .as_object()
        .expect("the schema must declare properties")
        .keys()
        .map(|k| k.as_str())
        .collect();
    let ir = ir_json(
        "VERSION 0.1\nDOMAIN software\nINTENT t\nGOAL\n  n\nCLAIM a REQUIRED\n  ENSURE x\n\
         VERIFY a\n  USING project.check\nACCEPT WHEN\n  required_claims == VERIFIED\n",
    );
    let emitted: Vec<&str> = ir.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    for key in &emitted {
        assert!(
            declared.contains(key),
            "the compiler emits '{key}' but the schema does not declare it (declared: {declared:?})"
        );
    }
    for key in &declared {
        assert!(
            emitted.contains(key),
            "the schema requires '{key}' but the compiler does not emit it (emitted: {emitted:?})"
        );
    }
}
