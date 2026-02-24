use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{CallToolRequestParams, Tool};
use serde::Deserialize;

use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{Provider, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum StepOutput {
    Text {
        text: String,
    },
    ToolRequest {
        id: String,
        name: String,
        #[serde(default)]
        arguments: HashMap<String, serde_json::Value>,
    },
}

#[derive(Debug, Deserialize)]
struct StepExpect {
    /// Substring that must appear in the last user text message.
    #[serde(default)]
    last_user_contains: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureStep {
    #[serde(default)]
    expect: Option<StepExpect>,
    output: StepOutput,
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    version: u32,
    steps: Vec<FixtureStep>,
}

/// Deterministic provider for hermetic integration tests.
///
/// This provider replays a fixed, ordered sequence of assistant outputs.
pub struct FixtureProvider {
    fixture: FixtureFile,
    cursor: Mutex<usize>,
}

impl FixtureProvider {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let fixture: FixtureFile = serde_json::from_str(&content)?;
        anyhow::ensure!(
            fixture.version == 1,
            "unsupported fixture version {}",
            fixture.version
        );
        Ok(Self {
            fixture,
            cursor: Mutex::new(0),
        })
    }

    fn next_step(&self) -> Result<FixtureStep, ProviderError> {
        let mut cursor = self.cursor.lock().unwrap();

        if self.fixture.steps.is_empty() {
            return Err(ProviderError::ExecutionError(
                "fixture has no steps".to_string(),
            ));
        }

        // The agent may call the provider multiple times per user turn (e.g. retries,
        // tool availability recovery). For hermetic tests we prefer deterministic
        // output over hard failure, so once the fixture is exhausted we keep returning
        // the last step.
        let idx = (*cursor).min(self.fixture.steps.len() - 1);
        let step =
            self.fixture.steps.get(idx).cloned().ok_or_else(|| {
                ProviderError::ExecutionError(format!("fixture missing step {idx}"))
            })?;

        if *cursor < self.fixture.steps.len() {
            *cursor += 1;
        }

        Ok(step)
    }

    fn validate_tools(&self, _tools: &[Tool]) -> Result<(), ProviderError> {
        // Intentionally no-op.
        //
        // Tool availability in Goose can be mode/route dependent (e.g. tool groups), and
        // the hermetic scenario suite validates tool execution by asserting on the
        // resulting assistant output, not on the prompt-time tool list.
        Ok(())
    }

    fn last_user_text(messages: &[Message]) -> Option<String> {
        messages.iter().rev().find_map(|m| {
            if m.role == rmcp::model::Role::User {
                let text = m.as_concat_text();
                (!text.trim().is_empty()).then_some(text)
            } else {
                None
            }
        })
    }

    fn apply_expect(
        expect: &Option<StepExpect>,
        messages: &[Message],
    ) -> Result<(), ProviderError> {
        let Some(expect) = expect else {
            return Ok(());
        };
        let Some(substr) = &expect.last_user_contains else {
            return Ok(());
        };

        let actual = Self::last_user_text(messages).unwrap_or_default();
        if !actual.contains(substr) {
            return Err(ProviderError::ExecutionError(format!(
                "fixture expectation failed: last_user_contains={substr:?}, actual_last_user={actual:?}"
            )));
        }

        Ok(())
    }

    fn step_to_message(step: &FixtureStep) -> Message {
        match &step.output {
            StepOutput::Text { text } => Message::assistant().with_text(text),
            StepOutput::ToolRequest {
                id,
                name,
                arguments,
            } => {
                let tool_call = CallToolRequestParams {
                    meta: None,
                    task: None,
                    name: name.clone().into(),
                    arguments: Some(
                        arguments
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    ),
                };
                Message::assistant().with_tool_request(id, Ok(tool_call))
            }
        }
    }
}

#[async_trait]
impl Provider for FixtureProvider {
    fn get_name(&self) -> &str {
        "fixture"
    }

    async fn complete_with_model(
        &self,
        _session_id: Option<&str>,
        _model_config: &ModelConfig,
        _system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        self.validate_tools(tools)?;

        let step = self.next_step()?;
        Self::apply_expect(&step.expect, messages)?;

        let msg = Self::step_to_message(&step);
        let usage = ProviderUsage::new("fixture".to_string(), Usage::default());
        Ok((msg, usage))
    }

    fn get_model_config(&self) -> ModelConfig {
        ModelConfig::new_or_fail("test-model")
    }
}

impl Clone for FixtureStep {
    fn clone(&self) -> Self {
        Self {
            expect: self.expect.clone(),
            output: match &self.output {
                StepOutput::Text { text } => StepOutput::Text { text: text.clone() },
                StepOutput::ToolRequest {
                    id,
                    name,
                    arguments,
                } => StepOutput::ToolRequest {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: arguments.clone(),
                },
            },
        }
    }
}

impl Clone for StepExpect {
    fn clone(&self) -> Self {
        Self {
            last_user_contains: self.last_user_contains.clone(),
        }
    }
}
