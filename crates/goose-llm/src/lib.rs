#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main_wasm() -> Result<(), JsValue> {
    // The console_error_panic_hook dependency is enabled by the "wasm" feature in Cargo.toml
    // So, if this code is compiled, the dependency is available.
    console_error_panic_hook::set_once();
    Ok(())
}

uniffi::setup_scaffolding!();

mod completion;
pub mod extractors;
pub mod message;
mod model;
mod prompt_template;
pub mod providers;
mod structured_outputs;
pub mod types;
#[cfg(feature = "wasm")]
mod wasm;
#[cfg(all(feature = "wasm", feature = "http"))]
mod wasm_providers;

pub use completion::completion;
pub use message::Message;
pub use model::ModelConfig;
pub use structured_outputs::generate_structured_outputs;

#[cfg(feature = "wasm")]
pub use wasm::*;
#[cfg(all(feature = "wasm", feature = "http"))]
pub use wasm_providers::*;