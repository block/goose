use anyhow::Result;
use console::style;
use etcetera::{choose_app_strategy, AppStrategy};
use goose::config::Config;
use serde_yaml;

fn print_aligned(label: &str, value: &str, width: usize) {
    println!("  {:<width$} {}", label, value, width = width);
}

pub fn handle_info(verbose: bool) -> Result<()> {
    let data_dir = choose_app_strategy(crate::APP_STRATEGY.clone())?;
    let logs_dir = data_dir
        .in_state_dir("logs")
        .unwrap_or_else(|| data_dir.in_data_dir("logs"));
    let sessions_dir = data_dir.in_data_dir("sessions");

    // Get paths using a stored reference to the global config
    let config = Config::global();
    let config_file = config.path();

    // Define the labels and their corresponding path values once.
    let paths = [
        ("Config file:", config_file.to_string()),
        ("Sessions dir:", sessions_dir.display().to_string()),
        ("Logs dir:", logs_dir.display().to_string()),
    ];

    // Calculate padding: use the max length of the label plus extra space.
    let basic_padding = paths.iter().map(|(l, _)| l.len()).max().unwrap_or(0) + 4;

    // Print version information
    println!("{}", style("Goose Version:").cyan().bold());
    print_aligned("Version:", env!("CARGO_PKG_VERSION"), basic_padding);
    println!();

    // Print location information
    println!("{}", style("Goose Locations:").cyan().bold());
    for (label, path) in &paths {
        print_aligned(label, path, basic_padding);
    }

    // Print verbose info if requested
    if verbose {
        println!("\n{}", style("Goose Configuration:").cyan().bold());
        match config.load_values() {
            Ok(values) => {
                if values.is_empty() {
                    println!("  No configuration values set");
                    println!(
                        "  Run '{}' to configure goose",
                        style("goose configure").cyan()
                    );
                } else {
                    let sorted_values: std::collections::BTreeMap<_, _> =
                        values.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                    if let Ok(yaml) = serde_yaml::to_string(&sorted_values) {
                        for line in yaml.lines() {
                            println!("  {}", line);
                        }
                    }
                }
            }
            Err(e) => println!("  Error loading configuration: {}", e),
        }
    }

    Ok(())
}
