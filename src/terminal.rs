use crossterm::{
    cursor,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Write},
    mem::MaybeUninit,
    os::unix::fs::MetadataExt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

const DISCONNECT_EVENTS: libc::c_short = libc::POLLHUP | libc::POLLERR | libc::POLLNVAL;
const DISCONNECT_CHECK_MILLIS: libc::c_int = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MonitorAction {
    Continue,
    Retry,
    Stop,
    Disconnect,
}

struct TerminalDisconnectGuard {
    stop: Arc<AtomicBool>,
    monitor: Option<thread::JoinHandle<()>>,
}

pub struct TerminalSession<W: Write> {
    writer: W,
    state: TerminalState,
    _disconnect_guard: TerminalDisconnectGuard,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TerminalState {
    raw_enabled: bool,
    alternate_screen: bool,
    cursor_hidden: bool,
    bracketed_paste: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupStep {
    DisableBracketedPaste,
    ShowCursor,
    LeaveAlternateScreen,
    DisableRawMode,
}

impl TerminalState {
    fn cleanup_steps(self) -> Vec<CleanupStep> {
        let mut steps = Vec::new();

        if self.bracketed_paste {
            steps.push(CleanupStep::DisableBracketedPaste);
        }

        if self.cursor_hidden {
            steps.push(CleanupStep::ShowCursor);
        }

        if self.alternate_screen {
            steps.push(CleanupStep::LeaveAlternateScreen);
        }

        if self.raw_enabled {
            steps.push(CleanupStep::DisableRawMode);
        }

        steps
    }
}

impl<W: Write> TerminalSession<W> {
    pub fn enter(writer: W) -> io::Result<Self> {
        let disconnect_guard = TerminalDisconnectGuard::start()?;
        let mut session = Self {
            writer,
            state: TerminalState::default(),
            _disconnect_guard: disconnect_guard,
        };

        terminal::enable_raw_mode().map_err(|error| {
            setup_error(
                "could not enable raw mode; Cortex must run in an interactive terminal",
                error,
            )
        })?;
        session.state.raw_enabled = true;

        if let Err(error) = execute!(session.writer, EnterAlternateScreen) {
            session.cleanup();
            return Err(setup_error("could not enter alternate screen", error));
        }
        session.state.alternate_screen = true;

        if let Err(error) = execute!(session.writer, cursor::Hide) {
            session.cleanup();
            return Err(setup_error("could not hide terminal cursor", error));
        }
        session.state.cursor_hidden = true;
        if let Err(error) = session.enable_bracketed_paste() {
            session.cleanup();
            return Err(error);
        }
        Ok(session)
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        // A successful write followed by a failed flush may have enabled the
        // terminal mode. Record the cleanup obligation before either can fail.
        self.state.bracketed_paste = true;
        execute!(self.writer, EnableBracketedPaste)
            .map_err(|error| setup_error("could not enable bracketed paste", error))
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    fn cleanup(&mut self) {
        for step in self.state.cleanup_steps() {
            match step {
                CleanupStep::DisableBracketedPaste => {
                    let _ = execute!(self.writer, DisableBracketedPaste);
                    self.state.bracketed_paste = false;
                }
                CleanupStep::ShowCursor => {
                    let _ = execute!(self.writer, cursor::Show);
                    self.state.cursor_hidden = false;
                }
                CleanupStep::LeaveAlternateScreen => {
                    let _ = execute!(self.writer, LeaveAlternateScreen);
                    self.state.alternate_screen = false;
                }
                CleanupStep::DisableRawMode => {
                    let _ = terminal::disable_raw_mode();
                    self.state.raw_enabled = false;
                }
            }
        }
    }
}

impl<W: Write> Drop for TerminalSession<W> {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl TerminalDisconnectGuard {
    fn start() -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let monitor = terminal_stdin_descriptor()?
            .map(|descriptor| {
                let monitor_stop = Arc::clone(&stop);
                thread::Builder::new()
                    .name("cortex-terminal-disconnect".to_string())
                    .spawn(move || monitor_disconnect(descriptor, monitor_stop))
            })
            .transpose()?;

        Ok(Self { stop, monitor })
    }
}

impl Drop for TerminalDisconnectGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(monitor) = self.monitor.take() {
            let _ = monitor.join();
        }
    }
}

fn terminal_stdin_descriptor() -> io::Result<Option<libc::c_int>> {
    if unsafe { libc::isatty(libc::STDIN_FILENO) } != 1 {
        return Ok(None);
    }

    // /dev/tty is a special device that macOS poll rejects even when it is
    // attached to a terminal. Compare device identities: ttyname_r can fail
    // during device-name lookup and must not silently disable the guard.
    let stdin_is_dev_tty = descriptor_is_dev_tty(libc::STDIN_FILENO)?;
    Ok(disconnect_descriptor(true, stdin_is_dev_tty))
}

fn descriptor_is_dev_tty(descriptor: libc::c_int) -> io::Result<bool> {
    let mut state = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(descriptor, state.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let device = unsafe { state.assume_init() }.st_rdev as u64;
    Ok(device == std::fs::metadata("/dev/tty")?.rdev())
}

fn disconnect_descriptor(stdin_is_terminal: bool, stdin_is_dev_tty: bool) -> Option<libc::c_int> {
    (stdin_is_terminal && !stdin_is_dev_tty).then_some(libc::STDIN_FILENO)
}

fn poll_disconnect(terminal_descriptor: libc::c_int) -> MonitorAction {
    let mut descriptor = libc::pollfd {
        fd: terminal_descriptor,
        // macOS does not report PTY hangups with an empty event mask.
        // Watching readability does not consume the editor's input.
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&mut descriptor, 1, DISCONNECT_CHECK_MILLIS) };
    let error_kind = (result == -1).then(|| io::Error::last_os_error().kind());
    monitor_action(result, descriptor.revents, error_kind)
}

fn monitor_disconnect(terminal_descriptor: libc::c_int, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        match poll_disconnect(terminal_descriptor) {
            MonitorAction::Continue => {}
            MonitorAction::Retry => {
                thread::sleep(Duration::from_millis(DISCONNECT_CHECK_MILLIS as u64));
            }
            MonitorAction::Stop => return,
            MonitorAction::Disconnect => {
                // The PTY controller is gone, so no terminal remains to restore.
                // Exit directly because Crossterm may have trapped the main thread.
                unsafe { libc::_exit(1) };
            }
        }
    }
}

fn monitor_action(
    result: libc::c_int,
    events: libc::c_short,
    error_kind: Option<io::ErrorKind>,
) -> MonitorAction {
    if result == -1 {
        return match error_kind {
            Some(io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock) => MonitorAction::Retry,
            _ => MonitorAction::Stop,
        };
    }
    if result > 0 && events & DISCONNECT_EVENTS != 0 {
        return MonitorAction::Disconnect;
    }
    if result > 0 {
        // Input may remain readable until the editor consumes it. Throttle
        // those wakeups so the monitor never spins while the editor is busy.
        return MonitorAction::Retry;
    }
    MonitorAction::Continue
}

fn setup_error(context: &str, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        disconnect_descriptor, monitor_action, poll_disconnect, CleanupStep, MonitorAction,
        TerminalState,
    };

    #[test]
    fn direct_pty_is_not_the_special_dev_tty_device() {
        use std::os::fd::AsRawFd;
        let (_master, slave) = test_pty();
        assert!(!super::descriptor_is_dev_tty(slave.as_raw_fd()).unwrap());
        assert!(super::descriptor_is_dev_tty(-1).is_err());
    }

    #[test]
    fn disconnect_poll_observes_a_real_pty_hangup() {
        let (master, slave) = test_pty();
        use std::os::fd::AsRawFd;
        assert_eq!(poll_disconnect(slave.as_raw_fd()), MonitorAction::Continue);
        // A concurrently spawned metadata tool must not inherit the PTY.
        let mut child = std::process::Command::new("/bin/cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        drop(master);
        let action = poll_disconnect(slave.as_raw_fd());
        drop(child.stdin.take());
        assert!(child.wait().unwrap().success());
        assert_eq!(action, MonitorAction::Disconnect);
    }

    #[test]
    fn disconnect_poll_leaves_readable_input_for_the_editor() {
        use std::{
            io::{Read, Write},
            os::fd::AsRawFd,
        };
        let (mut master, mut slave) = test_pty();
        master.write_all(b"hello\n").unwrap();
        assert_eq!(poll_disconnect(slave.as_raw_fd()), MonitorAction::Retry);
        let mut input = [0; 6];
        slave.read_exact(&mut input).unwrap();
        assert_eq!(&input, b"hello\n");
    }

    fn test_pty() -> (std::fs::File, std::fs::File) {
        use std::{
            ffi::CStr,
            os::fd::{AsRawFd, FromRawFd},
        };
        // libc does not yet expose Darwin's ptsname_r binding.
        unsafe extern "C" {
            fn ptsname_r(
                fd: libc::c_int,
                name: *mut libc::c_char,
                len: libc::size_t,
            ) -> libc::c_int;
        }
        // Other unit tests spawn metadata tools. Set CLOEXEC atomically so
        // those children cannot keep this test's controller alive.
        let master = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC) };
        assert!(
            master >= 0,
            "open PTY controller: {}",
            std::io::Error::last_os_error()
        );
        let master = unsafe { std::fs::File::from_raw_fd(master) };
        assert_eq!(unsafe { libc::grantpt(master.as_raw_fd()) }, 0);
        assert_eq!(unsafe { libc::unlockpt(master.as_raw_fd()) }, 0);
        let mut name = [0; libc::PATH_MAX as usize];
        assert_eq!(
            unsafe { ptsname_r(master.as_raw_fd(), name.as_mut_ptr(), name.len()) },
            0
        );
        let path = unsafe { CStr::from_ptr(name.as_ptr()) };
        let slave = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
            )
        };
        assert!(
            slave >= 0,
            "open PTY slave: {}",
            std::io::Error::last_os_error()
        );
        (master, unsafe { std::fs::File::from_raw_fd(slave) })
    }

    #[test]
    fn disconnect_monitor_requires_terminal_stdin() {
        assert_eq!(
            disconnect_descriptor(true, false),
            Some(libc::STDIN_FILENO),
            "a direct terminal stdin is safe to monitor"
        );
        assert_eq!(
            disconnect_descriptor(false, false),
            None,
            "redirected stdin must not fall back to another descriptor"
        );
        assert_eq!(
            disconnect_descriptor(true, true),
            None,
            "/dev/tty stdin must not use macOS poll"
        );
    }

    #[test]
    fn disconnect_monitor_only_exits_for_explicit_disconnect_events() {
        assert_eq!(
            monitor_action(1, libc::POLLHUP, None),
            MonitorAction::Disconnect
        );
        assert_eq!(
            monitor_action(-1, 0, Some(std::io::ErrorKind::Interrupted)),
            MonitorAction::Retry
        );
        assert_eq!(
            monitor_action(-1, 0, Some(std::io::ErrorKind::WouldBlock)),
            MonitorAction::Retry
        );
        assert_eq!(
            monitor_action(-1, 0, Some(std::io::ErrorKind::Other)),
            MonitorAction::Stop
        );
        assert_eq!(monitor_action(0, 0, None), MonitorAction::Continue);
    }

    #[test]
    fn failed_paste_enable_flush_still_disables_the_mode() {
        use std::{
            io::{self, Write},
            sync::{atomic::AtomicBool, Arc},
        };
        struct FailFirstFlush {
            output: Vec<u8>,
            fail: bool,
        }
        impl Write for FailFirstFlush {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.output.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                if std::mem::take(&mut self.fail) {
                    Err(io::Error::other("flush failed"))
                } else {
                    Ok(())
                }
            }
        }
        let mut session = super::TerminalSession {
            writer: FailFirstFlush {
                output: Vec::new(),
                fail: true,
            },
            state: TerminalState::default(),
            _disconnect_guard: super::TerminalDisconnectGuard {
                stop: Arc::new(AtomicBool::new(false)),
                monitor: None,
            },
        };
        assert!(session.enable_bracketed_paste().is_err());
        assert!(session.state.bracketed_paste);
        session.cleanup();
        assert_eq!(session.writer.output, b"\x1b[?2004h\x1b[?2004l");
        assert!(!session.state.bracketed_paste);
        session.cleanup();
        assert_eq!(session.writer.output, b"\x1b[?2004h\x1b[?2004l");
    }

    #[test]
    fn cleanup_steps_restore_terminal_in_reverse_setup_order() {
        let state = TerminalState {
            raw_enabled: true,
            alternate_screen: true,
            cursor_hidden: true,
            bracketed_paste: true,
        };

        assert_eq!(
            state.cleanup_steps(),
            vec![
                CleanupStep::DisableBracketedPaste,
                CleanupStep::ShowCursor,
                CleanupStep::LeaveAlternateScreen,
                CleanupStep::DisableRawMode
            ]
        );
    }

    #[test]
    fn cleanup_steps_handle_partial_setup() {
        let state = TerminalState {
            raw_enabled: true,
            alternate_screen: true,
            cursor_hidden: false,
            bracketed_paste: false,
        };

        assert_eq!(
            state.cleanup_steps(),
            vec![
                CleanupStep::LeaveAlternateScreen,
                CleanupStep::DisableRawMode
            ]
        );
    }
}
