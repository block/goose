use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use goose::{
    agent::Agent as GooseAgent, message::Message, providers::base::ProviderUsage, systems::System,
};

#[async_trait]
pub trait Agent {
    fn add_system(&mut self, system: Box<dyn System>);
    async fn reply(&self, messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>>;
    async fn usage(&self) -> Result<Vec<ProviderUsage>>;
}

#[async_trait]
impl Agent for GooseAgent {
    fn add_system(&mut self, system: Box<dyn System>) {
        self.add_system(system);
    }

    async fn reply(&self, messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>> {
        self.reply(messages).await
    }

    async fn usage(&self) -> Result<Vec<ProviderUsage>> {
        self.usage().await
    }
}
