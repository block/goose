pub mod chat;
pub mod input;
pub mod status;
pub mod info;
pub mod popups;

use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::state::AppState;
use anyhow::Result;
use ratatui::{layout::Rect, Frame};

pub trait Component {
    fn handle_event(&mut self, _event: &Event, _state: &AppState) -> Result<Option<Action>> {
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState);
}
