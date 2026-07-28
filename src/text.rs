use ropey::{iter::Chunks, str_utils::byte_to_char_idx, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

struct RopeGraphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    chunk: &'a str,
    chunk_start: usize,
    cursor: GraphemeCursor,
}

impl<'a> RopeGraphemes<'a> {
    fn new(text: RopeSlice<'a>) -> Self {
        let mut chunks = text.chunks();
        let chunk = chunks.next().unwrap_or("");
        Self {
            text,
            chunks,
            chunk,
            chunk_start: 0,
            cursor: GraphemeCursor::new(0, text.len_bytes(), true),
        }
    }
}

impl<'a> Iterator for RopeGraphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.cursor.cur_cursor();
        let end = loop {
            match self.cursor.next_boundary(self.chunk, self.chunk_start) {
                Ok(boundary) => break boundary?,
                Err(GraphemeIncomplete::NextChunk) => {
                    self.chunk_start += self.chunk.len();
                    self.chunk = self.chunks.next().unwrap_or("");
                }
                Err(GraphemeIncomplete::PreContext(byte_idx)) => {
                    let (chunk, chunk_start, _, _) =
                        self.text.chunk_at_byte(byte_idx.saturating_sub(1));
                    self.cursor.provide_context(chunk, chunk_start);
                }
                Err(_) => unreachable!("rope chunks must cover the grapheme cursor"),
            }
        };

        Some(
            self.text
                .slice(self.text.byte_to_char(start)..self.text.byte_to_char(end)),
        )
    }
}

pub(crate) fn grapheme_char_indices(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut char_index = 0;
    text.graphemes(true).map(move |grapheme| {
        let start = char_index;
        char_index += grapheme.chars().count();
        (start, grapheme)
    })
}

pub(crate) fn grapheme_width(grapheme: &str, current_column: usize) -> usize {
    if grapheme == "\t" {
        return 4 - (current_column % 4);
    }
    if grapheme.chars().any(char::is_control) {
        return 1;
    }

    UnicodeWidthStr::width(grapheme).max(1)
}

pub(crate) fn grapheme_has_zero_width(grapheme: &str) -> bool {
    UnicodeWidthStr::width(grapheme) == 0
}

pub(crate) fn measure_width(text: &str, max_width: usize) -> usize {
    let mut width = 0;

    for (_, grapheme) in grapheme_char_indices(text) {
        if width >= max_width {
            break;
        }
        width = width
            .saturating_add(grapheme_width(grapheme, width))
            .min(max_width);
    }

    width
}

pub(crate) fn pop_grapheme(text: &mut String) {
    if let Some((byte_index, _)) = text.grapheme_indices(true).next_back() {
        text.truncate(byte_index);
    }
}

pub(crate) fn rope_boundary_at_or_before(text: RopeSlice<'_>, char_index: usize) -> usize {
    if is_rope_grapheme_boundary(text, char_index) {
        char_index
    } else {
        previous_rope_boundary(text, char_index)
    }
}

pub(crate) fn rope_boundary_at_or_after(text: RopeSlice<'_>, char_index: usize) -> usize {
    if is_rope_grapheme_boundary(text, char_index) {
        char_index
    } else {
        next_rope_boundary(text, char_index)
    }
}

pub(crate) fn next_rope_boundary(text: RopeSlice<'_>, char_index: usize) -> usize {
    let byte_index = text.char_to_byte(char_index);
    let (mut chunk, mut chunk_start, mut chunk_char_start, _) = text.chunk_at_byte(byte_index);
    let mut cursor = GraphemeCursor::new(byte_index, text.len_bytes(), true);

    loop {
        match cursor.next_boundary(chunk, chunk_start) {
            Ok(None) => return text.len_chars(),
            Ok(Some(boundary)) => {
                return chunk_char_start + byte_to_char_idx(chunk, boundary - chunk_start);
            }
            Err(GraphemeIncomplete::NextChunk) => {
                chunk_start += chunk.len();
                let (next_chunk, _, next_char_start, _) = text.chunk_at_byte(chunk_start);
                chunk = next_chunk;
                chunk_char_start = next_char_start;
            }
            Err(GraphemeIncomplete::PreContext(byte_idx)) => {
                let (context, context_start, _, _) = text.chunk_at_byte(byte_idx.saturating_sub(1));
                cursor.provide_context(context, context_start);
            }
            Err(_) => unreachable!("rope chunks must cover the grapheme cursor"),
        }
    }
}

pub(crate) fn previous_rope_boundary(text: RopeSlice<'_>, char_index: usize) -> usize {
    let byte_index = text.char_to_byte(char_index);
    let (mut chunk, mut chunk_start, mut chunk_char_start, _) = text.chunk_at_byte(byte_index);
    let mut cursor = GraphemeCursor::new(byte_index, text.len_bytes(), true);

    loop {
        match cursor.prev_boundary(chunk, chunk_start) {
            Ok(None) => return 0,
            Ok(Some(boundary)) => {
                return chunk_char_start + byte_to_char_idx(chunk, boundary - chunk_start);
            }
            Err(GraphemeIncomplete::PrevChunk) => {
                let (previous_chunk, previous_start, previous_char_start, _) =
                    text.chunk_at_byte(chunk_start - 1);
                chunk = previous_chunk;
                chunk_start = previous_start;
                chunk_char_start = previous_char_start;
            }
            Err(GraphemeIncomplete::PreContext(byte_idx)) => {
                let (context, context_start, _, _) = text.chunk_at_byte(byte_idx.saturating_sub(1));
                cursor.provide_context(context, context_start);
            }
            Err(_) => unreachable!("rope chunks must cover the grapheme cursor"),
        }
    }
}

pub(crate) fn measure_rope_width(text: RopeSlice<'_>, max_width: usize) -> usize {
    let mut width = 0;

    for grapheme in RopeGraphemes::new(text) {
        if width >= max_width {
            break;
        }
        with_rope_slice_str(grapheme, |grapheme| {
            width = width
                .saturating_add(grapheme_width(grapheme, width))
                .min(max_width);
        });
    }

    width
}

pub(crate) fn rope_char_index_at_column(text: RopeSlice<'_>, target_column: usize) -> usize {
    let mut column: usize = 0;
    let mut boundary = 0;

    for grapheme in RopeGraphemes::new(text) {
        let width = with_rope_slice_str(grapheme, |grapheme| grapheme_width(grapheme, column));
        if column.saturating_add(width) > target_column {
            return boundary;
        }
        column += width;
        boundary += grapheme.len_chars();
    }

    boundary
}

pub(crate) fn rope_prefix_for_width(text: RopeSlice<'_>, max_width: usize) -> String {
    let mut prefix = String::new();
    let mut width = 0;

    for grapheme in RopeGraphemes::new(text) {
        let grapheme_width =
            with_rope_slice_str(grapheme, |grapheme| grapheme_width(grapheme, width));
        if width.saturating_add(grapheme_width) > max_width {
            break;
        }
        for chunk in grapheme.chunks() {
            prefix.push_str(chunk);
        }
        width += grapheme_width;
    }

    prefix
}

fn is_rope_grapheme_boundary(text: RopeSlice<'_>, char_index: usize) -> bool {
    if char_index == 0 || char_index == text.len_chars() {
        return true;
    }

    let byte_index = text.char_to_byte(char_index);
    let (chunk, chunk_start, _, _) = text.chunk_at_byte(byte_index);
    let mut cursor = GraphemeCursor::new(byte_index, text.len_bytes(), true);
    loop {
        match cursor.is_boundary(chunk, chunk_start) {
            Ok(is_boundary) => return is_boundary,
            Err(GraphemeIncomplete::PreContext(byte_idx)) => {
                let (context, context_start, _, _) = text.chunk_at_byte(byte_idx.saturating_sub(1));
                cursor.provide_context(context, context_start);
            }
            Err(_) => unreachable!("rope chunks must cover the grapheme cursor"),
        }
    }
}

fn with_rope_slice_str<T>(slice: RopeSlice<'_>, f: impl FnOnce(&str) -> T) -> T {
    if let Some(text) = slice.as_str() {
        f(text)
    } else {
        f(&slice.to_string())
    }
}

#[cfg(test)]
mod tests {
    use ropey::Rope;

    use super::{
        grapheme_char_indices, measure_rope_width, measure_width, next_rope_boundary, pop_grapheme,
        previous_rope_boundary, rope_boundary_at_or_after, rope_boundary_at_or_before,
        rope_char_index_at_column, rope_prefix_for_width,
    };

    #[test]
    fn boundaries_keep_common_extended_graphemes_whole() {
        let text = "e\u{301}👨‍💻🇺🇸👍🏽✈️界";
        let rope = Rope::from_str(text);
        let rope_text = rope.slice(..);
        let ranges: Vec<_> = grapheme_char_indices(text)
            .map(|(start, grapheme)| (start, start + grapheme.chars().count()))
            .collect();

        assert_eq!(
            ranges,
            vec![(0, 2), (2, 5), (5, 7), (7, 9), (9, 11), (11, 12)]
        );
        assert_eq!(next_rope_boundary(rope_text, 0), 2);
        assert_eq!(previous_rope_boundary(rope_text, 5), 2);
        assert_eq!(rope_boundary_at_or_before(rope_text, 4), 2);
        assert_eq!(rope_boundary_at_or_after(rope_text, 4), 5);
    }

    #[test]
    fn width_uses_complete_grapheme_sequences() {
        assert_eq!(measure_width("e\u{301}", usize::MAX), 1);
        assert_eq!(measure_width("👨‍💻", usize::MAX), 2);
        assert_eq!(measure_width("🇺🇸", usize::MAX), 2);
        assert_eq!(measure_width("👍🏽", usize::MAX), 2);
        assert_eq!(measure_width("✈️", usize::MAX), 2);
        assert_eq!(measure_width("界", usize::MAX), 2);
        assert_eq!(measure_width("\tX", usize::MAX), 5);
    }

    #[test]
    fn display_columns_never_land_inside_wide_graphemes() {
        let rope = Rope::from_str("a界b");
        let text = rope.slice(..);

        assert_eq!(rope_char_index_at_column(text, 0), 0);
        assert_eq!(rope_char_index_at_column(text, 1), 1);
        assert_eq!(rope_char_index_at_column(text, 2), 1);
        assert_eq!(rope_char_index_at_column(text, 3), 2);
        assert_eq!(rope_char_index_at_column(text, 4), 3);
    }

    #[test]
    fn pop_removes_one_complete_grapheme() {
        let mut text = "e\u{301}👨‍💻".to_string();

        pop_grapheme(&mut text);
        assert_eq!(text, "e\u{301}");
        pop_grapheme(&mut text);
        assert_eq!(text, "");
    }

    #[test]
    fn rope_boundaries_keep_a_cluster_whole_across_chunks() {
        let marks = "\u{301}".repeat(2_000);
        let rope = Rope::from_str(&format!("a{marks}b\r\n"));
        let text = rope.slice(..);
        let cluster_end = 2_001;

        assert!(text.chunks().count() > 1);
        assert_eq!(next_rope_boundary(text, 0), cluster_end);
        assert_eq!(rope_boundary_at_or_before(text, 1_000), 0);
        assert_eq!(rope_boundary_at_or_after(text, 1_000), cluster_end);
        assert_eq!(previous_rope_boundary(text, cluster_end), 0);
        assert_eq!(next_rope_boundary(text, cluster_end + 1), cluster_end + 3);
    }

    #[test]
    fn rope_column_helpers_stop_after_the_visible_prefix() {
        let rope = Rope::from_str(&format!("👨‍💻{}", "x".repeat(1_000_000)));
        let text = rope.slice(..);
        let prefix = rope_prefix_for_width(text, 80);

        assert_eq!(measure_width(&prefix, usize::MAX), 80);
        assert_eq!(prefix.chars().count(), 81);
        assert_eq!(rope_char_index_at_column(text, 1), 0);
        assert_eq!(rope_char_index_at_column(text, 2), 3);
        assert_eq!(rope_char_index_at_column(text, 80), 81);
        assert_eq!(measure_rope_width(text.slice(..81), usize::MAX), 80);
    }
}
