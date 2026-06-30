use console::Term;
use std::io::Write;

/// Restore terminal cursor visibility after cliclack dialogs (DECTCEM show).
pub fn restore_terminal() {
    let term = Term::stderr();
    let _ = term.show_cursor();
    // Belt-and-suspenders: ensure DECTCEM show even if Term API misses it.
    let _ = std::io::stderr().write_all(b"\x1b[?25h");
    let _ = term.flush();
}

/// Ensure terminal state is restored on normal exit, panic, or SIGINT.
pub struct TerminalGuard {
    #[cfg(unix)]
    old_sigint: libc::sighandler_t,
}

impl TerminalGuard {
    pub fn new() -> Self {
        #[cfg(unix)]
        let old_sigint = unsafe {
            libc::signal(libc::SIGINT, sigint_handler as libc::sighandler_t)
        };
        Self {
            #[cfg(unix)]
            old_sigint,
        }
    }
}

#[cfg(unix)]
extern "C" fn sigint_handler(_: libc::c_int) {
    restore_terminal();
    unsafe {
        libc::_exit(130);
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGINT, self.old_sigint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_terminal_does_not_panic() {
        restore_terminal();
    }
}
