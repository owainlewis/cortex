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
    ops::Range,
    path::{Path, PathBuf},
};

const DIRTY_QUIT_PROMPT: &str = "Buffer modified; quit without saving? (y or n)";
const COMMAND_HELP: &str =
    "Commands: /help, /commands, /open <path>, /search <text>, /next, /save, /undo, /redo, /quit, /quit!";

#[derive(Debug, Default, PartialEq, Eq)]
struct AppState {
    status_message: Option<String>,
    status_kind: Option<StatusKind>,
    dirty_quit_prompt: bool,
    command_line: Option<String>,
    keycast: Option<String>,
    last_search: Option<String>,
    mark: Option<usize>,
    kill_ring: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppAction {
    Continue,
    OpenFilePicker,
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
                    match app_state.handle_key(key, &mut keymap, &mut buffer, &mut view) {
                        AppAction::Continue => {}
                        AppAction::OpenFilePicker => {
                            open_file_from_picker(terminal, &mut buffer, &mut view, &mut app_state)?
                        }
                        AppAction::Quit => break,
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
                        DirectoryPickerAction::Browse(path) => match DirectoryPicker::read(&path) {
                            Ok(next_picker) => {
                                picker = next_picker;
                                render_directory_picker(&renderer, terminal.writer_mut(), &picker)?;
                            }
                            Err(error) => {
                                picker.set_status_message(format!("Open failed: {error}"));
                                render_directory_picker(&renderer, terminal.writer_mut(), &picker)?;
                            }
                        },
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

fn open_file_from_picker<W: io::Write>(
    terminal: &mut TerminalSession<W>,
    buffer: &mut Buffer,
    view: &mut View,
    app_state: &mut AppState,
) -> io::Result<()> {
    let directory = picker_directory(buffer.path());
    let picker = match DirectoryPicker::read(&directory) {
        Ok(picker) => picker,
        Err(error) => {
            app_state.set_status(format!("Open failed: {error}"), StatusKind::Error);
            return Ok(());
        }
    };

    let Some(path) = run_directory_picker(terminal, picker)? else {
        app_state.set_status("Open canceled", StatusKind::Info);
        return Ok(());
    };

    match Buffer::open(&path) {
        Ok(opened) => {
            *buffer = opened;
            *view = View::new();
            app_state.mark = None;
            app_state.set_status(format!("Opened {}", path.display()), StatusKind::Success);
        }
        Err(error) => {
            app_state.set_status(format!("Open failed: {error}"), StatusKind::Error);
        }
    }

    Ok(())
}

fn picker_directory(file_path: &Path) -> PathBuf {
    file_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

impl AppState {
    fn handle_key(
        &mut self,
        key: crate::input::Key,
        keymap: &mut Keymap,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        self.keycast = keycast_text(key);

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
            KeymapResult::Command(commands::Command::SetMark) => self.set_mark(view),
            KeymapResult::Command(commands::Command::KillRegion) => self.kill_region(buffer, view),
            KeymapResult::Command(commands::Command::KillLine) => self.kill_line(buffer, view),
            KeymapResult::Command(commands::Command::Yank) => self.yank(buffer, view),
            KeymapResult::Command(commands::Command::RepeatSearch) => {
                self.repeat_search(buffer, view)
            }
            KeymapResult::Command(command) => self.dispatch_command(command, buffer, view),
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

    fn active_region(&self, buffer: &Buffer, view: &View) -> Option<Range<usize>> {
        let len_chars = buffer.len_chars();
        let mark = self.mark?.min(len_chars);
        let point = view.point().min(len_chars);

        if mark == point {
            return None;
        }

        Some(mark.min(point)..mark.max(point))
    }

    fn set_mark(&mut self, view: &View) -> AppAction {
        self.mark = Some(view.point());
        self.set_status("Mark set", StatusKind::Info);
        AppAction::Continue
    }

    fn kill_region(&mut self, buffer: &mut Buffer, view: &mut View) -> AppAction {
        let Some(region) = self.active_region(buffer, view) else {
            self.set_status("No active region", StatusKind::Error);
            return AppAction::Continue;
        };

        let text = buffer.text_range(region.clone());
        buffer.delete_with_points(region.clone(), view.point(), region.start);
        view.set_point(region.start, buffer);
        self.kill_ring = Some(text);
        self.mark = None;
        self.set_status("Cut region", StatusKind::Success);
        AppAction::Continue
    }

    fn kill_line(&mut self, buffer: &mut Buffer, view: &mut View) -> AppAction {
        let point = view.point();
        let Some(region) = kill_line_range(buffer, point) else {
            self.set_status("Nothing to cut", StatusKind::Error);
            return AppAction::Continue;
        };

        let text = buffer.text_range(region.clone());
        buffer.delete_with_points(region, point, point);
        view.set_point(point, buffer);
        self.kill_ring = Some(text);
        self.mark = None;
        self.set_status("Cut line", StatusKind::Success);
        AppAction::Continue
    }

    fn yank(&mut self, buffer: &mut Buffer, view: &mut View) -> AppAction {
        let Some(text) = self.kill_ring.clone().filter(|text| !text.is_empty()) else {
            self.set_status("No cut text", StatusKind::Error);
            return AppAction::Continue;
        };

        let point = view.point();
        buffer.insert(point, &text);
        view.set_point(point + text.chars().count(), buffer);
        self.mark = None;
        self.set_status("Yanked", StatusKind::Success);
        AppAction::Continue
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

    fn run_command_line(&mut self, input: &str, buffer: &mut Buffer, view: &mut View) -> AppAction {
        let trimmed = input.trim();
        let Some(command_text) = trimmed.strip_prefix('/') else {
            self.set_status("Commands must start with /", StatusKind::Error);
            return AppAction::Continue;
        };
        let command_text = command_text.trim();

        match command_text {
            "" | "help" | "commands" => {
                self.set_status(COMMAND_HELP, StatusKind::Info);
                AppAction::Continue
            }
            "save" => self.dispatch_command(commands::Command::SaveBuffer, buffer, view),
            "undo" => self.dispatch_command(commands::Command::Undo, buffer, view),
            "redo" => self.dispatch_command(commands::Command::Redo, buffer, view),
            "quit" => self.dispatch_command(commands::Command::Quit, buffer, view),
            "quit!" => AppAction::Quit,
            command if command == "search" || command.starts_with("search ") => {
                self.run_search_command(command, buffer, view)
            }
            "next" => self.repeat_search(buffer, view),
            command if command == "open" || command.starts_with("open ") => {
                self.run_open_command(command, buffer, view)
            }
            command => {
                self.set_status(format!("Unknown command: /{command}"), StatusKind::Error);
                AppAction::Continue
            }
        }
    }

    fn run_search_command(&mut self, command: &str, buffer: &Buffer, view: &mut View) -> AppAction {
        let query = command
            .strip_prefix("search")
            .map(str::trim)
            .unwrap_or_default();

        if query.is_empty() {
            self.set_status("Usage: /search <text>", StatusKind::Error);
            return AppAction::Continue;
        }

        self.last_search = Some(query.to_string());
        self.find_search_match(buffer, view, query, view.point())
    }

    fn repeat_search(&mut self, buffer: &Buffer, view: &mut View) -> AppAction {
        let Some(query) = self.last_search.clone() else {
            self.set_status("No previous search", StatusKind::Error);
            return AppAction::Continue;
        };

        let start = view.point().saturating_add(1).min(buffer.len_chars());
        self.find_search_match(buffer, view, &query, start)
    }

    fn find_search_match(
        &mut self,
        buffer: &Buffer,
        view: &mut View,
        query: &str,
        start: usize,
    ) -> AppAction {
        match buffer.find_forward(query, start) {
            Some(point) => {
                view.set_point(point, buffer);
                self.set_status(format!("Found: {query}"), StatusKind::Success);
            }
            None => {
                self.set_status(format!("Not found: {query}"), StatusKind::Error);
            }
        }

        AppAction::Continue
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
                    self.mark = None;
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

    fn dispatch_command(
        &mut self,
        command: commands::Command,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        let clear_mark = command_clears_mark(command);
        let outcome = commands::dispatch(command, buffer, view);

        if clear_mark {
            self.mark = None;
        }

        self.apply_outcome(outcome)
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

        if outcome.open_file_picker {
            self.clear_status();
            return AppAction::OpenFilePicker;
        }

        if outcome.open_file_blocked {
            self.status_message = outcome.status_message;
            self.status_kind = Some(StatusKind::Prompt);
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
        app_state.active_region(buffer, view),
        app_state.command_line.as_deref(),
        app_state.keycast.as_deref(),
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

fn keycast_text(key: crate::input::Key) -> Option<String> {
    match key {
        crate::input::Key::Char(ch) => Some(ch.to_string()),
        crate::input::Key::Ctrl(' ') => Some("C-Space".to_string()),
        crate::input::Key::Ctrl(ch) => Some(format!("C-{ch}")),
        crate::input::Key::Command(ch) => Some(format!("Cmd-{ch}")),
        crate::input::Key::Enter => Some("Enter".to_string()),
        crate::input::Key::Escape => Some("Esc".to_string()),
        crate::input::Key::Backspace => Some("Backspace".to_string()),
        crate::input::Key::Delete => Some("Delete".to_string()),
        crate::input::Key::Left => Some("Left".to_string()),
        crate::input::Key::Right => Some("Right".to_string()),
        crate::input::Key::Up => Some("Up".to_string()),
        crate::input::Key::Down => Some("Down".to_string()),
        crate::input::Key::Unhandled => None,
    }
}

fn kill_line_range(buffer: &Buffer, point: usize) -> Option<Range<usize>> {
    if point >= buffer.len_chars() {
        return None;
    }

    let line_idx = buffer.line_for_char(point);
    let line_end = buffer.line_end_char(line_idx);

    if point < line_end {
        Some(point..line_end)
    } else {
        Some(point..point + 1)
    }
}

fn command_clears_mark(command: commands::Command) -> bool {
    matches!(
        command,
        commands::Command::Insert(_)
            | commands::Command::InsertNewline
            | commands::Command::DeleteBackward
            | commands::Command::DeleteForward
            | commands::Command::Undo
            | commands::Command::Redo
    )
}

#[cfg(test)]
mod tests {
    use super::{picker_directory, AppAction, AppState, COMMAND_HELP, DIRTY_QUIT_PROMPT};
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
    fn bare_slash_and_help_commands_list_available_commands() {
        for command in ["/", "/help", "/commands"] {
            let mut app = AppState::default();
            let mut keymap = Keymap::new();
            let mut buffer = buffer_with_text("notes.txt", "old");
            let mut view = View::new();

            let action = run_slash_command(command, &mut app, &mut keymap, &mut buffer, &mut view);

            assert_eq!(action, AppAction::Continue);
            assert_eq!(app.command_line, None);
            assert_eq!(app.status_message.as_deref(), Some(COMMAND_HELP));
            assert_eq!(app.status_kind, Some(StatusKind::Info));
            assert_eq!(buffer.text(), "old");
            assert!(!buffer.is_dirty());
        }
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
    fn keypress_updates_the_keycast_display() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.keycast.as_deref(), Some("C-x"));

        app.handle_key(Key::Enter, &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.keycast.as_deref(), Some("Enter"));
    }

    #[test]
    fn ctrl_space_marks_region_and_ctrl_w_cuts_it() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "abcd");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl(' '), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.active_region(&buffer, &view), Some(1..3));

        let action = app.handle_key(Key::Ctrl('w'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "ad");
        assert_eq!(view.point(), 1);
        assert_eq!(app.kill_ring.as_deref(), Some("bc"));
        assert_eq!(app.mark, None);
        assert_eq!(app.status_message.as_deref(), Some("Cut region"));
    }

    #[test]
    fn editing_after_mark_clears_the_region() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "abcd");
        let mut view = View::new();

        app.handle_key(Key::Ctrl(' '), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(app.mark, None);
        assert_eq!(app.active_region(&buffer, &view), None);
    }

    #[test]
    fn stale_mark_is_clamped_before_cutting_region() {
        let mut app = AppState {
            mark: Some(8),
            ..AppState::default()
        };
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "abc");
        let mut view = View::new();

        let action = app.handle_key(Key::Ctrl('w'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "");
        assert_eq!(view.point(), 0);
        assert_eq!(app.kill_ring.as_deref(), Some("abc"));
        assert_eq!(app.mark, None);
    }

    #[test]
    fn ctrl_k_cuts_to_line_end_and_ctrl_y_yanks_it() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "alpha\nbeta");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('k'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "al\nbeta");
        assert_eq!(view.point(), 2);
        assert_eq!(app.kill_ring.as_deref(), Some("pha"));

        let action = app.handle_key(Key::Ctrl('y'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "alpha\nbeta");
        assert_eq!(view.point(), 5);
        assert_eq!(app.status_message.as_deref(), Some("Yanked"));
    }

    #[test]
    fn ctrl_k_at_line_end_cuts_the_newline() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "alpha\nbeta");
        let mut view = View::new();

        view.move_to_line_end(&buffer);
        let action = app.handle_key(Key::Ctrl('k'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "alphabeta");
        assert_eq!(view.point(), 5);
        assert_eq!(app.kill_ring.as_deref(), Some("\n"));
    }

    #[test]
    fn ctrl_x_ctrl_f_requests_file_picker_when_buffer_is_clean() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::OpenFilePicker);
        assert_eq!(app.status_message, None);
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn ctrl_x_ctrl_f_keeps_dirty_buffer_in_place() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "xold");
        assert!(buffer.is_dirty());
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open canceled: current buffer has unsaved changes")
        );
        assert_eq!(app.status_kind, Some(StatusKind::Prompt));
    }

    #[test]
    fn file_picker_starts_from_current_file_parent_or_current_directory() {
        assert_eq!(
            picker_directory(PathBuf::from("/tmp/current.txt").as_path()),
            PathBuf::from("/tmp")
        );
        assert_eq!(
            picker_directory(PathBuf::from("current.txt").as_path()),
            PathBuf::from(".")
        );
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
    fn undo_key_reverses_the_last_edit() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('/'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "old");
        assert_eq!(view.point(), 0);
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn command_z_reverses_the_last_edit() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Command('z'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "old");
        assert_eq!(view.point(), 0);
        assert_eq!(app.keycast.as_deref(), Some("Cmd-z"));
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn slash_undo_and_redo_reuse_edit_history() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "xold");

        let action = run_slash_command("/undo", &mut app, &mut keymap, &mut buffer, &mut view);
        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());

        let action = run_slash_command("/redo", &mut app, &mut keymap, &mut buffer, &mut view);
        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "xold");
        assert!(buffer.is_dirty());
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
        assert_eq!(
            app.status_message.as_deref(),
            Some(expected_status.as_str())
        );
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
    fn slash_search_moves_point_to_next_match_and_remembers_query() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "alpha beta alpha");
        let mut view = View::new();

        view.move_forward_char(&buffer);
        let action = run_slash_command(
            "/search alpha",
            &mut app,
            &mut keymap,
            &mut buffer,
            &mut view,
        );

        assert_eq!(action, AppAction::Continue);
        assert_eq!(view.point(), 11);
        assert_eq!(app.last_search.as_deref(), Some("alpha"));
        assert_eq!(app.status_message.as_deref(), Some("Found: alpha"));
        assert_eq!(app.status_kind, Some(StatusKind::Success));
    }

    #[test]
    fn slash_search_finds_match_at_current_point() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "alpha beta alpha");
        let mut view = View::new();

        let action = run_slash_command(
            "/search alpha",
            &mut app,
            &mut keymap,
            &mut buffer,
            &mut view,
        );

        assert_eq!(action, AppAction::Continue);
        assert_eq!(view.point(), 0);
        assert_eq!(app.last_search.as_deref(), Some("alpha"));
        assert_eq!(app.status_message.as_deref(), Some("Found: alpha"));
    }

    #[test]
    fn ctrl_s_repeats_the_previous_search_and_wraps() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "alpha beta alpha");
        let mut view = View::new();

        run_slash_command(
            "/search alpha",
            &mut app,
            &mut keymap,
            &mut buffer,
            &mut view,
        );
        assert_eq!(view.point(), 0);

        let action = app.handle_key(Key::Ctrl('s'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(view.point(), 11);
        assert_eq!(app.status_message.as_deref(), Some("Found: alpha"));

        let action = app.handle_key(Key::Ctrl('s'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(view.point(), 0);
        assert_eq!(app.status_message.as_deref(), Some("Found: alpha"));
    }

    #[test]
    fn search_reports_missing_and_empty_queries() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "alpha beta");
        let mut view = View::new();

        run_slash_command("/search", &mut app, &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.status_message.as_deref(), Some("Usage: /search <text>"));
        assert_eq!(app.status_kind, Some(StatusKind::Error));

        run_slash_command(
            "/search missing",
            &mut app,
            &mut keymap,
            &mut buffer,
            &mut view,
        );
        assert_eq!(view.point(), 0);
        assert_eq!(app.status_message.as_deref(), Some("Not found: missing"));
        assert_eq!(app.status_kind, Some(StatusKind::Error));
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
        assert_eq!(
            app.status_message.as_deref(),
            Some("Unknown command: /bogus")
        );
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
