pub fn has_input_placeholder(args: &serde_json::Value) -> bool {
    match args {
        serde_json::Value::String(s) => s.contains("{input}"),
        serde_json::Value::Object(obj) => obj.values().any(has_input_placeholder),
        serde_json::Value::Array(arr) => arr.iter().any(has_input_placeholder),
        _ => false,
    }
}
