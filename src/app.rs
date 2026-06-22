use crate::{
    buffer::Buffer,
    commands,
    input::key_from_event,
    keymap::{Keymap, KeymapResult},
    picker::{DirectoryPicker, DirectoryPickerAction},
    renderer::{Renderer, StatusKind, TerminalSize},
    terminal::TerminalSession,
    view::View,
};
use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const DIRTY_QUIT_PROMPT: &str = "Buffer modified; quit without saving? (y or n)";
const COMMAND_HELP: &str = "Commands: /open <path>, /save, /quit, /quit!";

#[derive(Debug, Default, PartialEq, Eq)]
struct AppState {
    status_message: Option<String>,
    status_kind: Option<StatusKind>,
    dirty_quit_prompt: bool,
    command_line: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppAction {
    Continue,
    Quit,
}

pub fn run(path: &Path) -> io::Result<()> {
    if is_directory_path(path)? {
        return run_directory_path(path);
    }

    let buffer = Buffer::open(path)?;
    let mut terminal = TerminalSession::enter(io::stdout())?;
    run_editor(&mut terminal, buffer)
}

fn run_directory_path(path: &Path) -> io::Result<()> {
    let picker = DirectoryPicker::read(path)?;
    let mut terminal = TerminalSession::enter(io::stdout())?;

    let Some(path) = run_directory_picker(&mut terminal, picker)? else {
        return Ok(());
    };

    let buffer = Buffer::open(path)?;
    run_editor(&mut terminal, buffer)
}

fn is_directory_path(path: &Path) -> io::Result<bool> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn run_editor<W: io::Write>(
    terminal: &mut TerminalSession<W>,
    mut buffer: Buffer,
) -> io::Result<()> {
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

fn run_directory_picker<W: io::Write>(
    terminal: &mut TerminalSession<W>,
    mut picker: DirectoryPicker,
) -> io::Result<Option<PathBuf>> {
    let renderer = Renderer::new();

    render_directory_picker(&renderer, terminal.writer_mut(), &picker)?;

    loop {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    let key = key_from_event(key);
                    match picker.handle_key(key) {
                        DirectoryPickerAction::Continue => {
                            render_directory_picker(&renderer, terminal.writer_mut(), &picker)?;
                        }
                        DirectoryPickerAction::Quit => return Ok(None),
                        DirectoryPickerAction::Open(path) => return Ok(Some(path)),
                    }
                }
            }
            Event::Resize(_, _) => {
                render_directory_picker(&renderer, terminal.writer_mut(), &picker)?
            }
            _ => {}
        }
    }
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

        if self.command_line.is_some() {
            return self.handle_command_line_key(key, buffer, view);
        }

        if key == crate::input::Key::Char('/') && !keymap.has_pending_prefix() {
            self.command_line = Some("/".to_string());
            self.clear_status();
            return AppAction::Continue;
        }

        match keymap.resolve(key) {
            KeymapResult::Command(command) => {
                let outcome = commands::dispatch(command, buffer, view);
                self.apply_outcome(outcome)
            }
            KeymapResult::PendingPrefix => {
                self.set_status("C-x", StatusKind::Prefix);
                AppAction::Continue
            }
            KeymapResult::Unbound => {
                self.clear_status();
                AppAction::Continue
            }
        }
    }

    fn handle_command_line_key(
        &mut self,
        key: crate::input::Key,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        match key {
            crate::input::Key::Char(ch) => {
                if let Some(input) = self.command_line.as_mut() {
                    input.push(ch);
                }
                AppAction::Continue
            }
            crate::input::Key::Backspace => {
                if let Some(input) = self.command_line.as_mut() {
                    input.pop();
                }
                AppAction::Continue
            }
            crate::input::Key::Enter => {
                let input = self.command_line.take().unwrap_or_default();
                self.run_command_line(&input, buffer, view)
            }
            crate::input::Key::Escape => {
                self.command_line = None;
                self.set_status("Command canceled", StatusKind::Info);
                AppAction::Continue
            }
            _ => AppAction::Continue,
        }
    }

    fn run_command_line(
        &mut self,
        input: &str,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        let trimmed = input.trim();
        let Some(command_text) = trimmed.strip_prefix('/') else {
            self.set_status("Commands must start with /", StatusKind::Error);
            return AppAction::Continue;
        };
        let command_text = command_text.trim();

        match command_text {
            "" => {
                self.set_status("No command entered", StatusKind::Error);
                AppAction::Continue
            }
            "save" => {
                let outcome = commands::dispatch(commands::Command::SaveBuffer, buffer, view);
                self.apply_outcome(outcome)
            }
            "quit" => {
                let outcome = commands::dispatch(commands::Command::Quit, buffer, view);
                self.apply_outcome(outcome)
            }
            "quit!" => AppAction::Quit,
            "help" => {
                self.set_status(COMMAND_HELP, StatusKind::Info);
                AppAction::Continue
            }
            command if command == "open" || command.starts_with("open ") => {
                self.run_open_command(command, buffer, view)
            }
            command => {
                self.set_status(format!("Unknown command: /{command}"), StatusKind::Error);
                AppAction::Continue
            }
        }
    }

    fn run_open_command(
        &mut self,
        command: &str,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        let path_text = command
            .strip_prefix("open")
            .map(str::trim)
            .unwrap_or_default();

        if path_text.is_empty() {
            self.set_status("Usage: /open <path>", StatusKind::Error);
            return AppAction::Continue;
        }

        if buffer.is_dirty() {
            self.set_status(
                "Open canceled: current buffer has unsaved changes",
                StatusKind::Prompt,
            );
            return AppAction::Continue;
        }

        let path = PathBuf::from(path_text);
        match is_directory_path(&path) {
            Ok(true) => {
                self.set_status(
                    format!("Open failed: {} is a directory", path.display()),
                    StatusKind::Error,
                );
                AppAction::Continue
            }
            Ok(false) => match Buffer::open(&path) {
                Ok(opened) => {
                    *buffer = opened;
                    *view = View::new();
                    self.set_status(format!("Opened {}", path.display()), StatusKind::Success);
                    AppAction::Continue
                }
                Err(error) => {
                    self.set_status(format!("Open failed: {error}"), StatusKind::Error);
                    AppAction::Continue
                }
            },
            Err(error) => {
                self.set_status(format!("Open failed: {error}"), StatusKind::Error);
                AppAction::Continue
            }
        }
    }

    fn handle_dirty_quit_key(&mut self, key: crate::input::Key) -> AppAction {
        match key {
            crate::input::Key::Char('y') => AppAction::Quit,
            crate::input::Key::Char('n') | crate::input::Key::Escape => {
                self.dirty_quit_prompt = false;
                self.set_status("Quit canceled", StatusKind::Info);
                AppAction::Continue
            }
            _ => {
                self.set_status(DIRTY_QUIT_PROMPT, StatusKind::Prompt);
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
            self.set_status(DIRTY_QUIT_PROMPT, StatusKind::Prompt);
            return AppAction::Continue;
        }

        self.status_kind = outcome.status_message.as_ref().map(|_| {
            if outcome.save_failed {
                StatusKind::Error
            } else {
                StatusKind::Success
            }
        });
        self.status_message = outcome.status_message;
        AppAction::Continue
    }

    fn set_status(&mut self, message: impl Into<String>, kind: StatusKind) {
        self.status_message = Some(message.into());
        self.status_kind = Some(kind);
    }

    fn clear_status(&mut self) {
        self.status_message = None;
        self.status_kind = None;
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
        app_state.status_kind,
        app_state.command_line.as_deref(),
    )
}

fn render_directory_picker<W: io::Write>(
    renderer: &Renderer,
    writer: &mut W,
    picker: &DirectoryPicker,
) -> io::Result<()> {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let size = TerminalSize { cols, rows };
    renderer.render_directory_picker(writer, picker, size)
}

#[cfg(test)]
mod tests {
    use super::{AppAction, AppState, DIRTY_QUIT_PROMPT};
    use crate::{buffer::Buffer, input::Key, keymap::Keymap, renderer::StatusKind, view::View};
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
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Wrote")));
        assert_eq!(app.status_kind, Some(StatusKind::Success));
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
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Save failed")));
        assert_eq!(app.status_kind, Some(StatusKind::Error));
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
        assert_eq!(app.status_kind, Some(StatusKind::Prompt));
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

    #[test]
    fn slash_starts_command_line_without_editing_the_buffer() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        let action = app.handle_key(Key::Char('/'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line.as_deref(), Some("/"));
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn slash_after_ctrl_x_resets_prefix_without_starting_command_line() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Char('/'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line, None);
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());

        app.handle_key(Key::Char('a'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "aold");
    }

    #[test]
    fn prefix_status_is_classified_for_rendering() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        let action = app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.status_message.as_deref(), Some("C-x"));
        assert_eq!(app.status_kind, Some(StatusKind::Prefix));
    }

    #[test]
    fn escape_cancels_command_line_without_editing_the_buffer() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Char('/'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Char('s'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Escape, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line, None);
        assert_eq!(app.status_message.as_deref(), Some("Command canceled"));
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn slash_save_reuses_save_command_behavior() {
        let dir = test_dir("slash-save");
        let path = dir.join("notes.txt");
        fs::write(&path, "old").unwrap();
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let action = run_slash_command("/save", &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert!(!buffer.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "xold");
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Wrote")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn slash_quit_uses_clean_and_dirty_quit_rules() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        let action = run_slash_command("/quit", &mut app, &mut keymap, &mut buffer, &mut view);
        assert_eq!(action, AppAction::Quit);

        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let action = run_slash_command("/quit", &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert!(app.dirty_quit_prompt);
        assert_eq!(app.status_message.as_deref(), Some(DIRTY_QUIT_PROMPT));
    }

    #[test]
    fn slash_quit_bang_forces_dirty_quit() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let action = run_slash_command("/quit!", &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Quit);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn slash_open_replaces_buffer_and_resets_view() {
        let dir = test_dir("slash-open");
        let current_path = dir.join("current.txt");
        let target_path = dir.join("target.txt");
        fs::write(&current_path, "current").unwrap();
        fs::write(&target_path, "target").unwrap();
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&current_path).unwrap();
        let mut view = View::new();
        view.move_forward_char(&buffer);

        let command = format!("/open {}", target_path.display());
        let action = run_slash_command(&command, &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.path(), target_path.as_path());
        assert_eq!(buffer.text(), "target");
        assert_eq!(view.point(), 0);
        let expected_status = format!("Opened {}", target_path.display());
        assert_eq!(app.status_message.as_deref(), Some(expected_status.as_str()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn slash_open_reports_directory_without_replacing_buffer() {
        let dir = test_dir("slash-open-directory");
        let current_path = dir.join("current.txt");
        let nested_dir = dir.join("nested");
        fs::write(&current_path, "current").unwrap();
        fs::create_dir(&nested_dir).unwrap();
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&current_path).unwrap();
        let mut view = View::new();

        let command = format!("/open {}", nested_dir.display());
        let action = run_slash_command(&command, &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.path(), current_path.as_path());
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("is a directory")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn slash_open_keeps_dirty_buffer_in_place() {
        let dir = test_dir("slash-open-dirty");
        let current_path = dir.join("current.txt");
        let target_path = dir.join("target.txt");
        fs::write(&current_path, "current").unwrap();
        fs::write(&target_path, "target").unwrap();
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&current_path).unwrap();
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let command = format!("/open {}", target_path.display());
        let action = run_slash_command(&command, &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.path(), current_path.as_path());
        assert_eq!(buffer.text(), "xcurrent");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open canceled: current buffer has unsaved changes")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unknown_slash_command_reports_status_and_keeps_open() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        let action = run_slash_command("/bogus", &mut app, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "old");
        assert_eq!(app.status_message.as_deref(), Some("Unknown command: /bogus"));
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

    fn run_slash_command(
        command: &str,
        app: &mut AppState,
        keymap: &mut Keymap,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        for ch in command.chars() {
            app.handle_key(Key::Char(ch), keymap, buffer, view);
        }
        app.handle_key(Key::Enter, keymap, buffer, view)
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
