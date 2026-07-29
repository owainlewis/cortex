use crossterm::{
    cursor, execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    ffi::CStr,
    io::{self, Write},
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupStep {
    ShowCursor,
    LeaveAlternateScreen,
    DisableRawMode,
}

impl TerminalState {
    fn cleanup_steps(self) -> Vec<CleanupStep> {
        let mut steps = Vec::new();

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
        Ok(session)
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    fn cleanup(&mut self) {
        for step in self.state.cleanup_steps() {
            match step {
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
        let monitor = terminal_stdin_descriptor()
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

fn terminal_stdin_descriptor() -> Option<libc::c_int> {
    let stdin_is_terminal = unsafe { libc::isatty(libc::STDIN_FILENO) == 1 };
    if !stdin_is_terminal {
        return None;
    }

    let stdin_is_dev_tty = stdin_terminal_name()?.as_slice() == b"/dev/tty";
    disconnect_descriptor(true, stdin_is_dev_tty)
}

fn stdin_terminal_name() -> Option<Vec<u8>> {
    let mut path = [0 as libc::c_char; libc::PATH_MAX as usize];
    if unsafe { libc::ttyname_r(libc::STDIN_FILENO, path.as_mut_ptr(), path.len()) } != 0 {
        return None;
    }

    Some(unsafe { CStr::from_ptr(path.as_ptr()) }.to_bytes().to_vec())
}

fn disconnect_descriptor(stdin_is_terminal: bool, stdin_is_dev_tty: bool) -> Option<libc::c_int> {
    (stdin_is_terminal && !stdin_is_dev_tty).then_some(libc::STDIN_FILENO)
}

fn monitor_disconnect(terminal_descriptor: libc::c_int, stop: Arc<AtomicBool>) {
    let mut descriptor = libc::pollfd {
        // Terminal stdin is the exact descriptor Crossterm reads. Redirected
        // stdin may make Crossterm open /dev/tty, but macOS poll rejects that
        // descriptor and no other standard descriptor is guaranteed to match.
        fd: terminal_descriptor,
        events: 0,
        revents: 0,
    };

    while !stop.load(Ordering::Acquire) {
        let result = unsafe { libc::poll(&mut descriptor, 1, DISCONNECT_CHECK_MILLIS) };
        let error_kind = (result == -1).then(|| io::Error::last_os_error().kind());
        match monitor_action(result, descriptor.revents, error_kind) {
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
    MonitorAction::Continue
}

fn setup_error(context: &str, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{disconnect_descriptor, monitor_action, CleanupStep, MonitorAction, TerminalState};

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
    fn cleanup_steps_restore_terminal_in_reverse_setup_order() {
        let state = TerminalState {
            raw_enabled: true,
            alternate_screen: true,
            cursor_hidden: true,
        };

        assert_eq!(
            state.cleanup_steps(),
            vec![
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
