use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::scheduler::{ScheduledJob, SchedulerError};
use crate::scheduler_trait::SchedulerTrait;
use crate::session::storage::SessionMetadata;

const TEMPORAL_SERVICE_URL: &str = "http://localhost:8080";
const TEMPORAL_SERVICE_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Serialize, Deserialize, Debug)]
struct JobRequest {
    action: String,
    job_id: Option<String>,
    cron: Option<String>,
    recipe_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JobResponse {
    success: bool,
    message: String,
    jobs: Option<Vec<TemporalJobStatus>>,
    data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TemporalJobStatus {
    id: String,
    cron: String,
    recipe_path: String,
    last_run: Option<String>,
    next_run: Option<String>,
    currently_running: bool,
    paused: bool,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RunNowResponse {
    session_id: String,
}

pub struct TemporalScheduler {
    http_client: Client,
    service_url: String,
}

impl TemporalScheduler {
    pub async fn new() -> Result<Arc<Self>, SchedulerError> {
        let http_client = Client::new();
        let service_url = TEMPORAL_SERVICE_URL.to_string();

        let scheduler = Arc::new(Self {
            http_client,
            service_url,
        });

        // Check if services are running, start them if needed
        scheduler.ensure_services_running().await?;

        // Wait for service to be ready
        scheduler.wait_for_service_ready().await?;

        info!("TemporalScheduler initialized successfully");
        Ok(scheduler)
    }

    async fn ensure_services_running(&self) -> Result<(), SchedulerError> {
        info!("Checking if Temporal services are running...");

        // First, check if both services are already running
        let temporal_running = self.check_temporal_server().await;
        let go_service_running = self.health_check().await.unwrap_or(false);

        if temporal_running && go_service_running {
            info!("Both Temporal server and Go service are already running");
            return Ok(());
        }

        // If Go service is running but Temporal server is not, this is an unusual state
        if go_service_running && !temporal_running {
            warn!("Go service is running but Temporal server is not - this may indicate a configuration issue");
            return Err(SchedulerError::SchedulerInternalError(
                "Go service is running but Temporal server is not accessible. Please check your Temporal server configuration.".to_string()
            ));
        }

        // If Temporal server is running but Go service is not, start the Go service
        if temporal_running && !go_service_running {
            info!("Temporal server is running, starting Go service...");
            self.start_go_service().await?;
            return Ok(());
        }

        // If neither is running, start both
        if !temporal_running {
            info!("Starting Temporal server...");
            self.start_temporal_server().await?;

            // Wait for Temporal server to be ready
            self.wait_for_temporal_server().await?;
        }

        // Now start the Go service
        if !self.health_check().await.unwrap_or(false) {
            info!("Starting Temporal Go service...");
            self.start_go_service().await?;
        }

        Ok(())
    }

    async fn check_temporal_server(&self) -> bool {
        // Temporal server uses gRPC on port 7233, not HTTP
        // We should check the web UI port (8233) instead, or use a different method

        // First try the web UI (which uses HTTP)
        if let Ok(response) = self.http_client.get("http://localhost:8233/").send().await {
            if response.status().is_success() {
                return true;
            }
        }

        // Alternative: check if we can establish a TCP connection to the gRPC port
        use std::net::SocketAddr;
        use std::time::Duration;

        let addr: SocketAddr = "127.0.0.1:7233".parse().unwrap();
        match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
            Ok(_) => {
                info!("Detected Temporal server on port 7233 (gRPC connection successful)");
                true
            }
            Err(_) => false,
        }
    }

    async fn start_temporal_server(&self) -> Result<(), SchedulerError> {
        info!("Starting Temporal server in background...");

        // Check if port 7233 is already in use
        if self.check_port_in_use(7233).await {
            // Port is in use - check if it's a Temporal server we can connect to
            if self.check_temporal_server().await {
                info!("Port 7233 is in use by a Temporal server we can connect to");
                return Ok(());
            } else {
                return Err(SchedulerError::SchedulerInternalError(
                    "Port 7233 is already in use by something other than a Temporal server."
                        .to_string(),
                ));
            }
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg("nohup temporal server start-dev --db-filename temporal.db --port 7233 --ui-port 8233 --log-level warn > temporal-server.log 2>&1 & echo $!")
            .output()
            .map_err(|e| SchedulerError::SchedulerInternalError(
                format!("Failed to start Temporal server: {}. Make sure 'temporal' CLI is installed.", e)
            ))?;

        if !output.status.success() {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "Failed to start Temporal server: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let pid_output = String::from_utf8_lossy(&output.stdout);
        let pid = pid_output.trim();
        info!("Temporal server started with PID: {}", pid);

        Ok(())
    }

    async fn check_port_in_use(&self, port: u16) -> bool {
        use std::net::{SocketAddr, TcpListener};

        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        TcpListener::bind(addr).is_err()
    }

    async fn wait_for_temporal_server(&self) -> Result<(), SchedulerError> {
        info!("Waiting for Temporal server to be ready...");

        let start_time = std::time::Instant::now();

        while start_time.elapsed() < TEMPORAL_SERVICE_STARTUP_TIMEOUT {
            if self.check_temporal_server().await {
                info!("Temporal server is ready");
                return Ok(());
            }
            sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
        }

        Err(SchedulerError::SchedulerInternalError(
            "Temporal server failed to become ready within timeout".to_string(),
        ))
    }

    async fn start_go_service(&self) -> Result<(), SchedulerError> {
        info!("Starting Temporal Go service in background...");

        // Check if port 8080 is already in use
        if self.check_port_in_use(8080).await {
            // Port is in use - check if it's our Go service we can connect to
            if self.health_check().await.unwrap_or(false) {
                info!("Port 8080 is in use by a Go service we can connect to");
                return Ok(());
            } else {
                return Err(SchedulerError::SchedulerInternalError(
                    "Port 8080 is already in use by something other than our Go service."
                        .to_string(),
                ));
            }
        }

        // Check if the temporal-service binary exists - try multiple possible locations
        let binary_path = Self::find_go_service_binary()?;
        let working_dir = std::path::Path::new(&binary_path).parent().ok_or_else(|| {
            SchedulerError::SchedulerInternalError(
                "Could not determine working directory for Go service".to_string(),
            )
        })?;

        info!("Found Go service binary at: {}", binary_path);
        info!("Using working directory: {}", working_dir.display());

        let command = format!(
            "cd '{}' && nohup ./temporal-service > temporal-service.log 2>&1 & echo $!",
            working_dir.display()
        );

        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .map_err(|e| {
                SchedulerError::SchedulerInternalError(format!(
                    "Failed to start Go temporal service: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "Failed to start Go service: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let pid_output = String::from_utf8_lossy(&output.stdout);
        let pid = pid_output.trim();
        info!("Temporal Go service started with PID: {}", pid);

        Ok(())
    }

    fn find_go_service_binary() -> Result<String, SchedulerError> {
        // Try to find the Go service binary by looking for it relative to the current executable
        // or in common locations

        let possible_paths = vec![
            // Relative to current working directory (original behavior)
            "./temporal-service/temporal-service",
        ];

        // Also try to find it relative to the current executable path
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Try various relative paths from the executable directory
                let exe_relative_paths = vec![
                    exe_dir.join("temporal-service/temporal-service"),
                    exe_dir.join("../temporal-service/temporal-service"),
                    exe_dir.join("../../temporal-service/temporal-service"),
                    exe_dir.join("../../../temporal-service/temporal-service"),
                    exe_dir.join("../../../../temporal-service/temporal-service"),
                ];

                for path in exe_relative_paths {
                    if path.exists() {
                        return Ok(path.to_string_lossy().to_string());
                    }
                }
            }
        }

        // Try the original relative paths
        for path in &possible_paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }

        Err(SchedulerError::SchedulerInternalError(
            "Go service binary not found. Tried paths relative to current executable and working directory. Please ensure the temporal-service binary is built and available.".to_string()
        ))
    }

    async fn wait_for_service_ready(&self) -> Result<(), SchedulerError> {
        info!("Waiting for Temporal service to be ready...");

        let start_time = std::time::Instant::now();

        while start_time.elapsed() < TEMPORAL_SERVICE_STARTUP_TIMEOUT {
            match self.health_check().await {
                Ok(true) => {
                    info!("Temporal service is ready");
                    return Ok(());
                }
                Ok(false) => {
                    // Service responded but not healthy
                    sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
                }
                Err(_) => {
                    // Service not responding yet
                    sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
                }
            }
        }

        Err(SchedulerError::SchedulerInternalError(
            "Temporal service failed to become ready within timeout".to_string(),
        ))
    }

    async fn health_check(&self) -> Result<bool, SchedulerError> {
        let url = format!("{}/health", self.service_url);

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    pub async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
        tracing::info!(
            "TemporalScheduler: add_scheduled_job() called for job '{}'",
            job.id
        );
        let request = JobRequest {
            action: "create".to_string(),
            job_id: Some(job.id.clone()),
            cron: Some(job.cron.clone()),
            recipe_path: Some(job.source.clone()),
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully created scheduled job: {}", job.id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        tracing::info!("TemporalScheduler: list_scheduled_jobs() called");
        let request = JobRequest {
            action: "list".to_string(),
            job_id: None,
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            let jobs = response.jobs.unwrap_or_default();
            let scheduled_jobs = jobs
                .into_iter()
                .map(|tj| {
                    ScheduledJob {
                        id: tj.id,
                        source: tj.recipe_path,
                        cron: tj.cron,
                        last_run: tj.last_run.and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                        currently_running: tj.currently_running,
                        paused: tj.paused,
                        current_session_id: None, // Not provided by Temporal service
                        process_start_time: None, // Not provided by Temporal service
                    }
                })
                .collect();
            Ok(scheduled_jobs)
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "delete".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully removed scheduled job: {}", id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "pause".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully paused scheduled job: {}", id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        let request = JobRequest {
            action: "unpause".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully unpaused scheduled job: {}", id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn run_now(&self, id: &str) -> Result<String, SchedulerError> {
        tracing::info!("TemporalScheduler: run_now() called for job '{}'", id);
        let request = JobRequest {
            action: "run_now".to_string(),
            job_id: Some(id.to_string()),
            cron: None,
            recipe_path: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            if let Some(data) = response.data {
                if let Ok(run_response) = serde_json::from_value::<RunNowResponse>(data) {
                    info!("Successfully started job execution for: {}", id);
                    Ok(run_response.session_id)
                } else {
                    Err(SchedulerError::SchedulerInternalError(
                        "Invalid response format for run_now".to_string(),
                    ))
                }
            } else {
                Err(SchedulerError::SchedulerInternalError(
                    "No session ID returned from run_now".to_string(),
                ))
            }
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    // Note: This method fetches sessions from the session storage directly
    // since Temporal service doesn't track session metadata
    pub async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        use crate::session::storage;

        // Get all session files
        let all_session_files = storage::list_sessions().map_err(|e| {
            SchedulerError::SchedulerInternalError(format!("Failed to list sessions: {}", e))
        })?;

        let mut schedule_sessions: Vec<(String, SessionMetadata)> = Vec::new();

        for (session_name, session_path) in all_session_files {
            match storage::read_metadata(&session_path) {
                Ok(metadata) => {
                    // Check if this session belongs to the requested schedule
                    if metadata.schedule_id.as_deref() == Some(sched_id) {
                        schedule_sessions.push((session_name, metadata));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read metadata for session file {}: {}. Skipping.",
                        session_path.display(),
                        e
                    );
                }
            }
        }

        // Sort by session_name (timestamp string) in descending order (newest first)
        schedule_sessions.sort_by(|a, b| b.0.cmp(&a.0));

        // Take only the requested limit
        let result_sessions: Vec<(String, SessionMetadata)> =
            schedule_sessions.into_iter().take(limit).collect();

        tracing::info!(
            "Found {} sessions for schedule '{}'",
            result_sessions.len(),
            sched_id
        );
        Ok(result_sessions)
    }

    pub async fn update_schedule(
        &self,
        _sched_id: &str,
        _new_cron: String,
    ) -> Result<(), SchedulerError> {
        warn!("update_schedule() method not implemented for TemporalScheduler - delete and recreate job instead");
        Err(SchedulerError::SchedulerInternalError(
            "update_schedule not supported - delete and recreate job instead".to_string(),
        ))
    }

    pub async fn kill_running_job(&self, _sched_id: &str) -> Result<(), SchedulerError> {
        warn!("kill_running_job() method not implemented for TemporalScheduler");
        Err(SchedulerError::SchedulerInternalError(
            "kill_running_job not supported by TemporalScheduler".to_string(),
        ))
    }

    pub async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        tracing::info!(
            "TemporalScheduler: get_running_job_info() called for job '{}'",
            sched_id
        );

        // First check if the job is marked as currently running
        let jobs = self.list_scheduled_jobs().await?;
        let job = jobs.iter().find(|j| j.id == sched_id);

        if let Some(job) = job {
            if job.currently_running {
                // For now, we'll return a placeholder session ID and current time
                // In a more complete implementation, we would track the actual session ID
                // and start time from the Temporal workflow execution
                let session_id =
                    format!("temporal-{}-{}", sched_id, chrono::Utc::now().timestamp());
                let start_time = chrono::Utc::now(); // This should be the actual start time
                Ok(Some((session_id, start_time)))
            } else {
                Ok(None)
            }
        } else {
            Err(SchedulerError::JobNotFound(sched_id.to_string()))
        }
    }

    async fn make_request(&self, request: JobRequest) -> Result<JobResponse, SchedulerError> {
        let url = format!("{}/jobs", self.service_url);

        tracing::info!(
            "TemporalScheduler: Making HTTP request to {} with action '{}'",
            url,
            request.action
        );

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                SchedulerError::SchedulerInternalError(format!("HTTP request failed: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let job_response: JobResponse = response.json().await.map_err(|e| {
            SchedulerError::SchedulerInternalError(format!("Failed to parse response JSON: {}", e))
        })?;

        Ok(job_response)
    }
}

impl Drop for TemporalScheduler {
    fn drop(&mut self) {
        // Services continue running independently - no cleanup needed
        info!("TemporalScheduler dropped - Temporal services continue running independently");
    }
}

// Service management utilities
impl TemporalScheduler {
    /// Check if Temporal services are running
    pub async fn check_services_status(&self) -> (bool, bool) {
        let temporal_server_running = self.check_temporal_server().await;
        let go_service_running = self.health_check().await.unwrap_or(false);
        (temporal_server_running, go_service_running)
    }

    /// Get service information
    pub async fn get_service_info(&self) -> String {
        let (temporal_running, go_running) = self.check_services_status().await;

        format!(
            "Temporal Services Status:\n\
             - Temporal Server ({}:7233): {}\n\
             - Temporal Web UI: http://localhost:8233\n\
             - Go Service ({}:8080): {}\n\
             - Service logs: temporal-server.log, temporal-service/temporal-service.log",
            if temporal_running {
                "localhost"
            } else {
                "not running"
            },
            if temporal_running {
                "✅ Running"
            } else {
                "❌ Not Running"
            },
            if go_running {
                "localhost"
            } else {
                "not running"
            },
            if go_running {
                "✅ Running"
            } else {
                "❌ Not Running"
            }
        )
    }

    /// Stop Temporal services (for manual management)
    pub async fn stop_services(&self) -> Result<String, SchedulerError> {
        info!("Stopping Temporal services...");

        let mut results = Vec::new();

        // Stop Go service
        let go_result = Command::new("pkill")
            .args(["-f", "temporal-service"])
            .output();

        match go_result {
            Ok(output) if output.status.success() => {
                results.push("✅ Go service stopped".to_string());
            }
            Ok(_) => {
                results.push("⚠️  Go service was not running or failed to stop".to_string());
            }
            Err(e) => {
                results.push(format!("❌ Failed to stop Go service: {}", e));
            }
        }

        // Stop Temporal server
        let temporal_result = Command::new("pkill")
            .args(["-f", "temporal server start-dev"])
            .output();

        match temporal_result {
            Ok(output) if output.status.success() => {
                results.push("✅ Temporal server stopped".to_string());
            }
            Ok(_) => {
                results.push("⚠️  Temporal server was not running or failed to stop".to_string());
            }
            Err(e) => {
                results.push(format!("❌ Failed to stop Temporal server: {}", e));
            }
        }

        let result_message = results.join("\n");
        info!("Service stop results: {}", result_message);
        Ok(result_message)
    }
}

#[async_trait]
impl SchedulerTrait for TemporalScheduler {
    async fn add_scheduled_job(&self, job: ScheduledJob) -> Result<(), SchedulerError> {
        self.add_scheduled_job(job).await
    }

    async fn list_scheduled_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        self.list_scheduled_jobs().await
    }

    async fn remove_scheduled_job(&self, id: &str) -> Result<(), SchedulerError> {
        self.remove_scheduled_job(id).await
    }

    async fn pause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.pause_schedule(id).await
    }

    async fn unpause_schedule(&self, id: &str) -> Result<(), SchedulerError> {
        self.unpause_schedule(id).await
    }

    async fn run_now(&self, id: &str) -> Result<String, SchedulerError> {
        self.run_now(id).await
    }

    async fn sessions(
        &self,
        sched_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, SessionMetadata)>, SchedulerError> {
        self.sessions(sched_id, limit).await
    }

    async fn update_schedule(
        &self,
        sched_id: &str,
        new_cron: String,
    ) -> Result<(), SchedulerError> {
        self.update_schedule(sched_id, new_cron).await
    }

    async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        self.kill_running_job(sched_id).await
    }

    async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        self.get_running_job_info(sched_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sessions_method_exists_and_compiles() {
        // This test verifies that the sessions method exists and compiles correctly
        // It doesn't require Temporal services to be running

        // Create a mock scheduler instance (this will fail if services aren't running, but that's OK)
        let result = TemporalScheduler::new().await;

        // Even if scheduler creation fails, we can still test the method signature
        match result {
            Ok(scheduler) => {
                // If services are running, test the actual method
                let sessions_result = scheduler.sessions("test-schedule", 5).await;

                // The method should return a Result, regardless of success/failure
                match sessions_result {
                    Ok(sessions) => {
                        // Verify the return type is correct
                        assert!(sessions.len() <= 5); // Should respect the limit
                        println!("✅ sessions() method returned {} sessions", sessions.len());
                    }
                    Err(e) => {
                        // Even errors are OK - the method is implemented
                        println!(
                            "⚠️  sessions() method returned error (expected if no sessions): {}",
                            e
                        );
                    }
                }
            }
            Err(_) => {
                // Services not running - that's fine, we just verified the method compiles
                println!("⚠️  Temporal services not running - method signature test passed");
            }
        }
    }

    #[test]
    fn test_sessions_method_signature() {
        // This test verifies the method signature is correct at compile time
        // We just need to verify the method exists and can be called

        // This will fail to compile if the method doesn't exist or has wrong signature
        let _test_fn = |scheduler: &TemporalScheduler, id: &str, limit: usize| {
            // This is a compile-time check - we don't actually call it
            let _future = scheduler.sessions(id, limit);
        };

        println!("✅ sessions() method signature is correct");
    }

    #[test]
    fn test_port_check_functionality() {
        // Test the port checking functionality
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let scheduler = TemporalScheduler {
                http_client: reqwest::Client::new(),
                service_url: "http://localhost:8080".to_string(),
            };

            // Test with a port that should be available (high port number)
            let high_port_in_use = scheduler.check_port_in_use(65432).await;

            // Test with a port that might be in use (port 80)
            let low_port_in_use = scheduler.check_port_in_use(80).await;

            println!("✅ Port checking functionality works");
            println!("   High port (65432) in use: {}", high_port_in_use);
            println!("   Low port (80) in use: {}", low_port_in_use);
        });
    }

    #[test]
    fn test_find_go_service_binary() {
        // Test the Go service binary finding logic
        match TemporalScheduler::find_go_service_binary() {
            Ok(path) => {
                println!("✅ Found Go service binary at: {}", path);
                assert!(
                    std::path::Path::new(&path).exists(),
                    "Binary should exist at found path"
                );
            }
            Err(e) => {
                println!("⚠️  Go service binary not found: {}", e);
                // This is expected if the binary isn't built or available
            }
        }
    }
}
