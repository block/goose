//! Temporary completion tracking for claude-agent-acp issue #934.
//!
//! Claude ACP can complete the original `session/prompt` before an injected steering command
//! finishes, which makes Goose close the stream before the steered output arrives. Remove this
//! module and its provider integration once Claude ACP keeps the prompt open until every accepted
//! injected command has completed.

use agent_client_protocol::schema::v1::{ExtNotification, Meta, PromptResponse};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

const SDK_MESSAGE_METHOD: &str = "claude/sdkMessage";
const MESSAGE_LIFECYCLE_CAPABILITY: &str = "msg_lifecycle_v1";

fn lifecycle_session_meta() -> Meta {
    let mut meta = Meta::new();
    meta.insert(
        "claudeCode".to_string(),
        serde_json::json!({
            "emitRawSDKMessages": [
                { "type": "system", "subtype": "init" },
                { "type": "user", "origin": "human" },
                { "type": "command_lifecycle" }
            ]
        }),
    );
    meta
}

enum ClaudeLifecycleSignal {
    Init {
        message_lifecycle: bool,
    },
    HumanUserEcho {
        command_uuid: String,
    },
    CommandLifecycle {
        command_uuid: String,
        state: ClaudeCommandLifecycleState,
    },
}

fn lifecycle_signal(
    notification: &ExtNotification,
) -> Result<Option<ClaudeLifecycleSignal>, serde_json::Error> {
    if notification.method.as_ref() != SDK_MESSAGE_METHOD {
        return Ok(None);
    }

    let notification: ClaudeSdkNotification = serde_json::from_str(notification.params.get())?;
    Ok(match notification.message {
        ClaudeSdkMessage::System {
            subtype,
            capabilities,
        } if subtype == "init" => Some(ClaudeLifecycleSignal::Init {
            message_lifecycle: capabilities
                .iter()
                .any(|capability| capability == MESSAGE_LIFECYCLE_CAPABILITY),
        }),
        ClaudeSdkMessage::User { uuid } => {
            Some(ClaudeLifecycleSignal::HumanUserEcho { command_uuid: uuid })
        }
        ClaudeSdkMessage::CommandLifecycle {
            command_uuid,
            state,
        } => Some(ClaudeLifecycleSignal::CommandLifecycle {
            command_uuid,
            state: serde_json::from_value(state).unwrap_or(ClaudeCommandLifecycleState::Unknown),
        }),
        ClaudeSdkMessage::System { .. } | ClaudeSdkMessage::Other => None,
    })
}

#[derive(Deserialize)]
struct ClaudeSdkNotification {
    message: ClaudeSdkMessage,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeSdkMessage {
    System {
        subtype: String,
        #[serde(default)]
        capabilities: Vec<String>,
    },
    User {
        uuid: String,
    },
    CommandLifecycle {
        command_uuid: String,
        #[serde(default)]
        state: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ClaudeCommandLifecycleState {
    Queued,
    Started,
    Completed,
    Cancelled,
    Discarded,
    #[serde(other)]
    Unknown,
}

impl ClaudeCommandLifecycleState {
    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Discarded | Self::Unknown
        )
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) struct ClaudeSteeringAttemptId(u64);

pub(super) enum ClaudeCompletionAction {
    None,
    Complete(PromptResponse),
    Fail(String),
}

#[derive(Clone, Default)]
pub(super) struct ClaudeSteeringCompletionWorkaround {
    lifecycle: Arc<Mutex<ClaudeSteeringLifecycle>>,
}

impl ClaudeSteeringCompletionWorkaround {
    pub(super) fn session_meta() -> Meta {
        lifecycle_session_meta()
    }

    pub(super) fn native_steering_available(&self) -> bool {
        self.lifecycle.lock().unwrap().native_steering_available()
    }

    pub(super) fn observe_notification(
        &self,
        notification: &ExtNotification,
    ) -> ClaudeCompletionAction {
        match lifecycle_signal(notification) {
            Ok(Some(signal)) => self.lifecycle.lock().unwrap().observe(signal),
            Ok(None) => ClaudeCompletionAction::None,
            Err(_) => self.lifecycle.lock().unwrap().malformed_notification(),
        }
    }

    pub(super) fn register_prompt(&self) {
        self.lifecycle.lock().unwrap().register_prompt();
    }

    pub(super) fn register_steering(&self) -> ClaudeSteeringAttemptId {
        self.lifecycle.lock().unwrap().register_steering()
    }

    pub(super) fn steering_injected(
        &self,
        attempt_id: ClaudeSteeringAttemptId,
    ) -> ClaudeCompletionAction {
        self.lifecycle.lock().unwrap().steering_injected(attempt_id)
    }

    pub(super) fn steering_not_injected(
        &self,
        attempt_id: ClaudeSteeringAttemptId,
    ) -> ClaudeCompletionAction {
        self.lifecycle
            .lock()
            .unwrap()
            .steering_not_injected(attempt_id)
    }

    pub(super) fn prompt_response(&self, response: PromptResponse) -> ClaudeCompletionAction {
        self.lifecycle.lock().unwrap().prompt_response(response)
    }

    pub(super) fn prompt_finished(&self) {
        self.lifecycle.lock().unwrap().prompt_finished();
    }
}

#[derive(Default)]
struct ClaudeSteeringLifecycle {
    lifecycle_supported: bool,
    native_steering_disabled: bool,
    prompt_active: bool,
    next_attempt_id: u64,
    expected_host_commands: VecDeque<ExpectedHostCommand>,
    command_owners: HashMap<String, HostCommandOwner>,
    early_lifecycle: HashMap<String, ClaudeCommandLifecycleState>,
    steering_attempts: HashMap<ClaudeSteeringAttemptId, SteeringAttempt>,
    deferred_prompt_response: Option<PromptResponse>,
}

impl ClaudeSteeringLifecycle {
    fn native_steering_available(&self) -> bool {
        self.lifecycle_supported && !self.native_steering_disabled
    }

    fn register_prompt(&mut self) {
        self.prompt_active = true;
        self.expected_host_commands
            .push_back(ExpectedHostCommand::Prompt);
    }

    fn register_steering(&mut self) -> ClaudeSteeringAttemptId {
        let attempt_id = ClaudeSteeringAttemptId(self.next_attempt_id);
        self.next_attempt_id += 1;
        self.steering_attempts
            .insert(attempt_id, SteeringAttempt::default());
        self.expected_host_commands
            .push_back(ExpectedHostCommand::Steering(attempt_id));
        attempt_id
    }

    fn steering_injected(&mut self, attempt_id: ClaudeSteeringAttemptId) -> ClaudeCompletionAction {
        if let Some(attempt) = self.steering_attempts.get_mut(&attempt_id) {
            attempt.injected = true;
        }
        self.completion_action()
    }

    fn steering_not_injected(
        &mut self,
        attempt_id: ClaudeSteeringAttemptId,
    ) -> ClaudeCompletionAction {
        self.remove_steering_attempt(attempt_id);
        self.completion_action()
    }

    fn prompt_response(&mut self, response: PromptResponse) -> ClaudeCompletionAction {
        if !self.prompt_active {
            return ClaudeCompletionAction::None;
        }
        if self.has_outstanding_steering() {
            self.deferred_prompt_response = Some(response);
            ClaudeCompletionAction::None
        } else {
            self.reset_active_prompt();
            ClaudeCompletionAction::Complete(response)
        }
    }

    fn prompt_finished(&mut self) {
        self.reset_active_prompt();
    }

    fn observe(&mut self, signal: ClaudeLifecycleSignal) -> ClaudeCompletionAction {
        match signal {
            ClaudeLifecycleSignal::Init {
                message_lifecycle: true,
            } => self.lifecycle_supported = true,
            ClaudeLifecycleSignal::Init {
                message_lifecycle: false,
            } => {}
            ClaudeLifecycleSignal::HumanUserEcho { command_uuid } => {
                self.associate_command(command_uuid);
            }
            ClaudeLifecycleSignal::CommandLifecycle {
                command_uuid,
                state,
            } => self.observe_command_lifecycle(command_uuid, state),
        }
        self.completion_action()
    }

    fn malformed_notification(&mut self) -> ClaudeCompletionAction {
        self.native_steering_disabled = true;
        if self.has_outstanding_steering() {
            self.reset_active_prompt();
            ClaudeCompletionAction::Fail(
                "Claude sent a malformed lifecycle notification for an injected command"
                    .to_string(),
            )
        } else {
            ClaudeCompletionAction::None
        }
    }

    fn associate_command(&mut self, command_uuid: String) {
        let Some(expected) = self.expected_host_commands.pop_front() else {
            return;
        };
        let owner = match expected {
            ExpectedHostCommand::Prompt => HostCommandOwner::Prompt,
            ExpectedHostCommand::Steering(attempt_id) => {
                let Some(attempt) = self.steering_attempts.get_mut(&attempt_id) else {
                    return;
                };
                attempt.command_uuid = Some(command_uuid.clone());
                HostCommandOwner::Steering(attempt_id)
            }
        };
        self.command_owners.insert(command_uuid.clone(), owner);
        if let Some(state) = self.early_lifecycle.remove(&command_uuid) {
            self.apply_command_lifecycle(&command_uuid, state);
        }
    }

    fn observe_command_lifecycle(
        &mut self,
        command_uuid: String,
        state: ClaudeCommandLifecycleState,
    ) {
        if self.command_owners.contains_key(&command_uuid) {
            self.apply_command_lifecycle(&command_uuid, state);
        } else if self.prompt_active && !self.expected_host_commands.is_empty() {
            let current = self.early_lifecycle.get(&command_uuid).copied();
            if !current.is_some_and(ClaudeCommandLifecycleState::is_terminal) {
                self.early_lifecycle.insert(command_uuid, state);
            }
        }
    }

    fn apply_command_lifecycle(&mut self, command_uuid: &str, state: ClaudeCommandLifecycleState) {
        let Some(owner) = self.command_owners.get(command_uuid).copied() else {
            return;
        };
        match owner {
            HostCommandOwner::Prompt => {
                if state.is_terminal() {
                    self.command_owners.remove(command_uuid);
                }
            }
            HostCommandOwner::Steering(attempt_id) => {
                if let Some(attempt) = self.steering_attempts.get_mut(&attempt_id) {
                    if !attempt
                        .lifecycle_state
                        .is_some_and(ClaudeCommandLifecycleState::is_terminal)
                    {
                        attempt.lifecycle_state = Some(state);
                    }
                }
            }
        }
    }

    fn remove_steering_attempt(&mut self, attempt_id: ClaudeSteeringAttemptId) {
        self.expected_host_commands
            .retain(|command| *command != ExpectedHostCommand::Steering(attempt_id));
        if let Some(attempt) = self.steering_attempts.remove(&attempt_id) {
            if let Some(command_uuid) = attempt.command_uuid {
                self.command_owners.remove(&command_uuid);
            }
        }
    }

    fn has_outstanding_steering(&self) -> bool {
        self.steering_attempts.values().any(|attempt| {
            !attempt.injected
                || attempt.lifecycle_state != Some(ClaudeCommandLifecycleState::Completed)
        })
    }

    fn completion_action(&mut self) -> ClaudeCompletionAction {
        let failure = self.steering_attempts.values().find_map(|attempt| {
            if !attempt.injected {
                return None;
            }
            match attempt.lifecycle_state {
                Some(ClaudeCommandLifecycleState::Cancelled) => {
                    Some("Claude cancelled an injected steering command")
                }
                Some(ClaudeCommandLifecycleState::Discarded) => {
                    Some("Claude discarded an injected steering command")
                }
                Some(ClaudeCommandLifecycleState::Unknown) => {
                    Some("Claude reported an unknown lifecycle state for an injected command")
                }
                _ => None,
            }
        });
        if let Some(message) = failure {
            self.native_steering_disabled = true;
            self.reset_active_prompt();
            return ClaudeCompletionAction::Fail(message.to_string());
        }
        if self.has_outstanding_steering() {
            return ClaudeCompletionAction::None;
        }
        if let Some(response) = self.deferred_prompt_response.take() {
            self.reset_active_prompt();
            return ClaudeCompletionAction::Complete(response);
        }
        ClaudeCompletionAction::None
    }

    fn reset_active_prompt(&mut self) {
        self.prompt_active = false;
        self.expected_host_commands.clear();
        self.command_owners.clear();
        self.early_lifecycle.clear();
        self.steering_attempts.clear();
        self.deferred_prompt_response = None;
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ExpectedHostCommand {
    Prompt,
    Steering(ClaudeSteeringAttemptId),
}

#[derive(Clone, Copy)]
enum HostCommandOwner {
    Prompt,
    Steering(ClaudeSteeringAttemptId),
}

#[derive(Default)]
struct SteeringAttempt {
    command_uuid: Option<String>,
    injected: bool,
    lifecycle_state: Option<ClaudeCommandLifecycleState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::StopReason;
    use serde_json::value::RawValue;
    use std::sync::Arc;

    fn response() -> PromptResponse {
        PromptResponse::new(StopReason::EndTurn)
    }

    fn echo(command_uuid: &str) -> ClaudeLifecycleSignal {
        ClaudeLifecycleSignal::HumanUserEcho {
            command_uuid: command_uuid.to_string(),
        }
    }

    fn command_lifecycle(
        command_uuid: &str,
        state: ClaudeCommandLifecycleState,
    ) -> ClaudeLifecycleSignal {
        ClaudeLifecycleSignal::CommandLifecycle {
            command_uuid: command_uuid.to_string(),
            state,
        }
    }

    fn sdk_notification(message: serde_json::Value) -> ExtNotification {
        let params = RawValue::from_string(
            serde_json::json!({
                "sessionId": "session",
                "message": message,
            })
            .to_string(),
        )
        .unwrap();
        ExtNotification::new(SDK_MESSAGE_METHOD, Arc::from(params))
    }

    fn active_lifecycle() -> ClaudeSteeringLifecycle {
        let mut lifecycle = ClaudeSteeringLifecycle::default();
        assert!(matches!(
            lifecycle.observe(ClaudeLifecycleSignal::Init {
                message_lifecycle: true,
            }),
            ClaudeCompletionAction::None
        ));
        lifecycle.register_prompt();
        assert!(matches!(
            lifecycle.observe(echo("prompt")),
            ClaudeCompletionAction::None
        ));
        lifecycle
    }

    fn register_injected(
        lifecycle: &mut ClaudeSteeringLifecycle,
        command_uuid: &str,
    ) -> ClaudeSteeringAttemptId {
        let attempt_id = lifecycle.register_steering();
        assert!(matches!(
            lifecycle.observe(echo(command_uuid)),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.steering_injected(attempt_id),
            ClaudeCompletionAction::None
        ));
        attempt_id
    }

    #[test]
    fn capability_must_be_observed_before_native_steering_is_available() {
        let mut lifecycle = ClaudeSteeringLifecycle::default();
        assert!(!lifecycle.native_steering_available());
        let _ = lifecycle.observe(ClaudeLifecycleSignal::Init {
            message_lifecycle: false,
        });
        assert!(!lifecycle.native_steering_available());
        let _ = lifecycle.observe(ClaudeLifecycleSignal::Init {
            message_lifecycle: true,
        });
        assert!(lifecycle.native_steering_available());
    }

    #[test]
    fn parses_only_the_required_sdk_signal_fields() {
        assert!(matches!(
            lifecycle_signal(&sdk_notification(serde_json::json!({
                "type": "system",
                "subtype": "init",
                "capabilities": ["msg_lifecycle_v1"],
            }))),
            Ok(Some(ClaudeLifecycleSignal::Init {
                message_lifecycle: true
            }))
        ));
        assert!(matches!(
            lifecycle_signal(&sdk_notification(serde_json::json!({
                "type": "user",
                "uuid": "steer",
                "message": { "content": "discarded by parser" },
            }))),
            Ok(Some(ClaudeLifecycleSignal::HumanUserEcho { command_uuid }))
                if command_uuid == "steer"
        ));
        assert!(matches!(
            lifecycle_signal(&sdk_notification(serde_json::json!({
                "type": "command_lifecycle",
                "command_uuid": "steer",
                "state": "completed",
            }))),
            Ok(Some(ClaudeLifecycleSignal::CommandLifecycle {
                command_uuid,
                state: ClaudeCommandLifecycleState::Completed,
            })) if command_uuid == "steer"
        ));
    }

    #[test]
    fn malformed_tracked_lifecycle_state_is_terminal_failure() {
        assert!(matches!(
            lifecycle_signal(&sdk_notification(serde_json::json!({
                "type": "command_lifecycle",
                "command_uuid": "steer",
                "state": { "unexpected": true },
            }))),
            Ok(Some(ClaudeLifecycleSignal::CommandLifecycle {
                command_uuid,
                state: ClaudeCommandLifecycleState::Unknown,
            })) if command_uuid == "steer"
        ));
    }

    #[test]
    fn malformed_notification_fails_outstanding_steering_and_disables_it() {
        let workaround = ClaudeSteeringCompletionWorkaround::default();
        assert!(matches!(
            workaround.observe_notification(&sdk_notification(serde_json::json!({
                "type": "system",
                "subtype": "init",
                "capabilities": ["msg_lifecycle_v1"],
            }))),
            ClaudeCompletionAction::None
        ));
        workaround.register_prompt();
        assert!(matches!(
            workaround.observe_notification(&sdk_notification(serde_json::json!({
                "type": "user",
                "uuid": "prompt",
            }))),
            ClaudeCompletionAction::None
        ));
        let attempt_id = workaround.register_steering();
        assert!(matches!(
            workaround.steering_injected(attempt_id),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            workaround.prompt_response(response()),
            ClaudeCompletionAction::None
        ));

        assert!(matches!(
            workaround.observe_notification(&sdk_notification(serde_json::json!({
                "type": "user"
            }))),
            ClaudeCompletionAction::Fail(_)
        ));
        assert!(!workaround.native_steering_available());
    }

    #[test]
    fn holds_prompt_response_until_injected_command_completes() {
        let mut lifecycle = active_lifecycle();
        register_injected(&mut lifecycle, "steer");

        assert!(matches!(
            lifecycle.prompt_response(response()),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.observe(command_lifecycle(
                "steer",
                ClaudeCommandLifecycleState::Started,
            )),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.observe(command_lifecycle(
                "steer",
                ClaudeCommandLifecycleState::Completed,
            )),
            ClaudeCompletionAction::Complete(_)
        ));
    }

    #[test]
    fn completed_command_allows_later_prompt_response_to_complete() {
        let mut lifecycle = active_lifecycle();
        register_injected(&mut lifecycle, "steer");

        assert!(matches!(
            lifecycle.observe(command_lifecycle(
                "steer",
                ClaudeCommandLifecycleState::Completed,
            )),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.prompt_response(response()),
            ClaudeCompletionAction::Complete(_)
        ));
    }

    #[test]
    fn unresolved_steering_response_holds_prompt_completion() {
        let mut lifecycle = active_lifecycle();
        let attempt_id = lifecycle.register_steering();
        let _ = lifecycle.observe(echo("steer"));

        assert!(matches!(
            lifecycle.prompt_response(response()),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.steering_injected(attempt_id),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.observe(command_lifecycle(
                "steer",
                ClaudeCommandLifecycleState::Completed,
            )),
            ClaudeCompletionAction::Complete(_)
        ));
    }

    #[test]
    fn lifecycle_before_user_echo_is_retained() {
        let mut lifecycle = active_lifecycle();
        let attempt_id = lifecycle.register_steering();
        let _ = lifecycle.observe(command_lifecycle(
            "steer",
            ClaudeCommandLifecycleState::Completed,
        ));
        let _ = lifecycle.observe(echo("steer"));
        let _ = lifecycle.steering_injected(attempt_id);

        assert!(matches!(
            lifecycle.prompt_response(response()),
            ClaudeCompletionAction::Complete(_)
        ));
    }

    #[test]
    fn prompt_required_releases_a_held_prompt_response() {
        let mut lifecycle = active_lifecycle();
        let attempt_id = lifecycle.register_steering();
        assert!(matches!(
            lifecycle.prompt_response(response()),
            ClaudeCompletionAction::None
        ));

        assert!(matches!(
            lifecycle.steering_not_injected(attempt_id),
            ClaudeCompletionAction::Complete(_)
        ));
    }

    #[test]
    fn waits_for_every_injected_command() {
        let mut lifecycle = active_lifecycle();
        register_injected(&mut lifecycle, "steer-1");
        register_injected(&mut lifecycle, "steer-2");
        let _ = lifecycle.prompt_response(response());

        assert!(matches!(
            lifecycle.observe(command_lifecycle(
                "steer-1",
                ClaudeCommandLifecycleState::Completed,
            )),
            ClaudeCompletionAction::None
        ));
        assert!(matches!(
            lifecycle.observe(command_lifecycle(
                "steer-2",
                ClaudeCommandLifecycleState::Completed,
            )),
            ClaudeCompletionAction::Complete(_)
        ));
    }

    #[test]
    fn terminal_failure_fails_once_and_disables_native_steering() {
        for state in [
            ClaudeCommandLifecycleState::Cancelled,
            ClaudeCommandLifecycleState::Discarded,
            ClaudeCommandLifecycleState::Unknown,
        ] {
            let mut lifecycle = active_lifecycle();
            register_injected(&mut lifecycle, "steer");
            let _ = lifecycle.prompt_response(response());

            assert!(matches!(
                lifecycle.observe(command_lifecycle("steer", state)),
                ClaudeCompletionAction::Fail(_)
            ));
            assert!(!lifecycle.native_steering_available());
            assert!(matches!(
                lifecycle.observe(command_lifecycle("steer", state)),
                ClaudeCompletionAction::None
            ));
        }
    }

    #[test]
    fn unrelated_lifecycle_event_does_not_affect_prompt_completion() {
        let mut lifecycle = active_lifecycle();
        let _ = lifecycle.observe(command_lifecycle(
            "unrelated",
            ClaudeCommandLifecycleState::Completed,
        ));
        assert!(matches!(
            lifecycle.prompt_response(response()),
            ClaudeCompletionAction::Complete(_)
        ));
    }
}
