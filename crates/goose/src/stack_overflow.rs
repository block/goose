//! Prints the faulting thread's backtrace when a thread overflows its stack.
//!
//! Rust's default handler only prints "thread '...' has overflowed its stack",
//! which doesn't identify the overflowing call chain. This installs a
//! process-wide SIGSEGV/SIGBUS handler (macOS delivers stack overflows as
//! SIGBUS) that prints a backtrace to stderr before the process dies. The
//! handler runs on the alternate signal stack std installs for every thread it
//! spawns, so it also covers tokio worker threads.

use std::backtrace::Backtrace;
use std::io::Write;

pub fn install_backtrace_handler() {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handle_fatal_signal as *const () as libc::sighandler_t;
        action.sa_flags = libc::SA_ONSTACK;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGSEGV, libc::SIGBUS] {
            libc::sigaction(signal, &action, std::ptr::null_mut());
        }
    }
}

extern "C" fn handle_fatal_signal(signal: libc::c_int) {
    let name = match signal {
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGBUS => "SIGBUS",
        _ => "fatal signal",
    };
    // Capturing and printing a backtrace is not async-signal-safe, but the
    // process is about to die anyway; the worst case is dying without the
    // backtrace, which is where we would be without this handler.
    let thread = std::thread::current();
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(
        stderr,
        "fatal signal {name} (possible stack overflow) on thread '{}'; backtrace:\n{}",
        thread.name().unwrap_or("<unnamed>"),
        Backtrace::force_capture()
    );
    let _ = stderr.flush();
    unsafe {
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}
