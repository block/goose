use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use goose::{models::message::Message, systems::System};

use crate::agents::agent::Agent;

pub struct MockAgent;

#[async_trait]
impl Agent for MockAgent {
    fn add_system(&mut self, _system: Box<dyn System>) {}

    async fn reply(&self, _messages: &[Message]) -> Result<BoxStream<'_, Result<Message>>> {
        Ok(Box::pin(futures::stream::empty()))
    }
}
