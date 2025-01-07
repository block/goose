use kill_tree::{blocking::kill_tree_with_config, Config};
use lazy_static::lazy_static;
use std::sync::Mutex;

// Singleton that will store process IDs for spawned child processes implementing agent tasks.
lazy_static! {
    static ref PROCESS_STORE: Mutex<Vec<u32>> = Mutex::new(Vec::new());
}

pub fn store_process(pid: u32) {
    let mut store = PROCESS_STORE.lock().unwrap();
    store.push(pid);
}

// This removes the record of a process from the store, it does not kill it or check that it is dead.
pub fn remove_process(pid: u32) -> bool {
    let mut store = PROCESS_STORE.lock().unwrap();
    if let Some(index) = store.iter().position(|&x| x == pid) {
        store.remove(index);
        true
    } else {
        false
    }
}

/// Kill all stored processes
pub fn kill_processes() {
    let mut killed_processes = Vec::new();
    {
        let store = PROCESS_STORE.lock().unwrap();
        for &pid in store.iter() {
            let config = Config {
                signal: "SIGKILL".to_string(),
                ..Default::default()
            };
            let outputs = match kill_tree_with_config(pid, &config) {
                Ok(outputs) => outputs,
                Err(e) => {
                    eprintln!("Failed to kill process {}: {}", pid, e);
                    continue;
                }
            };
            for output in outputs {
                match output {
                    kill_tree::Output::Killed { process_id, .. } => {
                        killed_processes.push(process_id);
                    }
                    kill_tree::Output::MaybeAlreadyTerminated { process_id, .. } => {
                        killed_processes.push(process_id);
                    }
                }
            }
        }
    }
    // Clean up the store
    for pid in killed_processes {
        remove_process(pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;
    use std::{fs, thread};
    use sysinfo::{Pid, ProcessesToUpdate, System};
    use tokio::process::Command;

    #[tokio::test]
    async fn test_kill_processes_with_children() {
        // Create a temporary script that spawns a child process
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("test_script.sh");
        let script_content = r#"#!/bin/bash
        # Sleep in the parent process
        sleep 300
        "#;

        fs::write(&script_path, script_content).unwrap();
        fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Start the parent process which will spawn a child
        let parent = Command::new("bash")
            .arg("-c")
            .arg(script_path.to_str().unwrap())
            .spawn()
            .expect("Failed to start parent process");

        let parent_pid = parent.id().unwrap() as u32;

        // Store the parent process ID
        store_process(parent_pid);

        let mut attempt = 0;
        let mut child_pids: Vec<u32>;
        loop {
            attempt += 1;
            if attempt > 100 {
                panic!("Failed to start processes for process kill test");
            }

            // Give processes time to start
            thread::sleep(Duration::from_millis(10));

            // Get the child process ID using pgrep
            let child_pids_cmd = Command::new("pgrep")
                .arg("-P")
                .arg(parent_pid.to_string())
                .output()
                .await
                .expect("Failed to get child PIDs");

            let child_pid_str = String::from_utf8_lossy(&child_pids_cmd.stdout);
            child_pids = child_pid_str
                .lines()
                .filter_map(|line| line.trim().parse::<u32>().ok())
                .collect::<Vec<u32>>();
            if child_pids.len() != 1 {
                print!("Waiting for 1 child_pids. Got {:?}", child_pids.len());
                continue;
            }

            if is_process_running(parent_pid).await && is_process_running(child_pids[0]).await {
                break;
            }
        }

        kill_processes();

        // Wait until processes are killed
        let mut attempts = 0;
        while attempts < 100 {
            if !is_process_running(parent_pid).await && !is_process_running(child_pids[0]).await {
                break;
            }
            thread::sleep(Duration::from_millis(10));
            attempts += 1;
        }

        // Verify processes are dead
        assert!(!is_process_running(parent_pid).await);
        assert!(!is_process_running(child_pids[0]).await);

        // Clean up the temporary script
        fs::remove_file(script_path).unwrap();
    }

    async fn is_process_running(pid: u32) -> bool {
        let mut system = System::new_all();
        system.refresh_processes(ProcessesToUpdate::All, true);

        match system.process(Pid::from_u32(pid)) {
            Some(process) => !matches!(
                process.status(),
                sysinfo::ProcessStatus::Stop
                    | sysinfo::ProcessStatus::Zombie
                    | sysinfo::ProcessStatus::Dead
            ),
            None => false,
        }
    }
}
