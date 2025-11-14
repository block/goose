use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PromptInjectionDetector: Send + Sync {
    async fn scan(&self, text: &str) -> Result<f32>;
}
