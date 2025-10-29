use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::base::{Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestInput {
    system: String,
    messages: Vec<Message>,
    tools: Vec<Tool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestOutput {
    message: Message,
    usage: ProviderUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestRecord {
    input: TestInput,
    output: TestOutput,
}

pub struct TestProvider {
    inner: Option<Arc<dyn Provider>>,
    records: Arc<Mutex<HashMap<String, TestRecord>>>,
    file_path: String,
    name: String,
}

impl TestProvider {
    pub fn new_recording(inner: Arc<dyn Provider>, file_path: impl Into<String>) -> Self {
        Self {
            inner: Some(inner),
            records: Arc::new(Mutex::new(HashMap::new())),
            file_path: file_path.into(),
            name: Self::metadata().name,
        }
    }

    pub fn new_replaying(file_path: impl Into<String>) -> Result<Self> {
        let file_path = file_path.into();
        let records = Self::load_records(&file_path)?;

        Ok(Self {
            inner: None,
            records: Arc::new(Mutex::new(records)),
            file_path,
            name: Self::metadata().name,
        })
    }

    pub fn finish_recording(self) -> Result<()> {
        if self.inner.is_some() {
            self.save_records()?;
        }
        Ok(())
    }

    fn hash_input(messages: &[Message]) -> String {
        let stable_messages: Vec<_> = messages
            .iter()
            .map(|msg| (msg.role.clone(), msg.content.clone()))
            .collect();
        let serialized = serde_json::to_string(&stable_messages).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn load_records(file_path: &str) -> Result<HashMap<String, TestRecord>> {
        if !Path::new(file_path).exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(file_path)?;
        let records: HashMap<String, TestRecord> = serde_json::from_str(&content)?;
        Ok(records)
    }

    pub fn save_records(&self) -> Result<()> {
        let records = self.records.lock().unwrap();
        let content = serde_json::to_string_pretty(&*records)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }

    pub fn get_record_count(&self) -> usize {
        self.records.lock().unwrap().len()
    }
}

#[async_trait]
impl Provider for TestProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "test",
            "Test Provider",
            "Provider for testing that can record/replay interactions",
            "test-model",
            vec!["test-model"],
            "",
            vec![],
        )
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn complete_with_model(
        &self,
        _model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let hash = Self::hash_input(messages);

        if let Some(inner) = &self.inner {
            let (message, usage) = inner.complete(system, messages, tools).await?;

            let record = TestRecord {
                input: TestInput {
                    system: system.to_string(),
                    messages: messages.to_vec(),
                    tools: tools.to_vec(),
                },
                output: TestOutput {
                    message: message.clone(),
                    usage: usage.clone(),
                },
            };

            {
                let mut records = self.records.lock().unwrap();
                records.insert(hash, record);
            }

            Ok((message, usage))
        } else {
            let records = self.records.lock().unwrap();
            if let Some(record) = records.get(&hash) {
                Ok((record.output.message.clone(), record.output.usage.clone()))
            } else {
                Err(ProviderError::ExecutionError(format!(
                    "No recorded response found for input hash: {}",
                    hash
                )))
            }
        }
    }

    fn get_model_config(&self) -> ModelConfig {
        ModelConfig::new_or_fail("test-model")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent};
    use crate::providers::base::{ProviderUsage, Usage};
    use chrono::Utc;
    use rmcp::model::{RawTextContent, Role, TextContent};
    use std::env;

    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
        response: String,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::new(
                "mock",
                "Mock Provider",
                "Mock provider for testing",
                "mock-model",
                vec!["mock-model"],
                "",
                vec![],
            )
        }

        fn get_name(&self) -> &str {
            "mock-testprovider"
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::new(
                    Role::Assistant,
                    Utc::now().timestamp(),
                    vec![MessageContent::Text(TextContent {
                        raw: RawTextContent {
                            text: self.response.clone(),
                            meta: None,
                        },
                        annotations: None,
                    })],
                ),
                ProviderUsage::new("mock-model".to_string(), Usage::default()),
            ))
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }
    }

    #[tokio::test]
    async fn test_record_and_replay() {
        let temp_file = format!(
            "{}/test_records_{}.json",
            env::temp_dir().display(),
            std::process::id()
        );

        let mock = Arc::new(MockProvider {
            model_config: ModelConfig::new_or_fail("mock-model"),
            response: "Hello, world!".to_string(),
        });

        {
            let test_provider = TestProvider::new_recording(mock, &temp_file);

            let result = test_provider.complete("You are helpful", &[], &[]).await;

            assert!(result.is_ok());
            let (message, _) = result.unwrap();

            if let MessageContent::Text(content) = &message.content[0] {
                assert_eq!(content.text, "Hello, world!");
            }

            assert_eq!(test_provider.get_record_count(), 1);
            test_provider.finish_recording().unwrap();
        }

        {
            let replay_provider = TestProvider::new_replaying(&temp_file).unwrap();

            let result = replay_provider.complete("You are helpful", &[], &[]).await;

            assert!(result.is_ok());
            let (message, _) = result.unwrap();

            if let MessageContent::Text(content) = &message.content[0] {
                assert_eq!(content.text, "Hello, world!");
            }
        }

        let _ = fs::remove_file(temp_file);
    }

    #[tokio::test]
    async fn test_replay_missing_record() {
        let temp_file = format!(
            "{}/test_missing_{}.json",
            env::temp_dir().display(),
            std::process::id()
        );

        let replay_provider = TestProvider::new_replaying(&temp_file).unwrap();

        let result = replay_provider
            .complete("Different system prompt", &[], &[])
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No recorded response found"));

        let _ = fs::remove_file(temp_file);
    }
}
