pub mod chat;
pub mod info;
pub mod input;
pub mod popups;
pub mod status;

use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use anyhow::Result;
use ratatui::{layout::Rect, Frame};

pub trait Component {
    fn handle_event(&mut self, _event: &Event, _state: &AppState) -> Result<Option<Action>> {
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState);
}
