use crate::services::config::CustomCommand;
use crate::state::state::ToolInfo;
use goose::session::Session;
use goose_server::routes::reply::MessageEvent;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    Resize,

    ServerMessage(Arc<MessageEvent>),
    SessionResumed(Box<Session>),
    SessionsListLoaded(Vec<Session>),
    ToolsLoaded(Vec<ToolInfo>),
    Error(String),

    SendMessage(goose::conversation::message::Message),
    Interrupt,
    ToggleInputMode,

    ToggleTodo,
    ToggleHelp,
    OpenSessionPicker,
    ResumeSession(String),
    ChangeTheme(String),
    ClearChat,
    ClosePopup,
    OpenMessageInfo(usize),
    SetInputEmpty(bool),

    DeleteCustomCommand(String),
    StartCommandBuilder,
    SubmitCommandBuilder(CustomCommand),
}
