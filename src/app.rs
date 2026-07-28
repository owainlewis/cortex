use crate::{
    buffer::Buffer,
    commands,
    editor::{Editor, OpenResult, SwitchError},
    input::key_from_event,
    keymap::{Keymap, KeymapResult},
    picker::{DirectoryPicker, DirectoryPickerAction},
    renderer::{Renderer, StatusKind, TerminalSize},
    signals::TerminationSignals,
    terminal::TerminalSession,
    text,
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
    time::{Duration, Instant},
};

const DIRTY_QUIT_PROMPT: &str = "Buffers modified; quit without saving? (y or n)";
const DISK_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const SIGNAL_CHECK_INTERVAL: Duration = Duration::from_millis(50);
const COMMAND_HELP: &str =
    "Commands: /help, /commands, /open <path>, /search <text>, /next, /reload, /save, /undo, /redo, /quit, /quit!";

#[derive(Debug, Default, PartialEq, Eq)]
struct AppState {
    status_message: Option<String>,
    status_kind: Option<StatusKind>,
    dirty_quit_prompt: bool,
    command_line: Option<String>,
    prompt_kind: Option<PromptKind>,
    keycast: Option<String>,
    last_search: Option<String>,
    mark: Option<usize>,
    kill_ring: Option<String>,
    last_disk_check: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    Command,
    FindFile,
    SwitchBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppAction {
    Continue,
    FindFile(PathBuf),
    OpenFile(PathBuf),
    SwitchBuffer(String),
    Quit,
    ForceQuit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppControl {
    Continue,
    BrowseDirectory(PathBuf),
    Quit,
}

pub fn run(path: &Path) -> io::Result<()> {
    let signals = TerminationSignals::register()?;
    let result = run_until_exit(path, &signals);
    let received_signal = signals.received_signal();
    drop(signals);

    if let Some(signal) = received_signal {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            format!("termination signal {signal} received"),
        ));
    }

    result
}

fn run_until_exit(path: &Path, signals: &TerminationSignals) -> io::Result<()> {
    if is_directory_path(path)? {
        return run_directory_path(path, signals);
    }

    let buffer = Buffer::open(path)?;
    let mut terminal = TerminalSession::enter(io::stdout())?;
    run_editor(&mut terminal, buffer, signals)
}

fn run_directory_path(path: &Path, signals: &TerminationSignals) -> io::Result<()> {
    let picker = DirectoryPicker::read(path)?;
    let mut terminal = TerminalSession::enter(io::stdout())?;

    let Some(path) = run_directory_picker(&mut terminal, picker, signals)? else {
        return Ok(());
    };

    let buffer = Buffer::open(path)?;
    run_editor(&mut terminal, buffer, signals)
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
    buffer: Buffer,
    signals: &TerminationSignals,
) -> io::Result<()> {
    let mut editor = Editor::new(buffer)?;
    let mut keymap = Keymap::new();
    let renderer = Renderer::new();
    let mut app_state = AppState::default();

    render_editor(
        &renderer,
        terminal.writer_mut(),
        &mut editor,
        &mut app_state,
    )?;

    while let Some(event) = next_event(signals)? {
        match event {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    let key = key_from_event(key);
                    let action = {
                        let (buffer, view) = editor.active_mut();
                        app_state.handle_key(key, &mut keymap, buffer, view)
                    };
                    match apply_app_action(&mut editor, &mut app_state, action) {
                        AppControl::Continue => {}
                        AppControl::BrowseDirectory(path) => {
                            browse_directory_in_editor(
                                terminal,
                                &mut editor,
                                &path,
                                &mut app_state,
                                signals,
                            )?;
                            if signals.received() {
                                break;
                            }
                            renderer.invalidate();
                        }
                        AppControl::Quit => break,
                    }
                    render_editor(
                        &renderer,
                        terminal.writer_mut(),
                        &mut editor,
                        &mut app_state,
                    )?;
                }
            }
            Event::Resize(_, _) => render_editor(
                &renderer,
                terminal.writer_mut(),
                &mut editor,
                &mut app_state,
            )?,
            _ => {}
        }
    }

    Ok(())
}

fn apply_app_action(
    editor: &mut Editor,
    app_state: &mut AppState,
    action: AppAction,
) -> AppControl {
    match action {
        AppAction::Continue => AppControl::Continue,
        AppAction::FindFile(path) => match is_directory_path(&path) {
            Ok(true) => AppControl::BrowseDirectory(path),
            Ok(false) => {
                open_file_in_editor(editor, &path, app_state);
                AppControl::Continue
            }
            Err(error) => {
                app_state.set_status(format!("Open failed: {error}"), StatusKind::Error);
                AppControl::Continue
            }
        },
        AppAction::OpenFile(path) => {
            open_file_in_editor(editor, &path, app_state);
            AppControl::Continue
        }
        AppAction::SwitchBuffer(name) => {
            switch_buffer(editor, &name, app_state);
            AppControl::Continue
        }
        AppAction::Quit if editor.any_dirty() => {
            app_state.request_dirty_quit();
            AppControl::Continue
        }
        AppAction::Quit | AppAction::ForceQuit => AppControl::Quit,
    }
}

fn browse_directory_in_editor<W: io::Write>(
    terminal: &mut TerminalSession<W>,
    editor: &mut Editor,
    path: &Path,
    app_state: &mut AppState,
    signals: &TerminationSignals,
) -> io::Result<()> {
    let picker = match DirectoryPicker::read(path) {
        Ok(picker) => picker,
        Err(error) => {
            app_state.set_status(format!("Open failed: {error}"), StatusKind::Error);
            return Ok(());
        }
    };

    let Some(path) = run_directory_picker(terminal, picker, signals)? else {
        if signals.received() {
            return Ok(());
        }
        app_state.set_status("Open canceled", StatusKind::Info);
        return Ok(());
    };

    open_file_in_editor(editor, &path, app_state);
    Ok(())
}

fn open_file_in_editor(editor: &mut Editor, path: &Path, app_state: &mut AppState) {
    match is_directory_path(path) {
        Ok(true) => app_state.set_status(
            format!("Open failed: {} is a directory", path.display()),
            StatusKind::Error,
        ),
        Ok(false) => match editor.open(path) {
            Ok(OpenResult::Opened) => {
                app_state.mark = None;
                app_state.set_status(format!("Opened {}", path.display()), StatusKind::Success);
            }
            Ok(OpenResult::AlreadyOpen) => {
                app_state.mark = None;
                app_state.set_status(
                    format!("Switched to {}", path.display()),
                    StatusKind::Success,
                );
            }
            Err(error) => app_state.set_status(format!("Open failed: {error}"), StatusKind::Error),
        },
        Err(error) => app_state.set_status(format!("Open failed: {error}"), StatusKind::Error),
    }
}

fn switch_buffer(editor: &mut Editor, name: &str, app_state: &mut AppState) {
    match editor.switch_to(name) {
        Ok(()) => {
            app_state.mark = None;
            let path = editor.active().0.path().display().to_string();
            app_state.set_status(format!("Switched to {path}"), StatusKind::Success);
        }
        Err(SwitchError::Ambiguous) => {
            app_state.set_status(format!("Ambiguous buffer name: {name}"), StatusKind::Error)
        }
        Err(SwitchError::NotFound) => app_state.set_status(
            format!("No open buffer named {name}. Open: {}", editor.names()),
            StatusKind::Error,
        ),
    }
}

fn run_directory_picker<W: io::Write>(
    terminal: &mut TerminalSession<W>,
    mut picker: DirectoryPicker,
    signals: &TerminationSignals,
) -> io::Result<Option<PathBuf>> {
    let renderer = Renderer::new();

    render_directory_picker(&renderer, terminal.writer_mut(), &picker)?;

    while let Some(event) = next_event(signals)? {
        match event {
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

    Ok(None)
}

fn next_event(signals: &TerminationSignals) -> io::Result<Option<Event>> {
    loop {
        if signals.received() {
            return Ok(None);
        }

        match event::poll(SIGNAL_CHECK_INTERVAL) {
            Ok(true) if signals.received() => return Ok(None),
            Ok(true) => return event::read().map(Some),
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted && signals.received() => {
                return Ok(None);
            }
            Err(error) => return Err(error),
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
        self.keycast = keycast_text(key);

        if self.dirty_quit_prompt {
            return self.handle_dirty_quit_key(key);
        }

        if self.command_line.is_some() {
            return self.handle_command_line_key(key, buffer, view);
        }

        match keymap.resolve(key) {
            KeymapResult::Command(commands::Command::OpenCommandLine) => self.start_command_line(),
            KeymapResult::Command(commands::Command::SetMark) => self.set_mark(view),
            KeymapResult::Command(commands::Command::KillRegion) => self.kill_region(buffer, view),
            KeymapResult::Command(commands::Command::KillLine) => self.kill_line(buffer, view),
            KeymapResult::Command(commands::Command::Yank) => self.yank(buffer, view),
            KeymapResult::Command(commands::Command::RepeatSearch) => {
                self.repeat_search(buffer, view)
            }
            KeymapResult::Command(commands::Command::OpenFile) => self.start_find_file(),
            KeymapResult::Command(commands::Command::SwitchBuffer) => self.start_switch_buffer(),
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

    fn start_command_line(&mut self) -> AppAction {
        self.command_line = Some("/".to_string());
        self.prompt_kind = Some(PromptKind::Command);
        self.clear_status();
        AppAction::Continue
    }

    fn start_find_file(&mut self) -> AppAction {
        self.command_line = Some(String::new());
        self.prompt_kind = Some(PromptKind::FindFile);
        self.clear_status();
        AppAction::Continue
    }

    fn start_switch_buffer(&mut self) -> AppAction {
        self.command_line = Some(String::new());
        self.prompt_kind = Some(PromptKind::SwitchBuffer);
        self.clear_status();
        AppAction::Continue
    }

    fn active_region(&self, buffer: &Buffer, view: &View) -> Option<Range<usize>> {
        let len_chars = buffer.len_chars();
        let mark = buffer.grapheme_boundary_at_or_before(self.mark?.min(len_chars));
        let point = buffer.grapheme_boundary_at_or_before(view.point().min(len_chars));

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
        let point_after = buffer.delete_with_points(region.clone(), view.point(), region.start);
        view.set_point(point_after, buffer);
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
        let point_after = buffer.delete_with_points(region, point, point);
        view.set_point(point_after, buffer);
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
        let point_after = buffer.insert(point, &text);
        view.set_point(point_after, buffer);
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
                    text::pop_grapheme(input);
                }
                AppAction::Continue
            }
            crate::input::Key::Enter => {
                let input = self.command_line.take().unwrap_or_default();
                match self.prompt_kind.take().unwrap_or(PromptKind::Command) {
                    PromptKind::Command => self.run_command_line(&input, buffer, view),
                    PromptKind::FindFile => self.submit_find_file(&input, buffer.path()),
                    PromptKind::SwitchBuffer => self.submit_switch_buffer(&input),
                }
            }
            crate::input::Key::Escape => {
                self.command_line = None;
                let message = match self.prompt_kind.take().unwrap_or(PromptKind::Command) {
                    PromptKind::Command => "Command canceled",
                    PromptKind::FindFile => "Find file canceled",
                    PromptKind::SwitchBuffer => "Switch buffer canceled",
                };
                self.set_status(message, StatusKind::Info);
                AppAction::Continue
            }
            _ => AppAction::Continue,
        }
    }

    fn submit_find_file(&mut self, input: &str, active_path: &Path) -> AppAction {
        if input.trim().is_empty() {
            self.set_status("Find file requires a path", StatusKind::Error);
            return AppAction::Continue;
        }

        let path = PathBuf::from(input);
        if path.is_absolute() {
            AppAction::FindFile(path)
        } else {
            let directory = active_path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            AppAction::FindFile(directory.join(path))
        }
    }

    fn submit_switch_buffer(&mut self, input: &str) -> AppAction {
        if input.trim().is_empty() {
            self.set_status("Switch buffer requires a name", StatusKind::Error);
            return AppAction::Continue;
        }

        AppAction::SwitchBuffer(input.to_string())
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
            "reload" => self.dispatch_command(commands::Command::ReloadBuffer, buffer, view),
            "undo" => self.dispatch_command(commands::Command::Undo, buffer, view),
            "redo" => self.dispatch_command(commands::Command::Redo, buffer, view),
            "quit" => self.dispatch_command(commands::Command::Quit, buffer, view),
            "quit!" => AppAction::ForceQuit,
            command if command == "search" || command.starts_with("search ") => {
                self.run_search_command(command, buffer, view)
            }
            "next" => self.repeat_search(buffer, view),
            command if command == "open" || command.starts_with("open ") => {
                self.run_open_command(command)
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

        let start = buffer.next_grapheme_boundary(view.point());
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

    fn run_open_command(&mut self, command: &str) -> AppAction {
        let path_text = command
            .strip_prefix("open")
            .map(str::trim)
            .unwrap_or_default();

        if path_text.is_empty() {
            self.set_status("Usage: /open <path>", StatusKind::Error);
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
            Ok(false) => AppAction::OpenFile(path),
            Err(error) => {
                self.set_status(format!("Open failed: {error}"), StatusKind::Error);
                AppAction::Continue
            }
        }
    }

    fn handle_dirty_quit_key(&mut self, key: crate::input::Key) -> AppAction {
        match key {
            crate::input::Key::Char('y') => AppAction::ForceQuit,
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

    fn request_dirty_quit(&mut self) {
        self.dirty_quit_prompt = true;
        self.set_status(DIRTY_QUIT_PROMPT, StatusKind::Prompt);
    }

    fn dispatch_command(
        &mut self,
        command: commands::Command,
        buffer: &mut Buffer,
        view: &mut View,
    ) -> AppAction {
        let clear_mark = command_clears_mark(command);
        let outcome = commands::dispatch(command, buffer, view);

        if clear_mark && !outcome.failed {
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

        self.status_kind = outcome.status_message.as_ref().map(|_| {
            if outcome.failed {
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

    fn prompt_text(&self) -> Option<String> {
        let input = self.command_line.as_ref()?;
        Some(match self.prompt_kind.unwrap_or(PromptKind::Command) {
            PromptKind::Command => input.clone(),
            PromptKind::FindFile => format!("Find file: {input}"),
            PromptKind::SwitchBuffer => format!("Switch buffer: {input}"),
        })
    }

    fn disk_check_due(&mut self, now: Instant) -> bool {
        let due = self.last_disk_check.is_none_or(|last_check| {
            now.checked_duration_since(last_check)
                .is_some_and(|elapsed| elapsed >= DISK_CHECK_INTERVAL)
        });
        if due {
            self.last_disk_check = Some(now);
        }
        due
    }
}

fn render_editor<W: io::Write>(
    renderer: &Renderer,
    writer: &mut W,
    editor: &mut Editor,
    app_state: &mut AppState,
) -> io::Result<()> {
    let (buffer, view) = editor.active_mut();
    if app_state.disk_check_due(Instant::now()) {
        buffer.refresh_disk_changed();
    }
    render(renderer, writer, buffer, view, app_state)
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
    let prompt_text = app_state.prompt_text();
    view.ensure_point_visible(
        buffer,
        renderer.viewport_height(size),
        renderer.viewport_width(buffer, size),
    );
    renderer.render(
        writer,
        buffer,
        view,
        size,
        app_state.status_message.as_deref(),
        app_state.status_kind,
        app_state.active_region(buffer, view),
        prompt_text.as_deref(),
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
        crate::input::Key::Meta(ch) => Some(format!("M-{ch}")),
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
        Some(point..buffer.next_grapheme_boundary(point))
    }
}

fn command_clears_mark(command: commands::Command) -> bool {
    matches!(
        command,
        commands::Command::Insert(_)
            | commands::Command::InsertNewline
            | commands::Command::DeleteBackward
            | commands::Command::DeleteForward
            | commands::Command::ReloadBuffer
            | commands::Command::Undo
            | commands::Command::Redo
    )
}

#[cfg(test)]
mod tests {
    use super::{
        apply_app_action, AppAction, AppControl, AppState, COMMAND_HELP, DIRTY_QUIT_PROMPT,
        DISK_CHECK_INTERVAL,
    };
    use crate::{
        buffer::Buffer, editor::Editor, input::Key, keymap::Keymap, renderer::StatusKind,
        view::View,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
        time::{Instant, SystemTime, UNIX_EPOCH},
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
    fn reload_key_sequence_reloads_external_changes() {
        let dir = test_dir("reload-key");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();
        app.mark = Some(1);
        fs::write(&path, "after").unwrap();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('r'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(buffer.text(), "after");
        assert_eq!(
            app.status_message,
            Some(format!("Reloaded {}", path.display()))
        );
        assert_eq!(app.status_kind, Some(StatusKind::Success));
        assert_eq!(app.mark, None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_reload_preserves_the_active_mark() {
        let dir = test_dir("reload-mark");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut app = AppState {
            mark: Some(1),
            ..AppState::default()
        };
        let mut keymap = Keymap::new();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();
        buffer.insert(0, "local ");
        fs::write(&path, "external").unwrap();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('r'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(app.mark, Some(1));
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Reload refused")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn disk_change_checks_are_rate_limited() {
        let start = Instant::now();
        let mut app = AppState::default();

        assert!(app.disk_check_due(start));
        assert!(!app.disk_check_due(start + DISK_CHECK_INTERVAL / 2));
        assert!(app.disk_check_due(start + DISK_CHECK_INTERVAL));
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

        assert_eq!(action, AppAction::ForceQuit);
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
    fn slash_inserts_at_point_without_starting_command_line() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "ac");
        let mut view = View::new();
        view.move_forward_char(&buffer);

        let action = app.handle_key(Key::Char('/'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line, None);
        assert_eq!(buffer.text(), "a/c");
        assert_eq!(view.point(), 2);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn meta_x_starts_command_line_without_editing_the_buffer() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        let action = app.handle_key(Key::Meta('x'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line.as_deref(), Some("/"));
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn command_line_backspace_removes_one_complete_grapheme() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Meta('x'), &mut keymap, &mut buffer, &mut view);
        for ch in "e\u{301}👨‍💻".chars() {
            app.handle_key(Key::Char(ch), &mut keymap, &mut buffer, &mut view);
        }

        app.handle_key(Key::Backspace, &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.command_line.as_deref(), Some("/e\u{301}"));
        app.handle_key(Key::Backspace, &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.command_line.as_deref(), Some("/"));
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
    fn selection_cut_yank_and_undo_keep_graphemes_whole() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "a👨‍💻🇺🇸b");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl(' '), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(app.active_region(&buffer, &view), Some(1..6));

        app.handle_key(Key::Ctrl('w'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ab");
        assert_eq!(app.kill_ring.as_deref(), Some("👨‍💻🇺🇸"));
        assert_eq!(view.point(), 1);

        app.handle_key(Key::Ctrl('y'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "a👨‍💻🇺🇸b");
        assert_eq!(view.point(), 6);

        app.handle_key(Key::Command('z'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ab");
        assert_eq!(view.point(), 1);
    }

    #[test]
    fn cut_and_history_keep_point_after_graphemes_merged_by_the_edit() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "🇦x🇧");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl(' '), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('w'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "🇦🇧");
        assert_eq!(view.point(), 2);

        app.dispatch_command(crate::commands::Command::Undo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "🇦x🇧");
        assert_eq!(view.point(), 2);
        app.dispatch_command(crate::commands::Command::Redo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "🇦🇧");
        assert_eq!(view.point(), 2);

        let mut buffer = buffer_with_text("notes.txt", "e\n\u{301}x");
        let mut view = View::new();
        view.move_forward_char(&buffer);
        app.handle_key(Key::Ctrl('k'), &mut keymap, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "e\u{301}x");
        assert_eq!(view.point(), 2);
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
    fn ctrl_x_ctrl_f_opens_an_editable_find_file_prompt() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line.as_deref(), Some(""));
        assert_eq!(app.prompt_text().as_deref(), Some("Find file: "));
        assert_eq!(app.status_message, None);
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn find_file_accepts_a_missing_path_while_current_buffer_is_dirty() {
        let dir = test_dir("find-file-missing");
        let target = dir.join("new.txt");
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        for ch in target.to_string_lossy().chars() {
            app.handle_key(Key::Char(ch), &mut keymap, &mut buffer, &mut view);
        }
        let action = app.handle_key(Key::Enter, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::FindFile(target));
        assert_eq!(buffer.text(), "xold");
        assert!(buffer.is_dirty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn escape_cancels_find_file_without_changing_the_buffer() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Ctrl('f'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Char('x'), &mut keymap, &mut buffer, &mut view);
        let action = app.handle_key(Key::Escape, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::Continue);
        assert_eq!(app.command_line, None);
        assert_eq!(app.status_message.as_deref(), Some("Find file canceled"));
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn find_file_preserves_whitespace_in_the_requested_path() {
        let mut app = AppState::default();

        assert_eq!(
            app.submit_find_file(
                " leading and trailing.txt ",
                Path::new("/tmp/project/current.txt")
            ),
            AppAction::FindFile(PathBuf::from("/tmp/project/ leading and trailing.txt "))
        );
    }

    #[test]
    fn find_file_resolves_relative_paths_from_the_active_buffer_directory() {
        let mut app = AppState::default();

        assert_eq!(
            app.submit_find_file("sibling.rs", Path::new("/tmp/project/src/main.rs")),
            AppAction::FindFile(PathBuf::from("/tmp/project/src/sibling.rs"))
        );
        assert_eq!(
            app.submit_find_file("/tmp/absolute.rs", Path::new("/tmp/project/src/main.rs")),
            AppAction::FindFile(PathBuf::from("/tmp/absolute.rs"))
        );
    }

    #[test]
    fn find_file_routes_existing_directories_to_the_picker() {
        let dir = test_dir("find-file-directory");
        let first = dir.join("first.txt");
        fs::write(&first, "first").unwrap();
        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        let mut app = AppState::default();

        assert_eq!(
            apply_app_action(&mut editor, &mut app, AppAction::FindFile(dir.clone())),
            AppControl::BrowseDirectory(dir.clone())
        );
        assert_eq!(editor.active().0.path(), first);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ctrl_x_b_opens_an_editable_switch_buffer_prompt() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Ctrl('x'), &mut keymap, &mut buffer, &mut view);
        app.handle_key(Key::Char('b'), &mut keymap, &mut buffer, &mut view);
        for ch in "other.txt".chars() {
            app.handle_key(Key::Char(ch), &mut keymap, &mut buffer, &mut view);
        }
        let action = app.handle_key(Key::Enter, &mut keymap, &mut buffer, &mut view);

        assert_eq!(action, AppAction::SwitchBuffer("other.txt".to_string()));
        assert_eq!(buffer.text(), "old");
        assert!(!buffer.is_dirty());
    }

    #[test]
    fn opening_and_switching_buffers_keeps_unsaved_edits() {
        let dir = test_dir("app-buffer-list");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.active_mut().0.insert(0, "dirty ");
        let mut app = AppState::default();

        assert_eq!(
            apply_app_action(&mut editor, &mut app, AppAction::OpenFile(second.clone())),
            AppControl::Continue
        );
        assert_eq!(editor.active().0.path(), second);
        assert_eq!(
            apply_app_action(
                &mut editor,
                &mut app,
                AppAction::SwitchBuffer("first.txt".to_string())
            ),
            AppControl::Continue
        );
        assert_eq!(editor.active().0.text(), "dirty first");
        assert!(editor.active().0.is_dirty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn quit_warns_when_only_an_inactive_buffer_is_dirty() {
        let dir = test_dir("inactive-dirty-quit");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.active_mut().0.insert(0, "dirty ");
        editor.open(&second).unwrap();
        let mut app = AppState::default();

        assert_eq!(
            apply_app_action(&mut editor, &mut app, AppAction::Quit),
            AppControl::Continue
        );
        assert!(app.dirty_quit_prompt);
        assert_eq!(app.status_message.as_deref(), Some(DIRTY_QUIT_PROMPT));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn escape_cancels_command_line_without_editing_the_buffer() {
        let mut app = AppState::default();
        let mut keymap = Keymap::new();
        let mut buffer = buffer_with_text("notes.txt", "old");
        let mut view = View::new();

        app.handle_key(Key::Meta('x'), &mut keymap, &mut buffer, &mut view);
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

        assert_eq!(action, AppAction::ForceQuit);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn slash_open_requests_another_buffer_without_replacing_the_current_one() {
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

        assert_eq!(action, AppAction::OpenFile(target_path));
        assert_eq!(buffer.path(), current_path.as_path());
        assert_eq!(buffer.text(), "current");
        assert_eq!(view.point(), 1);
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
    fn slash_open_allows_another_buffer_when_current_buffer_is_dirty() {
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

        assert_eq!(action, AppAction::OpenFile(target_path));
        assert_eq!(buffer.path(), current_path.as_path());
        assert_eq!(buffer.text(), "xcurrent");
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
        app.handle_key(Key::Meta('x'), keymap, buffer, view);
        for ch in command.strip_prefix('/').unwrap_or(command).chars() {
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
