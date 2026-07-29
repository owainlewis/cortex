use crossterm::{
    cursor, execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

const DISCONNECT_EVENTS: libc::c_short = libc::POLLHUP | libc::POLLERR | libc::POLLNVAL;
const DISCONNECT_CHECK_MILLIS: libc::c_int = 50;

pub struct TerminalDisconnectGuard {
    stop: Arc<AtomicBool>,
    monitor: Option<thread::JoinHandle<()>>,
}

pub struct TerminalSession<W: Write> {
    writer: W,
    state: TerminalState,
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
        let mut session = Self {
            writer,
            state: TerminalState::default(),
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
    pub fn start() -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let monitor_stop = Arc::clone(&stop);
        let monitor = thread::Builder::new()
            .name("cortex-terminal-disconnect".to_string())
            .spawn(move || monitor_disconnect(monitor_stop))?;

        Ok(Self {
            stop,
            monitor: Some(monitor),
        })
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

fn monitor_disconnect(stop: Arc<AtomicBool>) {
    let mut descriptor = libc::pollfd {
        // Crossterm may open /dev/tty when stdin is redirected, but macOS
        // poll reports POLLNVAL for that opened descriptor. Cortex renders to
        // the inherited terminal on stdout, which observes the same hangup.
        fd: libc::STDOUT_FILENO,
        events: 0,
        revents: 0,
    };

    while !stop.load(Ordering::Acquire) {
        let result = unsafe { libc::poll(&mut descriptor, 1, DISCONNECT_CHECK_MILLIS) };
        if result == -1 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        if result == -1 || result > 0 && descriptor.revents & DISCONNECT_EVENTS != 0 {
            // The PTY controller is gone, so no terminal remains to restore.
            // Exit directly because Crossterm may have trapped the main thread.
            unsafe { libc::_exit(1) };
        }
    }
}

fn setup_error(context: &str, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{CleanupStep, TerminalState};

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
