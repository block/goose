use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// Status of an inference server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceServerStatus {
    pub job_id: String,
    pub port: u16,
    pub base_model: String,
    pub adapter_path: String,
    pub status: ServerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Starting,
    Running,
    Stopped,
    Error,
}

/// Manages inference server processes
pub struct InferenceManager {
    servers: Arc<Mutex<HashMap<String, InferenceServer>>>,
}

struct InferenceServer {
    job_id: String,
    port: u16,
    base_model: String,
    adapter_path: PathBuf,
    process: Option<Child>,
    status: ServerStatus,
}

impl InferenceManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Find an available port
    fn find_available_port() -> Result<u16> {
        // Try ports in range 8000-9000
        for port in 8000..9000 {
            if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
                drop(listener);
                return Ok(port);
            }
        }
        anyhow::bail!("No available ports found in range 8000-9000")
    }

    /// Start an inference server for a fine-tuned model
    pub fn start_server(
        &self,
        job_id: String,
        base_model: String,
        adapter_path: PathBuf,
    ) -> Result<InferenceServerStatus> {
        let mut servers = self.servers.lock().unwrap();

        // Check if server already exists
        if let Some(server) = servers.get(&job_id) {
            if server.status == ServerStatus::Running {
                info!("Inference server already running for job {}", job_id);
                return Ok(InferenceServerStatus {
                    job_id: job_id.clone(),
                    port: server.port,
                    base_model: server.base_model.clone(),
                    adapter_path: server.adapter_path.display().to_string(),
                    status: ServerStatus::Running,
                });
            }
        }

        // Find available port
        let port = Self::find_available_port()
            .context("Failed to find available port for inference server")?;

        info!(
            "Starting inference server for job {} on port {}",
            job_id, port
        );

        // Get Python executable from venv
        let venv_path = crate::config::Config::global()
            .get_param::<PathBuf>("GOOSE_TRAINING_VENV")
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
                    .join("goose")
                    .join("axolotl-venv")
            });

        let python_executable = if cfg!(windows) {
            venv_path.join("Scripts").join("python.exe")
        } else {
            venv_path.join("bin").join("python")
        };

        // Verify adapter path exists
        if !adapter_path.exists() {
            anyhow::bail!("Adapter path does not exist: {}", adapter_path.display());
        }

        // Write the serve_lora.py script to a temporary location
        let script_content = include_str!("serve_lora.py");
        let script_path = std::env::temp_dir().join("serve_lora.py");
        std::fs::write(&script_path, script_content)
            .context("Failed to write serve_lora.py script")?;

        // Start the inference server process
        let mut cmd = Command::new(&python_executable);
        cmd.arg(&script_path)
            .arg("--base-model")
            .arg(&base_model)
            .arg("--adapter")
            .arg(&adapter_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--host")
            .arg("127.0.0.1");

        // Set environment variables
        cmd.env("PYTHONUNBUFFERED", "1");

        info!("Starting inference server with command: {:?}", cmd);

        let child = cmd
            .spawn()
            .context("Failed to spawn inference server process")?;

        let server = InferenceServer {
            job_id: job_id.clone(),
            port,
            base_model: base_model.clone(),
            adapter_path: adapter_path.clone(),
            process: Some(child),
            status: ServerStatus::Starting,
        };

        let status = InferenceServerStatus {
            job_id: job_id.clone(),
            port,
            base_model: base_model.clone(),
            adapter_path: adapter_path.display().to_string(),
            status: ServerStatus::Starting,
        };

        servers.insert(job_id.clone(), server);

        // TODO: Wait for server to be ready by checking health endpoint
        // For now, just wait a few seconds
        std::thread::sleep(std::time::Duration::from_secs(3));

        // Update status to running
        if let Some(server) = servers.get_mut(&job_id) {
            server.status = ServerStatus::Running;
        }

        info!("Inference server started successfully on port {}", port);

        Ok(InferenceServerStatus {
            status: ServerStatus::Running,
            ..status
        })
    }

    /// Stop an inference server
    pub fn stop_server(&self, job_id: &str) -> Result<()> {
        let mut servers = self.servers.lock().unwrap();

        if let Some(mut server) = servers.remove(job_id) {
            info!("Stopping inference server for job {}", job_id);

            if let Some(mut process) = server.process.take() {
                // Try graceful shutdown first
                if let Err(e) = process.kill() {
                    warn!("Failed to kill inference server process: {}", e);
                }

                // Wait for process to exit
                match process.wait() {
                    Ok(status) => {
                        info!("Inference server exited with status: {}", status);
                    }
                    Err(e) => {
                        warn!("Failed to wait for inference server process: {}", e);
                    }
                }
            }

            server.status = ServerStatus::Stopped;
            Ok(())
        } else {
            anyhow::bail!("No inference server found for job {}", job_id)
        }
    }

    /// Get status of an inference server
    pub fn get_status(&self, job_id: &str) -> Option<InferenceServerStatus> {
        let servers = self.servers.lock().unwrap();

        servers.get(job_id).map(|server| InferenceServerStatus {
            job_id: server.job_id.clone(),
            port: server.port,
            base_model: server.base_model.clone(),
            adapter_path: server.adapter_path.display().to_string(),
            status: server.status.clone(),
        })
    }

    /// List all running inference servers
    pub fn list_servers(&self) -> Vec<InferenceServerStatus> {
        let servers = self.servers.lock().unwrap();

        servers
            .values()
            .map(|server| InferenceServerStatus {
                job_id: server.job_id.clone(),
                port: server.port,
                base_model: server.base_model.clone(),
                adapter_path: server.adapter_path.display().to_string(),
                status: server.status.clone(),
            })
            .collect()
    }

    /// Stop all inference servers
    pub fn stop_all_servers(&self) -> Result<()> {
        let job_ids: Vec<String> = {
            let servers = self.servers.lock().unwrap();
            servers.keys().cloned().collect()
        };

        for job_id in job_ids {
            if let Err(e) = self.stop_server(&job_id) {
                error!("Failed to stop server {}: {}", job_id, e);
            }
        }

        Ok(())
    }
}

impl Drop for InferenceManager {
    fn drop(&mut self) {
        // Clean up all servers on drop
        if let Err(e) = self.stop_all_servers() {
            error!("Failed to stop all servers on drop: {}", e);
        }
    }
}

// Global inference manager instance
lazy_static::lazy_static! {
    pub static ref INFERENCE_MANAGER: InferenceManager = InferenceManager::new();
}
