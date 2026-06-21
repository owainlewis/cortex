use crate::{
    buffer::Buffer,
    commands,
    input::key_from_event,
    keymap::{Keymap, KeymapResult},
    renderer::{Renderer, TerminalSize},
    terminal::TerminalSession,
    view::View,
};
use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal,
};
use std::{
    io,
    path::Path,
};

const DIRTY_QUIT_PROMPT: &str = "Buffer modified; quit without saving? (y or n)";

#[derive(Debug, Default, PartialEq, Eq)]
struct AppState {
    status_message: Option<String>,
    dirty_quit_prompt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppAction {
    Continue,
    Quit,
}

pub fn run(path: &Path) -> io::Result<()> {
    let mut buffer = Buffer::open(path)?;
    let mut terminal = TerminalSession::enter(io::stdout())?;
    let mut view = View::new();
    let mut keymap = Keymap::new();
    let renderer = Renderer::new();
    let mut app_state = AppState::default();

    render(
        &renderer,
        terminal.writer_mut(),
        &buffer,
        &mut view,
        &app_state,
    )?;

    loop {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    let key = key_from_event(key);
                    if app_state.handle_key(key, &mut keymap, &mut buffer, &mut view)
                        == AppAction::Quit
                    {
                        break;
                    }
                    render(
                        &renderer,
                        terminal.writer_mut(),
                        &buffer,
                        &mut view,
                        &app_state,
                    )?;
                }
            }
            Event::Resize(_, _) => render(
                &renderer,
                terminal.writer_mut(),
                &buffer,
                &mut view,
                &app_state,
            )?,
            _ => {}
        }
    }

    Ok(())
}

impl AppState {
    fn handle_key(
        &mut self,
        key: crate::input::Key,
        keymap: &mut Keymap,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        if self.dirty_quit_prompt {
            return self.handle_dirty_quit_key(key);
        }

        match keymap.resolve(key) {
            KeymapResult::Command(command) => {
                let outcome = commands::dispatch(command, buffer, view);
                self.apply_outcome(outcome)
            }
            KeymapResult::PendingPrefix => {
                self.status_message = Some("C-x".to_string());
                AppAction::Continue
            }
            KeymapResult::Unbound => {
                self.status_message = None;
                AppAction::Continue
            }
        }
    }

    fn handle_dirty_quit_key(&mut self, key: crate::input::Key) -> AppAction {
        match key {
            crate::input::Key::Char('y') => AppAction::Quit,
            crate::input::Key::Char('n') | crate::input::Key::Escape => {
                self.dirty_quit_prompt = false;
                self.status_message = Some("Quit canceled".to_string());
                AppAction::Continue
            }
            _ => {
                self.status_message = Some(DIRTY_QUIT_PROMPT.to_string());
                AppAction::Continue
            }
        }
    }

    fn apply_outcome(&mut self, outcome: commands::CommandOutcome) -> AppAction {
        if outcome.quit {
            return AppAction::Quit;
        }

        if outcome.dirty_quit_blocked {
            self.dirty_quit_prompt = true;
            self.status_message = Some(DIRTY_QUIT_PROMPT.to_string());
            return AppAction::Continue;
        }

        self.status_message = outcome.status_message;
        AppAction::Continue
    }
}

fn render<W: io::Write>(
    renderer: &Renderer,
    writer: &mut W,
    buffer: &Buffer,
    view: &mut View,
    app_state: &AppState,
) -> io::Result<()> {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let size = TerminalSize { cols, rows };
    view.ensure_point_visible(buffer, renderer.viewport_height(size));
    renderer.render(
        writer,
        buffer,
        view,
        size,
        app_state.status_message.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::{AppAction, AppState, DIRTY_QUIT_PROMPT};
    use crate::{buffer::Buffer, input::Key, keymap::Keymap, view::View};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn save_key_sequence_saves_clears_dirty_state_and_shows_status() {
        let dir = test_dir("save-status");
        let path = dir.join("notes.txt");
        fs::write(&path, "old").unwrap();
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        assert!(buffer.is_dirty());

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('s'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert!(!buffer.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "xold");
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|message| message.contains("Wrote"))
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_failure_shows_status_keeps_dirty_state_and_stays_open() {
        let dir = test_dir("save-failure");
        let path = dir.join("missing").join("notes.txt");
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('s'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert!(buffer.is_dirty());
        assert!(
            app.status_message
                .as_deref()
                .is_some_and(|message| message.contains("Save failed"))
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clean_quit_exits_immediately() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('c'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Quit);
        assert!(!app.dirty_quit_prompt);
    }

    #[test]
    fn dirty_quit_prompts_without_exiting() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('c'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert!(app.dirty_quit_prompt);
        assert_eq!(app.status_message.as_deref(), Some(DIRTY_QUIT_PROMPT));
    }

    #[test]
    fn y_confirms_dirty_quit_without_saving() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        start_dirty_quit_prompt(&mut app, &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Char('y'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Quit);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn n_and_escape_cancel_dirty_quit() {
        for key in [Key::Char('n'), Key::Escape] {
            let mut app = AppState::default();
            let mut keymap = Keymap::new();
            let mut buffer = buffer_with_text("notes.txt", "");
            let mut view = View::new();

            start_dirty_quit_prompt(&mut app, &mut keymap, &mut buffer, &mut view);
            let action = app.handle_key(key, &mut keymap, &mut buffer, &mut view);

            assert_eq!(action, AppAction::Continue);
            assert!(!app.dirty_quit_prompt);
            assert_eq!(app.status_message.as_deref(), Some("Quit canceled"));
            assert!(buffer.is_dirty());
        }
    }

    #[test]
    fn other_keys_do_not_confirm_dirty_quit_or_edit_the_buffer() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        start_dirty_quit_prompt(&mut app, &mut keymap, &mut buffer, &mut view);
        let text_before = buffer.text();
        let action = app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert!(app.dirty_quit_prompt);
        assert_eq!(buffer.text(), text_before);
        assert_eq!(app.status_message.as_deref(), Some(DIRTY_QUIT_PROMPT));
    }

    fn start_dirty_quit_prompt(
        app: &mut AppState,
        keymap: &mut Keymap,
        buffer: &mut Buffer,
        view: &mut View,
    ) {
        app.handle_key(Key::Char('x'), keymap, buffer, view);
        app.handle_key(Key::Ctrl('x'), keymap, buffer, view);
        app.handle_key(Key::Ctrl('c'), keymap, buffer, view);
    }

    fn buffer_with_text(file_name: &str, text: &str) -> Buffer {
        let dir = test_dir("app");
        let path = dir.join(file_name);
        fs::write(&path, text).unwrap();
        let buffer = Buffer::open(&path).unwrap();
        fs::remove_dir_all(dir).unwrap();
        buffer
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cortex-app-test-{}-{name}-{unique}-{counter}",
            std::process::id(),
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
}
