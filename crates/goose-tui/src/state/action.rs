use crate::services::config::CustomCommand;
use goose::config::ExtensionEntry;
use goose::session::Session;
use goose_client::{ProviderDetails, ScheduledJob, SessionDisplayInfo, ToolInfo};
use goose_server::routes::reply::MessageEvent;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    Refresh,
    Resize,

    ServerMessage(Arc<MessageEvent>),
    SessionResumed(Box<Session>),
    SessionsListLoaded(Vec<Session>),
    ToolsLoaded(Vec<ToolInfo>),
    ProvidersLoaded(Vec<ProviderDetails>),
    ExtensionsLoaded(Vec<ExtensionEntry>),
    ModelsLoaded {
        provider: String,
        models: Vec<String>,
    },
    ConfigLoaded(serde_json::Value),
    Error(String),
    ShowFlash(String),

    SendMessage(goose::conversation::message::Message),
    SendMessageWithFlash {
        message: goose::conversation::message::Message,
        flash: String,
    },
    Interrupt,
    ToggleInputMode,

    ToggleTodo,
    ToggleHelp,
    OpenSessionPicker,
    ResumeSession(String),
    CreateNewSession,
    PreviewTheme(String),
    ChangeTheme(String),
    OpenThemePicker,
    ClearChat,
    ClosePopup,
    OpenMessageInfo(usize),
    SetInputEmpty(bool),
    OpenConfig,
    OpenMcp,
    FetchModels(String),
    UpdateProvider {
        provider: String,
        model: String,
    },
    ToggleExtension {
        name: String,
        enabled: bool,
    },

    DeleteCustomCommand(String),
    StartCommandBuilder,
    SubmitCommandBuilder(CustomCommand, String),

    ToggleCopyMode,
    CopyToClipboard(String),
    YankVisualSelection {
        text: String,
        count: usize,
    },
    EnterVisualMode,
    ExitVisualMode,
    ForkFromMessage(usize),
    SetGooseMode(String),
    ConfirmToolCall {
        id: String,
        approved: bool,
    },
    CwdAnalysisComplete(Option<String>),

    OpenSchedulePopup,
    RefreshSchedules,
    CreateSchedule {
        id: String,
        recipe_source: String,
        cron: String,
    },
    UpdateScheduleCron {
        id: String,
        cron: String,
    },
    DeleteSchedule(String),
    RunScheduleNow(String),
    PauseSchedule(String),
    UnpauseSchedule(String),
    KillSchedule(String),
    FetchScheduleSessions(String),
    #[allow(dead_code)]
    ScheduleListLoaded(Vec<ScheduledJob>),
    #[allow(dead_code)]
    ScheduleSessionsLoaded {
        schedule_id: String,
        sessions: Vec<SessionDisplayInfo>,
    },
    #[allow(dead_code)]
    ScheduleOperationSuccess(String),
    #[allow(dead_code)]
    ScheduleOperationFailed(String),
}
