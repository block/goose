use cliclack;
use console::style;
use goose::config::Config;
use serde_json::Value;
use std::error::Error;

pub fn configure_nested_hints_dialog() -> Result<(), Box<dyn Error>> {
    let config = Config::global();

    if std::env::var("GOOSE_NESTED_HINTS").is_ok() {
        let _ = cliclack::log::info("Notice: GOOSE_NESTED_HINTS environment variable is set and will override the configuration here.");
    }

    let current_enabled: bool = config.get_param("NESTED_GOOSE_HINTS").unwrap_or(false);

    println!(
        "Current nested hints setting: {}",
        style(if current_enabled {
            "enabled"
        } else {
            "disabled"
        })
        .cyan()
    );

    let enable = cliclack::confirm("Enable nested hint files loading (eg: .goosehints)?")
        .initial_value(current_enabled)
        .interact()?;

    config.set_param("NESTED_GOOSE_HINTS", Value::Bool(enable))?;

    if enable {
        cliclack::outro("✓ Nested hints enabled - Goose will load hint files from current directory upwards to project root (.git) or current directory if no .git directory found")?;
    } else {
        cliclack::outro("✓ Nested hints disabled - Goose will only load hint files from the current working directory")?;
    }

    Ok(())
}
