pub mod acp_mcp_adapter;
pub mod client;
pub mod health;
pub mod service_broker;
pub mod spawner;
pub mod task;

pub use acp_mcp_adapter::AcpMcpAdapter;
pub use health::{AgentHealth, AgentState, AgentStatus};
pub use service_broker::ServiceBroker;
pub use task::{TaskManager, TaskState, TaskStatus};

// Re-export commonly used ACP schema types for downstream crates
pub use agent_client_protocol_schema::{
    NewSessionRequest, NewSessionResponse, SessionId, SessionModeId, SetSessionModeRequest,
    SetSessionModeResponse,
};
