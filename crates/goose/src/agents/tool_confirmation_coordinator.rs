use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{anyhow, Result};
use tokio::sync::{Mutex, Notify, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;

use crate::agents::state_machine::ToolConfirmationDecision;
use crate::permission::Permission;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ConfirmationAnswer {
    LiveHandled,
    StateMachine(Permission),
}

#[derive(Default)]
struct PendingConfirmationBatch {
    request_ids: HashSet<String>,
    answers: HashMap<String, ConfirmationAnswer>,
}

#[derive(Default)]
struct TurnState {
    active_request_ids: HashSet<String>,
    pending_batch: Option<PendingConfirmationBatch>,
}

pub(super) struct SessionToolConfirmationState {
    execution: Arc<Mutex<()>>,
    pub(super) submission: Mutex<()>,
    turn: StdMutex<TurnState>,
    notification: Notify,
}

impl SessionToolConfirmationState {
    fn new() -> Self {
        Self {
            execution: Arc::new(Mutex::new(())),
            submission: Mutex::new(()),
            turn: StdMutex::new(TurnState::default()),
            notification: Notify::new(),
        }
    }

    pub(super) fn try_start_turn(self: &Arc<Self>) -> Result<ActiveTurnGuard> {
        let execution_guard = self
            .execution
            .clone()
            .try_lock_owned()
            .map_err(|_| anyhow!("session already has an active turn"))?;
        Ok(ActiveTurnGuard {
            state: self.clone(),
            _execution_guard: execution_guard,
        })
    }

    pub(super) fn register_request(&self, request_id: String) {
        let mut turn = self.turn.lock().expect("tool confirmation state poisoned");
        turn.active_request_ids.insert(request_id.clone());
        turn.pending_batch
            .get_or_insert_with(PendingConfirmationBatch::default)
            .request_ids
            .insert(request_id);
    }

    pub(super) fn answer(&self, request_id: &str) -> Option<ConfirmationAnswer> {
        self.turn
            .lock()
            .expect("tool confirmation state poisoned")
            .pending_batch
            .as_ref()
            .and_then(|batch| batch.answers.get(request_id).cloned())
    }

    pub(super) fn contains_request(&self, request_id: &str) -> bool {
        self.turn
            .lock()
            .expect("tool confirmation state poisoned")
            .active_request_ids
            .contains(request_id)
    }

    pub(super) fn record_answer(&self, request_id: &str, answer: ConfirmationAnswer) -> Result<()> {
        let mut turn = self.turn.lock().expect("tool confirmation state poisoned");
        let batch = turn
            .pending_batch
            .as_mut()
            .ok_or_else(|| anyhow!("tool confirmation request is no longer active"))?;
        if !batch.request_ids.contains(request_id) {
            return Err(anyhow!("tool confirmation request is no longer active"));
        }
        match batch.answers.entry(request_id.to_string()) {
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err(anyhow!("tool confirmation request was already answered"));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(answer);
            }
        }
        drop(turn);
        self.notification.notify_waiters();
        Ok(())
    }

    pub(super) async fn wait_for_batch(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<ToolConfirmationDecision>> {
        loop {
            let notified = self.notification.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let completed = {
                let turn = self.turn.lock().expect("tool confirmation state poisoned");
                turn.pending_batch.as_ref().and_then(|batch| {
                    (batch.request_ids.len() == batch.answers.len()).then(|| {
                        let mut decisions: Vec<_> = batch
                            .answers
                            .iter()
                            .filter_map(|(request_id, answer)| match answer {
                                ConfirmationAnswer::LiveHandled => None,
                                ConfirmationAnswer::StateMachine(permission) => {
                                    Some(ToolConfirmationDecision {
                                        request_id: request_id.clone(),
                                        permission: permission.clone(),
                                    })
                                }
                            })
                            .collect();
                        decisions.sort_by(|left, right| left.request_id.cmp(&right.request_id));
                        decisions
                    })
                })
            };
            if let Some(decisions) = completed {
                return Ok(decisions);
            }

            tokio::select! {
                _ = notified => {}
                _ = cancel.cancelled() => return Err(anyhow!("state-machine turn cancelled")),
            }
        }
    }

    pub(super) fn clear_batch(&self) {
        let mut turn = self.turn.lock().expect("tool confirmation state poisoned");
        turn.active_request_ids.clear();
        turn.pending_batch = None;
    }
}

pub(super) struct ActiveTurnGuard {
    state: Arc<SessionToolConfirmationState>,
    _execution_guard: OwnedMutexGuard<()>,
}

impl ActiveTurnGuard {
    pub(super) fn state(&self) -> &Arc<SessionToolConfirmationState> {
        &self.state
    }
}

impl Drop for ActiveTurnGuard {
    fn drop(&mut self) {
        self.state.clear_batch();
    }
}

pub(super) struct ToolConfirmationCoordinator {
    sessions: StdMutex<HashMap<String, Arc<SessionToolConfirmationState>>>,
}

impl ToolConfirmationCoordinator {
    pub(super) fn new() -> Self {
        Self {
            sessions: StdMutex::new(HashMap::new()),
        }
    }

    pub(super) fn session(&self, session_id: &str) -> Arc<SessionToolConfirmationState> {
        self.sessions
            .lock()
            .expect("tool confirmation coordinator poisoned")
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(SessionToolConfirmationState::new()))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_second_active_turn_and_releases_on_drop() {
        let coordinator = ToolConfirmationCoordinator::new();
        let session = coordinator.session("session");
        let guard = session.try_start_turn().unwrap();

        assert!(session.try_start_turn().is_err());

        drop(guard);
        assert!(session.try_start_turn().is_ok());
    }

    #[tokio::test]
    async fn waits_for_every_confirmation_in_the_batch() {
        let coordinator = ToolConfirmationCoordinator::new();
        let session = coordinator.session("session");
        let _guard = session.try_start_turn().unwrap();
        session.register_request("request-1".to_string());
        session.register_request("request-2".to_string());
        session
            .record_answer(
                "request-1",
                ConfirmationAnswer::StateMachine(Permission::AllowOnce),
            )
            .unwrap();
        session
            .record_answer("request-2", ConfirmationAnswer::LiveHandled)
            .unwrap();

        let decisions = session
            .wait_for_batch(&CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(
            decisions,
            vec![ToolConfirmationDecision {
                request_id: "request-1".to_string(),
                permission: Permission::AllowOnce,
            }]
        );
    }

    #[test]
    fn active_turn_drop_clears_pending_requests() {
        let coordinator = ToolConfirmationCoordinator::new();
        let session = coordinator.session("session");
        let guard = session.try_start_turn().unwrap();
        session.register_request("request".to_string());
        assert!(session.contains_request("request"));

        drop(guard);

        assert!(!session.contains_request("request"));
        assert!(session.answer("request").is_none());
    }

    #[test]
    fn first_answer_is_immutable() {
        let coordinator = ToolConfirmationCoordinator::new();
        let session = coordinator.session("session");
        let _guard = session.try_start_turn().unwrap();
        session.register_request("request".to_string());
        session
            .record_answer(
                "request",
                ConfirmationAnswer::StateMachine(Permission::AllowOnce),
            )
            .unwrap();

        assert!(session
            .record_answer(
                "request",
                ConfirmationAnswer::StateMachine(Permission::DenyOnce),
            )
            .is_err());
        assert_eq!(
            session.answer("request"),
            Some(ConfirmationAnswer::StateMachine(Permission::AllowOnce))
        );
    }
}
