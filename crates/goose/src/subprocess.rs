use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

#[allow(unused_variables)]
pub fn configure_subprocess(command: &mut Command) {
    // Isolate subprocess into its own process group so it does not receive
    // SIGINT when the user presses Ctrl+C in the terminal.
    #[cfg(unix)]
    command.process_group(0);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW_FLAG);
}
