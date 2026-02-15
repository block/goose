use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Healthy,
    Degraded,
    Dead,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Dead => write!(f, "dead"),
        }
    }
}

pub struct AgentHealth {
    last_activity: Mutex<Instant>,
    consecutive_failures: AtomicU32,
    max_failures_before_degraded: u32,
    max_failures_before_dead: u32,
    stale_timeout: Duration,
}

impl AgentHealth {
    pub fn new() -> Self {
        Self {
            last_activity: Mutex::new(Instant::now()),
            consecutive_failures: AtomicU32::new(0),
            max_failures_before_degraded: 3,
            max_failures_before_dead: 10,
            stale_timeout: Duration::from_secs(300),
        }
    }

    pub fn with_thresholds(degraded_after: u32, dead_after: u32, stale_timeout: Duration) -> Self {
        Self {
            last_activity: Mutex::new(Instant::now()),
            consecutive_failures: AtomicU32::new(0),
            max_failures_before_degraded: degraded_after,
            max_failures_before_dead: dead_after,
            stale_timeout,
        }
    }

    pub async fn record_success(&self) {
        *self.last_activity.lock().await = Instant::now();
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    pub async fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn state(&self) -> AgentState {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        let last = *self.last_activity.lock().await;
        let stale = last.elapsed() > self.stale_timeout;

        if failures >= self.max_failures_before_dead || stale {
            AgentState::Dead
        } else if failures >= self.max_failures_before_degraded {
            AgentState::Degraded
        } else {
            AgentState::Healthy
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    pub async fn last_activity(&self) -> Instant {
        *self.last_activity.lock().await
    }

    pub async fn reset(&self) {
        *self.last_activity.lock().await = Instant::now();
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }
}

impl Default for AgentHealth {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub agent_id: String,
    pub state: AgentState,
    pub consecutive_failures: u32,
    pub last_activity_secs_ago: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_agent_is_healthy() {
        let health = AgentHealth::new();
        assert_eq!(health.state().await, AgentState::Healthy);
        assert_eq!(health.consecutive_failures(), 0);
    }

    #[tokio::test]
    async fn test_failures_degrade_health() {
        let health = AgentHealth::with_thresholds(2, 5, Duration::from_secs(300));
        health.record_failure().await;
        assert_eq!(health.state().await, AgentState::Healthy);

        health.record_failure().await;
        assert_eq!(health.state().await, AgentState::Degraded);

        health.record_failure().await;
        assert_eq!(health.state().await, AgentState::Degraded);
    }

    #[tokio::test]
    async fn test_many_failures_mark_dead() {
        let health = AgentHealth::with_thresholds(2, 5, Duration::from_secs(300));
        for _ in 0..5 {
            health.record_failure().await;
        }
        assert_eq!(health.state().await, AgentState::Dead);
    }

    #[tokio::test]
    async fn test_success_resets_failures() {
        let health = AgentHealth::with_thresholds(2, 5, Duration::from_secs(300));
        health.record_failure().await;
        health.record_failure().await;
        assert_eq!(health.state().await, AgentState::Degraded);

        health.record_success().await;
        assert_eq!(health.state().await, AgentState::Healthy);
        assert_eq!(health.consecutive_failures(), 0);
    }

    #[tokio::test]
    async fn test_stale_agent_is_dead() {
        let health = AgentHealth::with_thresholds(2, 5, Duration::from_millis(1));
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(health.state().await, AgentState::Dead);
    }

    #[tokio::test]
    async fn test_reset_restores_health() {
        let health = AgentHealth::with_thresholds(2, 5, Duration::from_secs(300));
        for _ in 0..5 {
            health.record_failure().await;
        }
        assert_eq!(health.state().await, AgentState::Dead);

        health.reset().await;
        assert_eq!(health.state().await, AgentState::Healthy);
    }
}
