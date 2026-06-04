/// Test: DelegateParams must deserialize an images array from JSON.
/// This is the core contract — the delegate tool receives images from the LLM.
#[test]
fn delegate_params_deserializes_images_array() {
    let json = r#"{
        "instructions": "describe this image",
        "images": [
            { "data": "iVBORw0KGgo=", "mime_type": "image/png" }
        ]
    }"#;

    let params: goose::agents::platform_extensions::summon::DelegateParams =
        serde_json::from_str(json).expect("deserialization should succeed");

    let images = params.images.as_ref().expect("images should be Some");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, "iVBORw0KGgo=");
    assert_eq!(images[0].mime_type, "image/png");
}

/// Test: DelegateParams defaults images to empty vec when omitted.
/// Backward compatibility — existing callers don't send images.
#[test]
fn delegate_params_defaults_images_to_empty() {
    let json = r#"{
        "instructions": "do something"
    }"#;

    let params: goose::agents::platform_extensions::summon::DelegateParams =
        serde_json::from_str(json).expect("deserialization should succeed");

    assert!(params.images.is_none());
}

/// Test: DelegateParams supports multiple images.
#[test]
fn delegate_params_deserializes_multiple_images() {
    let json = r#"{
        "images": [
            { "data": "aaa", "mime_type": "image/png" },
            { "data": "bbb", "mime_type": "image/jpeg" },
            { "data": "ccc", "mime_type": "image/webp" }
        ]
    }"#;

    let params: goose::agents::platform_extensions::summon::DelegateParams =
        serde_json::from_str(json).expect("deserialization should succeed");

    let images = params.images.as_ref().expect("images should be Some");
    assert_eq!(images.len(), 3);
    assert_eq!(images[1].mime_type, "image/jpeg");
}

/// Test: Message::with_image produces ImageContent on the message.
/// This is the building block — images get attached to the subagent's user message.
#[test]
fn message_builder_with_image_creates_image_content() {
    use goose::conversation::message::{Message, MessageContent};

    let msg = Message::user()
        .with_text("describe this")
        .with_image("base64data", "image/png");

    assert_eq!(msg.content.len(), 2);

    match &msg.content[0] {
        MessageContent::Text(t) => assert_eq!(t.text, "describe this"),
        other => panic!("expected Text, got {:?}", other),
    }

    match &msg.content[1] {
        MessageContent::Image(img) => {
            assert_eq!(img.mime_type, "image/png");
        }
        other => panic!("expected Image, got {:?}", other),
    }
}
