use anyhow::Result;

pub fn handle_session(verbose: bool, format: String) -> Result<()> {
    let sessions = match goose::session::list_sessions() {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!("Failed to list sessions: {:?}", e);
            return Err(anyhow::anyhow!("Failed to list sessions"));
        }
    };

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string(&sessions)?);
        }
        _ => {
            if sessions.is_empty() {
                println!("No sessions found");
                return Ok(());
            } else {
                println!("Available sessions:");
                for (id, path) in sessions {
                    if verbose {
                        println!("  {} ({})", id, path.display());
                    } else {
                        println!("  {}", id);
                    }
                }
            }
        }
    }
    Ok(())
}
