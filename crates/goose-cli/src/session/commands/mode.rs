use anyhow::Result;
use goose::config::Config;
use goose::message::Message;
use goose::providers::base::Provider;
use serde_json::Value;
use std::sync::Arc;

use crate::session::{output, utils, RunMode, Session};

impl Session {
    pub fn handle_goose_mode_setting(&self, mode: &str) -> Result<bool> {
        let config = Config::global();
        let mode = mode.to_lowercase();

        // Check if mode is valid
        if !["auto", "approve", "chat", "smart_approve"].contains(&mode.as_str()) {
            output::render_error(&format!(
                "Invalid mode '{}'. Mode must be one of: auto, approve, chat",
                mode
            ));
            return Ok(false);
        }

        config
            .set_param("GOOSE_MODE", Value::String(mode.to_string()))
            .unwrap();
        output::goose_mode_message(&format!("Goose mode set to '{}'", mode));

        Ok(true)
    }

    pub async fn handle_normal_mode_message(&mut self, content: &str) -> Result<()> {
        self.add_user_message_to_history(content).await?;
        self.update_project_tracker(content)?;

        output::show_thinking();
        self.process_agent_response(true).await?;
        output::hide_thinking();

        Ok(())
    }

    pub async fn handle_plan_mode_message(&mut self, content: &str) -> Result<()> {
        let mut plan_messages = self.messages.clone();
        plan_messages.push(Message::user().with_text(content));
        let reasoner = utils::get_reasoner()?;
        self.plan_with_reasoner_model(plan_messages, reasoner)
            .await?;

        Ok(())
    }

    pub async fn plan_with_reasoner_model(
        &mut self,
        plan_messages: Vec<Message>,
        reasoner: Arc<dyn Provider>,
    ) -> Result<(), anyhow::Error> {
        let plan_prompt = self.agent.get_plan_prompt().await?;
        output::show_thinking();
        let (plan_response, _usage) = reasoner.complete(&plan_prompt, &plan_messages, &[]).await?;
        output::render_message(&plan_response, self.debug);
        output::hide_thinking();
        let planner_response_type = utils::classify_planner_response(
            plan_response.as_concat_text(),
            self.agent.provider().await?,
        )
        .await?;

        match planner_response_type {
            utils::PlannerResponseType::Plan => {
                println!();
                let should_act = match cliclack::confirm(
                    "Do you want to clear message history & act on this plan?",
                )
                .initial_value(true)
                .interact()
                {
                    Ok(choice) => choice,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::Interrupted {
                            false // If interrupted, set should_act to false
                        } else {
                            return Err(e.into());
                        }
                    }
                };
                if should_act {
                    output::render_act_on_plan();
                    self.run_mode = RunMode::Normal;
                    // set goose mode: auto if that isn't already the case
                    let config = Config::global();
                    let curr_goose_mode =
                        config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
                    if curr_goose_mode != "auto" {
                        config
                            .set_param("GOOSE_MODE", Value::String("auto".to_string()))
                            .unwrap();
                    }

                    // clear the messages before acting on the plan
                    self.messages.clear();
                    // add the plan response as a user message
                    let plan_message = Message::user().with_text(plan_response.as_concat_text());
                    self.messages.push(plan_message);
                    // act on the plan
                    output::show_thinking();
                    self.process_agent_response(true).await?;
                    output::hide_thinking();

                    // Reset run & goose mode
                    if curr_goose_mode != "auto" {
                        config
                            .set_param("GOOSE_MODE", Value::String(curr_goose_mode.to_string()))
                            .unwrap();
                    }
                } else {
                    // add the plan response (assistant message) & carry the conversation forward
                    // in the next round, the user might wanna slightly modify the plan
                    self.messages.push(plan_response);
                }
            }
            utils::PlannerResponseType::ClarifyingQuestions => {
                // add the plan response (assistant message) & carry the conversation forward
                // in the next round, the user will answer the clarifying questions
                self.messages.push(plan_response);
            }
        }

        Ok(())
    }
}
