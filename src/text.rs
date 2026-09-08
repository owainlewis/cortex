use std::borrow::Cow;

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

// Retain Unicode context while walking a run of graphemes. In particular, regional
// indicators need the parity of the preceding run in either direction.
struct RopeBoundaryCursor<'a> {
    text: RopeSlice<'a>,
    chunk: Cow<'a, str>,
    chunk_start: usize,
    chunk_char_start: usize,
    cursor: GraphemeCursor,
    context_bytes: usize,
}

impl<'a> RopeBoundaryCursor<'a> {
    fn new(text: RopeSlice<'a>, char_index: usize) -> Self {
        let byte_index = text.char_to_byte(char_index);
        let (chunk, chunk_start, chunk_char_start, _) = text.chunk_at_byte(byte_index);
        Self {
            text,
            chunk: Cow::Borrowed(chunk),
            chunk_start,
            chunk_char_start,
            cursor: GraphemeCursor::new(byte_index, text.len_bytes(), true),
            context_bytes: 0,
        }
    }

    fn boundary(&mut self, forward: bool) -> usize {
        loop {
            let result = if forward {
                self.cursor.next_boundary(&self.chunk, self.chunk_start)
            } else {
                self.cursor.prev_boundary(&self.chunk, self.chunk_start)
            };
            match result {
                Ok(None) => return if forward { self.text.len_chars() } else { 0 },
                Ok(Some(boundary)) => {
                    return self.chunk_char_start
                        + byte_to_char_idx(&self.chunk, boundary - self.chunk_start);
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    self.set_chunk(self.chunk_start + self.chunk.len(), true);
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    self.set_chunk(self.chunk_start - 1, false);
                }
                Err(GraphemeIncomplete::PreContext(byte_idx)) => {
                    let (context, context_start, _, _) =
                        self.text.chunk_at_byte(byte_idx.saturating_sub(1));
                    let context = &context[..byte_idx - context_start];
                    self.context_bytes += context.len();
                    self.cursor.provide_context(context, context_start);
                }
                Err(_) => unreachable!("rope chunks must cover the grapheme cursor"),
            }
        }
    }

    fn set_chunk(&mut self, byte_index: usize, overlap: bool) {
        let (chunk, chunk_start, chunk_char_start, _) = self.text.chunk_at_byte(byte_index);
        self.chunk = Cow::Borrowed(chunk);
        self.chunk_start = chunk_start;
        self.chunk_char_start = chunk_char_start;
        if overlap && chunk_char_start > 0 {
            // GraphemeCursor requests fresh RI context at an exact chunk start,
            // even with cached parity. One overlapping scalar keeps that parity
            // usable while copying only one rope chunk, never a growing prefix.
            let previous = self.text.char(chunk_char_start - 1);
            let mut joined = String::with_capacity(previous.len_utf8() + chunk.len());
            joined.push(previous);
            joined.push_str(chunk);
            self.chunk = Cow::Owned(joined);
            self.chunk_start -= previous.len_utf8();
            self.chunk_char_start -= 1;
        }
    }
}

pub(crate) fn next_rope_boundary(text: RopeSlice<'_>, char_index: usize) -> usize {
    RopeBoundaryCursor::new(text, char_index).boundary(true)
}

pub(crate) fn previous_rope_boundary(text: RopeSlice<'_>, char_index: usize) -> usize {
    RopeBoundaryCursor::new(text, char_index).boundary(false)
}

pub(crate) fn rope_word_boundary(
    text: RopeSlice<'_>,
    point: usize,
    forward: bool,
) -> (usize, usize) {
    let mut point = rope_boundary_at_or_before(text, point.min(text.len_chars()));
    let mut cursor = RopeBoundaryCursor::new(text, point);
    let mut found_word = false;
    while if forward {
        point < text.len_chars()
    } else {
        point > 0
    } {
        let next = cursor.boundary(forward);
        let word = text
            .slice(point.min(next)..point.max(next))
            .chars()
            .any(|ch| ch.is_alphanumeric() || ch == '_');
        if found_word && !word {
            break;
        }
        found_word |= word;
        point = next;
    }
    (point, cursor.context_bytes)
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn rope_char_index_at_or_after_column(
    text: RopeSlice<'_>,
    target_column: usize,
) -> (usize, usize) {
    let (char_index, column, _, _) = rope_column_at_or_after_from(text, target_column, 0);
    (char_index, column)
}

pub(crate) fn rope_column_at_or_after_from(
    text: RopeSlice<'_>,
    target_column: usize,
    initial_column: usize,
) -> (usize, usize, Vec<(usize, usize)>, usize) {
    const CHECKPOINT_GRAPHEMES: usize = 1_024;

    let mut column = initial_column;
    let mut boundary = 0;
    let mut visited = 0;
    let mut checkpoints = Vec::new();

    for grapheme in RopeGraphemes::new(text) {
        if column >= target_column {
            break;
        }
        let width = with_rope_slice_str(grapheme, |grapheme| grapheme_width(grapheme, column));
        column = column.saturating_add(width);
        boundary += grapheme.len_chars();
        visited += 1;
        if visited % CHECKPOINT_GRAPHEMES == 0 {
            checkpoints.push((boundary, column));
        }
    }

    (boundary, column, checkpoints, visited)
}

pub(crate) fn rope_column_for_char_from(
    text: RopeSlice<'_>,
    target_chars: usize,
    initial_column: usize,
) -> (usize, Vec<(usize, usize)>, usize) {
    const CHECKPOINT_GRAPHEMES: usize = 1_024;

    let mut column = initial_column;
    let mut boundary = 0;
    let mut visited = 0;
    let mut checkpoints = Vec::new();

    for grapheme in RopeGraphemes::new(text) {
        if boundary >= target_chars {
            break;
        }
        let width = with_rope_slice_str(grapheme, |grapheme| grapheme_width(grapheme, column));
        column = column.saturating_add(width);
        boundary += grapheme.len_chars();
        visited += 1;
        if visited % CHECKPOINT_GRAPHEMES == 0 {
            checkpoints.push((boundary, column));
        }
    }

    (column, checkpoints, visited)
}

pub(crate) fn rope_prefix_for_width(
    text: RopeSlice<'_>,
    max_width: usize,
    initial_column: usize,
) -> String {
    let mut prefix = String::new();
    let mut width = 0;

    for grapheme in RopeGraphemes::new(text) {
        let grapheme_width = with_rope_slice_str(grapheme, |grapheme| {
            grapheme_width(grapheme, initial_column.saturating_add(width))
        });
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
        rope_char_index_at_column, rope_char_index_at_or_after_column, rope_prefix_for_width,
        rope_word_boundary,
    };

    #[test]
    fn retained_cursor_matches_flat_graphemes_across_rope_chunks() {
        for value in [
            format!("{}end", "🇦🇧🇨".repeat(1_001)),
            "a\u{301}👨‍💻🇦🇧👍🏽\u{600}界क्ष\r\n".repeat(301),
        ] {
            let rope = Rope::from_str(&value);
            let mut boundaries: Vec<_> = grapheme_char_indices(&value)
                .map(|(start, _)| start)
                .collect();
            boundaries.push(rope.len_chars());
            let mut forward = super::RopeBoundaryCursor::new(rope.slice(..), 0);
            for expected in boundaries.iter().skip(1) {
                assert_eq!(forward.boundary(true), *expected);
            }
            let mut backward = super::RopeBoundaryCursor::new(rope.slice(..), rope.len_chars());
            for expected in boundaries.iter().rev().skip(1) {
                assert_eq!(backward.boundary(false), *expected);
            }
        }
    }

    #[test]
    fn word_traversal_keeps_regional_indicator_context_in_both_directions() {
        for count in [1_000, 4_000, 16_000] {
            let flags = "🇦🇧".repeat(count);
            for (value, point, forward, expected) in [
                (format!("{flags}end"), 0, true, count * 2 + 3),
                (format!("start{flags}"), count * 2 + 5, false, 0),
                (format!("{flags}end"), count, true, count * 2 + 3),
                (format!("start{flags}"), count + 5, false, 0),
            ] {
                let rope = Rope::from_str(&value);
                let (boundary, context_bytes) = rope_word_boundary(rope.slice(..), point, forward);
                assert_eq!(boundary, expected);
                // One initial context scan is enough. Restarting the cursor for
                // every flag makes this grow quadratically across rope chunks.
                assert!(
                    context_bytes <= rope.len_bytes() * 2,
                    "n={count} point={point} forward={forward} context={context_bytes}"
                );
            }
        }
    }

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
        assert_eq!(rope_char_index_at_or_after_column(text, 2), (2, 3));
        assert_eq!(rope_char_index_at_or_after_column(text, 3), (2, 3));
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
        let prefix = rope_prefix_for_width(text, 80, 0);

        assert_eq!(measure_width(&prefix, usize::MAX), 80);
        assert_eq!(prefix.chars().count(), 81);
        assert_eq!(rope_char_index_at_column(text, 1), 0);
        assert_eq!(rope_char_index_at_column(text, 2), 3);
        assert_eq!(rope_char_index_at_column(text, 80), 81);
        assert_eq!(measure_rope_width(text.slice(..81), usize::MAX), 80);
    }
}
