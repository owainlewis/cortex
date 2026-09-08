use crate::{
    buffer::{Buffer, ReloadError, UndoGroupKind},
    view::View,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Insert(char),
    InsertNewline,
    Indent,
    Outdent,
    DeleteBackward,
    DeleteForward,
    OpenCommandLine,
    OpenPath,
    Search,
    Help,
    ForceQuit,
    KillLine,
    KillRegion,
    MoveForwardChar,
    MoveBackwardChar,
    MoveNextLine,
    MovePreviousLine,
    MoveToLineStart,
    MoveToLineEnd,
    OpenFile,
    Redo,
    ReloadBuffer,
    RepeatSearch,
    SaveBuffer,
    SetMark,
    SwitchBuffer,
    Undo,
    Yank,
    YankPop,
    Quit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    pub quit: bool,
    pub dirty_quit_blocked: bool,
    pub failed: bool,
    pub status_message: Option<String>,
}

impl Command {
    pub(crate) fn continues_undo_group(self) -> bool {
        match self {
            Self::Insert(ch) => !ch.is_control() && !matches!(ch, '\u{2028}' | '\u{2029}'),
            Self::DeleteBackward | Self::DeleteForward => true,
            _ => false,
        }
    }
}

pub fn dispatch(command: Command, buffer: &mut Buffer, view: &mut View) -> CommandOutcome {
    dispatch_at(command, buffer, view, std::time::Instant::now())
}

pub(crate) fn dispatch_at(
    command: Command,
    buffer: &mut Buffer,
    view: &mut View,
    now: std::time::Instant,
) -> CommandOutcome {
    if !command.continues_undo_group() {
        buffer.break_undo_group();
    }
    match command {
        Command::Insert(ch) => {
            let point = view.point();
            let point_after = if command.continues_undo_group() {
                buffer.insert_typed(point, ch, now)
            } else {
                buffer.insert(point, &ch.to_string())
            };
            view.set_point(point_after, buffer);
            CommandOutcome::default()
        }
        Command::InsertNewline => {
            let point = view.point();
            let line = buffer.line_for_char(point);
            let indentation: String =
                buffer.leading_indentation(line, point - buffer.line_start_char(line));
            let inserted = format!("{}{indentation}", buffer.newline_at(line));
            let point_after = buffer.insert(point, &inserted);
            view.set_point(point_after, buffer);
            CommandOutcome::default()
        }
        Command::Indent => {
            let point = view.point();
            let spaces = " ".repeat(4 - buffer.display_column(point) % 4);
            let point_after = buffer.insert(point, &spaces);
            view.set_point(point_after, buffer);
            CommandOutcome::default()
        }
        Command::Outdent => {
            indent_region(buffer, view, view.point()..view.point(), true);
            CommandOutcome::default()
        }
        Command::DeleteBackward => {
            let point = view.point();
            if point > 0 {
                let start = buffer.previous_grapheme_boundary(point);
                let point_after = buffer.delete_typed(
                    start..point,
                    point,
                    start,
                    UndoGroupKind::DeleteBackward,
                    now,
                );
                view.set_point(point_after, buffer);
            }
            CommandOutcome::default()
        }
        Command::DeleteForward => {
            let point = view.point();
            if point < buffer.len_chars() {
                let end = buffer.next_grapheme_boundary(point);
                let point_after = buffer.delete_typed(
                    point..end,
                    point,
                    point,
                    UndoGroupKind::DeleteForward,
                    now,
                );
                view.set_point(point_after, buffer);
            }
            CommandOutcome::default()
        }
        Command::KillLine
        | Command::KillRegion
        | Command::OpenPath
        | Command::Search
        | Command::Help
        | Command::ForceQuit
        | Command::OpenCommandLine
        | Command::OpenFile
        | Command::SetMark
        | Command::SwitchBuffer
        | Command::Yank
        | Command::YankPop => CommandOutcome::default(),
        Command::Undo => {
            if let Some(point) = buffer.undo() {
                view.set_point(point, buffer);
            }
            CommandOutcome::default()
        }
        Command::Redo => {
            if let Some(point) = buffer.redo() {
                view.set_point(point, buffer);
            }
            CommandOutcome::default()
        }
        Command::ReloadBuffer => reload_buffer(buffer, view),
        Command::RepeatSearch => CommandOutcome::default(),
        Command::MoveForwardChar => {
            view.move_forward_char(buffer);
            CommandOutcome::default()
        }
        Command::MoveBackwardChar => {
            view.move_backward_char(buffer);
            CommandOutcome::default()
        }
        Command::MoveNextLine => {
            view.move_next_line(buffer);
            CommandOutcome::default()
        }
        Command::MovePreviousLine => {
            view.move_previous_line(buffer);
            CommandOutcome::default()
        }
        Command::MoveToLineStart => {
            view.move_to_line_start(buffer);
            CommandOutcome::default()
        }
        Command::MoveToLineEnd => {
            view.move_to_line_end(buffer);
            CommandOutcome::default()
        }
        Command::SaveBuffer => match buffer.save() {
            Ok(()) => CommandOutcome {
                status_message: Some(format!("Wrote {}", buffer.path().display())),
                ..CommandOutcome::default()
            },
            Err(error) => CommandOutcome {
                failed: true,
                status_message: Some(format!("Save failed: {error}")),
                ..CommandOutcome::default()
            },
        },
        Command::Quit if buffer.is_dirty() => CommandOutcome {
            dirty_quit_blocked: true,
            ..CommandOutcome::default()
        },
        Command::Quit => CommandOutcome {
            quit: true,
            ..CommandOutcome::default()
        },
    }
}

pub fn indent_region(
    buffer: &mut Buffer,
    view: &mut View,
    region: std::ops::Range<usize>,
    outdent: bool,
) -> std::ops::Range<usize> {
    let first = buffer.line_for_char(region.start);
    let mut last = buffer.line_for_char(region.end);
    if !region.is_empty() && region.end == buffer.line_start_char(last) {
        last = last.saturating_sub(1);
    }
    let removals: Vec<_> = (first..=last)
        .map(|line| {
            if outdent {
                outdent_chars(&buffer.leading_indentation(line, 4))
            } else {
                0
            }
        })
        .collect();
    if outdent && removals.iter().all(|count| *count == 0) {
        return region;
    }
    // The last line's body is unchanged. End at its edited prefix so a
    // single-line operation never copies that body into text or history.
    let replaced =
        buffer.line_start_char(first)..buffer.line_start_char(last) + removals[last - first];
    let mut inserted = String::new();
    let mut mapped = region.clone();
    for (line, removed) in (first..=last).zip(removals) {
        let start = buffer.line_start_char(line);
        let end = if line == last {
            replaced.end
        } else {
            buffer.line_start_char(line + 1)
        };
        let added = if outdent { 0 } else { 4 };
        if !outdent {
            inserted.push_str("    ");
        }
        inserted.push_str(&buffer.text_range(start + removed..end));
        for (before, after) in [
            (region.start, &mut mapped.start),
            (region.end, &mut mapped.end),
        ] {
            if before > start {
                *after = after.saturating_sub(removed.min(before - start)) + added;
            }
        }
    }
    let point_after = if view.point() == region.start {
        mapped.start
    } else {
        mapped.end
    };
    let point_after = buffer.replace_with_points(replaced, &inserted, view.point(), point_after);
    view.set_point(point_after, buffer);
    mapped.start = buffer.grapheme_boundary_at_or_before(mapped.start);
    mapped.end = buffer.grapheme_boundary_at_or_before(mapped.end);
    mapped
}

fn outdent_chars(indentation: &str) -> usize {
    let mut columns = 0;
    let mut chars = 0;
    for ch in indentation.chars() {
        if columns >= 4 {
            break;
        }
        columns += if ch == '\t' { 4 - columns % 4 } else { 1 };
        chars += 1;
    }
    chars
}

fn reload_buffer(buffer: &mut Buffer, view: &mut View) -> CommandOutcome {
    let line = buffer.line_for_char(view.point());
    let column = buffer.display_column(view.point());

    match buffer.reload() {
        Ok(()) => {
            let line = line.min(buffer.len_lines().saturating_sub(1));
            view.set_point(buffer.char_at_display_column(line, column), buffer);
            CommandOutcome {
                status_message: Some(format!("Reloaded {}", buffer.path().display())),
                ..CommandOutcome::default()
            }
        }
        Err(ReloadError::Dirty) => CommandOutcome {
            failed: true,
            status_message: Some("Reload refused: buffer has unsaved changes".to_string()),
            ..CommandOutcome::default()
        },
        Err(ReloadError::Io(error)) => {
            let message = if error.kind() == std::io::ErrorKind::NotFound {
                format!(
                    "Reload failed: file does not exist: {}",
                    buffer.path().display()
                )
            } else {
                format!("Reload failed: {error}")
            };
            CommandOutcome {
                failed: true,
                status_message: Some(message),
                ..CommandOutcome::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{dispatch, Command};
    use crate::{buffer::Buffer, view::View};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn movement_and_deliberate_edits_end_typing_groups() {
        use super::dispatch_at;
        let now = std::time::Instant::now();
        for boundary in [
            Command::MoveToLineEnd,
            Command::MoveNextLine,
            Command::InsertNewline,
            Command::Insert('\u{2028}'),
            Command::Indent,
            Command::Outdent,
        ] {
            let mut buffer = buffer_with_text("group-boundary.txt", "");
            let mut view = View::new();
            for ch in "ab".chars() {
                dispatch_at(Command::Insert(ch), &mut buffer, &mut view, now);
            }
            dispatch_at(boundary, &mut buffer, &mut view, now);
            let after_boundary = buffer.text();
            for ch in "cd".chars() {
                dispatch_at(Command::Insert(ch), &mut buffer, &mut view, now);
            }
            dispatch_at(Command::Undo, &mut buffer, &mut view, now);
            assert_eq!(buffer.text(), after_boundary, "{boundary:?}");
            if after_boundary != "ab" {
                dispatch_at(Command::Undo, &mut buffer, &mut view, now);
                assert_eq!(buffer.text(), "ab", "{boundary:?}");
            }
            dispatch_at(Command::Undo, &mut buffer, &mut view, now);
            assert_eq!(buffer.text(), "", "{boundary:?}");
        }
    }

    #[test]
    fn printable_and_newline_commands_insert_at_point() {
        let mut buffer = buffer_with_text("notes.txt", "ac");
        let mut view = View::new();
        view.move_forward_char(&buffer);

        dispatch(Command::Insert('b'), &mut buffer, &mut view);
        dispatch(Command::InsertNewline, &mut buffer, &mut view);

        assert_eq!(buffer.text(), "ab\nc");
        assert_eq!(view.point(), 3);
        assert!(buffer.is_dirty());
    }

    #[test]
    fn tab_inserts_spaces_to_the_next_display_stop() {
        for (source, point, expected, after) in [
            ("", 0, "    ", 4),
            ("a", 1, "a   ", 4),
            ("界", 1, "界  ", 3),
            ("a\tb", 2, "a\t    b", 6),
            ("e\u{301}", 2, "e\u{301}   ", 5),
            ("    ", 4, "        ", 8),
        ] {
            let mut buffer = buffer_with_text("tab.txt", source);
            let mut view = View::new();
            view.set_point(point, &buffer);
            dispatch(Command::Indent, &mut buffer, &mut view);
            assert_eq!(buffer.text(), expected);
            assert_eq!(view.point(), after);
            dispatch(Command::Undo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), source);
            assert_eq!(view.point(), point);
            assert!(!buffer.is_dirty());
        }
    }

    #[test]
    fn newline_carries_only_indentation_before_point_and_preserves_line_endings() {
        for (source, point, expected, after) in [
            ("    ab", 6, "    ab\n    ", 11),
            ("\t  ab", 3, "\t  \n\t  ab", 7),
            ("    code", 2, "  \n    code", 5),
            ("  a\r\n  b", 3, "  a\r\n  \r\n  b", 7),
            ("a\r\n  b", 6, "a\r\n  b\r\n  ", 10),
            ("a\r\n  b\n", 6, "a\r\n  b\n  \n", 9),
            (" \u{301}ab", 4, " \u{301}ab\n", 5),
            (" \u{301}ab", 0, "\n \u{301}ab", 1),
        ] {
            let mut buffer = buffer_with_text("newline.txt", source);
            let mut view = View::new();
            view.set_point(point, &buffer);
            dispatch(Command::InsertNewline, &mut buffer, &mut view);
            assert_eq!(buffer.text(), expected, "{source:?} at {point}");
            assert_eq!(view.point(), after);
            dispatch(Command::Undo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), source);
            assert_eq!(view.point(), point);
            dispatch(Command::Redo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), expected);
            assert_eq!(view.point(), after);
        }
    }

    #[test]
    fn outdent_removes_one_display_level_without_converting_remaining_tabs() {
        for (source, point, expected, after) in [
            ("\t  abc", 6, "  abc", 5),
            ("  \tabc", 6, "abc", 3),
            ("    \tabc", 8, "\tabc", 4),
            ("   abc", 1, "abc", 0),
            ("  abc", 0, "abc", 0),
        ] {
            let mut buffer = buffer_with_text("outdent.txt", source);
            let mut view = View::new();
            view.set_point(point, &buffer);
            dispatch(Command::Outdent, &mut buffer, &mut view);
            assert_eq!(buffer.text(), expected);
            assert_eq!(view.point(), after);
            dispatch(Command::Undo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), source);
            assert_eq!(view.point(), point);
        }
    }

    #[test]
    fn outdent_noop_preserves_graphemes_clean_state_and_history() {
        for source in ["abc", " \u{301}abc", "\u{3000}abc", ""] {
            let mut buffer = buffer_with_text("unchanged.txt", source);
            let mut view = View::new();
            dispatch(Command::Outdent, &mut buffer, &mut view);
            assert_eq!(buffer.text(), source);
            assert!(!buffer.is_dirty());
            assert_eq!(buffer.undo(), None);
        }
    }

    #[test]
    fn region_indent_excludes_end_at_line_start_and_is_one_undo_step() {
        for ending in ["\n", "\r\n"] {
            let source = format!("a{ending}b{ending}c{ending}");
            let mut buffer = buffer_with_text("region.txt", &source);
            let mut view = View::new();
            let end = buffer.line_start_char(2);
            view.set_point(end, &buffer);
            let region = super::indent_region(&mut buffer, &mut view, 0..end, false);
            let expected = format!("    a{ending}    b{ending}c{ending}");
            assert_eq!(buffer.text(), expected);
            assert_eq!(region, 0..end + 8);
            assert_eq!(view.point(), end + 8);
            dispatch(Command::Undo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), source);
            assert_eq!(view.point(), end);
            assert!(!buffer.is_dirty());
            assert_eq!(buffer.undo(), None);
            dispatch(Command::Redo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), expected);
            assert_eq!(view.point(), end + 8);
        }
    }

    #[test]
    fn sequential_unicode_input_keeps_point_after_the_complete_grapheme() {
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        for ch in "e\u{301}👨‍💻".chars() {
            dispatch(Command::Insert(ch), &mut buffer, &mut view);
        }

        assert_eq!(buffer.text(), "e\u{301}👨‍💻");
        assert_eq!(view.point(), 5);
        view.move_backward_char(&buffer);
        assert_eq!(view.point(), 2);
        view.move_backward_char(&buffer);
        assert_eq!(view.point(), 0);
    }

    #[test]
    fn delete_commands_remove_backward_and_forward() {
        let mut buffer = buffer_with_text("notes.txt", "abcd");
        let mut view = View::new();
        for _ in 0..2 {
            view.move_forward_char(&buffer);
        }

        dispatch(Command::DeleteBackward, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "acd");
        assert_eq!(view.point(), 1);

        dispatch(Command::DeleteForward, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ad");
        assert_eq!(view.point(), 1);
    }

    #[test]
    fn delete_and_undo_keep_extended_graphemes_whole() {
        for grapheme in ["e\u{301}", "👨‍💻", "🇺🇸", "👍🏽", "✈️", "界"] {
            let original = format!("a{grapheme}b");
            let mut buffer = buffer_with_text("notes.txt", &original);
            let mut view = View::new();
            view.move_forward_char(&buffer);
            view.move_forward_char(&buffer);

            dispatch(Command::DeleteBackward, &mut buffer, &mut view);
            assert_eq!(buffer.text(), "ab");
            assert_eq!(view.point(), 1);

            dispatch(Command::Undo, &mut buffer, &mut view);
            assert_eq!(buffer.text(), original);
            assert_eq!(view.point(), 1 + grapheme.chars().count());

            view.move_backward_char(&buffer);
            dispatch(Command::DeleteForward, &mut buffer, &mut view);
            assert_eq!(buffer.text(), "ab");
            assert_eq!(view.point(), 1);
        }
    }

    #[test]
    fn undo_and_redo_commands_restore_text_and_point() {
        let mut buffer = buffer_with_text("notes.txt", "ac");
        let mut view = View::new();
        view.move_forward_char(&buffer);

        dispatch(Command::Insert('b'), &mut buffer, &mut view);
        assert_eq!(buffer.text(), "abc");
        assert_eq!(view.point(), 2);

        dispatch(Command::Undo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ac");
        assert_eq!(view.point(), 1);

        dispatch(Command::Redo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "abc");
        assert_eq!(view.point(), 2);
    }

    #[test]
    fn undo_commands_restore_newline_and_deletes() {
        let mut buffer = buffer_with_text("notes.txt", "ab");
        let mut view = View::new();
        view.move_forward_char(&buffer);

        dispatch(Command::InsertNewline, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "a\nb");
        assert_eq!(view.point(), 2);
        dispatch(Command::Undo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ab");
        assert_eq!(view.point(), 1);

        dispatch(Command::DeleteBackward, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "b");
        assert_eq!(view.point(), 0);
        dispatch(Command::Undo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ab");
        assert_eq!(view.point(), 1);

        dispatch(Command::DeleteForward, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "a");
        assert_eq!(view.point(), 1);
        dispatch(Command::Undo, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "ab");
        assert_eq!(view.point(), 1);
    }

    #[test]
    fn delete_commands_clamp_at_buffer_edges() {
        let mut buffer = buffer_with_text("notes.txt", "a");
        let mut view = View::new();

        dispatch(Command::DeleteBackward, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "a");
        assert_eq!(view.point(), 0);

        view.move_forward_char(&buffer);
        dispatch(Command::DeleteForward, &mut buffer, &mut view);
        assert_eq!(buffer.text(), "a");
        assert_eq!(view.point(), 1);
    }

    #[test]
    fn movement_commands_update_view_point() {
        let mut buffer = buffer_with_text("notes.txt", "ab\ncd");
        let mut view = View::new();

        dispatch(Command::MoveForwardChar, &mut buffer, &mut view);
        dispatch(Command::MoveNextLine, &mut buffer, &mut view);
        assert_eq!(view.point(), 4);

        dispatch(Command::MoveToLineEnd, &mut buffer, &mut view);
        assert_eq!(view.point(), 5);

        dispatch(Command::MoveToLineStart, &mut buffer, &mut view);
        dispatch(Command::MovePreviousLine, &mut buffer, &mut view);
        dispatch(Command::MoveBackwardChar, &mut buffer, &mut view);
        assert_eq!(view.point(), 0);
    }

    #[test]
    fn save_command_writes_to_disk_and_reports_failures_without_quitting() {
        let dir = test_dir("commands-save");
        let path = dir.join("notes.txt");
        fs::write(&path, "old").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();

        dispatch(Command::Insert('!'), &mut buffer, &mut view);
        let outcome = dispatch(Command::SaveBuffer, &mut buffer, &mut view);

        assert!(!outcome.quit);
        assert!(!outcome.failed);
        assert!(outcome
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Wrote")));
        assert!(!buffer.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "!old");
        fs::remove_dir_all(dir).unwrap();

        let dir = test_dir("commands-save-fail");
        let missing_path = dir.join("missing").join("notes.txt");
        let mut buffer = Buffer::open(&missing_path).unwrap();
        let mut view = View::new();
        dispatch(Command::Insert('x'), &mut buffer, &mut view);

        let outcome = dispatch(Command::SaveBuffer, &mut buffer, &mut view);

        assert!(outcome.failed);
        assert!(!outcome.quit);
        assert!(outcome
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Save failed")));
        assert!(buffer.is_dirty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reload_command_preserves_line_and_column_with_clamping() {
        let dir = test_dir("commands-reload");
        let path = dir.join("notes.txt");
        fs::write(&path, "abc\ndefgh\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();
        view.set_point(8, &buffer);
        fs::write(&path, "x\nyz\n").unwrap();

        let outcome = dispatch(Command::ReloadBuffer, &mut buffer, &mut view);

        assert!(!outcome.failed);
        assert_eq!(buffer.text(), "x\nyz\n");
        assert_eq!(view.point(), 4);
        assert_eq!(
            outcome.status_message,
            Some(format!("Reloaded {}", path.display()))
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reload_command_reports_deleted_files_without_changing_the_buffer() {
        let dir = test_dir("commands-reload-deleted");
        let path = dir.join("notes.txt");
        fs::write(&path, "keep me").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();
        fs::remove_file(&path).unwrap();

        let outcome = dispatch(Command::ReloadBuffer, &mut buffer, &mut view);

        assert!(outcome.failed);
        assert_eq!(buffer.text(), "keep me");
        assert!(outcome
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("file does not exist")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reload_command_reports_permission_errors_without_changing_the_buffer() {
        let dir = test_dir("commands-reload-permission");
        let path = dir.join("notes.txt");
        fs::write(&path, "keep me").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        let mut view = View::new();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

        let outcome = dispatch(Command::ReloadBuffer, &mut buffer, &mut view);

        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(outcome.failed);
        assert_eq!(buffer.text(), "keep me");
        assert!(outcome
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains("Permission denied")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn quit_command_exits_only_when_buffer_is_clean() {
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        let outcome = dispatch(Command::Quit, &mut buffer, &mut view);
        assert!(outcome.quit);
        assert!(!outcome.dirty_quit_blocked);

        dispatch(Command::Insert('x'), &mut buffer, &mut view);
        let outcome = dispatch(Command::Quit, &mut buffer, &mut view);
        assert!(!outcome.quit);
        assert!(outcome.dirty_quit_blocked);
    }

    fn buffer_with_text(file_name: &str, text: &str) -> Buffer {
        let dir = test_dir("commands");
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
            "cortex-commands-test-{}-{name}-{unique}-{counter}",
            std::process::id(),
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
}
