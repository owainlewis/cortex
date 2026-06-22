use crate::{
    buffer::Buffer,
    highlighter::{HighlightKind, HighlightSpan, SyntaxHighlighter},
    picker::{DirectoryEntry, DirectoryEntryKind, DirectoryPicker},
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
    cell::RefCell,
    io::{self, Write},
};

const TAB_WIDTH: usize = 4;
const EMPTY_SPACE_MARKER: &str = " ~";

const THEME: Theme = Theme {
    editor_fg: Color::Rgb {
        r: 214,
        g: 219,
        b: 220,
    },
    empty_fg: Color::Rgb {
        r: 80,
        g: 88,
        b: 92,
    },
    modeline_fg: Color::Rgb {
        r: 225,
        g: 230,
        b: 232,
    },
    modeline_bg: Color::Rgb {
        r: 42,
        g: 48,
        b: 52,
    },
    dirty_fg: Color::Rgb {
        r: 244,
        g: 191,
        b: 117,
    },
    success_fg: Color::Rgb {
        r: 132,
        g: 204,
        b: 159,
    },
    error_fg: Color::Rgb {
        r: 238,
        g: 126,
        b: 126,
    },
    prefix_fg: Color::Rgb {
        r: 142,
        g: 190,
        b: 241,
    },
    prompt_fg: Color::Rgb {
        r: 236,
        g: 211,
        b: 124,
    },
    command_fg: Color::Rgb {
        r: 245,
        g: 247,
        b: 248,
    },
    command_bg: Color::Rgb {
        r: 38,
        g: 55,
        b: 63,
    },
    picker_fg: Color::Rgb {
        r: 222,
        g: 226,
        b: 227,
    },
    picker_selected_fg: Color::Rgb {
        r: 245,
        g: 247,
        b: 248,
    },
    picker_selected_bg: Color::Rgb {
        r: 57,
        g: 75,
        b: 83,
    },
    syntax_attribute: Color::Rgb {
        r: 213,
        g: 164,
        b: 111,
    },
    syntax_boolean: Color::Rgb {
        r: 238,
        g: 126,
        b: 126,
    },
    syntax_comment: Color::Rgb {
        r: 112,
        g: 124,
        b: 128,
    },
    syntax_constant: Color::Rgb {
        r: 238,
        g: 126,
        b: 126,
    },
    syntax_constructor: Color::Rgb {
        r: 137,
        g: 203,
        b: 187,
    },
    syntax_function: Color::Rgb {
        r: 236,
        g: 211,
        b: 124,
    },
    syntax_keyword: Color::Rgb {
        r: 142,
        g: 190,
        b: 241,
    },
    syntax_markup: Color::Rgb {
        r: 189,
        g: 174,
        b: 232,
    },
    markdown_heading: Color::Rgb {
        r: 245,
        g: 203,
        b: 128,
    },
    markdown_link: Color::Rgb {
        r: 151,
        g: 202,
        b: 222,
    },
    markdown_uri: Color::Rgb {
        r: 132,
        g: 204,
        b: 159,
    },
    markdown_code: Color::Rgb {
        r: 244,
        g: 191,
        b: 117,
    },
    markdown_code_bg: Color::Rgb {
        r: 34,
        g: 42,
        b: 46,
    },
    markdown_marker: Color::Rgb {
        r: 112,
        g: 124,
        b: 128,
    },
    markdown_quote: Color::Rgb {
        r: 189,
        g: 174,
        b: 232,
    },
    syntax_number: Color::Rgb {
        r: 244,
        g: 191,
        b: 117,
    },
    syntax_operator: Color::Rgb {
        r: 176,
        g: 184,
        b: 187,
    },
    syntax_property: Color::Rgb {
        r: 151,
        g: 202,
        b: 222,
    },
    syntax_punctuation: Color::Rgb {
        r: 150,
        g: 159,
        b: 163,
    },
    syntax_string: Color::Rgb {
        r: 132,
        g: 204,
        b: 159,
    },
    syntax_tag: Color::Rgb {
        r: 142,
        g: 190,
        b: 241,
    },
    syntax_type: Color::Rgb {
        r: 137,
        g: 203,
        b: 187,
    },
    syntax_variable: Color::Rgb {
        r: 214,
        g: 219,
        b: 220,
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

#[derive(Default)]
pub struct Renderer {
    highlighter: RefCell<SyntaxHighlighter>,
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
struct ScreenLine {
    text: String,
    kind: ScreenLineKind,
    segments: Vec<StyledSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StyledSegment {
    text: String,
    highlight: Option<HighlightKind>,
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
    modeline_fg: Color,
    modeline_bg: Color,
    dirty_fg: Color,
    success_fg: Color,
    error_fg: Color,
    prefix_fg: Color,
    prompt_fg: Color,
    command_fg: Color,
    command_bg: Color,
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
    markdown_code_bg: Color,
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

#[derive(Debug, Clone, Copy)]
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

    pub fn render<W: Write>(
        &self,
        writer: &mut W,
        buffer: &Buffer,
        view: &View,
        size: TerminalSize,
        status_message: Option<&str>,
        status_kind: Option<StatusKind>,
        command_line: Option<&str>,
    ) -> io::Result<()> {
        let viewport_height = self.viewport_height(size);
        let visible_lines = visible_lines(buffer, view, size.cols as usize, viewport_height);
        let highlighted_lines =
            self.highlighter
                .borrow_mut()
                .highlight_visible_lines(buffer.path(), &visible_lines);
        let frame = build_frame_with_highlights(
            buffer,
            view,
            size,
            status_message,
            status_kind,
            command_line,
            &highlighted_lines,
        );

        queue!(
            writer,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        if size.cols == 0 || size.rows == 0 {
            queue!(writer, ResetColor, cursor::Show)?;
            return writer.flush();
        }

        for (row, line) in frame.lines.iter().enumerate() {
            render_editor_line(writer, row as u16, line, size.cols as usize)?;
        }

        let modeline_row = size.rows.saturating_sub(1);
        render_modeline(writer, modeline_row, &frame.modeline, frame.modeline_style)?;
        queue!(writer, cursor::MoveTo(frame.cursor.col, frame.cursor.row), cursor::Show)?;
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
            queue!(writer, ResetColor, cursor::Show)?;
            return writer.flush();
        }

        for (row, line) in frame.lines.iter().enumerate() {
            render_picker_line(writer, row as u16, line, size.cols as usize)?;
        }

        let modeline_row = size.rows.saturating_sub(1);
        let modeline_style = frame
            .status_kind
            .map(modeline_style_for_status)
            .unwrap_or(ModelineStyle::Info);
        render_modeline(writer, modeline_row, &frame.modeline, modeline_style)?;
        queue!(writer, cursor::MoveTo(frame.cursor.col, frame.cursor.row), cursor::Show)?;
        writer.flush()
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
    build_frame_with_highlights(
        buffer,
        view,
        size,
        status_message,
        status_kind,
        command_line,
        &[],
    )
}

fn build_frame_with_highlights(
    buffer: &Buffer,
    view: &View,
    size: TerminalSize,
    status_message: Option<&str>,
    status_kind: Option<StatusKind>,
    command_line: Option<&str>,
    highlighted_lines: &[Vec<HighlightSpan>],
) -> Frame {
    let width = size.cols as usize;
    let viewport_height = size.rows.saturating_sub(1) as usize;
    let mut lines = Vec::with_capacity(viewport_height);

    for screen_row in 0..viewport_height {
        let line_idx = view.scroll_line().saturating_add(screen_row);
        let screen_line = if line_idx < buffer.len_lines() {
            let raw_text = buffer.line_prefix_text(line_idx, width);
            let spans = highlighted_lines
                .get(screen_row)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let segments = fit_line_segments(&raw_text, spans, width);
            let text = segments_text(&segments);
            ScreenLine {
                text,
                kind: ScreenLineKind::Text,
                segments,
            }
        } else {
            let text = empty_space_line(width);
            ScreenLine {
                segments: vec![StyledSegment {
                    text: text.clone(),
                    highlight: None,
                }],
                text,
                kind: ScreenLineKind::EmptySpace,
            }
        };
        lines.push(screen_line);
    }

    let modeline = command_line
        .map(command_line_text)
        .unwrap_or_else(|| modeline_text(buffer, view, status_message));
    let cursor = command_line
        .map(|input| command_line_cursor(input, size))
        .unwrap_or_else(|| cursor_position(buffer, view, size));
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

fn visible_lines(
    buffer: &Buffer,
    view: &View,
    width: usize,
    viewport_height: usize,
) -> Vec<String> {
    (0..viewport_height)
        .map(|screen_row| view.scroll_line().saturating_add(screen_row))
        .take_while(|line_idx| *line_idx < buffer.len_lines())
        .map(|line_idx| buffer.line_prefix_text(line_idx, width))
        .collect()
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
        lines.push(PickerLine {
            text: fit_line_cells(&format!(" Open file in {}", picker.directory().display()), width),
            selected: false,
        });
    }

    if viewport_height > 1 {
        lines.push(PickerLine {
            text: String::new(),
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

    if picker.entries().is_empty() && lines.len() < viewport_height {
        lines.push(PickerLine {
            text: fit_line_cells("  No visible files", width),
            selected: false,
        });
    } else {
        for entry in picker.entries().iter().skip(first_entry).take(entry_rows) {
            let selected = picker.selected_entry() == Some(entry);
            lines.push(PickerLine {
                text: fit_line_cells(&picker_entry_text(entry, selected), width),
                selected,
            });
        }
    }

    while lines.len() < viewport_height {
        lines.push(PickerLine {
            text: empty_space_line(width),
            selected: false,
        });
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
        status_kind: picker.status_message().map(status_kind_for_message),
    }
}

fn render_editor_line<W: Write>(
    writer: &mut W,
    row: u16,
    line: &ScreenLine,
    width: usize,
) -> io::Result<()> {
    let foreground = match line.kind {
        ScreenLineKind::Text => THEME.editor_fg,
        ScreenLineKind::EmptySpace => THEME.empty_fg,
    };

    queue!(
        writer,
        cursor::MoveTo(0, row),
        ResetColor,
        SetForegroundColor(foreground)
    )?;

    for segment in &line.segments {
        let style = segment
            .highlight
            .map_or_else(|| plain_style(foreground), highlight_style);
        queue!(writer, SetAttribute(Attribute::Reset), ResetColor)?;
        apply_text_style(writer, style)?;
        queue!(writer, Print(&segment.text))?;
    }

    let remaining_width = width.saturating_sub(measure_cells(&line.text, width));
    if remaining_width > 0 {
        queue!(
            writer,
            SetAttribute(Attribute::Reset),
            ResetColor,
            SetForegroundColor(foreground),
            Print(" ".repeat(remaining_width))
        )?;
    }

    queue!(writer, SetAttribute(Attribute::Reset), ResetColor)
}

fn render_picker_line<W: Write>(
    writer: &mut W,
    row: u16,
    line: &PickerLine,
    width: usize,
) -> io::Result<()> {
    let foreground = if line.selected {
        THEME.picker_selected_fg
    } else {
        THEME.picker_fg
    };
    queue!(writer, cursor::MoveTo(0, row), ResetColor)?;

    if line.selected {
        queue!(writer, SetBackgroundColor(THEME.picker_selected_bg))?;
    }

    queue!(
        writer,
        SetForegroundColor(foreground),
        Print(fit_status_line(&line.text, width)),
        ResetColor
    )
}

fn render_modeline<W: Write>(
    writer: &mut W,
    row: u16,
    modeline: &str,
    style: ModelineStyle,
) -> io::Result<()> {
    let (foreground, background) = modeline_colors(style);
    queue!(
        writer,
        cursor::MoveTo(0, row),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        Print(modeline),
        SetAttribute(Attribute::Reset),
        ResetColor
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
    let mut text = " Enter open/browse  C-n/C-p move  Esc/C-x C-c quit ".to_string();

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
) -> Vec<StyledSegment> {
    let mut segments = Vec::new();
    let mut cells = 0;

    for (byte_idx, ch) in line.char_indices() {
        if cells >= width {
            break;
        }

        let highlight = highlight_for_byte(highlight_spans, byte_idx);
        if ch == '\t' {
            let spaces = tab_spaces(cells).min(width - cells);
            push_segment(
                &mut segments,
                &std::iter::repeat(' ').take(spaces).collect::<String>(),
                highlight,
            );
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

        let display_char = if ch.is_control() { ' ' } else { ch };
        push_segment(&mut segments, &display_char.to_string(), highlight);
        cells += char_width;
    }

    segments
}

fn push_segment(
    segments: &mut Vec<StyledSegment>,
    text: &str,
    highlight: Option<HighlightKind>,
) {
    if text.is_empty() {
        return;
    }

    if let Some(segment) = segments
        .last_mut()
        .filter(|segment| segment.highlight == highlight)
    {
        segment.text.push_str(text);
        return;
    }

    segments.push(StyledSegment {
        text: text.to_string(),
        highlight,
    });
}

fn highlight_for_byte(
    highlight_spans: &[HighlightSpan],
    byte_idx: usize,
) -> Option<HighlightKind> {
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
        | HighlightKind::MarkupRawInline => {
            style.background = Some(THEME.markdown_code_bg);
        }
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

fn empty_space_line(width: usize) -> String {
    fit_line_cells(EMPTY_SPACE_MARKER, width)
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
        CursorPosition, Frame, ModelineStyle, ScreenLineKind, StatusKind, TerminalSize,
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["alpha", "beta"]);
        assert!(frame.modeline.contains("notes.txt"));
        assert!(frame.modeline.contains("clean"));
        assert!(frame.modeline.contains("Ln 1, Col 1"));
        assert_eq!(frame.modeline_style, ModelineStyle::Clean);
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
            None,
        );

        assert!(frame.modeline.contains("modified"));
        assert_eq!(frame.modeline_style, ModelineStyle::Dirty);
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["two", "three"]);
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["zabc"]);
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
            None,
        );

        assert_eq!(line_texts(&frame), vec!["    ab", ""]);
        assert_eq!(frame.cursor, CursorPosition { col: 4, row: 0 });
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
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[38;2;142;190;241m"));
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
            )
            .unwrap();
        let output = String::from_utf8_lossy(&output);

        assert!(output.contains("\x1b[38;2;245;203;128m"));
        assert!(output.contains("\x1b[48;2;34;42;46m"));
        assert!(output.contains("\x1b[4m"));
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
        assert!(!frame.modeline.contains("old status"));
        assert_eq!(frame.modeline_style, ModelineStyle::CommandLine);
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

        assert!(frame.lines[0].text.contains("/tmp/project"));
        assert_eq!(frame.lines[2].text, "> dir  src/");
        assert!(frame.lines[2].selected);
        assert_eq!(frame.lines[3].text, "  file main.rs");
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

        assert_eq!(frame.lines[2].text, "  file c.txt");
        assert_eq!(frame.lines[3].text, "> file d.txt");
        assert!(frame.lines[3].selected);
        assert_eq!(frame.cursor, CursorPosition { col: 0, row: 3 });
    }

    fn line_texts(frame: &Frame) -> Vec<&str> {
        frame.lines.iter().map(|line| line.text.as_str()).collect()
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
