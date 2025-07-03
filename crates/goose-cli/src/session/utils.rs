use anyhow::Result;
use console::Color;
use goose::agents::Agent;
use goose::config::Config;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::base::Provider;
use goose::providers::create;
use std::sync::Arc;

use crate::session::output;

pub enum PlannerResponseType {
    Plan,
    ClarifyingQuestions,
}

/// Decide if the planner's response is a plan or a clarifying question
///
/// This function is called after the planner has generated a response
/// to the user's message. The response is either a plan or a clarifying
/// question.
pub async fn classify_planner_response(
    message_text: String,
    provider: Arc<dyn Provider>,
) -> Result<PlannerResponseType> {
    let prompt = format!("The text below is the output from an AI model which can either provide a plan or list of clarifying questions. Based on the text below, decide if the output is a \"plan\" or \"clarifying questions\".\n---\n{message_text}");

    // Generate the description
    let message = Message::user().with_text(&prompt);
    let (result, _usage) = provider
        .complete(
            "Reply only with the classification label: \"plan\" or \"clarifying questions\"",
            &[message],
            &[],
        )
        .await?;

    // println!("classify_planner_response: {result:?}\n"); // TODO: remove

    let predicted = result.as_concat_text();
    if predicted.to_lowercase().contains("plan") {
        Ok(PlannerResponseType::Plan)
    } else {
        Ok(PlannerResponseType::ClarifyingQuestions)
    }
}

/// Get a reasoner provider based on configuration
///
/// Tries planner-specific provider/model first, falls back to default provider/model
pub fn get_reasoner() -> Result<Arc<dyn Provider>, anyhow::Error> {
    let config = Config::global();

    // Try planner-specific provider first, fallback to default provider
    let provider = if let Ok(provider) = config.get_param::<String>("GOOSE_PLANNER_PROVIDER") {
        provider
    } else {
        println!("WARNING: GOOSE_PLANNER_PROVIDER not found. Using default provider...");
        config
            .get_param::<String>("GOOSE_PROVIDER")
            .expect("No provider configured. Run 'goose configure' first")
    };

    // Try planner-specific model first, fallback to default model
    let model = if let Ok(model) = config.get_param::<String>("GOOSE_PLANNER_MODEL") {
        model
    } else {
        println!("WARNING: GOOSE_PLANNER_MODEL not found. Using default model...");
        config
            .get_param::<String>("GOOSE_MODEL")
            .expect("No model configured. Run 'goose configure' first")
    };

    let model_config = ModelConfig::new(model);
    let reasoner = create(&provider, model_config)?;

    Ok(reasoner)
}

/// Helper function to summarize context messages
///
/// This is a stateless utility function that summarizes messages to fit within context length
pub async fn summarize_context_messages(
    messages: &mut Vec<Message>,
    agent: &Agent,
    message_suffix: &str,
) -> Result<()> {
    // Summarize messages to fit within context length
    let (summarized_messages, _) = agent.summarize_context(messages).await?;
    let msg = format!("Context maxed out\n{}\n{}", "-".repeat(50), message_suffix);
    output::render_text(&msg, Some(Color::Yellow), true);
    *messages = summarized_messages;

    Ok(())
}
