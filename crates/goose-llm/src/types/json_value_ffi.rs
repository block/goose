use serde_json::Value;

// `serde_json::Value` gets converted to a `String` to pass across the FFI.
// https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/custom_types.md?plain=1
// https://github.com/mozilla/uniffi-rs/blob/c7f6caa3d1bf20f934346cefd8e82b5093f0dc6f/examples/custom-types/src/lib.rs#L63-L69

uniffi::custom_type!(Value, String, {
    // Remote is required since 'Value' is from a different crate
    remote,
    lower: |obj| {
        serde_json::to_string(&obj).unwrap()
    },
    try_lift: |val| {
        Ok(serde_json::from_str(&val).unwrap() )
    },
});

pub type JsonValueFfi = Value;
