use crate::{
    buffer::Buffer,
    highlighter::{language_label_for_path, HighlightKind, HighlightSpan, SyntaxHighlighter},
    picker::{DirectoryEntryKind, DirectoryPicker, DirectoryPickerRow},
    text,
    view::View,
};
use crossterm::{
    cursor, queue,
    style::{
        force_color_output, Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{self, ClearType},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    io::{self, Write},
    ops::Range,
};
use unicode_segmentation::UnicodeSegmentation;

const EMPTY_SPACE_MARKER: &str = " ~";
const MODELINE_BRAND: &str = "CORTEX";
const MIN_GUTTER_TOTAL_WIDTH: usize = 12;
const MAX_PICKER_INDENT_DEPTH: usize = 4;
const COMMAND_LINE_HINT: &str =
    "/help  /commands  /open <path>  /search <text>  /next  /reload  /save  /undo  /redo  /quit  /quit!";

const THEME: Theme = Theme {
    editor_fg: Color::Rgb {
        r: 205,
        g: 214,
        b: 244,
    },
    empty_fg: Color::Rgb {
        r: 69,
        g: 71,
        b: 90,
    },
    gutter_fg: Color::Rgb {
        r: 127,
        g: 132,
        b: 156,
    },
    gutter_current_fg: Color::Rgb {
        r: 203,
        g: 166,
        b: 247,
    },
    modeline_fg: Color::Rgb {
        r: 205,
        g: 214,
        b: 244,
    },
    modeline_bg: Color::Rgb {
        r: 30,
        g: 30,
        b: 46,
    },
    modeline_brand_fg: Color::Rgb {
        r: 17,
        g: 17,
        b: 27,
    },
    modeline_brand_bg: Color::Rgb {
        r: 203,
        g: 166,
        b: 247,
    },
    dirty_fg: Color::Rgb {
        r: 250,
        g: 179,
        b: 135,
    },
    success_fg: Color::Rgb {
        r: 166,
        g: 227,
        b: 161,
    },
    error_fg: Color::Rgb {
        r: 243,
        g: 139,
        b: 168,
    },
    prefix_fg: Color::Rgb {
        r: 137,
        g: 180,
        b: 250,
    },
    prompt_fg: Color::Rgb {
        r: 249,
        g: 226,
        b: 175,
    },
    command_fg: Color::Rgb {
        r: 205,
        g: 214,
        b: 244,
    },
    command_bg: Color::Rgb {
        r: 49,
        g: 50,
        b: 68,
    },
    selection_bg: Color::Rgb {
        r: 69,
        g: 71,
        b: 90,
    },
    picker_fg: Color::Rgb {
        r: 205,
        g: 214,
        b: 244,
    },
    picker_selected_fg: Color::Rgb {
        r: 245,
        g: 224,
        b: 220,
    },
    picker_selected_bg: Color::Rgb {
        r: 49,
        g: 50,
        b: 68,
    },
    syntax_attribute: Color::Rgb {
        r: 250,
        g: 179,
        b: 135,
    },
    syntax_boolean: Color::Rgb {
        r: 243,
        g: 139,
        b: 168,
    },
    syntax_comment: Color::Rgb {
        r: 127,
        g: 132,
        b: 156,
    },
    syntax_constant: Color::Rgb {
        r: 243,
        g: 139,
        b: 168,
    },
    syntax_constructor: Color::Rgb {
        r: 148,
        g: 226,
        b: 213,
    },
    syntax_function: Color::Rgb {
        r: 249,
        g: 226,
        b: 175,
    },
    syntax_keyword: Color::Rgb {
        r: 137,
        g: 180,
        b: 250,
    },
    syntax_markup: Color::Rgb {
        r: 203,
        g: 166,
        b: 247,
    },
    markdown_heading: Color::Rgb {
        r: 250,
        g: 179,
        b: 135,
    },
    markdown_link: Color::Rgb {
        r: 137,
        g: 180,
        b: 250,
    },
    markdown_uri: Color::Rgb {
        r: 166,
        g: 227,
        b: 161,
    },
    markdown_code: Color::Rgb {
        r: 250,
        g: 179,
        b: 135,
    },
    markdown_marker: Color::Rgb {
        r: 127,
        g: 132,
        b: 156,
    },
    markdown_quote: Color::Rgb {
        r: 203,
        g: 166,
        b: 247,
    },
    syntax_number: Color::Rgb {
        r: 250,
        g: 179,
        b: 135,
    },
    syntax_operator: Color::Rgb {
        r: 186,
        g: 194,
        b: 222,
    },
    syntax_property: Color::Rgb {
        r: 116,
        g: 199,
        b: 236,
    },
    syntax_punctuation: Color::Rgb {
        r: 147,
        g: 153,
        b: 178,
    },
    syntax_string: Color::Rgb {
        r: 166,
        g: 227,
        b: 161,
    },
    syntax_tag: Color::Rgb {
        r: 137,
        g: 180,
        b: 250,
    },
    syntax_type: Color::Rgb {
        r: 148,
        g: 226,
        b: 213,
    },
    syntax_variable: Color::Rgb {
        r: 205,
        g: 214,
        b: 244,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Success,
    Error,
    Prefix,
    Prompt,
}

pub struct Renderer {
    highlighter: RefCell<SyntaxHighlighter>,
    last_frame: RefCell<Option<CellFrame>>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            highlighter: RefCell::new(SyntaxHighlighter::default()),
            last_frame: RefCell::new(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Frame {
    lines: Vec<ScreenLine>,
    modeline: String,
    cursor: CursorPosition,
    modeline_style: ModelineStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CursorPosition {
    col: u16,
    row: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PickerFrame {
    lines: Vec<PickerLine>,
    modeline: String,
    cursor: CursorPosition,
    status_kind: Option<StatusKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CellFrame {
    size: TerminalSize,
    cells: Vec<Cell>,
    cursor: CursorPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    content: CellContent,
    style: TextStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CellContent {
    Text(char),
    Grapheme(String),
    Continuation,
}

impl CellContent {
    fn append_to(&self, text: &mut String) -> bool {
        match self {
            Self::Text(ch) => text.push(*ch),
            Self::Grapheme(grapheme) => text.push_str(grapheme),
            Self::Continuation => return false,
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenLine {
    text: String,
    kind: ScreenLineKind,
    gutter: String,
    current: bool,
    segments: Vec<StyledSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StyledSegment {
    text: String,
    highlight: Option<HighlightKind>,
    selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenLineKind {
    Text,
    EmptySpace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PickerLine {
    text: String,
    selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelineStyle {
    Clean,
    Dirty,
    Info,
    Success,
    Error,
    Prefix,
    Prompt,
    CommandLine,
}

#[derive(Debug, Clone, Copy)]
struct Theme {
    editor_fg: Color,
    empty_fg: Color,
    gutter_fg: Color,
    gutter_current_fg: Color,
    modeline_fg: Color,
    modeline_bg: Color,
    modeline_brand_fg: Color,
    modeline_brand_bg: Color,
    dirty_fg: Color,
    success_fg: Color,
    error_fg: Color,
    prefix_fg: Color,
    prompt_fg: Color,
    command_fg: Color,
    command_bg: Color,
    selection_bg: Color,
    picker_fg: Color,
    picker_selected_fg: Color,
    picker_selected_bg: Color,
    syntax_attribute: Color,
    syntax_boolean: Color,
    syntax_comment: Color,
    syntax_constant: Color,
    syntax_constructor: Color,
    syntax_function: Color,
    syntax_keyword: Color,
    syntax_markup: Color,
    markdown_heading: Color,
    markdown_link: Color,
    markdown_uri: Color,
    markdown_code: Color,
    markdown_marker: Color,
    markdown_quote: Color,
    syntax_number: Color,
    syntax_operator: Color,
    syntax_property: Color,
    syntax_punctuation: Color,
    syntax_string: Color,
    syntax_tag: Color,
    syntax_type: Color,
    syntax_variable: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextStyle {
    foreground: Color,
    background: Option<Color>,
    bold: bool,
    italic: bool,
    underlined: bool,
    dim: bool,
}

impl Renderer {
    pub fn new() -> Self {
        force_color_output(true);
        Self::default()
    }

    pub fn viewport_height(&self, size: TerminalSize) -> usize {
        size.rows.saturating_sub(1) as usize
    }

    pub fn viewport_width(&self, buffer: &Buffer, size: TerminalSize) -> usize {
        (size.cols as usize).saturating_sub(editor_gutter_width(buffer, size.cols as usize))
    }

    pub(crate) fn invalidate(&self) {
        self.last_frame.borrow_mut().take();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render<W: Write>(
        &self,
        writer: &mut W,
        buffer: &Buffer,
        view: &View,
        size: TerminalSize,
        status_message: Option<&str>,
        status_kind: Option<StatusKind>,
        selection_range: Option<Range<usize>>,
        command_line: Option<&str>,
        keycast: Option<&str>,
    ) -> io::Result<()> {
        let viewport_height = self.viewport_height(size);
        let first_visible_line = view.scroll_line();
        let highlighted_lines = self.highlighter.borrow_mut().highlight_visible_lines(
            buffer,
            first_visible_line..first_visible_line.saturating_add(viewport_height),
        );
        let frame = build_frame_with_highlights(
            buffer,
            view,
            size,
            status_message,
            status_kind,
            selection_range,
            command_line,
            keycast,
            &highlighted_lines,
        );

        self.paint(writer, paint_editor_frame(frame, size))
    }

    pub fn render_directory_picker<W: Write>(
        &self,
        writer: &mut W,
        picker: &DirectoryPicker,
        size: TerminalSize,
    ) -> io::Result<()> {
        let frame = build_picker_frame(picker, size);

        self.paint(writer, paint_picker_frame(frame, size))
    }

    fn paint<W: Write>(&self, writer: &mut W, frame: CellFrame) -> io::Result<()> {
        queue!(writer, cursor::Hide)?;

        let full_redraw = self
            .last_frame
            .borrow()
            .as_ref()
            .is_none_or(|previous| previous.size != frame.size);
        if full_redraw {
            queue!(writer, terminal::Clear(ClearType::All))?;
        }

        if frame.size.cols > 0 && frame.size.rows > 0 {
            let width = frame.size.cols as usize;
            let previous = self.last_frame.borrow();
            let cell_changed = |index: usize| {
                full_redraw
                    || previous
                        .as_ref()
                        .is_none_or(|previous| previous.cells[index] != frame.cells[index])
            };
            let mut index = 0;
            while index < frame.cells.len() {
                if !cell_changed(index) {
                    index += 1;
                    continue;
                }
                let cell = &frame.cells[index];
                let mut run = String::new();
                if !cell.content.append_to(&mut run) {
                    index += 1;
                    continue;
                }
                let row = (index / width) as u16;
                let col = (index % width) as u16;
                let row_end = index - (index % width) + width;
                let mut next = index + 1;
                while next < row_end && cell_changed(next) && frame.cells[next].style == cell.style
                {
                    frame.cells[next].content.append_to(&mut run);
                    next += 1;
                }
                queue!(
                    writer,
                    cursor::MoveTo(col, row),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                )?;
                apply_text_style(writer, cell.style)?;
                queue!(writer, Print(run))?;
                index = next;
            }
        }

        queue!(
            writer,
            SetAttribute(Attribute::Reset),
            ResetColor,
            cursor::MoveTo(frame.cursor.col, frame.cursor.row),
            cursor::Show
        )?;
        writer.flush()?;
        self.last_frame.replace(Some(frame));
        Ok(())
    }
}

#[cfg(test)]
fn build_frame(
    buffer: &Buffer,
    view: &View,
    size: TerminalSize,
    status_message: Option<&str>,
    status_kind: Option<StatusKind>,
    command_line: Option<&str>,
) -> Frame {
    build_frame_with_selection(
        buffer,
        view,
        size,
        status_message,
        status_kind,
        None,
        command_line,
    )
}

#[cfg(test)]
fn build_frame_with_selection(
    buffer: &Buffer,
    view: &View,
    size: TerminalSize,
    status_message: Option<&str>,
    status_kind: Option<StatusKind>,
    selection_range: Option<Range<usize>>,
    command_line: Option<&str>,
) -> Frame {
    build_frame_with_highlights(
        buffer,
        view,
        size,
        status_message,
        status_kind,
        selection_range,
        command_line,
        None,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
fn build_frame_with_highlights(
    buffer: &Buffer,
    view: &View,
    size: TerminalSize,
    status_message: Option<&str>,
    status_kind: Option<StatusKind>,
    selection_range: Option<Range<usize>>,
    command_line: Option<&str>,
    keycast: Option<&str>,
    highlighted_lines: &[Vec<HighlightSpan>],
) -> Frame {
    let width = size.cols as usize;
    let viewport_height = size.rows.saturating_sub(1) as usize;
    let gutter_width = editor_gutter_width(buffer, width);
    let text_width = width.saturating_sub(gutter_width);
    let point_line = buffer.line_for_char(view.point());
    let mut lines = Vec::with_capacity(viewport_height);

    for screen_row in 0..viewport_height {
        let line_idx = view.scroll_line().saturating_add(screen_row);
        let screen_line = if line_idx < buffer.len_lines() {
            let window = buffer.line_window(line_idx, view.scroll_column(), text_width);
            let left_padding = window
                .start_column
                .saturating_sub(view.scroll_column())
                .min(text_width);
            let spans = highlighted_lines
                .get(screen_row)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut segments = Vec::new();
            push_segment(&mut segments, &" ".repeat(left_padding), None, false);
            for segment in fit_line_segments(
                &window.text,
                spans,
                text_width.saturating_sub(left_padding),
                window.start_char,
                window.start_byte,
                window.start_column,
                selection_range.as_ref(),
            ) {
                push_segment(
                    &mut segments,
                    &segment.text,
                    segment.highlight,
                    segment.selected,
                );
            }
            let text = segments_text(&segments);
            ScreenLine {
                text,
                kind: ScreenLineKind::Text,
                gutter: line_gutter(
                    line_idx,
                    point_line,
                    buffer.line_changed(line_idx),
                    gutter_width,
                ),
                current: line_idx == point_line,
                segments,
            }
        } else {
            let text = empty_space_line(text_width);
            ScreenLine {
                segments: vec![StyledSegment {
                    text: text.clone(),
                    highlight: None,
                    selected: false,
                }],
                text,
                kind: ScreenLineKind::EmptySpace,
                gutter: empty_gutter(gutter_width),
                current: false,
            }
        };
        lines.push(screen_line);
    }

    let mut modeline = command_line
        .map(command_line_text)
        .unwrap_or_else(|| modeline_text(buffer, view, status_message));
    if command_line.is_none() {
        if let Some(keycast) = keycast {
            modeline = modeline_with_keycast(&modeline, keycast);
        }
    }
    let cursor = command_line
        .map(|input| command_line_cursor(input, size))
        .unwrap_or_else(|| cursor_position(buffer, view, size, gutter_width));
    let modeline_style = if command_line.is_some() {
        ModelineStyle::CommandLine
    } else if let Some(status_kind) = status_kind {
        modeline_style_for_status(status_kind)
    } else if buffer.is_dirty() {
        ModelineStyle::Dirty
    } else {
        ModelineStyle::Clean
    };

    Frame {
        lines,
        modeline: fit_status_line(&modeline, width),
        cursor,
        modeline_style,
    }
}

fn editor_gutter_width(buffer: &Buffer, terminal_width: usize) -> usize {
    if terminal_width < MIN_GUTTER_TOTAL_WIDTH {
        return 0;
    }

    line_number_width(buffer).saturating_add(3)
}

fn line_number_width(buffer: &Buffer) -> usize {
    buffer.len_lines().max(1).to_string().len().max(3)
}

fn line_gutter(line_idx: usize, point_line: usize, changed: bool, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let number_width = width.saturating_sub(3);
    let change_marker = if changed { "+" } else { " " };
    let point_marker = if line_idx == point_line { ">" } else { " " };
    let number = if line_idx == point_line {
        line_idx + 1
    } else {
        line_idx.abs_diff(point_line)
    };

    format!("{change_marker}{point_marker}{number:>number_width$} ")
}

fn empty_gutter(width: usize) -> String {
    " ".repeat(width)
}

fn command_line_text(input: &str) -> String {
    if input == "/" {
        return format!(" /  {COMMAND_LINE_HINT}");
    }

    format!(" {input}")
}

fn command_line_cursor(input: &str, size: TerminalSize) -> CursorPosition {
    if size.cols == 0 || size.rows == 0 {
        return CursorPosition { col: 0, row: 0 };
    }

    let text_before_cursor = format!(" {input}");
    let col = measure_cells(&text_before_cursor, size.cols as usize)
        .min(size.cols.saturating_sub(1) as usize) as u16;

    CursorPosition {
        col,
        row: size.rows.saturating_sub(1),
    }
}

fn cursor_position(
    buffer: &Buffer,
    view: &View,
    size: TerminalSize,
    gutter_width: usize,
) -> CursorPosition {
    if size.cols == 0 || size.rows == 0 {
        return CursorPosition { col: 0, row: 0 };
    }

    let point_line = buffer.line_for_char(view.point());
    let text_width = (size.cols as usize).saturating_sub(gutter_width);
    let point_cell_col = buffer
        .display_column(view.point())
        .saturating_sub(view.scroll_column());
    let point_cell_col = if point_cell_col < text_width {
        point_cell_col
    } else {
        let cursor_point = buffer.char_at_display_column(
            point_line,
            view.scroll_column()
                .saturating_add(text_width.saturating_sub(1)),
        );
        buffer
            .display_column(cursor_point)
            .saturating_sub(view.scroll_column())
    };
    let viewport_height = size.rows.saturating_sub(1) as usize;
    let row = point_line
        .saturating_sub(view.scroll_line())
        .min(viewport_height.saturating_sub(1)) as u16;
    let col = gutter_width
        .saturating_add(point_cell_col)
        .min(size.cols.saturating_sub(1) as usize) as u16;

    CursorPosition { col, row }
}

fn modeline_text(buffer: &Buffer, view: &View, status_message: Option<&str>) -> String {
    let line_idx = buffer.line_for_char(view.point());
    let column = buffer.display_column(view.point());
    let dirty_state = if buffer.is_dirty() {
        "MODIFIED"
    } else {
        "CLEAN"
    };
    let disk_state = if buffer.disk_changed() {
        "  [disk-changed]"
    } else {
        ""
    };
    let file_name = buffer
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| buffer.path().display().to_string(), ToOwned::to_owned);

    let mut text = format!(
        " {MODELINE_BRAND}  {}  {}{}  Ln {}, Col {} ",
        file_name,
        dirty_state,
        disk_state,
        line_idx + 1,
        column + 1
    );

    if let Some(language) = language_label_for_path(buffer.path()) {
        text.push_str(" | ");
        text.push_str(language);
        text.push(' ');
    }

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
    let visible_rows = picker.visible_rows();
    let mut lines = Vec::with_capacity(viewport_height);

    if viewport_height > 0 {
        lines.push(PickerLine {
            text: fit_line_cells(&picker_header_text(picker), width),
            selected: false,
        });
    }

    if viewport_height > 1 {
        lines.push(PickerLine {
            text: fit_line_cells(" Name                                        Kind", width),
            selected: false,
        });
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

    if visible_rows.is_empty() && lines.len() < viewport_height {
        lines.push(PickerLine {
            text: fit_line_cells("  No visible files", width),
            selected: false,
        });
    } else {
        for row in visible_rows.iter().skip(first_entry).take(entry_rows) {
            lines.push(PickerLine {
                text: fit_line_cells(&picker_entry_text(row), width),
                selected: row.is_selected(),
            });
        }
    }

    while lines.len() < viewport_height {
        lines.push(PickerLine {
            text: empty_space_line(width),
            selected: false,
        });
    }

    let cursor_row = if visible_rows.is_empty() {
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
        status_kind: picker.status_message().map(status_kind_for_message),
    }
}

fn paint_editor_frame(frame: Frame, size: TerminalSize) -> CellFrame {
    let width = size.cols as usize;
    let mut cells = Vec::with_capacity(width.saturating_mul(size.rows as usize));

    for line in &frame.lines {
        let row_start = cells.len();
        let foreground = match line.kind {
            ScreenLineKind::Text => THEME.editor_fg,
            ScreenLineKind::EmptySpace => THEME.empty_fg,
        };

        if !line.gutter.is_empty() {
            let gutter_foreground = if line.current {
                THEME.gutter_current_fg
            } else {
                THEME.gutter_fg
            };
            push_cells(
                &mut cells,
                &line.gutter,
                plain_style(gutter_foreground),
                width,
            );
        }

        for segment in &line.segments {
            let mut style = segment
                .highlight
                .map_or_else(|| plain_style(foreground), highlight_style);
            if segment.selected {
                style.background = Some(THEME.selection_bg);
            }
            let remaining = width.saturating_sub(cells.len() - row_start);
            push_cells(&mut cells, &segment.text, style, remaining);
        }

        fill_row(&mut cells, row_start, width, plain_style(foreground));
    }

    if size.rows > 0 {
        paint_modeline(&mut cells, &frame.modeline, frame.modeline_style, width);
    }

    debug_assert_eq!(
        cells.len(),
        width.saturating_mul(size.rows as usize),
        "painted editor frame must cover every terminal cell"
    );
    CellFrame {
        size,
        cells,
        cursor: frame.cursor,
    }
}

fn paint_picker_frame(frame: PickerFrame, size: TerminalSize) -> CellFrame {
    let width = size.cols as usize;
    let mut cells = Vec::with_capacity(width.saturating_mul(size.rows as usize));

    for line in &frame.lines {
        let row_start = cells.len();
        let style = TextStyle {
            foreground: if line.selected {
                THEME.picker_selected_fg
            } else {
                THEME.picker_fg
            },
            background: line.selected.then_some(THEME.picker_selected_bg),
            ..plain_style(THEME.picker_fg)
        };
        push_cells(&mut cells, &line.text, style, width);
        fill_row(&mut cells, row_start, width, style);
    }

    if size.rows > 0 {
        let modeline_style = frame
            .status_kind
            .map(modeline_style_for_status)
            .unwrap_or(ModelineStyle::Info);
        paint_modeline(&mut cells, &frame.modeline, modeline_style, width);
    }

    debug_assert_eq!(
        cells.len(),
        width.saturating_mul(size.rows as usize),
        "painted picker frame must cover every terminal cell"
    );
    CellFrame {
        size,
        cells,
        cursor: frame.cursor,
    }
}

fn paint_modeline(cells: &mut Vec<Cell>, modeline: &str, style: ModelineStyle, width: usize) {
    let row_start = cells.len();
    let (foreground, background) = modeline_colors(style);
    let modeline_style = TextStyle {
        foreground,
        background: Some(background),
        bold: true,
        ..plain_style(foreground)
    };
    let brand = format!(" {MODELINE_BRAND} ");

    if let Some(rest) = modeline.strip_prefix(&brand) {
        let brand_style = TextStyle {
            foreground: THEME.modeline_brand_fg,
            background: Some(THEME.modeline_brand_bg),
            bold: true,
            ..plain_style(THEME.modeline_brand_fg)
        };
        push_cells(cells, &brand, brand_style, width);
        push_cells(
            cells,
            rest,
            modeline_style,
            width.saturating_sub(cells.len() - row_start),
        );
    } else {
        push_cells(cells, modeline, modeline_style, width);
    }

    fill_row(cells, row_start, width, modeline_style);
}

fn push_cells(cells: &mut Vec<Cell>, text: &str, style: TextStyle, max_cells: usize) {
    let mut used: usize = 0;
    for (_, grapheme) in text::grapheme_char_indices(text) {
        let cell_width = text::grapheme_width(grapheme, used);
        if used.saturating_add(cell_width) > max_cells {
            break;
        }

        cells.push(Cell {
            content: grapheme_cell_content(grapheme),
            style,
        });
        for _ in 1..cell_width {
            cells.push(Cell {
                content: CellContent::Continuation,
                style,
            });
        }
        used += cell_width;
    }
}

fn fill_row(cells: &mut Vec<Cell>, row_start: usize, width: usize, style: TextStyle) {
    cells.resize(
        row_start.saturating_add(width),
        Cell {
            content: CellContent::Text(' '),
            style,
        },
    );
}

fn grapheme_cell_content(grapheme: &str) -> CellContent {
    let grapheme = display_grapheme(grapheme);
    if let Some(ch) = single_char(&grapheme) {
        CellContent::Text(ch)
    } else {
        CellContent::Grapheme(grapheme.into_owned())
    }
}

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}

fn display_grapheme(grapheme: &str) -> Cow<'_, str> {
    if grapheme.chars().any(char::is_control) {
        Cow::Borrowed(" ")
    } else if text::grapheme_has_zero_width(grapheme) {
        Cow::Owned(format!("◌{grapheme}"))
    } else {
        Cow::Borrowed(grapheme)
    }
}

fn keycast_text(keycast: &str) -> String {
    format!(" KEYS {keycast} ")
}

fn modeline_with_keycast(modeline: &str, keycast: &str) -> String {
    let brand = format!(" {MODELINE_BRAND} ");
    let keycast = keycast_text(keycast);

    modeline.strip_prefix(&brand).map_or_else(
        || format!("{keycast}{modeline}"),
        |rest| format!("{brand}{keycast}{rest}"),
    )
}

fn modeline_colors(style: ModelineStyle) -> (Color, Color) {
    let foreground = match style {
        ModelineStyle::Clean | ModelineStyle::Info => THEME.modeline_fg,
        ModelineStyle::Dirty => THEME.dirty_fg,
        ModelineStyle::Success => THEME.success_fg,
        ModelineStyle::Error => THEME.error_fg,
        ModelineStyle::Prefix => THEME.prefix_fg,
        ModelineStyle::Prompt => THEME.prompt_fg,
        ModelineStyle::CommandLine => THEME.command_fg,
    };
    let background = match style {
        ModelineStyle::CommandLine => THEME.command_bg,
        _ => THEME.modeline_bg,
    };

    (foreground, background)
}

fn modeline_style_for_status(kind: StatusKind) -> ModelineStyle {
    match kind {
        StatusKind::Info => ModelineStyle::Info,
        StatusKind::Success => ModelineStyle::Success,
        StatusKind::Error => ModelineStyle::Error,
        StatusKind::Prefix => ModelineStyle::Prefix,
        StatusKind::Prompt => ModelineStyle::Prompt,
    }
}

fn status_kind_for_message(message: &str) -> StatusKind {
    if message == "C-x" {
        StatusKind::Prefix
    } else if message.contains("failed")
        || message.contains("Only")
        || message.contains("No files")
        || message.contains("No visible")
    {
        StatusKind::Error
    } else {
        StatusKind::Info
    }
}

fn picker_header_text(picker: &DirectoryPicker) -> String {
    let count = picker.visible_len();
    let noun = if count == 1 { "item" } else { "items" };
    format!(
        " CORTEX FIND  {}  {count} {noun}",
        picker.directory().display()
    )
}

fn picker_entry_text(row: &DirectoryPickerRow) -> String {
    let marker = if row.is_selected() { ">" } else { " " };
    let indent = "  ".repeat(row.depth().min(MAX_PICKER_INDENT_DEPTH));
    let suffix = if row.is_directory() { "/" } else { "" };
    let branch = if row.is_directory() {
        if row.is_expanded() {
            "- "
        } else {
            "+ "
        }
    } else {
        "  "
    };
    let kind = match row.kind() {
        DirectoryEntryKind::File => "FILE",
        DirectoryEntryKind::Directory => "DIR ",
        DirectoryEntryKind::Other => "ITEM",
    };

    format!(
        "{marker} {indent}{branch}{:<42} {kind}",
        format!("{}{suffix}", row.name())
    )
}

fn picker_modeline_text(picker: &DirectoryPicker) -> String {
    let mut text = format!(
        " {MODELINE_BRAND}  PICKER  Enter open/expand  Left collapse  Backspace parent  Esc quit "
    );

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

fn fit_line_segments(
    line: &str,
    highlight_spans: &[HighlightSpan],
    width: usize,
    line_start: usize,
    line_byte_start: usize,
    initial_column: usize,
    selection_range: Option<&Range<usize>>,
) -> Vec<StyledSegment> {
    let mut segments = Vec::new();
    let mut cells = 0;

    let mut line_char_idx = 0;
    for (byte_idx, grapheme) in line.grapheme_indices(true) {
        if cells >= width {
            break;
        }

        let char_idx = line_start + line_char_idx;
        let highlight =
            highlight_for_byte(highlight_spans, line_byte_start.saturating_add(byte_idx));
        let selected = selection_range.is_some_and(|range| range.contains(&char_idx));
        if grapheme == "\t" {
            let spaces = tab_spaces(initial_column.saturating_add(cells)).min(width - cells);
            push_segment(&mut segments, &" ".repeat(spaces), highlight, selected);
            cells += spaces;
            line_char_idx += 1;
            continue;
        }

        let grapheme_width = text::grapheme_width(grapheme, initial_column.saturating_add(cells));
        if cells + grapheme_width > width {
            break;
        }

        push_segment(
            &mut segments,
            &display_grapheme(grapheme),
            highlight,
            selected,
        );
        cells += grapheme_width;
        line_char_idx += grapheme.chars().count();
    }

    segments
}

fn push_segment(
    segments: &mut Vec<StyledSegment>,
    text: &str,
    highlight: Option<HighlightKind>,
    selected: bool,
) {
    if text.is_empty() {
        return;
    }

    if let Some(segment) = segments
        .last_mut()
        .filter(|segment| segment.highlight == highlight && segment.selected == selected)
    {
        segment.text.push_str(text);
        return;
    }

    segments.push(StyledSegment {
        text: text.to_string(),
        highlight,
        selected,
    });
}

fn highlight_for_byte(highlight_spans: &[HighlightSpan], byte_idx: usize) -> Option<HighlightKind> {
    highlight_spans
        .iter()
        .rev()
        .find(|span| span.range.contains(&byte_idx))
        .map(|span| span.kind)
}

fn segments_text(segments: &[StyledSegment]) -> String {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

fn plain_style(foreground: Color) -> TextStyle {
    TextStyle {
        foreground,
        background: None,
        bold: false,
        italic: false,
        underlined: false,
        dim: false,
    }
}

fn highlight_style(kind: HighlightKind) -> TextStyle {
    let mut style = plain_style(match kind {
        HighlightKind::Attribute => THEME.syntax_attribute,
        HighlightKind::Boolean => THEME.syntax_boolean,
        HighlightKind::Comment => THEME.syntax_comment,
        HighlightKind::Constant => THEME.syntax_constant,
        HighlightKind::Constructor => THEME.syntax_constructor,
        HighlightKind::Function => THEME.syntax_function,
        HighlightKind::Keyword => THEME.syntax_keyword,
        HighlightKind::Markup => THEME.syntax_markup,
        HighlightKind::MarkupBold => THEME.editor_fg,
        HighlightKind::MarkupHeading => THEME.markdown_heading,
        HighlightKind::MarkupItalic => THEME.editor_fg,
        HighlightKind::MarkupLink => THEME.markdown_link,
        HighlightKind::MarkupLinkUrl => THEME.markdown_uri,
        HighlightKind::MarkupList => THEME.markdown_marker,
        HighlightKind::MarkupQuote => THEME.markdown_quote,
        HighlightKind::MarkupRaw
        | HighlightKind::MarkupRawBlock
        | HighlightKind::MarkupRawInline => THEME.markdown_code,
        HighlightKind::Number => THEME.syntax_number,
        HighlightKind::Operator => THEME.syntax_operator,
        HighlightKind::Property => THEME.syntax_property,
        HighlightKind::Punctuation => THEME.syntax_punctuation,
        HighlightKind::PunctuationDelimiter => THEME.markdown_marker,
        HighlightKind::PunctuationSpecial => THEME.markdown_marker,
        HighlightKind::String => THEME.syntax_string,
        HighlightKind::StringEscape => THEME.syntax_operator,
        HighlightKind::Tag => THEME.syntax_tag,
        HighlightKind::Type => THEME.syntax_type,
        HighlightKind::Variable => THEME.syntax_variable,
    });

    match kind {
        HighlightKind::MarkupBold | HighlightKind::MarkupHeading => style.bold = true,
        HighlightKind::MarkupItalic => style.italic = true,
        HighlightKind::MarkupLinkUrl => style.underlined = true,
        HighlightKind::MarkupRaw
        | HighlightKind::MarkupRawBlock
        | HighlightKind::MarkupRawInline => style.bold = true,
        HighlightKind::Comment
        | HighlightKind::PunctuationDelimiter
        | HighlightKind::PunctuationSpecial => style.dim = true,
        _ => {}
    }

    style
}

fn apply_text_style<W: Write>(writer: &mut W, style: TextStyle) -> io::Result<()> {
    queue!(writer, SetForegroundColor(style.foreground))?;

    if let Some(background) = style.background {
        queue!(writer, SetBackgroundColor(background))?;
    }
    if style.bold {
        queue!(writer, SetAttribute(Attribute::Bold))?;
    }
    if style.italic {
        queue!(writer, SetAttribute(Attribute::Italic))?;
    }
    if style.underlined {
        queue!(writer, SetAttribute(Attribute::Underlined))?;
    }
    if style.dim {
        queue!(writer, SetAttribute(Attribute::Dim))?;
    }

    Ok(())
}

fn fit_line_cells(line: &str, width: usize) -> String {
    let mut fitted = String::new();
    let mut cells = 0;

    for (_, grapheme) in text::grapheme_char_indices(line) {
        if cells >= width {
            break;
        }

        if grapheme == "\t" {
            let spaces = tab_spaces(cells).min(width - cells);
            fitted.push_str(&" ".repeat(spaces));
            cells += spaces;
            continue;
        }

        let grapheme_width = text::grapheme_width(grapheme, cells);
        if cells + grapheme_width > width {
            break;
        }

        fitted.push_str(&display_grapheme(grapheme));
        cells += grapheme_width;
    }

    fitted
}

fn fit_status_line(line: &str, width: usize) -> String {
    let mut fitted = fit_line_cells(line, width);
    let remaining_width = width.saturating_sub(measure_cells(&fitted, width));
    fitted.push_str(&" ".repeat(remaining_width));
    fitted
}

fn empty_space_line(width: usize) -> String {
    fit_line_cells(EMPTY_SPACE_MARKER, width)
}

fn measure_cells(line: &str, max_width: usize) -> usize {
    text::measure_width(line, max_width)
}

fn tab_spaces(current_col: usize) -> usize {
    text::grapheme_width("\t", current_col)
}

#[cfg(test)]
mod tests {
    use super::{
        build_frame, build_frame_with_selection, build_picker_frame, fill_row, fit_line_cells,
        fit_status_line, measure_cells, modeline_text, plain_style, push_cells, CellFrame,
        CursorPosition, Frame, ModelineStyle, Renderer, ScreenLineKind, StatusKind, TerminalSize,
        COMMAND_LINE_HINT, THEME,
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
    fn fit_line_cells_preserves_and_clips_complete_graphemes() {
        assert_eq!(fit_line_cells("e\u{301}x", 1), "e\u{301}");
        assert_eq!(fit_line_cells("👨‍💻x", 2), "👨‍💻");
        assert_eq!(fit_line_cells("🇺🇸x", 2), "🇺🇸");
        assert_eq!(fit_line_cells("👍🏽x", 2), "👍🏽");
        assert_eq!(fit_line_cells("✈️x", 2), "✈️");
        assert_eq!(fit_line_cells("a界b", 2), "a");
        assert_eq!(fit_line_cells("a界b", 3), "a界");
    }

    #[test]
    fn fit_line_cells_gives_standalone_zero_width_clusters_a_visible_cell() {
        assert_eq!(fit_line_cells("\u{301}x", 1), "◌\u{301}");
        assert_eq!(measure_cells("\u{301}x", 1), 1);
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["alpha", "beta"]);
        assert!(frame.modeline.contains("CORTEX"));
        assert!(frame.modeline.contains("notes.txt"));
        assert!(frame.modeline.contains("CLEAN"));
        assert!(frame.modeline.contains("Ln 1, Col 1"));
        assert_eq!(frame.modeline_style, ModelineStyle::Clean);
        assert_eq!(frame.lines[0].gutter, " >  1 ");
        assert!(frame.lines[0].current);
        assert_eq!(frame.lines[1].gutter, "    1 ");
        assert!(!frame.lines[1].current);
        assert_eq!(frame.cursor, CursorPosition { col: 6, row: 0 });
    }

    #[test]
    fn frame_modeline_keeps_brand_visible_when_viewport_is_full() {
        let buffer = buffer_with_text("notes.txt", "alpha\nbeta\n");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
            None,
        );

        assert!(frame.modeline.starts_with(" CORTEX "));
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
            None,
        );

        assert!(frame.modeline.contains("MODIFIED"));
        assert_eq!(frame.modeline_style, ModelineStyle::Dirty);
        assert!(frame.lines[0].gutter.starts_with("+>"));
    }

    #[test]
    fn frame_modeline_shows_when_the_disk_copy_changed() {
        let dir = test_dir("modeline-disk-changed");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        fs::write(&path, "after with a different length").unwrap();
        buffer.refresh_disk_changed();

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 60, rows: 3 },
            None,
            None,
            None,
        );

        assert!(frame.modeline.contains("[disk-changed]"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn frame_marks_a_final_newline_only_change_in_the_gutter() {
        let mut buffer = buffer_with_text("notes.txt", "alpha\n");
        buffer.delete(5..6);

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
            None,
        );

        assert_eq!(frame.lines[0].gutter, "+>  1 ");
    }

    #[test]
    fn frame_uses_scroll_line_to_keep_point_visible() {
        let buffer = buffer_with_text("notes.txt", "one\ntwo\nthree\nfour\n");
        let mut view = View::new();

        view.move_next_line(&buffer);
        view.move_next_line(&buffer);
        view.ensure_point_visible(&buffer, 2, 34);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
            None,
        );

        assert_eq!(line_texts(&frame), vec!["two", "three"]);
        assert_eq!(frame.cursor, CursorPosition { col: 6, row: 1 });
        assert!(frame.modeline.contains("Ln 3, Col 1"));
    }

    #[test]
    fn frame_marks_a_change_deep_in_a_large_buffer() {
        let text = (0..100_000)
            .map(|line| format!("line {line}\n"))
            .collect::<String>();
        let mut buffer = buffer_with_text("large.txt", &text);
        let changed_line = 90_000;
        let changed_start = buffer.line_start_char(changed_line);
        buffer.insert(changed_start, "x");
        let mut view = View::new();
        view.set_point(changed_start, &buffer);
        view.ensure_point_visible(&buffer, 5, 31);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 40, rows: 6 },
            None,
            None,
            None,
        );

        assert_eq!(frame.lines.len(), 5);
        assert_eq!(
            frame
                .lines
                .iter()
                .filter(|line| line.gutter.starts_with('+'))
                .count(),
            1
        );
        assert!(frame.lines[4].text.starts_with("xline 90000"));
        assert!(frame.lines[4].gutter.starts_with("+>"));
    }

    #[test]
    fn repeated_deep_horizontal_frames_reuse_line_column_anchors() {
        let long_line = format!("{}tail\n", "x".repeat(10_000));
        let buffer = buffer_with_text("large.txt", &long_line.repeat(12));
        let mut view = View::new();
        view.move_to_line_end(&buffer);
        buffer.take_line_column_graphemes_visited();

        let size = TerminalSize { cols: 20, rows: 11 };
        view.ensure_point_visible(&buffer, 10, 14);
        let first = build_frame(&buffer, &view, size, None, None, None);
        let first_walk = buffer.take_line_column_graphemes_visited();
        view.ensure_point_visible(&buffer, 10, 14);
        let repeated = build_frame(&buffer, &view, size, None, None, None);
        let repeated_walk = buffer.take_line_column_graphemes_visited();

        assert_eq!(line_texts(&first), line_texts(&repeated));
        assert!(first_walk >= 80_000);
        assert_eq!(repeated_walk, 0);
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["zabc"]);
        assert_eq!(frame.modeline.chars().count(), 4);
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 0 });
    }

    #[test]
    fn frame_keeps_point_visible_at_the_end_of_a_long_line() {
        let buffer = buffer_with_text("notes.txt", "abcdefghij");
        let mut view = View::new();
        view.move_to_line_end(&buffer);
        view.ensure_point_visible(&buffer, 1, 6);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 6, rows: 2 },
            None,
            None,
            None,
        );

        assert_eq!(line_texts(&frame), vec!["fghij"]);
        assert_eq!(frame.cursor, CursorPosition { col: 5, row: 0 });
    }

    #[test]
    fn horizontal_window_never_draws_half_a_grapheme() {
        let buffer = buffer_with_text("notes.txt", "abcdef\n界abcdef");
        let mut view = View::new();
        view.move_to_line_end(&buffer);
        view.ensure_point_visible(&buffer, 2, 6);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 6, rows: 3 },
            None,
            None,
            None,
        );

        assert_eq!(view.scroll_column(), 1);
        assert_eq!(line_texts(&frame), vec!["bcdef", " abcde"]);
        assert_eq!(frame.cursor, CursorPosition { col: 5, row: 0 });
    }

    #[test]
    fn horizontal_window_preserves_tab_stops_and_cluster_widths() {
        let buffer = buffer_with_text("notes.txt", "\t界\u{301}x");
        let mut view = View::new();
        view.move_to_line_end(&buffer);
        view.ensure_point_visible(&buffer, 1, 5);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 5, rows: 2 },
            None,
            None,
            None,
        );

        assert_eq!(view.scroll_column(), 4);
        assert_eq!(line_texts(&frame), vec!["界\u{301}x"]);
        assert_eq!(frame.cursor, CursorPosition { col: 3, row: 0 });
    }

    #[test]
    fn modeline_column_stays_buffer_relative_after_horizontal_scroll() {
        let buffer = buffer_with_text("notes.txt", "abcdefghij");
        let mut view = View::new();
        view.move_to_line_end(&buffer);
        view.ensure_point_visible(&buffer, 1, 6);

        assert_eq!(view.scroll_column(), 5);
        assert!(modeline_text(&buffer, &view, None).contains("Col 11"));
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["    ab", ""]);
        assert_eq!(frame.cursor, CursorPosition { col: 4, row: 0 });
    }

    #[test]
    fn frame_cursor_uses_complete_grapheme_widths() {
        let buffer = buffer_with_text("notes.txt", "e\u{301}👨‍💻界x\n");
        let mut view = View::new();

        view.move_forward_char(&buffer);
        view.move_forward_char(&buffer);
        view.move_forward_char(&buffer);
        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
            None,
        );

        assert_eq!(line_texts(&frame), vec!["e\u{301}👨‍💻界x", ""]);
        assert_eq!(frame.cursor, CursorPosition { col: 11, row: 0 });
        assert!(frame.modeline.contains("Col 6"));
    }

    #[test]
    fn frame_cursor_never_clamps_into_a_wide_grapheme() {
        let buffer = buffer_with_text("notes.txt", "界");
        let mut view = View::new();
        view.move_forward_char(&buffer);

        let frame = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 2, rows: 2 },
            None,
            None,
            None,
        );

        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 0 });
    }

    #[test]
    fn frame_marks_empty_space_after_end_of_buffer() {
        let buffer = buffer_with_text("notes.txt", "alpha");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 10, rows: 4 },
            None,
            None,
            None,
        );

        assert_eq!(frame.lines[0].kind, ScreenLineKind::Text);
        assert_eq!(frame.lines[0].text, "alpha");
        assert_eq!(frame.lines[1].kind, ScreenLineKind::EmptySpace);
        assert_eq!(frame.lines[1].text, " ~");
        assert_eq!(frame.lines[2].kind, ScreenLineKind::EmptySpace);
    }

    #[test]
    fn frame_marks_active_selection_segments() {
        let buffer = buffer_with_text("notes.txt", "alpha\nbeta\n");

        let frame = build_frame_with_selection(
            &buffer,
            &View::new(),
            TerminalSize { cols: 40, rows: 3 },
            None,
            None,
            Some(1..7),
            None,
        );

        assert!(frame.lines[0]
            .segments
            .iter()
            .any(|segment| { segment.selected && segment.text.contains("lpha") }));
        assert!(frame.lines[1]
            .segments
            .iter()
            .any(|segment| segment.selected && segment.text.contains("b")));
    }

    #[test]
    fn frame_selection_styles_complete_graphemes() {
        let buffer = buffer_with_text("notes.txt", "a👨‍💻b\n");

        let frame = build_frame_with_selection(
            &buffer,
            &View::new(),
            TerminalSize { cols: 20, rows: 3 },
            None,
            None,
            Some(1..4),
            None,
        );

        let selected: String = frame.lines[0]
            .segments
            .iter()
            .filter(|segment| segment.selected)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(selected, "👨‍💻");
    }

    #[test]
    fn horizontal_window_keeps_selection_at_its_buffer_position() {
        let buffer = buffer_with_text("notes.txt", "aébcdef");
        let mut view = View::new();
        view.move_to_line_end(&buffer);
        view.ensure_point_visible(&buffer, 1, 4);

        let frame = build_frame_with_selection(
            &buffer,
            &view,
            TerminalSize { cols: 4, rows: 2 },
            None,
            None,
            Some(4..5),
            None,
        );

        assert_eq!(frame.lines[0].text, "def");
        let selected: String = frame.lines[0]
            .segments
            .iter()
            .filter(|segment| segment.selected)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(selected, "d");
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
            None,
        );
        let modeline_only = build_frame(
            &buffer,
            &view,
            TerminalSize { cols: 8, rows: 1 },
            None,
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
            Some(StatusKind::Error),
            None,
        );

        assert!(frame.modeline.contains("notes.txt"));
        assert!(frame.modeline.contains("Save failed"));
        assert_eq!(frame.modeline_style, ModelineStyle::Error);
    }

    #[test]
    fn frame_modeline_styles_status_states_distinctly() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");

        let success = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 80, rows: 3 },
            Some("Wrote notes.txt"),
            Some(StatusKind::Success),
            None,
        );
        let prefix = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 80, rows: 3 },
            Some("C-x"),
            Some(StatusKind::Prefix),
            None,
        );
        let prompt = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 80, rows: 3 },
            Some("Buffer modified; quit without saving? (y or n)"),
            Some(StatusKind::Prompt),
            None,
        );

        assert_eq!(success.modeline_style, ModelineStyle::Success);
        assert_eq!(prefix.modeline_style, ModelineStyle::Prefix);
        assert_eq!(prompt.modeline_style, ModelineStyle::Prompt);
    }

    #[test]
    fn render_emits_truecolor_sequences() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");
        let renderer = super::Renderer::new();
        let mut output = Vec::new();

        renderer
            .render(
                &mut output,
                &buffer,
                &View::new(),
                TerminalSize { cols: 20, rows: 3 },
                Some("Wrote notes.txt"),
                Some(StatusKind::Success),
                None,
                None,
                None,
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[38;2;"));
        assert!(output.contains("\x1b[48;2;"));
    }

    #[test]
    fn render_emits_syntax_highlight_colors_for_known_file_types() {
        let buffer = buffer_with_text("main.rs", "fn main() {}\n");
        let renderer = super::Renderer::new();
        let mut output = Vec::new();

        renderer
            .render(
                &mut output,
                &buffer,
                &View::new(),
                TerminalSize { cols: 40, rows: 3 },
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[38;2;137;180;250m"));
    }

    #[test]
    fn render_emits_markdown_document_styles() {
        let buffer = buffer_with_text(
            "notes.md",
            "## Heading\nUse **bold**, `code`, and [link](https://example.com).\n",
        );
        let renderer = super::Renderer::new();
        let mut output = Vec::new();

        renderer
            .render(
                &mut output,
                &buffer,
                &View::new(),
                TerminalSize { cols: 100, rows: 4 },
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[38;2;250;179;135m"));
        assert!(output.contains("\x1b[4m"));
    }

    #[test]
    fn render_emits_keycast_badge_when_present() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");
        let renderer = super::Renderer::new();
        let mut output = Vec::new();

        renderer
            .render(
                &mut output,
                &buffer,
                &View::new(),
                TerminalSize { cols: 40, rows: 3 },
                None,
                None,
                None,
                None,
                Some("C-x"),
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("KEYS C-x"));
    }

    #[test]
    fn paint_skips_unchanged_cells() {
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 3, rows: 1 };
        let mut output = Vec::new();

        renderer
            .paint(&mut output, painted_text_frame("ABC", size))
            .unwrap();
        output.clear();
        renderer
            .paint(&mut output, painted_text_frame("AXC", size))
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains('X'));
        assert!(!output.contains('A'));
        assert!(!output.contains('C'));
        assert!(!output.contains("\x1b[2J"));
    }

    #[test]
    fn paint_resize_clears_and_repaints_the_full_frame() {
        let renderer = Renderer::new();
        let mut output = Vec::new();

        renderer
            .paint(
                &mut output,
                painted_text_frame("AB", TerminalSize { cols: 2, rows: 1 }),
            )
            .unwrap();
        output.clear();
        renderer
            .paint(
                &mut output,
                painted_text_frame("ABC", TerminalSize { cols: 3, rows: 1 }),
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[2J"));
        assert!(output.contains("ABC"));
    }

    #[test]
    fn paint_diffs_zwj_graphemes_without_a_full_redraw() {
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 10, rows: 1 };
        let mut output = Vec::new();

        renderer
            .paint(&mut output, painted_text_frame("👨‍💻A", size))
            .unwrap();
        output.clear();
        renderer
            .paint(&mut output, painted_text_frame("👨‍💻B", size))
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(!output.contains("\x1b[2J"));
        assert!(output.contains('B'));
        assert!(!output.contains("👨‍💻"));
    }

    #[test]
    fn paint_keeps_graphemes_whole_across_style_boundaries() {
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 10, rows: 1 };
        let mut frame = painted_text_frame("👨‍💻;", size);
        frame.cells[0].style.bold = true;
        frame.cells[1].style.bold = true;
        frame.cells[2].style = plain_style(THEME.syntax_punctuation);
        let mut output = Vec::new();

        renderer.paint(&mut output, frame).unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[2J"));
        assert!(output.contains("👨‍💻"));
        assert!(output.contains("\x1b[1;3H"));
    }

    #[test]
    fn paint_diffs_regional_indicator_flags_without_a_full_redraw() {
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 10, rows: 1 };
        let mut output = Vec::new();

        renderer
            .paint(&mut output, painted_text_frame("🇺🇸A", size))
            .unwrap();
        output.clear();
        renderer
            .paint(&mut output, painted_text_frame("🇺🇦A", size))
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(!output.contains("\x1b[2J"));
        assert!(output.contains("🇺🇦"));
        assert!(!output.contains('A'));
    }

    #[test]
    fn paint_diffs_width_expansion_and_contraction_without_stale_cells() {
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 4, rows: 1 };
        let mut output = Vec::new();

        renderer
            .paint(&mut output, painted_text_frame("✈︎A", size))
            .unwrap();
        output.clear();
        renderer
            .paint(&mut output, painted_text_frame("✈️A", size))
            .unwrap();
        let expanded = String::from_utf8_lossy(&output);
        assert!(!expanded.contains("\x1b[2J"));
        assert!(expanded.contains("✈️A"));
        assert!(!expanded.contains("\x1b[1;2H"));

        output.clear();
        renderer
            .paint(&mut output, painted_text_frame("✈︎A", size))
            .unwrap();
        let contracted = String::from_utf8_lossy(&output);
        assert!(!contracted.contains("\x1b[2J"));
        assert!(contracted.contains("✈︎A "));
        assert!(!contracted.contains("\x1b[1;2H"));
    }

    #[test]
    fn paint_emits_a_spacing_base_for_standalone_zero_width_clusters() {
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 3, rows: 1 };
        let mut output = Vec::new();

        renderer
            .paint(&mut output, painted_text_frame("\u{301}A", size))
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("◌\u{301}A"));
    }

    #[test]
    fn cursor_movement_does_not_rewrite_unchanged_buffer_text() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");
        let renderer = Renderer::new();
        let size = TerminalSize { cols: 40, rows: 3 };
        let mut view = View::new();
        let mut output = Vec::new();

        renderer
            .render(
                &mut output,
                &buffer,
                &view,
                size,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        output.clear();
        view.move_forward_char(&buffer);
        renderer
            .render(
                &mut output,
                &buffer,
                &view,
                size,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(!output.contains("alpha"));
        assert!(!output.contains("\x1b[2J"));
    }

    #[test]
    fn frame_modeline_shows_active_command_line_and_moves_cursor_to_it() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 80, rows: 3 },
            Some("old status"),
            Some(StatusKind::Info),
            Some("/save"),
        );

        assert!(frame.modeline.starts_with(" /save"));
        assert!(!frame.modeline.contains("CORTEX"));
        assert!(!frame.modeline.contains("old status"));
        assert_eq!(frame.modeline_style, ModelineStyle::CommandLine);
        assert_eq!(frame.cursor, CursorPosition { col: 6, row: 2 });
    }

    #[test]
    fn frame_modeline_lists_commands_when_slash_prompt_opens() {
        let buffer = buffer_with_text("notes.txt", "alpha\n");

        let frame = build_frame(
            &buffer,
            &View::new(),
            TerminalSize { cols: 120, rows: 3 },
            None,
            None,
            Some("/"),
        );

        assert!(frame.modeline.contains(COMMAND_LINE_HINT));
        assert!(frame.modeline.contains("/open <path>"));
        assert!(frame.modeline.contains("/search <text>"));
        assert_eq!(frame.cursor, CursorPosition { col: 2, row: 2 });
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

        assert!(frame.lines[0].text.contains("CORTEX FIND"));
        assert!(frame.lines[0].text.contains("/tmp/project"));
        assert!(frame.lines[2].text.contains("> + src/"));
        assert!(frame.lines[2].text.contains("DIR"));
        assert!(frame.lines[2].selected);
        assert!(frame.lines[3].text.contains("main.rs"));
        assert!(frame.lines[3].text.contains("FILE"));
        assert!(frame.modeline.contains("Enter open/expand"));
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

        assert!(frame.lines[2].text.contains("c.txt"));
        assert!(frame.lines[3].text.contains(">   d.txt"));
        assert!(frame.lines[3].selected);
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 3 });
    }

    fn line_texts(frame: &Frame) -> Vec<&str> {
        frame.lines.iter().map(|line| line.text.as_str()).collect()
    }

    fn painted_text_frame(text: &str, size: TerminalSize) -> CellFrame {
        let style = plain_style(THEME.editor_fg);
        let mut cells = Vec::new();
        push_cells(&mut cells, text, style, size.cols as usize);
        fill_row(&mut cells, 0, size.cols as usize, style);
        CellFrame {
            size,
            cells,
            cursor: CursorPosition { col: 0, row: 0 },
        }
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
