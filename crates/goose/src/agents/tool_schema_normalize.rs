use rmcp::model::JsonObject;
use serde_json::{Map, Value};

/// Normalize an rmcp tool `input_schema` in place, returning `true` if changed.
///
/// Thin wrapper over [`collapse_const_unions`] that operates on the
/// `JsonObject` map carried by `rmcp::model::Tool`.
pub fn normalize_input_schema(schema: &mut JsonObject) -> bool {
    let mut value = Value::Object(std::mem::take(schema));
    let changed = collapse_const_unions(&mut value);
    if let Value::Object(obj) = value {
        *schema = obj;
    }
    changed
}

/// Collapse `oneOf`/`anyOf` unions whose members are all string `const`s into a
/// single `{ "type": "string", "enum": [...] }`, folding any per-variant
/// descriptions into the enclosing description.
///
/// schemars emits documented unit enums (e.g. a Rust enum whose variants carry
/// `///` docs) as a `$defs` entry containing `oneOf: [{const, type:"string",
/// description}, ...]`, referenced via `$ref`. That shape is ~9x larger than an
/// equivalent `enum`, and strict validators (notably Moonshot's) reject a
/// `$ref -> oneOf` outright. Collapsing to a plain `enum` is smaller, loses no
/// information, and is universally supported.
///
/// Genuine unions (untagged enums with data-carrying variants, nullable
/// wrappers like `anyOf: [{$ref}, {type:"null"}]`, etc.) are left untouched:
/// only unions where *every* member is a bare string const are collapsed.
///
/// Returns `true` if the schema was modified.
pub fn collapse_const_unions(schema: &mut Value) -> bool {
    let mut changed = collapse_node(schema);
    if inline_trivial_defs(schema) {
        changed = true;
    }
    changed
}

fn collapse_node(node: &mut Value) -> bool {
    let mut changed = false;
    match node {
        Value::Object(obj) => {
            for key in ["oneOf", "anyOf"] {
                if let Some(collapsed) = try_collapse_union(obj, key) {
                    obj.remove(key);
                    let existing = obj
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    obj.insert("type".to_string(), Value::String("string".to_string()));
                    obj.insert("enum".to_string(), Value::Array(collapsed.values));
                    if let Some(merged) = merge_descriptions(existing, collapsed.descriptions) {
                        obj.insert("description".to_string(), Value::String(merged));
                    }
                    changed = true;
                }
            }
            for value in obj.values_mut() {
                if collapse_node(value) {
                    changed = true;
                }
            }
        }
        Value::Array(arr) => {
            for value in arr {
                if collapse_node(value) {
                    changed = true;
                }
            }
        }
        _ => {}
    }
    changed
}

struct CollapsedUnion {
    values: Vec<Value>,
    descriptions: Vec<String>,
}

/// If `obj[key]` is an array where every member is a bare `{type:"string",
/// const:X, description?}`, return the collected consts and descriptions.
fn try_collapse_union(obj: &Map<String, Value>, key: &str) -> Option<CollapsedUnion> {
    let members = obj.get(key)?.as_array()?;
    if members.is_empty() {
        return None;
    }

    let mut values = Vec::with_capacity(members.len());
    let mut descriptions = Vec::new();
    for member in members {
        let member = member.as_object()?;
        let allowed = member
            .keys()
            .all(|k| matches!(k.as_str(), "type" | "const" | "description"));
        if !allowed {
            return None;
        }
        if member.get("type").and_then(Value::as_str) != Some("string") {
            return None;
        }
        let konst = member.get("const")?;
        if !konst.is_string() {
            return None;
        }
        values.push(konst.clone());
        if let (Some(c), Some(d)) = (
            konst.as_str(),
            member.get("description").and_then(Value::as_str),
        ) {
            descriptions.push(format!("{c}: {d}"));
        }
    }

    Some(CollapsedUnion {
        values,
        descriptions,
    })
}

fn merge_descriptions(existing: Option<String>, variant_descs: Vec<String>) -> Option<String> {
    let variants = if variant_descs.is_empty() {
        None
    } else {
        Some(format!("One of: {}", variant_descs.join("; ")))
    };
    match (existing, variants) {
        (Some(base), Some(v)) => Some(format!("{base}. {v}")),
        (Some(base), None) => Some(base),
        (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}

/// After collapsing, `$defs` entries that became simple string enums and are
/// referenced from exactly one place can be inlined, then `$defs` dropped if
/// nothing still references it. Only inlines `$ref`s that resolve to a value
/// with no remaining `$defs`/`$ref` of its own (i.e. a leaf string enum).
fn inline_trivial_defs(schema: &mut Value) -> bool {
    let Some(defs) = schema.get("$defs").and_then(Value::as_object).cloned() else {
        return false;
    };

    let inlinable: Map<String, Value> = defs
        .iter()
        .filter(|(_, def)| is_leaf_string_enum(def))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if inlinable.is_empty() {
        return false;
    }

    let mut changed = false;
    inline_refs(schema, &inlinable, &mut changed);

    let still_used = collect_used_defs(schema);
    if let Some(Value::Object(defs_obj)) = schema.get_mut("$defs") {
        defs_obj.retain(|name, _| still_used.contains(name));
        if defs_obj.is_empty() {
            schema.as_object_mut().unwrap().remove("$defs");
            changed = true;
        }
    }
    changed
}

fn is_leaf_string_enum(def: &Value) -> bool {
    let Some(obj) = def.as_object() else {
        return false;
    };
    obj.get("type").and_then(Value::as_str) == Some("string")
        && obj.get("enum").is_some_and(Value::is_array)
        && !obj.contains_key("$ref")
        && !obj.contains_key("$defs")
}

fn inline_refs(node: &mut Value, inlinable: &Map<String, Value>, changed: &mut bool) {
    match node {
        Value::Object(obj) => {
            if let Some(name) = obj
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"))
            {
                if let Some(target) = inlinable.get(name).cloned() {
                    let outer_desc = obj
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    obj.clear();
                    if let Value::Object(t) = target {
                        for (k, v) in t {
                            obj.insert(k, v);
                        }
                    }
                    if let Some(outer) = outer_desc {
                        let merged = match obj.get("description").and_then(Value::as_str) {
                            Some(inner) => format!("{outer}. {inner}"),
                            None => outer,
                        };
                        obj.insert("description".to_string(), Value::String(merged));
                    }
                    *changed = true;
                    return;
                }
            }
            for value in obj.values_mut() {
                inline_refs(value, inlinable, changed);
            }
        }
        Value::Array(arr) => {
            for value in arr {
                inline_refs(value, inlinable, changed);
            }
        }
        _ => {}
    }
}

fn collect_used_defs(schema: &Value) -> std::collections::HashSet<String> {
    let mut used = std::collections::HashSet::new();
    fn walk(node: &Value, in_defs: bool, used: &mut std::collections::HashSet<String>) {
        match node {
            Value::Object(obj) => {
                for (k, v) in obj {
                    if k == "$defs" {
                        walk(v, true, used);
                        continue;
                    }
                    if k == "$ref" && !in_defs {
                        if let Some(name) = v.as_str().and_then(|r| r.strip_prefix("#/$defs/")) {
                            used.insert(name.to_string());
                        }
                    }
                    walk(v, in_defs, used);
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    walk(v, in_defs, used);
                }
            }
            _ => {}
        }
    }
    // Only count refs from outside $defs so we can drop defs used by nothing else.
    if let Some(obj) = schema.as_object() {
        for (k, v) in obj {
            if k == "$defs" {
                continue;
            }
            walk(v, false, &mut used);
        }
    }
    used
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collapses_ref_oneof_of_consts_into_enum() {
        let mut schema = json!({
            "type": "object",
            "$defs": {
                "CacheCommand": {
                    "description": "Enum for command",
                    "oneOf": [
                        {"description": "List all", "type": "string", "const": "list"},
                        {"description": "Clear all", "type": "string", "const": "clear"}
                    ]
                }
            },
            "properties": {
                "command": {"description": "The command", "$ref": "#/$defs/CacheCommand"}
            },
            "required": ["command"]
        });

        assert!(collapse_const_unions(&mut schema));

        // No oneOf, no $defs, no $ref anywhere.
        let s = serde_json::to_string(&schema).unwrap();
        assert!(!s.contains("oneOf"), "oneOf should be gone: {s}");
        assert!(!s.contains("$defs"), "$defs should be inlined: {s}");
        assert!(!s.contains("$ref"), "$ref should be inlined: {s}");

        let command = &schema["properties"]["command"];
        assert_eq!(command["type"], "string");
        assert_eq!(command["enum"], json!(["list", "clear"]));
        let desc = command["description"].as_str().unwrap();
        assert!(desc.contains("The command"));
        assert!(desc.contains("list: List all"));
        assert!(desc.contains("clear: Clear all"));
    }

    #[test]
    fn collapses_inline_anyof_of_consts() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "mode": {
                    "anyOf": [
                        {"type": "string", "const": "a"},
                        {"type": "string", "const": "b"}
                    ]
                }
            }
        });

        assert!(collapse_const_unions(&mut schema));
        assert_eq!(schema["properties"]["mode"]["type"], "string");
        assert_eq!(schema["properties"]["mode"]["enum"], json!(["a", "b"]));
    }

    #[test]
    fn leaves_nullable_enum_ref_untouched() {
        // Option<TextAlignment>: anyOf: [ {$ref}, {type:null} ] — a real union.
        let mut schema = json!({
            "type": "object",
            "$defs": {
                "Align": {"type": "string", "enum": ["left", "right"]}
            },
            "properties": {
                "alignment": {
                    "description": "Text alignment",
                    "anyOf": [
                        {"$ref": "#/$defs/Align"},
                        {"type": "null"}
                    ]
                }
            }
        });

        let before = schema.clone();
        collapse_const_unions(&mut schema);
        // The nullable anyOf wrapper must remain (still has a null branch).
        let align = &schema["properties"]["alignment"];
        assert!(
            align.get("anyOf").is_some(),
            "nullable anyOf must be preserved: {align}"
        );
        // $defs may be inlined into the $ref, but the union shape stays.
        let s = serde_json::to_string(&schema).unwrap();
        assert!(s.contains("null"), "null branch preserved");
        // Sanity: we did not turn it into a bare string enum.
        assert!(align.get("enum").is_none(), "must not flatten to enum");
        let _ = before;
    }

    #[test]
    fn leaves_data_carrying_union_untouched() {
        // Untagged enum: Number(f64) | LabeledValue{...}
        let mut schema = json!({
            "type": "object",
            "properties": {
                "value": {
                    "anyOf": [
                        {"type": "number"},
                        {"type": "object", "properties": {"label": {"type": "string"}}}
                    ]
                }
            }
        });

        let before = schema.clone();
        collapse_const_unions(&mut schema);
        assert_eq!(schema, before, "data-carrying union must be unchanged");
    }

    #[test]
    fn no_change_for_plain_schema() {
        let mut schema = json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        });
        let before = schema.clone();
        assert!(!collapse_const_unions(&mut schema));
        assert_eq!(schema, before);
    }
}
