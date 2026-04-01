use std::io::{self, IsTerminal, Read};

#[cfg(unix)]
use anyhow::Context;

#[cfg(unix)]
use std::os::fd::AsRawFd;

#[derive(Default)]
struct LineBuffer {
    pending: Vec<u8>,
}

impl LineBuffer {
    fn push_bytes(&mut self, bytes: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(bytes);
        self.take_complete_lines()
    }

    fn take_complete_lines(&mut self) -> Vec<String> {
        let mut lines = Vec::new();

        while let Some(pos) = self.pending.iter().position(|byte| *byte == b'\n') {
            let raw_line = self.pending.drain(..=pos).collect::<Vec<_>>();
            let line = String::from_utf8_lossy(&raw_line)
                .trim_end_matches(['\r', '\n'])
                .to_string();
            lines.push(line);
        }

        lines
    }
}

pub struct QueuedInputCapture {
    #[cfg(unix)]
    unix: Option<UnixQueuedInputCapture>,
}

impl QueuedInputCapture {
    pub fn new(enabled: bool) -> Self {
        if !enabled || !io::stdin().is_terminal() {
            return Self::disabled();
        }

        #[cfg(unix)]
        {
            match UnixQueuedInputCapture::new() {
                Ok(unix) => Self { unix: Some(unix) },
                Err(_) => Self::disabled(),
            }
        }

        #[cfg(not(unix))]
        {
            Self::disabled()
        }
    }

    pub fn disabled() -> Self {
        Self {
            #[cfg(unix)]
            unix: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        #[cfg(unix)]
        {
            self.unix.is_some()
        }

        #[cfg(not(unix))]
        {
            false
        }
    }

    pub fn read_available_lines(&mut self) -> anyhow::Result<Vec<String>> {
        #[cfg(unix)]
        {
            if let Some(unix) = self.unix.as_mut() {
                return unix.read_available_lines();
            }
        }

        Ok(Vec::new())
    }

    pub fn pause(&mut self) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            if let Some(unix) = self.unix.as_mut() {
                unix.pause()?;
            }
        }

        Ok(())
    }

    pub fn resume(&mut self) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            if let Some(unix) = self.unix.as_mut() {
                unix.resume()?;
            }
        }

        Ok(())
    }

    pub fn disable(&mut self) {
        let _ = self.pause();

        #[cfg(unix)]
        {
            self.unix = None;
        }
    }
}

#[cfg(unix)]
struct UnixQueuedInputCapture {
    stdin: io::Stdin,
    line_buffer: LineBuffer,
    original_flags: i32,
    nonblocking: bool,
}

#[cfg(unix)]
impl UnixQueuedInputCapture {
    fn new() -> anyhow::Result<Self> {
        let stdin = io::stdin();
        let fd = stdin.as_raw_fd();
        let original_flags = get_flags(fd)?;
        set_flags(fd, original_flags | libc::O_NONBLOCK)?;

        Ok(Self {
            stdin,
            line_buffer: LineBuffer::default(),
            original_flags,
            nonblocking: true,
        })
    }

    fn read_available_lines(&mut self) -> anyhow::Result<Vec<String>> {
        if !self.nonblocking {
            return Ok(Vec::new());
        }

        let mut lines = Vec::new();
        let mut buf = [0_u8; 1024];

        loop {
            match self.stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(count) => lines.extend(self.line_buffer.push_bytes(&buf[..count])),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error).context("failed to read queued stdin input"),
            }
        }

        Ok(lines)
    }

    fn pause(&mut self) -> anyhow::Result<()> {
        if self.nonblocking {
            set_flags(self.stdin.as_raw_fd(), self.original_flags)?;
            self.nonblocking = false;
        }

        Ok(())
    }

    fn resume(&mut self) -> anyhow::Result<()> {
        if !self.nonblocking {
            set_flags(
                self.stdin.as_raw_fd(),
                self.original_flags | libc::O_NONBLOCK,
            )?;
            self.nonblocking = true;
        }

        Ok(())
    }
}

#[cfg(unix)]
impl Drop for UnixQueuedInputCapture {
    fn drop(&mut self) {
        let _ = self.pause();
    }
}

#[cfg(unix)]
fn get_flags(fd: i32) -> anyhow::Result<i32> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error()).context("failed to read stdin flags");
    }

    Ok(flags)
}

#[cfg(unix)]
fn set_flags(fd: i32, flags: i32) -> anyhow::Result<()> {
    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
    if result < 0 {
        return Err(io::Error::last_os_error()).context("failed to update stdin flags");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::LineBuffer;

    #[test]
    fn extracts_complete_lines() {
        let mut buffer = LineBuffer::default();

        assert_eq!(buffer.push_bytes(b"hello\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn preserves_partial_lines_between_reads() {
        let mut buffer = LineBuffer::default();

        assert!(buffer.push_bytes(b"hello").is_empty());
        assert_eq!(
            buffer.push_bytes(b" world\n"),
            vec!["hello world".to_string()]
        );
    }

    #[test]
    fn trims_crlf_and_extracts_multiple_lines() {
        let mut buffer = LineBuffer::default();

        assert_eq!(
            buffer.push_bytes(b"first\r\nsecond\n"),
            vec!["first".to_string(), "second".to_string()]
        );
    }
}
