//! Message generator for scenario tests with convenience methods to
//! just generate an image or text message.

use crate::scenario_tests::scenario_runner::SCENARIO_TESTS_DIR;
use base64::engine::general_purpose;
use base64::Engine;
use goose::conversation::message::Message;
use goose::providers::base::Provider;

pub type MessageGenerator<'a> = Box<dyn Fn(&dyn Provider) -> Message + 'a>;

pub fn text(text: &str) -> MessageGenerator<'static> {
    let text = text.to_string();
    Box::new(move |_provider| Message::user().with_text(&text))
}

pub fn image(text: &str, image_name: &str) -> MessageGenerator<'static> {
    let text = text.to_string();
    let image_name = image_name.to_string();
    Box::new(move |_provider| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let image_path = format!(
            "{}/{}/test_data/{}.jpg",
            manifest_dir, SCENARIO_TESTS_DIR, image_name
        );

        let image_data = std::fs::read(image_path).expect("Failed to read image");
        let base64_data = general_purpose::STANDARD.encode(&image_data);
        Message::user()
            .with_text(&text)
            .with_image(base64_data, "image/jpeg")
    })
}
