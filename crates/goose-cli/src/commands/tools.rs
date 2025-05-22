use crate::session::{build_session, SessionBuilderConfig};
use anyhow::Result;
use console::style;
use mcp_core::tool::Tool;

// Helper function to print tool information
fn print_tools(tools: &[Tool], ext_name: &str) {
    if tools.is_empty() {
        println!(
            "{} {}",
            style("No tools found for extension:").yellow(),
            style(ext_name).cyan().bold()
        );
        return;
    }

    println!(
        "\n{}: {}",
        style("Extension").green().bold(),
        style(ext_name).cyan()
    );

    for tool in tools {
        println!(
            "  {} {}",
            style("Tool:").blue().bold(),
            style(&tool.name).cyan(),
        );

        // Print description with proper indentation
        for line in tool.description.lines() {
            println!("      {}", line);
        }
        println!();

        println!("      {}:", style("Arguments (input schema)").dim().bold());
        for line in serde_json::to_string_pretty(&tool.input_schema)
            .unwrap_or_else(|_| "        <invalid schema>".to_string())
            .lines()
        {
            println!("        {}", line);
        }
        println!();
    }
}

pub async fn handle_tools(extension: Option<String>) -> Result<()> {
    let session = build_session(SessionBuilderConfig {
        identifier: None,
        resume: false,
        no_session: true,
        extensions: Vec::new(),
        remote_extensions: Vec::new(),
        builtins: Vec::new(),
        extensions_override: None,
        additional_system_prompt: None,
        debug: false,
        max_tool_repetitions: None,
    })
    .await;

    if let Some(ext_name) = extension {
        let tools = session.list_tools(Some(ext_name.clone())).await;
        print_tools(&tools, &ext_name);
    } else {
        // list all extensions and their tools
        let all_extension_names = session.list_extension_names().await;
        for ext in all_extension_names {
            let tools = session.list_tools(Some(ext.clone())).await;
            if tools.is_empty() {
                println!(
                    "\n{}: {}",
                    style("Extension").green().bold(),
                    style(&ext).cyan()
                );
                println!("  {}", style("(no tools)").dim());
            } else {
                print_tools(&tools, &ext);
            }
        }
    }
    Ok(())
}
