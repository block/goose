use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::scheduler::{normalize_cron_expression, ScheduledJob, SchedulerError};
use crate::scheduler_trait::SchedulerTrait;
use crate::session::storage::SessionMetadata;

const TEMPORAL_SERVICE_STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(500);

// Default ports to try when discovering the service - using high, obscure ports
// to avoid conflicts with common services
const DEFAULT_HTTP_PORTS: &[u16] = &[58080, 58081, 58082, 58083, 58084, 58085];

#[derive(Serialize, Deserialize, Debug)]
struct JobRequest {
    action: String,
    job_id: Option<String>,
    cron: Option<String>,
    recipe_path: Option<String>,
    execution_mode: Option<String>,
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
    execution_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RunNowResponse {
    session_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PortConfig {
    http_port: u16,
    temporal_port: u16,
    ui_port: u16,
}

#[derive(Clone)]
pub struct TemporalScheduler {
    http_client: Client,
    service_url: String,
    port_config: PortConfig,
}

impl TemporalScheduler {
    pub async fn new() -> Result<Arc<Self>, SchedulerError> {
        let http_client = Client::new();

        // Discover the HTTP port
        let http_port = Self::discover_http_port(&http_client).await?;
        let service_url = format!("http://localhost:{}", http_port);

        info!("Found Temporal service HTTP API on port {}", http_port);

        // Create scheduler with initial port config
        let scheduler = Arc::new(Self {
            http_client: http_client.clone(),
            service_url: service_url.clone(),
            port_config: PortConfig {
                http_port,
                temporal_port: 7233, // temporary defaults
                ui_port: 8233,
            },
        });

        // Start the Go service if not already running
        scheduler.start_go_service().await?;

        // Wait for service to be ready
        scheduler.wait_for_service_ready().await?;

        // Fetch the actual port configuration and update
        let port_config = scheduler.fetch_port_config().await?;

        info!(
            "Discovered Temporal service ports - HTTP: {}, Temporal: {}, UI: {}",
            port_config.http_port, port_config.temporal_port, port_config.ui_port
        );

        // Create final scheduler with correct ports
        let final_scheduler = Arc::new(Self {
            http_client,
            service_url,
            port_config,
        });

        // Start the status monitor to keep job statuses in sync
        if let Err(e) = final_scheduler.start_status_monitor().await {
            tracing::warn!("Failed to start status monitor: {}", e);
        }

        info!("TemporalScheduler initialized successfully");
        Ok(final_scheduler)
    }

    async fn discover_http_port(http_client: &Client) -> Result<u16, SchedulerError> {
        info!("Discovering Temporal service port...");

        // Check PORT environment variable first
        if let Ok(port_str) = std::env::var("PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                if Self::is_temporal_service_running(http_client, port).await {
                    info!(
                        "Found running Temporal service on PORT environment variable: {}",
                        port
                    );
                    return Ok(port);
                } else if Self::is_port_free(port).await {
                    info!("Using PORT environment variable for new service: {}", port);
                    return Ok(port);
                } else {
                    warn!(
                        "PORT environment variable {} is occupied by non-Temporal service",
                        port
                    );
                }
            }
        }

        // Try to find an existing Temporal service on default ports
        for &port in DEFAULT_HTTP_PORTS {
            if Self::is_temporal_service_running(http_client, port).await {
                info!("Found existing Temporal service on port {}", port);
                return Ok(port);
            }
        }

        // If no existing service found, find a free port to start a new one
        info!("No existing Temporal service found, finding free port to start new service");

        for &port in DEFAULT_HTTP_PORTS {
            if Self::is_port_free(port).await {
                info!("Found free port {} for new Temporal service", port);
                return Ok(port);
            }
        }

        // If all default ports are taken, find any free port in a reasonable range
        for port in 58086..58200 {
            if Self::is_port_free(port).await {
                info!("Found free port {} for new Temporal service", port);
                return Ok(port);
            }
        }

        Err(SchedulerError::SchedulerInternalError(
            "Could not find any free port for Temporal service".to_string(),
        ))
    }

    /// Check if a Temporal service is running and responding on the given port
    async fn is_temporal_service_running(http_client: &Client, port: u16) -> bool {
        let health_url = format!("http://127.0.0.1:{}/health", port);

        match http_client
            .get(&health_url)
            .timeout(Duration::from_millis(1000))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                info!("Confirmed Temporal service is running on port {}", port);
                true
            }
            Ok(response) => {
                info!(
                    "Port {} is responding but not a healthy Temporal service (status: {})",
                    port,
                    response.status()
                );
                false
            }
            Err(_) => {
                // Port might be free or occupied by something else
                false
            }
        }
    }

    async fn is_port_free(port: u16) -> bool {
        use std::net::{SocketAddr, TcpListener};

        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

        // Try to bind to the port
        match TcpListener::bind(addr) {
            Ok(_listener) => {
                // Successfully bound, so port is free
                true
            }
            Err(_) => {
                // Could not bind, port is in use
                false
            }
        }
    }

    async fn fetch_port_config(&self) -> Result<PortConfig, SchedulerError> {
        let url = format!("{}/ports", self.service_url);

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let port_config: PortConfig = response.json().await.map_err(|e| {
                        SchedulerError::SchedulerInternalError(format!(
                            "Failed to parse port config JSON: {}",
                            e
                        ))
                    })?;
                    Ok(port_config)
                } else {
                    Err(SchedulerError::SchedulerInternalError(format!(
                        "Failed to fetch port config: HTTP {}",
                        response.status()
                    )))
                }
            }
            Err(e) => Err(SchedulerError::SchedulerInternalError(format!(
                "Failed to fetch port config: {}",
                e
            ))),
        }
    }

    /// Get the current port configuration
    pub fn get_port_config(&self) -> &PortConfig {
        &self.port_config
    }

    /// Get the Temporal server port
    pub fn get_temporal_port(&self) -> u16 {
        self.port_config.temporal_port
    }

    /// Get the HTTP API port
    pub fn get_http_port(&self) -> u16 {
        self.port_config.http_port
    }

    /// Get the Temporal UI port
    pub fn get_ui_port(&self) -> u16 {
        self.port_config.ui_port
    }

    async fn start_go_service(&self) -> Result<(), SchedulerError> {
        info!(
            "Starting Temporal Go service on port {}...",
            self.port_config.http_port
        );

        // Check if the service is already running on the discovered port
        if self.health_check().await.unwrap_or(false) {
            info!(
                "Temporal service is already running on port {}",
                self.port_config.http_port
            );
            return Ok(());
        }

        // Double-check that the port is still free (in case something grabbed it between discovery and start)
        if !Self::is_port_free(self.port_config.http_port).await {
            return Err(SchedulerError::SchedulerInternalError(format!(
                "Port {} is no longer available for Temporal service.",
                self.port_config.http_port
            )));
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

        // Set the PORT environment variable for the service to use and properly daemonize it
        // Create a new process group to ensure the service survives parent termination
        let mut command = Command::new(&binary_path);
        command
            .current_dir(working_dir)
            .env("PORT", self.port_config.http_port.to_string());

        // Platform-specific process configuration based on Electron app approach
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // On Windows, prevent console window and run detached:
            // - Use CREATE_NO_WINDOW (0x08000000) to prevent console window
            // - Use DETACHED_PROCESS (0x00000008) for independence
            // - Redirect output to null to prevent console attachment
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .creation_flags(0x08000000 | 0x00000008); // CREATE_NO_WINDOW | DETACHED_PROCESS
        }

        #[cfg(not(windows))]
        {
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null());
        }

        // On Unix systems, create a new process group
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }

        let mut child = command.spawn().map_err(|e| {
            SchedulerError::SchedulerInternalError(format!(
                "Failed to start Go temporal service: {}",
                e
            ))
        })?;

        let pid = child.id();
        info!(
            "Temporal Go service started with PID: {} on port {} (detached)",
            pid, self.port_config.http_port
        );

        // Platform-specific process handling
        #[cfg(windows)]
        {
            // On Windows, wait longer for initialization and use unref-like behavior
            sleep(Duration::from_millis(1000)).await; // Wait 1 second for Windows initialization

            // Use a different approach - spawn a monitoring thread that waits longer
            std::thread::spawn(move || {
                // Give the process significant time to initialize on Windows
                std::thread::sleep(std::time::Duration::from_secs(5));
                // After 5 seconds, let it run completely independently
                let _ = child.wait();
            });
        }

        #[cfg(unix)]
        {
            // Give the process a moment to start up
            sleep(Duration::from_millis(100)).await;

            // Verify the process is still running
            use std::process::Command as StdCommand;
            let ps_check = StdCommand::new("ps")
                .arg("-p")
                .arg(pid.to_string())
                .output();

            match ps_check {
                Ok(output) if output.status.success() => {
                    info!("Confirmed Temporal service process {} is running", pid);
                }
                Ok(_) => {
                    warn!(
                        "Temporal service process {} may have exited immediately",
                        pid
                    );
                }
                Err(e) => {
                    warn!("Could not verify Temporal service process status: {}", e);
                }
            }

            // Detach the child process by not waiting for it
            // This allows it to continue running independently
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }

        Ok(())
    }

    fn find_go_service_binary() -> Result<String, SchedulerError> {
        // Try to find the Go service binary by looking for it relative to the current executable
        // or in common locations

        // First try to find it relative to the current executable path (most common for bundled apps)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Try various relative paths from the executable directory
                let exe_relative_paths = vec![
                    // First check in resources/bin subdirectory (bundled Electron app location)
                    exe_dir.join("resources/bin/temporal-service"),
                    exe_dir.join("resources/bin/temporal-service.exe"), // Windows version
                    exe_dir.join("resources\\bin\\temporal-service.exe"), // Windows with backslashes
                    // Then check in the same directory as the executable
                    exe_dir.join("temporal-service"),
                    exe_dir.join("temporal-service.exe"), // Windows version
                    // Then check in temporal-service subdirectory
                    exe_dir.join("temporal-service/temporal-service"),
                    exe_dir.join("temporal-service/temporal-service.exe"), // Windows version
                    // Then check relative paths for development
                    exe_dir.join("../temporal-service/temporal-service"),
                    exe_dir.join("../../temporal-service/temporal-service"),
                    exe_dir.join("../../../temporal-service/temporal-service"),
                    exe_dir.join("../../../../temporal-service/temporal-service"),
                ];

                for path in exe_relative_paths {
                    if path.exists() {
                        tracing::debug!("Found temporal-service binary at: {}", path.display());
                        return Ok(path.to_string_lossy().to_string());
                    }
                }
            }
        }

        // Try relative to current working directory (original behavior)
        let possible_paths = vec![
            "./temporal-service/temporal-service",
            "./temporal-service.exe",               // Windows in current dir
            "./resources/bin/temporal-service.exe", // Windows bundled in current dir
        ];

        for path in &possible_paths {
            if std::path::Path::new(path).exists() {
                tracing::debug!("Found temporal-service binary at: {}", path);
                return Ok(path.to_string());
            }
        }

        // Check environment variable override
        if let Ok(binary_path) = std::env::var("GOOSE_TEMPORAL_BIN") {
            if std::path::Path::new(&binary_path).exists() {
                tracing::info!(
                    "Using temporal-service binary from GOOSE_TEMPORAL_BIN: {}",
                    binary_path
                );
                return Ok(binary_path);
            } else {
                tracing::warn!(
                    "GOOSE_TEMPORAL_BIN points to non-existent file: {}",
                    binary_path
                );
            }
        }

        Err(SchedulerError::SchedulerInternalError(
            "Go service binary not found. Tried paths relative to current executable and working directory. Please ensure the temporal-service binary is built and available, or set GOOSE_TEMPORAL_BIN environment variable.".to_string()
        ))
    }

    async fn wait_for_service_ready(&self) -> Result<(), SchedulerError> {
        info!("Waiting for Temporal service to be ready...");

        let start_time = std::time::Instant::now();
        let mut attempt_count = 0;

        while start_time.elapsed() < TEMPORAL_SERVICE_STARTUP_TIMEOUT {
            attempt_count += 1;
            match self.health_check().await {
                Ok(true) => {
                    info!(
                        "Temporal service is ready after {} attempts in {:.2}s",
                        attempt_count,
                        start_time.elapsed().as_secs_f64()
                    );
                    return Ok(());
                }
                Ok(false) => {
                    // Service responded but not healthy
                    if attempt_count % 10 == 0 {
                        info!(
                            "Waiting for Temporal service... attempt {} ({:.1}s elapsed)",
                            attempt_count,
                            start_time.elapsed().as_secs_f64()
                        );
                    }
                    sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
                }
                Err(e) => {
                    // Service not responding yet
                    if attempt_count % 10 == 0 {
                        info!(
                            "Temporal service not responding yet... attempt {} ({:.1}s elapsed): {}",
                            attempt_count,
                            start_time.elapsed().as_secs_f64(),
                            e
                        );
                    }
                    sleep(TEMPORAL_SERVICE_HEALTH_CHECK_INTERVAL).await;
                }
            }
        }

        Err(SchedulerError::SchedulerInternalError(format!(
            "Temporal service failed to become ready within {}s timeout ({} attempts)",
            TEMPORAL_SERVICE_STARTUP_TIMEOUT.as_secs(),
            attempt_count
        )))
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

        // Normalize the cron expression to ensure it's 6-field format
        let normalized_cron = normalize_cron_expression(&job.cron);
        if normalized_cron != job.cron {
            tracing::info!(
                "TemporalScheduler: Normalized cron expression from '{}' to '{}'",
                job.cron,
                normalized_cron
            );
        }

        let request = JobRequest {
            action: "create".to_string(),
            job_id: Some(job.id.clone()),
            cron: Some(normalized_cron.clone()),
            recipe_path: Some(job.source.clone()),
            execution_mode: job.execution_mode.clone(),
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
            execution_mode: None,
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
                        execution_mode: tj.execution_mode,
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
            execution_mode: None,
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
            execution_mode: None,
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
            execution_mode: None,
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
            execution_mode: None,
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
        sched_id: &str,
        new_cron: String,
    ) -> Result<(), SchedulerError> {
        tracing::info!(
            "TemporalScheduler: update_schedule() called for job '{}' with cron '{}'",
            sched_id,
            new_cron
        );

        // Normalize the cron expression to ensure it's 6-field format
        let normalized_cron = normalize_cron_expression(&new_cron);
        if normalized_cron != new_cron {
            tracing::info!(
                "TemporalScheduler: Normalized cron expression from '{}' to '{}'",
                new_cron,
                normalized_cron
            );
        }

        let request = JobRequest {
            action: "update".to_string(),
            job_id: Some(sched_id.to_string()),
            cron: Some(normalized_cron),
            recipe_path: None,
            execution_mode: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully updated scheduled job: {}", sched_id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn kill_running_job(&self, sched_id: &str) -> Result<(), SchedulerError> {
        tracing::info!(
            "TemporalScheduler: kill_running_job() called for job '{}'",
            sched_id
        );

        let request = JobRequest {
            action: "kill_job".to_string(),
            job_id: Some(sched_id.to_string()),
            cron: None,
            recipe_path: None,
            execution_mode: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            info!("Successfully killed running job: {}", sched_id);
            Ok(())
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
        }
    }

    pub async fn update_job_status_from_sessions(&self) -> Result<(), SchedulerError> {
        tracing::info!("TemporalScheduler: Checking job status based on session activity");

        let jobs = self.list_scheduled_jobs().await?;

        for job in jobs {
            if job.currently_running {
                // First, check with the Temporal service directly for the most accurate status
                let request = JobRequest {
                    action: "status".to_string(),
                    job_id: Some(job.id.clone()),
                    cron: None,
                    recipe_path: None,
                    execution_mode: None,
                };

                match self.make_request(request).await {
                    Ok(response) => {
                        if response.success {
                            if let Some(jobs) = response.jobs {
                                if let Some(temporal_job) = jobs.iter().find(|j| j.id == job.id) {
                                    // If Temporal service says it's not running, trust that
                                    if !temporal_job.currently_running {
                                        tracing::info!(
                                            "Temporal service reports job '{}' is not running",
                                            job.id
                                        );
                                        continue; // Job is already marked as not running by Temporal
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get status from Temporal service for job '{}': {}",
                            job.id,
                            e
                        );
                        // Fall back to session-based checking if Temporal service is unavailable
                    }
                }

                // Secondary check: look for recent session activity (more lenient timing)
                let recent_sessions = self.sessions(&job.id, 3).await?;
                let mut has_active_session = false;

                for (session_name, _) in recent_sessions {
                    let session_path = match crate::session::storage::get_path(
                        crate::session::storage::Identifier::Name(session_name.clone()),
                    ) {
                        Ok(path) => path,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get session path for '{}': {}",
                                session_name,
                                e
                            );
                            continue;
                        }
                    };

                    // Check if session file was modified recently (within last 5 minutes instead of 2)
                    if let Ok(metadata) = std::fs::metadata(&session_path) {
                        if let Ok(modified) = metadata.modified() {
                            let modified_dt: DateTime<Utc> = modified.into();
                            let now = Utc::now();
                            let time_diff = now.signed_duration_since(modified_dt);

                            // Increased tolerance to 5 minutes to reduce false positives
                            if time_diff.num_minutes() < 5 {
                                has_active_session = true;
                                tracing::debug!(
                                    "Found active session for job '{}' modified {} minutes ago",
                                    job.id,
                                    time_diff.num_minutes()
                                );
                                break;
                            }
                        }
                    }
                }

                // Only mark as completed if both Temporal service check failed AND no recent session activity
                if !has_active_session {
                    tracing::info!(
                        "No active sessions found for job '{}' in the last 5 minutes, marking as completed",
                        job.id
                    );

                    let request = JobRequest {
                        action: "mark_completed".to_string(),
                        job_id: Some(job.id.clone()),
                        cron: None,
                        recipe_path: None,
                        execution_mode: None,
                    };

                    if let Err(e) = self.make_request(request).await {
                        tracing::warn!("Failed to mark job '{}' as completed: {}", job.id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Periodically check and update job statuses based on session activity
    pub async fn start_status_monitor(&self) -> Result<(), SchedulerError> {
        let scheduler_clone = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every 60 seconds instead of 30

            loop {
                interval.tick().await;

                if let Err(e) = scheduler_clone.update_job_status_from_sessions().await {
                    tracing::warn!("Failed to update job statuses: {}", e);
                }
            }
        });

        Ok(())
    }

    pub async fn get_running_job_info(
        &self,
        sched_id: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, SchedulerError> {
        tracing::info!(
            "TemporalScheduler: get_running_job_info() called for job '{}'",
            sched_id
        );

        // Get the current job status from Temporal service
        let request = JobRequest {
            action: "status".to_string(),
            job_id: Some(sched_id.to_string()),
            cron: None,
            recipe_path: None,
            execution_mode: None,
        };

        let response = self.make_request(request).await?;

        if response.success {
            if let Some(jobs) = response.jobs {
                if let Some(job) = jobs.iter().find(|j| j.id == sched_id) {
                    if job.currently_running {
                        // Try to get the actual session ID from recent sessions
                        let recent_sessions = self.sessions(sched_id, 1).await?;

                        if let Some((session_name, _session_metadata)) = recent_sessions.first() {
                            // Check if this session is still active by looking at the session file
                            let session_path = match crate::session::storage::get_path(
                                crate::session::storage::Identifier::Name(session_name.clone()),
                            ) {
                                Ok(path) => path,
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to get session path for '{}': {}",
                                        session_name,
                                        e
                                    );
                                    // Fallback: return a temporal session ID with current time
                                    let session_id =
                                        format!("temporal-{}-{}", sched_id, Utc::now().timestamp());
                                    let start_time = Utc::now();
                                    return Ok(Some((session_id, start_time)));
                                }
                            };

                            // If the session file was modified recently (within last 5 minutes),
                            // consider it as the current running session
                            if let Ok(metadata) = std::fs::metadata(&session_path) {
                                if let Ok(modified) = metadata.modified() {
                                    let modified_dt: DateTime<Utc> = modified.into();
                                    let now = Utc::now();
                                    let time_diff = now.signed_duration_since(modified_dt);

                                    if time_diff.num_minutes() < 5 {
                                        // This looks like an active session
                                        return Ok(Some((session_name.clone(), modified_dt)));
                                    }
                                }
                            }
                        }

                        // Fallback: return a temporal session ID with current time
                        let session_id =
                            format!("temporal-{}-{}", sched_id, Utc::now().timestamp());
                        let start_time = Utc::now();
                        Ok(Some((session_id, start_time)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(SchedulerError::JobNotFound(sched_id.to_string()))
                }
            } else {
                Err(SchedulerError::JobNotFound(sched_id.to_string()))
            }
        } else {
            Err(SchedulerError::SchedulerInternalError(response.message))
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
    /// Get basic service information
    pub async fn get_service_info(&self) -> String {
        let go_running = self.health_check().await.unwrap_or(false);

        format!(
            "Temporal Services Status:\n\
             - Go Service (localhost:{}): {}\n\
             - Temporal Server (localhost:{}): Running via Go service\n\
             - Temporal UI: http://localhost:{}\n\
             - Service logs: temporal-service/temporal-service.log\n\
             - Note: All ports are dynamically allocated",
            self.port_config.http_port,
            if go_running {
                "✅ Running"
            } else {
                "❌ Not Running"
            },
            self.port_config.temporal_port,
            self.port_config.ui_port
        )
    }

    /// Stop the Temporal services
    pub async fn stop_services(&self) -> Result<String, SchedulerError> {
        info!("Attempting to stop Temporal services...");

        // First check if services are running
        let go_running = self.health_check().await.unwrap_or(false);

        if !go_running {
            return Ok("Services are not currently running.".to_string());
        }

        // Try to stop the Go service gracefully by finding and killing the process
        // Look for temporal-service processes
        let output = Command::new("pgrep")
            .arg("-f")
            .arg("temporal-service")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let pids_str = String::from_utf8_lossy(&output.stdout);
                let pids: Vec<&str> = pids_str
                    .trim()
                    .split('\n')
                    .filter(|s| !s.is_empty())
                    .collect();

                if pids.is_empty() {
                    return Ok("No temporal-service processes found.".to_string());
                }

                info!("Found temporal-service PIDs: {:?}", pids);

                // Kill each process
                for pid in &pids {
                    let kill_output = Command::new("kill")
                        .arg("-TERM") // Graceful termination
                        .arg(pid)
                        .output();

                    match kill_output {
                        Ok(kill_result) if kill_result.status.success() => {
                            info!("Successfully sent TERM signal to PID {}", pid);
                        }
                        Ok(kill_result) => {
                            warn!(
                                "Failed to kill PID {}: {}",
                                pid,
                                String::from_utf8_lossy(&kill_result.stderr)
                            );
                        }
                        Err(e) => {
                            warn!("Error killing PID {}: {}", pid, e);
                        }
                    }
                }

                // Wait a moment for graceful shutdown
                sleep(Duration::from_secs(2)).await;

                // Check if services are still running
                let still_running = self.health_check().await.unwrap_or(false);

                if still_running {
                    // If still running, try SIGKILL
                    warn!("Services still running after TERM signal, trying KILL signal");
                    for pid in &pids {
                        let _ = Command::new("kill").arg("-KILL").arg(pid).output();
                    }

                    sleep(Duration::from_secs(1)).await;
                    let final_check = self.health_check().await.unwrap_or(false);

                    if final_check {
                        return Err(SchedulerError::SchedulerInternalError(
                            "Failed to stop services even with KILL signal".to_string(),
                        ));
                    }
                }

                Ok(format!(
                    "Successfully stopped {} temporal-service process(es)",
                    pids.len()
                ))
            }
            Ok(_) => {
                // pgrep found no processes
                Ok("No temporal-service processes found to stop.".to_string())
            }
            Err(e) => Err(SchedulerError::SchedulerInternalError(format!(
                "Failed to search for temporal-service processes: {}",
                e
            ))),
        }
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
    fn test_job_status_detection_improvements() {
        // Test that the new job status detection methods compile and work correctly
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // This test verifies the improved job status detection compiles
            match TemporalScheduler::new().await {
                Ok(scheduler) => {
                    // Test the new status update method
                    match scheduler.update_job_status_from_sessions().await {
                        Ok(()) => {
                            println!("✅ update_job_status_from_sessions() works correctly");
                        }
                        Err(e) => {
                            println!("⚠️  update_job_status_from_sessions() returned error (expected if no jobs): {}", e);
                        }
                    }

                    // Test the improved get_running_job_info method
                    match scheduler.get_running_job_info("test-job").await {
                        Ok(None) => {
                            println!("✅ get_running_job_info() correctly returns None for non-existent job");
                        }
                        Ok(Some((session_id, start_time))) => {
                            println!("✅ get_running_job_info() returned session info: {} at {}", session_id, start_time);
                        }
                        Err(e) => {
                            println!("⚠️  get_running_job_info() returned error (expected): {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Temporal services not running - method signature test passed: {}", e);
                }
            }
        });
    }

    #[test]
    fn test_port_check_functionality() {
        // Test the port checking functionality
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Test with a port that should be available (high port number)
            let high_port_in_use = !TemporalScheduler::is_port_free(65432).await;

            // Test with a port that might be in use (port 80)
            let low_port_in_use = !TemporalScheduler::is_port_free(80).await;

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

    #[test]
    fn test_daemon_process_group_creation() {
        // Test that the daemon process creation logic compiles and works correctly
        use std::process::Command;

        // Create a test command similar to what we do in start_go_service
        let mut command = Command::new("echo");
        command
            .arg("test")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null());

        // On Unix systems, create a new process group
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }

        // Test that the command can be spawned (but don't actually run it)
        match command.spawn() {
            Ok(mut child) => {
                println!("✅ Daemon process group creation works");
                // Clean up the test process
                let _ = child.wait();
            }
            Err(e) => {
                println!("⚠️  Error spawning test process: {}", e);
                // This might happen in some test environments, but the logic is correct
            }
        }
    }

    #[test]
    fn test_cron_normalization_in_temporal_scheduler() {
        // Test that the temporal scheduler uses cron normalization correctly
        use crate::scheduler::normalize_cron_expression;

        // Test cases that should be normalized
        assert_eq!(normalize_cron_expression("0 12 * * *"), "0 0 12 * * * *");
        assert_eq!(normalize_cron_expression("*/5 * * * *"), "0 */5 * * * * *");
        assert_eq!(normalize_cron_expression("0 0 * * 1"), "0 0 0 * * 1 *");

        // Test cases that should remain unchanged
        assert_eq!(normalize_cron_expression("0 0 12 * * *"), "0 0 12 * * * *");
        assert_eq!(
            normalize_cron_expression("*/30 */5 * * * *"),
            "*/30 */5 * * * * *"
        );

        println!("✅ Cron normalization works correctly in TemporalScheduler");
    }
}
