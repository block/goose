use serde::{Deserialize, Serialize};
use serde_json::Value;

// `serde_json::Value` gets converted to a `String` to pass across the FFI.
// https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/custom_types.md?plain=1
// https://github.com/mozilla/uniffi-rs/blob/c7f6caa3d1bf20f934346cefd8e82b5093f0dc6f/examples/custom-types/src/lib.rs#L63-L69

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonValueFfi(Value);

impl From<JsonValueFfi> for Value {
    fn from(val: JsonValueFfi) -> Self {
        val.0
    }
}

impl Into<JsonValueFfi> for Value {
    fn into(self) -> JsonValueFfi {
        JsonValueFfi(self)
    }
}

uniffi::custom_type!(JsonValueFfi, String, {
    lower: |obj| {
        serde_json::to_string(&obj.0).unwrap()
    },
    try_lift: |val| {
        Ok(serde_json::from_str(&val).unwrap() )
    },
});

// Write some tests to ensure that the conversion works as expected
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_value_ffi_conversion() {
        let original = JsonValueFfi(json!({"key": "value"}));
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: JsonValueFfi = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original.0, deserialized.0);
    }

    #[test]
    fn test_json_value_ffi_into() {
        let original = JsonValueFfi(json!({"key": "value"}));
        let value: Value = original.into();
        assert_eq!(value, json!({"key": "value"}));
    }

    #[test]
    fn test_json_value_ffi_from() {
        let value = json!({"key": "value"});
        let original: JsonValueFfi = value.into();
        assert_eq!(original.0, json!({"key": "value"}));
    }

    #[test]
    fn test_json_value_ffi_lower() {
        let original = JsonValueFfi(json!({"key": "value"}));
        let serialized = serde_json::to_string(&original).unwrap();

        assert_eq!(serialized, "{\"key\":\"value\"}");
    }

    #[test]
    fn test_json_value_ffi_try_lift() {
        let json_str = "{\"key\":\"value\"}";
        let deserialized: JsonValueFfi = serde_json::from_str(json_str).unwrap();
        let expected = JsonValueFfi(json!({"key": "value"}));
        assert_eq!(deserialized.0, expected.0);
    }

    #[test]
    fn test_json_value_ffi_custom_type() {
        let json_str = "{\"key\":\"value\"}";
        let deserialized: JsonValueFfi = serde_json::from_str(json_str).unwrap();
        let serialized = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(serialized, json_str);
    }
}
