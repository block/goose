//! ACP (Agent Communication Protocol) v0.2.0 compatibility layer.
//!
//! Provides ACP-compliant types and bidirectional converters between
//! goose's internal Message/Event model and the ACP REST wire format.

pub mod events;
pub mod manifest;
pub mod message;
pub mod types;

pub use events::{goosed_events_to_acp, AcpEvent, AcpEventContext, AcpEventType};
pub use manifest::{
    AgentDependency, AgentManifest, AgentMetadata, AgentModeInfo, AgentStatus, Link, Person,
};
pub use message::{
    acp_message_to_goose, goose_message_to_acp, AcpMessage, AcpMessagePart, AcpRole,
};
pub use types::{
    AcpError, AcpRun, AcpRunStatus, AcpSession, AwaitRequest, AwaitResume, RunCreateRequest,
    RunMode, RunResumeRequest,
};
