use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::thread::{self, JoinHandle};

// Generic function to handle output streams (stdout/stderr)
fn handle_output_stream<R: BufRead + Send + 'static>(
    reader: R,
    mut log_file: File,
    mut output_writer: Box<dyn Write + Send>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    // Log the output
                    if let Err(e) = writeln!(log_file, "{}", line) {
                        eprintln!("Error writing to log file: {}", e);
                    }
                    log_file.flush().ok();

                    // Forward to output
                    if writeln!(output_writer, "{}", line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

// Handle stdin separately since it has different logic
fn handle_stdin_stream(mut child_stdin: ChildStdin, mut log_file: File) -> JoinHandle<()> {
    thread::spawn(move || {
        let stdin = io::stdin();

        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    // Log the input
                    if let Err(e) = writeln!(log_file, "{}", line) {
                        eprintln!("Error writing to stdin.log: {}", e);
                    }
                    log_file.flush().ok();

                    // Forward to child process
                    if writeln!(child_stdin, "{}", line).is_err() {
                        break; // Child process closed stdin
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!("Example: {{}} ls -la");
        std::process::exit(1);
    }

    // Extract command and arguments
    let cmd = &args[1];
    let cmd_args = &args[2..];

    // Create log files
    let stdin_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("stdin.log")?;

    let stdout_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("stdout.log")?;

    let stderr_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("stderr.log")?;

    // Spawn the child process
    let mut child = Command::new(cmd)
        .args(cmd_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            eprintln!("Failed to execute command '{}': {}", cmd, e);
            e
        })?;

    // Get handles to child's stdio
    let child_stdin = child.stdin.take().unwrap();
    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    // Start I/O handling threads
    let stdin_handle = handle_stdin_stream(child_stdin, stdin_log);
    let stdout_handle = handle_output_stream(
        BufReader::new(child_stdout),
        stdout_log,
        Box::new(io::stdout()),
    );
    let stderr_handle = handle_output_stream(
        BufReader::new(child_stderr),
        stderr_log,
        Box::new(io::stderr()),
    );

    // Wait for the child process to complete
    let exit_status = child.wait()?;

    // Wait for all I/O threads to finish processing
    stdin_handle.join().ok();
    stdout_handle.join().ok();
    stderr_handle.join().ok();

    // Print completion message
    println!(
        "\nCommand completed with exit code: {:?}",
        exit_status.code()
    );
    println!("Logs written to:");
    println!("  - stdin.log  (input sent to the command)");
    println!("  - stdout.log (standard output from the command)");
    println!("  - stderr.log (error output from the command)");

    // Exit with the same code as the child process
    std::process::exit(exit_status.code().unwrap_or(1));
}
