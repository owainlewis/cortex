use crossterm::{
    cursor,
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

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
            vec![CleanupStep::LeaveAlternateScreen, CleanupStep::DisableRawMode]
        );
    }
}
