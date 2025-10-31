use anyhow::Result;
use console::style;
use goose::config::Config;
use goose::counsel::{CounselOrchestrator, CounselResult};
use goose::model::ModelConfig;
use goose::providers::create;

/// Handle the counsel command
pub async fn handle_counsel(prompt: String, show_all: bool, format: String) -> Result<()> {
    // Get the provider from config
    let config = Config::global();
    let provider_name = config.get_goose_provider()?;
    let model_name = config.get_goose_model()?;

    let model_config = ModelConfig::new(&model_name)?;
    let provider = create(&provider_name, model_config).await?;

    // Create the orchestrator
    let orchestrator = CounselOrchestrator::new(provider);

    // Print header
    println!(
        "\n{}",
        style("🎭 Counsel of 9 - Deliberating...").bold().cyan()
    );
    println!("{}", style("─".repeat(60)).dim());
    println!();

    // Run the counsel process
    println!(
        "{}",
        style("Phase 1: Gathering opinions from 9 personas...").yellow()
    );
    println!("{}", style("Phase 2: Conducting voting...").yellow());
    println!();

    let result = orchestrator.run(prompt).await?;

    // Display results based on format
    match format.as_str() {
        "json" => display_json(&result)?,
        _ => {
            if show_all {
                display_all_opinions(&result)?;
            } else {
                display_winner_only(&result)?;
            }
        }
    }

    Ok(())
}

/// Display only the winner
fn display_winner_only(result: &CounselResult) -> Result<()> {
    let winner_votes = result
        .vote_counts
        .get(&result.winner.member_id)
        .copied()
        .unwrap_or(0);

    println!("{}", style("─".repeat(60)).dim());
    println!(
        "\n{} {} ({} votes)\n",
        style("🏆 Winner:").bold().green(),
        style(&result.winner.member_name).bold(),
        style(winner_votes).yellow()
    );

    // Display the winning opinion with word wrapping
    let wrapped = textwrap::wrap(&result.winner.content, 70);
    for line in wrapped {
        println!("  {}", line);
    }

    println!();

    // Show summary of voting
    if result.total_votes > 0 {
        println!("{}", style("─".repeat(60)).dim());
        println!("\n{}", style("📊 Voting Summary:").bold());

        // Sort by vote count
        let mut vote_list: Vec<_> = result.vote_counts.iter().collect();
        vote_list.sort_by(|a, b| b.1.cmp(a.1));

        for (member_id, count) in vote_list.iter().take(3) {
            if let Some(opinion) = result
                .all_opinions
                .iter()
                .find(|o| &o.member_id == *member_id)
            {
                let bar = "█".repeat(**count as usize * 2);
                println!(
                    "  {} {} {}",
                    style(&opinion.member_name).cyan(),
                    style(&bar).green(),
                    style(format!("({} votes)", count)).dim()
                );
            }
        }
        println!();
    }

    // Show unavailable members if any
    if !result.unavailable_members.is_empty() {
        println!("{}", style("⚠️  Unavailable members:").yellow());
        for member in &result.unavailable_members {
            println!("  • {}", style(member).dim());
        }
        println!();
    }

    println!("{}", style("─".repeat(60)).dim());
    println!(
        "\n{}\n",
        style("Run with --show-all to see all opinions and detailed voting.").dim()
    );

    Ok(())
}

/// Display all opinions and full voting details
fn display_all_opinions(result: &CounselResult) -> Result<()> {
    println!("{}", style("─".repeat(60)).dim());
    println!("\n{}", style("📊 Complete Results").bold().cyan());
    println!("{}", style("─".repeat(60)).dim());

    // Show winner first
    let winner_votes = result
        .vote_counts
        .get(&result.winner.member_id)
        .copied()
        .unwrap_or(0);

    println!(
        "\n{} {} ({} votes)\n",
        style("🏆 Winner:").bold().green(),
        style(&result.winner.member_name).bold(),
        style(winner_votes).yellow()
    );

    let wrapped = textwrap::wrap(&result.winner.content, 70);
    for line in wrapped {
        println!("  {}", line);
    }
    println!();

    // Show all opinions sorted by vote count
    println!("{}", style("─".repeat(60)).dim());
    println!("\n{}", style("📋 All Opinions:").bold());
    println!();

    let mut opinions_with_votes: Vec<_> = result
        .all_opinions
        .iter()
        .map(|opinion| {
            let votes = result
                .vote_counts
                .get(&opinion.member_id)
                .copied()
                .unwrap_or(0);
            (opinion, votes)
        })
        .collect();

    opinions_with_votes.sort_by(|a, b| b.1.cmp(&a.1));

    for (idx, (opinion, votes)) in opinions_with_votes.iter().enumerate() {
        let is_winner = opinion.member_id == result.winner.member_id;
        let number_style = if is_winner {
            style(format!("{}.", idx + 1)).bold().green()
        } else {
            style(format!("{}.", idx + 1)).bold()
        };

        let name_style = if is_winner {
            style(&opinion.member_name).bold().green()
        } else {
            style(&opinion.member_name).bold().cyan()
        };

        let bar = "█".repeat(*votes as usize * 2);
        let bar_style = if is_winner {
            style(&bar).green()
        } else {
            style(&bar).blue()
        };

        println!(
            "{} {} {} {}",
            number_style,
            name_style,
            bar_style,
            style(format!("({} votes)", votes)).dim()
        );

        let wrapped = textwrap::wrap(&opinion.content, 68);
        for line in wrapped {
            println!("   {}", style(line).dim());
        }
        println!();
    }

    // Show unavailable members if any
    if !result.unavailable_members.is_empty() {
        println!("{}", style("─".repeat(60)).dim());
        println!("\n{}", style("⚠️  Unavailable Members:").yellow());
        for member in &result.unavailable_members {
            println!("  • {}", style(member).dim());
        }
        println!();
    }

    println!("{}", style("─".repeat(60)).dim());
    println!(
        "\n{} {}/{} members participated\n",
        style("✓").green(),
        style(result.all_opinions.len()).bold(),
        style("9").bold()
    );

    Ok(())
}

/// Display results as JSON
fn display_json(result: &CounselResult) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::counsel::{CounselResult, Opinion};
    use std::collections::HashMap;

    fn create_test_result() -> CounselResult {
        let opinions = vec![
            Opinion::new(
                "pragmatist",
                "The Pragmatist",
                "Start with a modular monolith. This is the most practical approach.",
                "Reasoning here",
            ),
            Opinion::new(
                "visionary",
                "The Visionary",
                "Think long-term. Microservices will enable future scaling.",
                "Reasoning here",
            ),
        ];

        let mut vote_counts = HashMap::new();
        vote_counts.insert("pragmatist".to_string(), 5);
        vote_counts.insert("visionary".to_string(), 2);

        CounselResult::new(opinions[0].clone(), opinions, vote_counts, 7, vec![])
    }

    #[test]
    fn test_display_winner_only() {
        let result = create_test_result();
        // Just ensure it doesn't panic
        let _ = display_winner_only(&result);
    }

    #[test]
    fn test_display_all_opinions() {
        let result = create_test_result();
        // Just ensure it doesn't panic
        let _ = display_all_opinions(&result);
    }

    #[test]
    fn test_display_json() {
        let result = create_test_result();
        let json_result = display_json(&result);
        assert!(json_result.is_ok());
    }
}
