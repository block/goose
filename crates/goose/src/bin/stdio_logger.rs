use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::thread::{self, JoinHandle};

fn handle_output_stream<R: BufRead + Send + 'static>(
    reader: R,
    mut log_file: File,
    mut output_writer: Box<dyn Write + Send>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if let Err(e) = writeln!(log_file, "{}", line) {
                        eprintln!("Error writing to log file: {}", e);
                    }
                    log_file.flush().ok();

                    if writeln!(output_writer, "{}", line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn handle_stdin_stream(mut child_stdin: ChildStdin, mut log_file: File) -> JoinHandle<()> {
    thread::spawn(move || {
        let stdin = io::stdin();

        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    if let Err(e) = writeln!(log_file, "{}", line) {
                        eprintln!("Error writing to stdin.log: {}", e);
                    }
                    log_file.flush().ok();

                    if writeln!(child_stdin, "{}", line).is_err() {
                        break;
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

    let cmd = &args[1];
    let cmd_args = &args[2..];

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

    let child_stdin = child.stdin.take().unwrap();
    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

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

    let exit_status = child.wait()?;

    stdin_handle.join().ok();
    stdout_handle.join().ok();
    stderr_handle.join().ok();

    std::process::exit(exit_status.code().unwrap_or(1));
}
