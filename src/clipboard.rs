use std::{
    io::{self, Read, Write},
    os::fd::{AsRawFd, RawFd},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const TRANSFER_LIMIT: usize = 16 * 1024 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq, Eq)]
pub struct Clipboard {
    copy_program: PathBuf,
    paste_program: PathBuf,
    timeout: Duration,
    transfer_limit: usize,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            copy_program: PathBuf::from("/usr/bin/pbcopy"),
            paste_program: PathBuf::from("/usr/bin/pbpaste"),
            timeout: PROCESS_TIMEOUT,
            transfer_limit: TRANSFER_LIMIT,
        }
    }
}

impl Clipboard {
    pub fn copy(&self, text: &str) -> io::Result<()> {
        self.check_size(text.len())?;
        let deadline = Instant::now() + self.timeout;
        let mut child = Command::new(&self.copy_program)
            .env("LC_ALL", "en_US.UTF-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let result = (|| {
            let mut input = child.stdin.take().unwrap();
            nonblocking(input.as_raw_fd())?;
            let mut remaining = text.as_bytes();
            while !remaining.is_empty() {
                check_deadline(deadline)?;
                match input.write(remaining) {
                    Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                    Ok(written) => remaining = &remaining[written..],
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        wait_for_pipe(input.as_raw_fd(), libc::POLLOUT, deadline)?;
                    }
                    Err(error) => return Err(error),
                }
            }
            drop(input);
            wait_for_exit(&mut child, deadline)
        })();
        reap_failed_child(&mut child, &result);
        result
    }

    pub fn paste(&self) -> io::Result<String> {
        let deadline = Instant::now() + self.timeout;
        let mut child = Command::new(&self.paste_program)
            .env("LC_ALL", "en_US.UTF-8")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let result = (|| {
            let mut output = child.stdout.take().unwrap();
            nonblocking(output.as_raw_fd())?;
            let mut bytes = Vec::new();
            let mut chunk = [0; 16 * 1024];
            loop {
                check_deadline(deadline)?;
                match output.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(read) => {
                        self.check_size(bytes.len() + read)?;
                        bytes.extend_from_slice(&chunk[..read]);
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        wait_for_pipe(output.as_raw_fd(), libc::POLLIN, deadline)?;
                    }
                    Err(error) => return Err(error),
                }
            }
            wait_for_exit(&mut child, deadline)?;
            String::from_utf8(bytes).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "clipboard text is not valid UTF-8",
                )
            })
        })();
        reap_failed_child(&mut child, &result);
        result
    }

    pub(crate) fn check_size(&self, bytes: usize) -> io::Result<()> {
        if bytes > self.transfer_limit {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("clipboard transfer exceeds {} bytes", self.transfer_limit),
            ))
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(crate) fn with_test_programs(copy_program: PathBuf, paste_program: PathBuf) -> Self {
        Self {
            copy_program,
            paste_program,
            ..Self::default()
        }
    }
}

fn nonblocking(fd: RawFd) -> io::Result<()> {
    // SAFETY: fd is borrowed from a live child pipe. These commands query and
    // update its descriptor flags without taking ownership.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn check_deadline(deadline: Instant) -> io::Result<()> {
    if Instant::now() >= deadline {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "clipboard command timed out",
        ))
    } else {
        Ok(())
    }
}

fn wait_for_pipe(fd: RawFd, events: libc::c_short, deadline: Instant) -> io::Result<()> {
    loop {
        check_deadline(deadline)?;
        let mut descriptor = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        let timeout = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(20) as i32;
        // SAFETY: descriptor contains a live pipe and remains valid during poll.
        let ready = unsafe { libc::poll(&mut descriptor, 1, timeout.max(1)) };
        if ready > 0 {
            return Ok(());
        }
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}

fn wait_for_exit(child: &mut Child, deadline: Instant) -> io::Result<()> {
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "clipboard command failed: {status}"
                )))
            };
        }
        check_deadline(deadline)?;
        thread::sleep(
            Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

fn reap_failed_child<T>(child: &mut Child, result: &io::Result<T>) {
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
pub(crate) fn test_program(directory: &std::path::Path, name: &str, body: &str) -> PathBuf {
    use std::{fs, os::unix::fs::PermissionsExt};
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[cfg(test)]
mod tests {
    use super::{test_program, Clipboard};
    use std::{
        fs, io,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cortex-clipboard-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn controlled_programs_round_trip_literal_utf8_without_the_system_clipboard() {
        let dir = fixture();
        let copy = test_program(&dir, "copy", "exec /bin/cat > \"${0%/*}/data\"");
        let paste = test_program(&dir, "paste", "exec /bin/cat \"${0%/*}/data\"");
        let clipboard = Clipboard::with_test_programs(copy, paste);
        let text = "  λ\tfoo\r\n\x18\x03👨‍💻  ".repeat(20_000);
        clipboard.copy(&text).unwrap();
        assert_eq!(clipboard.paste().unwrap(), text);
        clipboard.copy("").unwrap();
        assert_eq!(clipboard.paste().unwrap(), "");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failures_invalid_utf8_and_limits_are_reported_without_returning_partial_text() {
        let dir = fixture();
        let fail = test_program(&dir, "fail", "printf 'private data'; exit 7");
        let mut clipboard = Clipboard::with_test_programs(fail.clone(), fail);
        assert!(clipboard.copy("anything").is_err());
        let error = clipboard.paste().unwrap_err().to_string();
        assert!(error.contains("clipboard command failed"));
        assert!(!error.contains("private data"));
        clipboard.paste_program = test_program(&dir, "invalid", "printf '\\377'");
        assert_eq!(
            clipboard.paste().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        clipboard.transfer_limit = 4;
        clipboard.paste_program = test_program(&dir, "large", "printf '12345'");
        assert_eq!(
            clipboard.paste().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        clipboard.copy_program = dir.join("missing");
        assert_eq!(
            clipboard.copy("12345").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            clipboard.copy("1234").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failure_cleanup_kills_and_reaps_the_owned_child() {
        let mut child = std::process::Command::new("/bin/sh")
            .args(["-c", "while :; do :; done"])
            .spawn()
            .unwrap();
        let pid = child.id() as i32;
        let failure: io::Result<()> = Err(io::ErrorKind::TimedOut.into());
        super::reap_failed_child(&mut child, &failure);
        assert!(child.try_wait().unwrap().is_some());
        // SAFETY: signal zero only checks whether the owned child exists.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    }

    #[test]
    fn timeouts_cover_blocked_readers_writers_and_closed_output() {
        let dir = fixture();
        let hang = test_program(&dir, "hang", "while :; do :; done");
        let mut clipboard = Clipboard::with_test_programs(hang.clone(), hang.clone());
        clipboard.timeout = Duration::from_millis(500);
        for copy in [false, true] {
            let result = if copy {
                clipboard.copy(&"x".repeat(1_000_000))
            } else {
                clipboard.paste().map(|_| ())
            };
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        }
        // Closing stdout does not let a still-running process bypass the deadline.
        clipboard.paste_program =
            test_program(&dir, "closed-output", "exec 1>&-; while :; do :; done");
        assert_eq!(
            clipboard.paste().unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
