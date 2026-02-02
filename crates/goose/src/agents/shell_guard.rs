use crate::approval::{
    detect_environment, ApprovalDecision, ApprovalManager, ApprovalPreset, ExecutionContext,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct ShellGuard {
    approval_manager: Arc<RwLock<ApprovalManager>>,
    context: ExecutionContext,
}

impl Clone for ShellGuard {
    fn clone(&self) -> Self {
        Self {
            approval_manager: Arc::clone(&self.approval_manager),
            context: self.context.clone(),
        }
    }
}

impl ShellGuard {
    pub fn new(preset: ApprovalPreset) -> Self {
        let context = ExecutionContext::new().with_environment(detect_environment());
        Self {
            approval_manager: Arc::new(RwLock::new(ApprovalManager::from_preset(preset))),
            context,
        }
    }

    pub fn with_environment(mut self, env: crate::approval::Environment) -> Self {
        self.context = self.context.with_environment(env);
        self
    }

    pub fn with_context(mut self, context: ExecutionContext) -> Self {
        self.context = context;
        self
    }

    pub async fn set_preset(&self, preset: ApprovalPreset) {
        let mut manager = self.approval_manager.write().await;
        *manager = ApprovalManager::from_preset(preset);
    }

    pub async fn check_command(&self, cmd: &str) -> Result<CommandCheck> {
        let manager = self.approval_manager.read().await;
        let decision = manager.evaluate(cmd, &self.context);

        match decision {
            ApprovalDecision::Approved => {
                info!("Shell command approved: {}", truncate_cmd(cmd));
                Ok(CommandCheck::Approved)
            }
            ApprovalDecision::RequiresApproval(request) => {
                warn!(
                    "Shell command requires approval: {} (risk: {}, patterns: {:?})",
                    truncate_cmd(cmd),
                    request.risk_level,
                    request.matched_patterns
                );
                Ok(CommandCheck::NeedsApproval {
                    command: request.command,
                    reason: request.reason,
                    risk_level: request.risk_level.to_string(),
                    patterns: request.matched_patterns,
                })
            }
            ApprovalDecision::Blocked(reason) => {
                warn!("Shell command blocked: {} - {}", truncate_cmd(cmd), reason);
                Ok(CommandCheck::Blocked { reason })
            }
        }
    }

    pub async fn policy_name(&self) -> String {
        let manager = self.approval_manager.read().await;
        manager.policy_name().to_string()
    }
}

impl Default for ShellGuard {
    fn default() -> Self {
        Self::new(ApprovalPreset::Safe)
    }
}

#[derive(Debug, Clone)]
pub enum CommandCheck {
    Approved,
    NeedsApproval {
        command: String,
        reason: String,
        risk_level: String,
        patterns: Vec<String>,
    },
    Blocked {
        reason: String,
    },
}

impl CommandCheck {
    pub fn is_approved(&self) -> bool {
        matches!(self, CommandCheck::Approved)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, CommandCheck::Blocked { .. })
    }

    pub fn needs_approval(&self) -> bool {
        matches!(self, CommandCheck::NeedsApproval { .. })
    }
}

fn truncate_cmd(cmd: &str) -> String {
    if cmd.len() > 100 {
        // Safely truncate using chars to handle UTF-8
        let truncated: String = cmd.chars().take(97).collect();
        format!("{}...", truncated)
    } else {
        cmd.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::Environment;

    #[tokio::test]
    async fn test_shell_guard_safe_mode() {
        let guard = ShellGuard::new(ApprovalPreset::Safe);

        let check = guard.check_command("ls -la").await.unwrap();
        assert!(check.is_approved());

        let check = guard.check_command("rm -rf /").await.unwrap();
        assert!(check.is_blocked());

        let check = guard.check_command("chmod 777 /tmp/file").await.unwrap();
        assert!(check.needs_approval());
    }

    #[tokio::test]
    async fn test_shell_guard_paranoid_mode() {
        let guard = ShellGuard::new(ApprovalPreset::Paranoid);

        let check = guard.check_command("ls -la").await.unwrap();
        assert!(check.needs_approval());

        let check = guard.check_command("rm -rf /").await.unwrap();
        assert!(check.is_blocked());
    }

    #[tokio::test]
    async fn test_shell_guard_autopilot_in_sandbox() {
        let context = ExecutionContext::new().with_environment(Environment::DockerSandbox);
        let guard = ShellGuard::new(ApprovalPreset::Autopilot).with_context(context);

        let check = guard.check_command("rm -rf /").await.unwrap();
        assert!(check.is_approved());
    }

    #[tokio::test]
    async fn test_shell_guard_autopilot_on_real() {
        let context = ExecutionContext::new().with_environment(Environment::RealFilesystem);
        let guard = ShellGuard::new(ApprovalPreset::Autopilot).with_context(context);

        let check = guard.check_command("ls -la").await.unwrap();
        assert!(check.is_approved());

        let check = guard.check_command("rm -rf /").await.unwrap();
        assert!(check.is_blocked());
    }

    #[tokio::test]
    async fn test_shell_guard_preset_change() {
        let guard = ShellGuard::new(ApprovalPreset::Safe);
        assert_eq!(guard.policy_name().await, "safe");

        guard.set_preset(ApprovalPreset::Paranoid).await;
        assert_eq!(guard.policy_name().await, "paranoid");
    }
}
