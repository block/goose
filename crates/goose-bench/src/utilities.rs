use anyhow::Result;
use std::env;
use std::process::{Child, Command};
use std::thread::JoinHandle;
use tracing;

pub fn await_process_exits(child_processes: &mut [Child], handles: Vec<JoinHandle<Result<()>>>) {
    for child in child_processes.iter_mut() {
        match child.wait() {
            Ok(status) => tracing::info!("Child exited with status: {}", status),
            Err(e) => tracing::error!("Error waiting for child: {}", e),
        }
    }

    for handle in handles {
        match handle.join() {
            Ok(_res) => (),
            Err(e) => {
                // Handle thread panic
                tracing::error!("Thread panicked: {:?}", e);
            }
        }
    }
}

pub fn parallel_bench_cmd(bench_cmd: String, config: String, envs: Vec<(String, String)>) -> Child {
    let current_exe = env::current_exe().expect("Failed to get current executable path");

    let mut cmd = Command::new(current_exe);
    cmd.arg("bench").arg(bench_cmd).arg("--config").arg(config);

    for (key, value) in envs.into_iter() {
        cmd.env(key, value);
    }

    cmd.spawn().expect("Failed to spawn child process")
}
