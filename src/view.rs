use crate::buffer::Buffer;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct View {
    point: usize,
    scroll_line: usize,
    preferred_column: Option<usize>,
}

impl View {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn point(&self) -> usize {
        self.point
    }

    pub fn scroll_line(&self) -> usize {
        self.scroll_line
    }

    pub fn set_point(&mut self, point: usize, buffer: &Buffer) {
        self.point = point.min(buffer.len_chars());
        self.clear_preferred_column();
    }

    pub fn move_forward_char(&mut self, buffer: &Buffer) {
        self.point = self.point.saturating_add(1).min(buffer.len_chars());
        self.clear_preferred_column();
    }

    pub fn move_backward_char(&mut self) {
        self.point = self.point.saturating_sub(1);
        self.clear_preferred_column();
    }

    pub fn move_next_line(&mut self, buffer: &Buffer) {
        self.move_vertical(buffer, 1);
    }

    pub fn move_previous_line(&mut self, buffer: &Buffer) {
        self.move_vertical(buffer, -1);
    }

    pub fn move_to_line_start(&mut self, buffer: &Buffer) {
        let line_idx = buffer.line_for_char(self.point);
        self.point = buffer.line_start_char(line_idx);
        self.clear_preferred_column();
    }

    pub fn move_to_line_end(&mut self, buffer: &Buffer) {
        let line_idx = buffer.line_for_char(self.point);
        self.point = buffer.line_end_char(line_idx);
        self.clear_preferred_column();
    }

    pub fn ensure_point_visible(&mut self, buffer: &Buffer, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }

        let point_line = buffer.line_for_char(self.point);
        if point_line < self.scroll_line {
            self.scroll_line = point_line;
        } else if point_line >= self.scroll_line.saturating_add(viewport_height) {
            self.scroll_line = point_line + 1 - viewport_height;
        }
    }

    fn move_vertical(&mut self, buffer: &Buffer, direction: isize) {
        let current_line = buffer.line_for_char(self.point);
        let last_line = buffer.len_lines().saturating_sub(1);
        let target_line = if direction.is_negative() {
            current_line.saturating_sub(direction.unsigned_abs())
        } else {
            current_line
                .saturating_add(direction as usize)
                .min(last_line)
        };
        let preferred_column = self
            .preferred_column
            .unwrap_or_else(|| self.current_column(buffer));
        let target_start = buffer.line_start_char(target_line);
        let target_end = buffer.line_end_char(target_line);
        let target_len = target_end - target_start;

        self.point = target_start + preferred_column.min(target_len);
        self.preferred_column = Some(preferred_column);
    }

    fn current_column(&self, buffer: &Buffer) -> usize {
        let line_idx = buffer.line_for_char(self.point);
        self.point - buffer.line_start_char(line_idx)
    }

    fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }
}

#[cfg(test)]
mod tests {
    use super::View;
    use crate::buffer::Buffer;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn forward_char_moves_one_character_and_clamps_at_eof() {
        let buffer = buffer_with_text("ab");
        let mut view = View::new();

        view.move_forward_char(&buffer);
        assert_eq!(view.point(), 1);

        view.move_forward_char(&buffer);
        view.move_forward_char(&buffer);
        assert_eq!(view.point(), 2);
    }

    #[test]
    fn backward_char_moves_one_character_and_clamps_at_bof() {
        let buffer = buffer_with_text("ab");
        let mut view = View::new();
        view.move_forward_char(&buffer);
        view.move_forward_char(&buffer);

        view.move_backward_char();
        assert_eq!(view.point(), 1);

        view.move_backward_char();
        view.move_backward_char();
        assert_eq!(view.point(), 0);
    }

    #[test]
    fn movement_clamps_in_empty_files() {
        let buffer = buffer_with_text("");
        let mut view = View::new();

        view.move_forward_char(&buffer);
        view.move_backward_char();
        view.move_next_line(&buffer);
        view.move_previous_line(&buffer);
        view.move_to_line_start(&buffer);
        view.move_to_line_end(&buffer);

        assert_eq!(view.point(), 0);
    }

    #[test]
    fn line_start_and_end_use_the_current_line() {
        let buffer = buffer_with_text("alpha\nbeta\n");
        let mut view = View::new();
        for _ in 0..8 {
            view.move_forward_char(&buffer);
        }

        view.move_to_line_start(&buffer);
        assert_eq!(view.point(), 6);

        view.move_to_line_end(&buffer);
        assert_eq!(view.point(), 10);
    }

    #[test]
    fn line_end_stops_before_the_newline() {
        let buffer = buffer_with_text("alpha\nbeta\n");
        let mut view = View::new();

        view.move_to_line_end(&buffer);

        assert_eq!(view.point(), 5);
    }

    #[test]
    fn next_and_previous_line_preserve_column_when_lines_are_long_enough() {
        let buffer = buffer_with_text("alpha\nbeta\ngamma\n");
        let mut view = View::new();
        for _ in 0..2 {
            view.move_forward_char(&buffer);
        }

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 8);

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 13);

        view.move_previous_line(&buffer);
        assert_eq!(view.point(), 8);
    }

    #[test]
    fn vertical_movement_clamps_to_short_lines_then_restores_preferred_column() {
        let buffer = buffer_with_text("abcdef\nxy\nabcdef\n");
        let mut view = View::new();
        for _ in 0..5 {
            view.move_forward_char(&buffer);
        }

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 9);

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 15);
    }

    #[test]
    fn horizontal_movement_resets_preferred_column() {
        let buffer = buffer_with_text("abcdef\nxy\nabcdef\n");
        let mut view = View::new();
        for _ in 0..5 {
            view.move_forward_char(&buffer);
        }
        view.move_next_line(&buffer);

        view.move_backward_char();
        view.move_next_line(&buffer);

        assert_eq!(view.point(), 11);
    }

    #[test]
    fn next_and_previous_line_clamp_at_file_edges() {
        let buffer = buffer_with_text("a\nb");
        let mut view = View::new();

        view.move_previous_line(&buffer);
        assert_eq!(view.point(), 0);

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 2);

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 2);
    }

    #[test]
    fn movement_handles_empty_lines() {
        let buffer = buffer_with_text("alpha\n\nomega");
        let mut view = View::new();
        for _ in 0..3 {
            view.move_forward_char(&buffer);
        }

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 6);

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 10);
    }

    #[test]
    fn movement_handles_final_lines_without_trailing_newline() {
        let buffer = buffer_with_text("alpha\nxy");
        let mut view = View::new();
        for _ in 0..4 {
            view.move_forward_char(&buffer);
        }

        view.move_next_line(&buffer);
        assert_eq!(view.point(), 8);

        view.move_to_line_end(&buffer);
        assert_eq!(view.point(), 8);

        view.move_previous_line(&buffer);
        assert_eq!(view.point(), 2);
    }

    #[test]
    fn line_start_on_final_empty_line_moves_to_eof_after_trailing_newline() {
        let buffer = buffer_with_text("alpha\n");
        let mut view = View::new();
        for _ in 0..buffer.len_chars() {
            view.move_forward_char(&buffer);
        }

        view.move_to_line_start(&buffer);

        assert_eq!(view.point(), buffer.len_chars());
    }

    #[test]
    fn ensure_point_visible_scrolls_down_and_up() {
        let buffer = buffer_with_text("one\ntwo\nthree\nfour\n");
        let mut view = View::new();
        for _ in 0..10 {
            view.move_forward_char(&buffer);
        }

        view.ensure_point_visible(&buffer, 2);
        assert_eq!(view.scroll_line(), 1);

        view.move_previous_line(&buffer);
        view.move_previous_line(&buffer);
        view.ensure_point_visible(&buffer, 2);
        assert_eq!(view.scroll_line(), 0);
    }

    fn buffer_with_text(text: &str) -> Buffer {
        let dir = test_dir("view");
        let path = dir.join("notes.txt");
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
            "cortex-view-test-{}-{name}-{unique}-{counter}",
            std::process::id(),
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
}
