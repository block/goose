use tokio::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

pub trait SubprocessExt {
    fn set_no_window(&mut self) -> &mut Self;
}

impl SubprocessExt for Command {
    fn set_no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW_FLAG);
        }
        self
    }
}

impl SubprocessExt for std::process::Command {
    fn set_no_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW_FLAG);
        }
        self
    }
}

#[allow(unused_variables)]
pub fn configure_subprocess(command: &mut Command) {
    // Isolate subprocess into its own process group so it does not receive
    // SIGINT when the user presses Ctrl+C in the terminal.
    #[cfg(unix)]
    command.process_group(0);
    command.set_no_window();
}
