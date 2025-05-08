// Tell UniFFI to generate glue for everything annotated below:
uniffi::setup_scaffolding!();

mod json_value_ffi;
pub use json_value_ffi::JsonValueFfi;

mod tool_result_serde;
pub mod types;

use crate::types::{ExtensionConfig, Message};

#[uniffi::export]
pub fn print_messages(messages: Vec<Message>) {
    for msg in messages {
        println!("[{:?} @ {}] {:?}", msg.role, msg.created, msg.content);
    }
}

#[uniffi::export(async_runtime = "tokio")]
pub async fn async_print(messages: Vec<Message>, extensions: Vec<ExtensionConfig>) {
    for ext in extensions {
        println!("Extension: {:?}", ext);
    }
    println!("--------");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    for msg in messages {
        println!("[{:?} @ {}] {:?}", msg.role, msg.created, msg.content);
    }
}
