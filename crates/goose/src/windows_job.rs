// Windows Job Object cleanup for child processes (Goose Windows support).
//
// Attaches spawned MCP subprocesses to a Job Object configured with
// JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE. When the Goose process exits (and the
// job handle is closed by the OS), Windows terminates every process in the
// job, preventing orphaned child processes. This is the Windows analog of the
// Linux PR_SET_PDEATHSIG behavior in subprocess.rs.

#![allow(dead_code)]

#[cfg(windows)]
mod windows_impl {
    use std::mem::{size_of, zeroed};
    use std::ptr::null_mut;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use winapi::shared::minwindef::FALSE;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::jobapi2::{AssignProcessToJobObject, CreateJobObjectW};
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winbase::SetInformationJobObject;
    use winapi::um::winnt::{
        JobObjectExtendedLimitInformation, HANDLE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    // HANDLE (*mut c_void) is not Send/Sync, so we store the handle as a usize
    // and cast back to HANDLE at use sites. 0 means "not yet created".
    static JOB_HANDLE: AtomicUsize = AtomicUsize::new(0);

    pub fn ensure_job_object() -> Option<HANDLE> {
        let existing = JOB_HANDLE.load(Ordering::Acquire);
        if existing != 0 {
            return Some(existing as HANDLE);
        }

        unsafe {
            let job = CreateJobObjectW(null_mut(), null_mut());
            if job.is_null() {
                return None;
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let set_res = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if set_res == FALSE {
                // If we fail to configure the job object to terminate on close, do not publish
                // this handle. Cleaning up here avoids mutating global state with a partially
                // configured Job Object.
                CloseHandle(job);
                return None;
            }

            // Publish the handle, but if another thread won the race, close ours.
            match JOB_HANDLE.compare_exchange(0, job as usize, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => Some(job),
                Err(winner) => {
                    CloseHandle(job);
                    Some(winner as HANDLE)
                }
            }
        }
    }

    pub fn attach_pid_to_job(pid: u32) {
        let job = match ensure_job_object() {
            Some(job) => job,
            None => return,
        };
        unsafe {
            // AssignProcessToJobObject requires PROCESS_SET_QUOTA in addition to
            // PROCESS_TERMINATE; without it the assignment fails and the child is
            // never tied to the job, leaving it orphaned on exit.
            let proc = OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, FALSE, pid);
            if !proc.is_null() {
                if AssignProcessToJobObject(job, proc) == FALSE {
                    tracing::warn!(pid, "failed to assign child process to Windows job object");
                }
                CloseHandle(proc);
            }
        }
    }

    pub fn init_windows_cleanup() {
        let _ = ensure_job_object();
    }

    pub fn windows_cleanup_enabled() -> bool {
        JOB_HANDLE.load(Ordering::Acquire) != 0
    }
}

#[cfg(windows)]
pub use windows_impl::{
    attach_pid_to_job, ensure_job_object, init_windows_cleanup, windows_cleanup_enabled,
};
