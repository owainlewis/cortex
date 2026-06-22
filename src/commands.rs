use crate::{buffer::Buffer, view::View};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Insert(char),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    MoveForwardChar,
    MoveBackwardChar,
    MoveNextLine,
    MovePreviousLine,
    MoveToLineStart,
    MoveToLineEnd,
    OpenFile,
    SaveBuffer,
    Quit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    pub quit: bool,
    pub dirty_quit_blocked: bool,
    pub open_file_blocked: bool,
    pub open_file_picker: bool,
    pub save_failed: bool,
    pub status_message: Option<String>,
}

pub fn dispatch(command: Command, buffer: &mut Buffer, view: &mut View) -> CommandOutcome {
    match command {
        Command::Insert(ch) => {
            let point = view.point();
            buffer.insert(point, &ch.to_string());
            view.set_point(point + 1, buffer);
            CommandOutcome::default()
        }
        Command::InsertNewline => {
            let point = view.point();
            buffer.insert(point, "\n");
            view.set_point(point + 1, buffer);
            CommandOutcome::default()
        }
        Command::DeleteBackward => {
            let point = view.point();
            if point > 0 {
                buffer.delete(point - 1..point);
                view.set_point(point - 1, buffer);
            }
            CommandOutcome::default()
        }
        Command::DeleteForward => {
            let point = view.point();
            if point < buffer.len_chars() {
                buffer.delete(point..point + 1);
                view.set_point(point, buffer);
            }
            CommandOutcome::default()
        }
        Command::MoveForwardChar => {
            view.move_forward_char(buffer);
            CommandOutcome::default()
        }
        Command::MoveBackwardChar => {
            view.move_backward_char();
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
        Command::OpenFile if buffer.is_dirty() => CommandOutcome {
            open_file_blocked: true,
            status_message: Some("Open canceled: current buffer has unsaved changes".to_string()),
            ..CommandOutcome::default()
        },
        Command::OpenFile => CommandOutcome {
            open_file_picker: true,
            ..CommandOutcome::default()
        },
        Command::SaveBuffer => match buffer.save() {
            Ok(()) => CommandOutcome {
                status_message: Some(format!("Wrote {}", buffer.path().display())),
                ..CommandOutcome::default()
            },
            Err(error) => CommandOutcome {
                save_failed: true,
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

#[cfg(test)]
mod tests {
    use super::{dispatch, Command};
    use crate::{buffer::Buffer, view::View};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
        assert!(!outcome.save_failed);
        assert!(
            outcome
                .status_message
                .as_deref()
                .is_some_and(|message| message.contains("Wrote"))
        );
        assert!(!buffer.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "!old");
        fs::remove_dir_all(dir).unwrap();

        let dir = test_dir("commands-save-fail");
        let missing_path = dir.join("missing").join("notes.txt");
        let mut buffer = Buffer::open(&missing_path).unwrap();
        let mut view = View::new();
        dispatch(Command::Insert('x'), &mut buffer, &mut view);

        let outcome = dispatch(Command::SaveBuffer, &mut buffer, &mut view);

        assert!(outcome.save_failed);
        assert!(!outcome.quit);
        assert!(
            outcome
                .status_message
                .as_deref()
                .is_some_and(|message| message.contains("Save failed"))
        );
        assert!(buffer.is_dirty());
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

    #[test]
    fn open_file_command_requests_picker_only_when_buffer_is_clean() {
        let mut buffer = buffer_with_text("notes.txt", "");
        let mut view = View::new();

        let outcome = dispatch(Command::OpenFile, &mut buffer, &mut view);
        assert!(outcome.open_file_picker);
        assert!(!outcome.quit);

        dispatch(Command::Insert('x'), &mut buffer, &mut view);
        let outcome = dispatch(Command::OpenFile, &mut buffer, &mut view);

        assert!(outcome.open_file_blocked);
        assert!(!outcome.open_file_picker);
        assert_eq!(
            outcome.status_message.as_deref(),
            Some("Open canceled: current buffer has unsaved changes")
        );
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
