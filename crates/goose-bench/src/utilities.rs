use std::collections::HashMap;
use std::env;
use std::process::{Child, Command};
use std::thread::JoinHandle;

pub fn union_hashmaps<K, V>(maps: Vec<HashMap<K, V>>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: Clone,
{
    // We can use the fold method to accumulate all maps into one
    maps.into_iter().fold(HashMap::new(), |mut result, map| {
        // For each map in the vector, extend the result with its entries
        result.extend(map);
        result
    })
}

pub fn await_process_exits(
    child_processes: &mut [Child],
    handles: Vec<JoinHandle<anyhow::Result<()>>>,
) {
    for child in child_processes.iter_mut() {
        match child.wait() {
            Ok(status) => println!("Child exited with status: {}", status),
            Err(e) => println!("Error waiting for child: {}", e),
        }
    }

    for handle in handles {
        match handle.join() {
            Ok(_res) => (),
            Err(e) => {
                // Handle thread panic
                println!("Thread panicked: {:?}", e);
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
