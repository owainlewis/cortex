use crate::{
    buffer::Buffer,
    picker::{DirectoryEntry, DirectoryEntryKind, DirectoryPicker},
    view::View,
};
use crossterm::{
    cursor, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Default)]
pub struct Renderer;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    lines: Vec<String>,
    modeline: String,
    cursor: CursorPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorPosition {
    col: u16,
    row: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PickerFrame {
    lines: Vec<String>,
    modeline: String,
    cursor: CursorPosition,
}

impl Renderer {
    pub fn new() -> Self {
        Self
    }

    pub fn viewport_height(&self, size: TerminalSize) -> usize {
        size.rows.saturating_sub(1) as usize
    }

    pub fn render<W: Write>(
        &self,
        writer: &mut W,
        buffer: &Buffer,
        view: &View,
        size: TerminalSize,
        status_message: Option<&str>,
        command_line: Option<&str>,
    ) -> io::Result<()> {
        let frame = build_frame(buffer, view, size, status_message, command_line);

        queue!(
            writer,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        if size.cols == 0 || size.rows == 0 {
            return writer.flush();
        }

        for (row, line) in frame.lines.iter().enumerate() {
            queue!(writer, cursor::MoveTo(0, row as u16), Print(line))?;
        }

        let modeline_row = size.rows.saturating_sub(1);
        queue!(
            writer,
            cursor::MoveTo(0, modeline_row),
            SetAttribute(Attribute::Reverse),
            Print(frame.modeline),
            SetAttribute(Attribute::Reset),
            cursor::MoveTo(frame.cursor.col, frame.cursor.row),
            cursor::Show
        )?;
        writer.flush()
    }

    pub fn render_directory_picker<W: Write>(
        &self,
        writer: &mut W,
        picker: &DirectoryPicker,
        size: TerminalSize,
    ) -> io::Result<()> {
        let frame = build_picker_frame(picker, size);

        queue!(
            writer,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        if size.cols == 0 || size.rows == 0 {
            return writer.flush();
        }

        for (row, line) in frame.lines.iter().enumerate() {
            queue!(writer, cursor::MoveTo(0, row as u16), Print(line))?;
        }

        let modeline_row = size.rows.saturating_sub(1);
        queue!(
            writer,
            cursor::MoveTo(0, modeline_row),
            SetAttribute(Attribute::Reverse),
            Print(frame.modeline),
            SetAttribute(Attribute::Reset),
            cursor::MoveTo(frame.cursor.col, frame.cursor.row),
            cursor::Show
        )?;
        writer.flush()
    }
}

fn build_frame(
    buffer: &Buffer,
    view: &View,
    size: TerminalSize,
    status_message: Option<&str>,
    command_line: Option<&str>,
) -> Frame {
    let width = size.cols as usize;
    let viewport_height = size.rows.saturating_sub(1) as usize;
    let mut lines = Vec::with_capacity(viewport_height);

    for screen_row in 0..viewport_height {
        let line_idx = view.scroll_line().saturating_add(screen_row);
        let line = if line_idx < buffer.len_lines() {
            fit_line_cells(&buffer.line_prefix_text(line_idx, width), width)
        } else {
            String::new()
        };
        lines.push(line);
    }

    let modeline = command_line
        .map(command_line_text)
        .unwrap_or_else(|| modeline_text(buffer, view, status_message));
    let cursor = command_line
        .map(|input| command_line_cursor(input, size))
        .unwrap_or_else(|| cursor_position(buffer, view, size));

    Frame {
        lines,
        modeline: fit_status_line(&modeline, width),
        cursor,
    }
}

fn command_line_text(input: &str) -> String {
    format!(" {input}")
}

fn command_line_cursor(input: &str, size: TerminalSize) -> CursorPosition {
    if size.cols == 0 || size.rows == 0 {
        return CursorPosition { col: 0, row: 0 };
    }

    let text_before_cursor = command_line_text(input);
    let col = measure_cells(&text_before_cursor, size.cols as usize)
        .min(size.cols.saturating_sub(1) as usize) as u16;

    CursorPosition {
        col,
        row: size.rows.saturating_sub(1),
    }
}

fn cursor_position(buffer: &Buffer, view: &View, size: TerminalSize) -> CursorPosition {
    if size.cols == 0 || size.rows == 0 {
        return CursorPosition { col: 0, row: 0 };
    }

    let point_line = buffer.line_for_char(view.point());
    let point_col = view.point() - buffer.line_start_char(point_line);
    let line_prefix = buffer.line_prefix_text(point_line, point_col.min(size.cols as usize));
    let point_cell_col = measure_cells(&line_prefix, size.cols as usize);
    let viewport_height = size.rows.saturating_sub(1) as usize;
    let row = point_line
        .saturating_sub(view.scroll_line())
        .min(viewport_height.saturating_sub(1)) as u16;
    let col = point_cell_col.min(size.cols.saturating_sub(1) as usize) as u16;

    CursorPosition { col, row }
}

fn modeline_text(buffer: &Buffer, view: &View, status_message: Option<&str>) -> String {
    let line_idx = buffer.line_for_char(view.point());
    let column = view.point() - buffer.line_start_char(line_idx);
    let dirty_state = if buffer.is_dirty() {
        "modified"
    } else {
        "clean"
    };
    let file_name = buffer
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| buffer.path().display().to_string(), ToOwned::to_owned);

    let mut text = format!(
        " {}  {}  Ln {}, Col {} ",
        file_name,
        dirty_state,
        line_idx + 1,
        column + 1
    );

    if let Some(message) = status_message.filter(|message| !message.is_empty()) {
        text.push_str(" | ");
        text.push_str(message);
        text.push(' ');
    }

    text
}

fn build_picker_frame(picker: &DirectoryPicker, size: TerminalSize) -> PickerFrame {
    let width = size.cols as usize;
    let viewport_height = size.rows.saturating_sub(1) as usize;
    let mut lines = Vec::with_capacity(viewport_height);

    if viewport_height > 0 {
        lines.push(fit_line_cells(
            &format!(" Open file in {}", picker.directory().display()),
            width,
        ));
    }

    if viewport_height > 1 {
        lines.push(String::new());
    }

    let entry_rows = viewport_height.saturating_sub(2);
    let first_entry = if entry_rows == 0 {
        0
    } else {
        picker
            .selected()
            .saturating_add(1)
            .saturating_sub(entry_rows)
    };

    if picker.entries().is_empty() && lines.len() < viewport_height {
        lines.push(fit_line_cells("  No visible files", width));
    } else {
        for entry in picker.entries().iter().skip(first_entry).take(entry_rows) {
            let selected = picker.selected_entry() == Some(entry);
            lines.push(fit_line_cells(&picker_entry_text(entry, selected), width));
        }
    }

    while lines.len() < viewport_height {
        lines.push(String::new());
    }

    let cursor_row = if picker.entries().is_empty() {
        0
    } else {
        2 + picker.selected().saturating_sub(first_entry)
    }
    .min(viewport_height.saturating_sub(1)) as u16;

    PickerFrame {
        lines,
        modeline: fit_status_line(&picker_modeline_text(picker), width),
        cursor: CursorPosition {
            col: 0,
            row: cursor_row,
        },
    }
}

fn picker_entry_text(entry: &DirectoryEntry, selected: bool) -> String {
    let marker = if selected { ">" } else { " " };
    let suffix = if entry.is_directory() { "/" } else { "" };
    let kind = match entry.kind() {
        DirectoryEntryKind::File => "file",
        DirectoryEntryKind::Directory => "dir ",
        DirectoryEntryKind::Other => "item",
    };

    format!("{marker} {kind} {}{suffix}", entry.name())
}

fn picker_modeline_text(picker: &DirectoryPicker) -> String {
    let mut text = " Enter open  C-n/C-p move  Esc/C-x C-c quit ".to_string();

    if let Some(message) = picker
        .status_message()
        .filter(|message| !message.is_empty())
    {
        text.push_str(" | ");
        text.push_str(message);
        text.push(' ');
    }

    text
}

fn fit_line_cells(line: &str, width: usize) -> String {
    let mut fitted = String::new();
    let mut cells = 0;

    for ch in line.chars() {
        if cells >= width {
            break;
        }

        if ch == '\t' {
            let spaces = tab_spaces(cells).min(width - cells);
            fitted.extend(std::iter::repeat(' ').take(spaces));
            cells += spaces;
            continue;
        }

        let char_width = char_cell_width(ch);
        if char_width == 0 {
            continue;
        }

        if cells + char_width > width {
            break;
        }

        fitted.push(if ch.is_control() { ' ' } else { ch });
        cells += char_width;
    }

    fitted
}

fn fit_status_line(line: &str, width: usize) -> String {
    let mut fitted = fit_line_cells(line, width);
    let remaining_width = width.saturating_sub(measure_cells(&fitted, width));
    fitted.extend(std::iter::repeat(' ').take(remaining_width));
    fitted
}

fn measure_cells(line: &str, max_width: usize) -> usize {
    let mut cells = 0;

    for ch in line.chars() {
        if cells >= max_width {
            break;
        }

        let char_width = if ch == '\t' {
            tab_spaces(cells)
        } else {
            char_cell_width(ch)
        };

        cells = cells.saturating_add(char_width).min(max_width);
    }

    cells
}

fn tab_spaces(current_col: usize) -> usize {
    TAB_WIDTH - (current_col % TAB_WIDTH)
}

fn char_cell_width(ch: char) -> usize {
    if is_zero_width(ch) {
        0
    } else if is_wide(ch) {
        2
    } else {
        1
    }
}

fn is_zero_width(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036F
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE20..=0xFE2F
    )
}

fn is_wide(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1FAFF
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_frame, build_picker_frame, fit_line_cells, fit_status_line, measure_cells,
        CursorPosition, TerminalSize,
    };
    use crate::{
        buffer::Buffer,
        picker::{DirectoryEntry, DirectoryEntryKind, DirectoryPicker},
        view::View,
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn fit_status_line_pads_to_terminal_width() {
        assert_eq!(fit_status_line("abc", 5), "abc  ");
        assert_eq!(fit_status_line("abcdef", 3), "abc");
        assert_eq!(fit_status_line("界", 4), "界  ");
    }

    #[test]
    fn fit_line_cells_expands_tabs_and_respects_wide_characters() {
        assert_eq!(fit_line_cells("\tab", 8), "    ab");
        assert_eq!(fit_line_cells("界界界", 5), "界界");
        assert_eq!(measure_cells("\tab", 8), 6);
        assert_eq!(measure_cells("界界界", 5), 5);
    }

    #[test]
    fn frame_contains_visible_file_lines_modeline_and_cursor() {
        let buffer = buffer_with_text("notes.txt", "alpha\nbeta\n");
        let view = View::new();

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
        );

        assert_eq!(frame.lines, vec!["alpha", "beta"]);
        assert!(frame.modeline.contains("notes.txt"));
        assert!(frame.modeline.contains("clean"));
        assert!(frame.modeline.contains("Ln 1, Col 1"));
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 0 });
    }

    #[test]
    fn frame_modeline_shows_dirty_state() {
        let mut buffer = buffer_with_text("notes.txt", "alpha\n");

        buffer.insert(0, "z");
        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
        );

        assert!(frame.modeline.contains("modified"));
    }

    #[test]
    fn frame_uses_scroll_line_to_keep_point_visible() {
        let buffer = buffer_with_text("notes.txt", "one\ntwo\nthree\nfour\n");
        let mut view = View::new();

        view.move_next_line(&buffer);
        view.move_next_line(&buffer);
        view.ensure_point_visible(&buffer, 2);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
        );

        assert_eq!(frame.lines, vec!["two", "three"]);
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 1 });
        assert!(frame.modeline.contains("Ln 3, Col 1"));
    }

    #[test]
    fn frame_truncates_long_lines_and_modeline_to_width() {
        let mut buffer = buffer_with_text("notes.txt", "abcdef");
        buffer.insert(0, "z");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 4, rows: 2 },
            None,
            None,
        );

        assert_eq!(frame.lines, vec!["zabc"]);
        assert_eq!(frame.modeline.chars().count(), 4);
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 0 });
    }

    #[test]
    fn frame_cursor_uses_terminal_cell_width() {
        let buffer = buffer_with_text("notes.txt", "\tab\n");
        let mut view = View::new();

        view.move_forward_char(&buffer);
        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 10, rows: 3 },
            None,
            None,
        );

        assert_eq!(frame.lines, vec!["    ab", ""]);
        assert_eq!(frame.cursor, CursorPosition { col: 4, row: 0 });
    }

    #[test]
    fn frame_handles_tiny_terminal_sizes() {
        let buffer = buffer_with_text("notes.txt", "abcdef");
        let view = View::new();

        let zero = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 0, rows: 0 },
            None,
            None,
        );
        let modeline_only = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 8, rows: 1 },
            None,
            None,
        );

        assert!(zero.lines.is_empty());
        assert_eq!(zero.modeline, "");
        assert_eq!(zero.cursor, CursorPosition { col: 0, row: 0 });
        assert!(modeline_only.lines.is_empty());
        assert!(modeline_only.modeline.chars().count() <= 8);
    }

    #[test]
    fn frame_modeline_shows_status_message() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 80, rows: 3 },
            Some("Save failed: parent directory does not exist"),
            None,
        );

        assert!(frame.modeline.contains("notes.txt"));
        assert!(frame.modeline.contains("Save failed"));
    }

    #[test]
    fn frame_modeline_shows_active_command_line_and_moves_cursor_to_it() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 80, rows: 3 },
            Some("old status"),
            Some("/save"),
        );

        assert!(frame.modeline.starts_with(" /save"));
        assert!(!frame.modeline.contains("old status"));
        assert_eq!(frame.cursor, CursorPosition { col: 6, row: 2 });
    }

    #[test]
    fn picker_frame_marks_selection_and_labels_entry_kinds() {
        let picker = DirectoryPicker::new(
            PathBuf::from("/tmp/project"),
            vec![
                DirectoryEntry::new(
                    "src".to_string(),
                    PathBuf::from("/tmp/project/src"),
                    DirectoryEntryKind::Directory,
                ),
                DirectoryEntry::new(
                    "main.rs".to_string(),
                    PathBuf::from("/tmp/project/main.rs"),
                    DirectoryEntryKind::File,
                ),
            ],
        );

        let frame = build_picker_frame(&picker, TerminalSize { cols: 80, rows: 6 });

        assert!(frame.lines[0].contains("/tmp/project"));
        assert_eq!(frame.lines[2], "> dir  src/");
        assert_eq!(frame.lines[3], "  file main.rs");
        assert!(frame.modeline.contains("Enter open"));
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 2 });
    }

    #[test]
    fn picker_frame_keeps_selected_entry_visible() {
        let mut picker = DirectoryPicker::new(
            PathBuf::from("/tmp/project"),
            vec![
                picker_entry("a.txt"),
                picker_entry("b.txt"),
                picker_entry("c.txt"),
                picker_entry("d.txt"),
            ],
        );

        picker.handle_key(crate::input::Key::Down);
        picker.handle_key(crate::input::Key::Down);
        picker.handle_key(crate::input::Key::Down);
        let frame = build_picker_frame(&picker, TerminalSize { cols: 80, rows: 5 });

        assert_eq!(frame.lines[2], "  file c.txt");
        assert_eq!(frame.lines[3], "> file d.txt");
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 3 });
    }

    fn buffer_with_text(file_name: &str, text: &str) -> Buffer {
        let dir = test_dir("renderer");
        let path = dir.join(file_name);
        fs::write(&path, text).unwrap();
        let buffer = Buffer::open(&path).unwrap();
        fs::remove_dir_all(dir).unwrap();
        buffer
    }

    fn picker_entry(name: &str) -> DirectoryEntry {
        DirectoryEntry::new(
            name.to_string(),
            PathBuf::from("/tmp/project").join(name),
            DirectoryEntryKind::File,
        )
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cortex-renderer-test-{}-{name}-{unique}-{counter}",
            std::process::id(),
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
}
