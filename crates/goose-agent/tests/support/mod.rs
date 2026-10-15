use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use goose_agent::{
    machine::{EffectHandler, MachineSession, SessionLoader},
    operation::{ConversationEffect, Emitter},
};
use goose_provider_types::conversation::{message::Message, Conversation};

#[derive(Clone)]
pub struct Session(pub Conversation);

impl MachineSession for Session {
    fn id(&self) -> &str {
        "session"
    }

    fn conversation(&self) -> Option<&Conversation> {
        Some(&self.0)
    }
}

pub struct Runtime(Mutex<Session>);

impl Runtime {
    pub fn new(messages: impl IntoIterator<Item = Message>) -> Self {
        Self(Mutex::new(Session(Conversation::new_unvalidated(messages))))
    }
}

#[async_trait]
impl SessionLoader<Session> for Runtime {
    async fn load(&self, _session_id: &str) -> Result<Session> {
        Ok(self.0.lock().unwrap().clone())
    }
}

#[async_trait]
impl EffectHandler<Session, ConversationEffect> for Runtime {
    async fn apply_effects(
        &self,
        _session: &Session,
        effects: &mut [ConversationEffect],
        _emit: &Emitter,
    ) -> Result<()> {
        for effect in effects {
            let ConversationEffect::AppendMessage(message) = effect else {
                panic!("unexpected effect");
            };
            assert!(message.id.is_some());
            self.0.lock().unwrap().0.push(message.clone());
        }
        Ok(())
    }
}

pub fn texts(session: &Session) -> Vec<String> {
    session.0.iter().map(Message::as_concat_text).collect()
}
