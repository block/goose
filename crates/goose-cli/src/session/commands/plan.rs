use anyhow::Result;
use goose::message::Message;

use crate::session::{input, output, utils, RunMode, Session};

impl Session {
    pub async fn handle_plan_command(&mut self, options: input::PlanCommandOptions) -> Result<()> {
        self.run_mode = RunMode::Plan;
        output::render_enter_plan_mode();

        let message_text = options.message_text;
        if !message_text.is_empty() {
            let mut plan_messages = self.messages.clone();
            plan_messages.push(Message::user().with_text(&message_text));

            let reasoner = utils::get_reasoner()?;
            self.plan_with_reasoner_model(plan_messages, reasoner)
                .await?;
        }

        Ok(())
    }
}
