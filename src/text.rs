use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(crate) fn grapheme_char_indices(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut char_index = 0;
    text.graphemes(true).map(move |grapheme| {
        let start = char_index;
        char_index += grapheme.chars().count();
        (start, grapheme)
    })
}

pub(crate) fn boundary_at_or_before(text: &str, char_index: usize) -> usize {
    let mut boundary = 0;

    for (start, grapheme) in grapheme_char_indices(text) {
        if start > char_index {
            break;
        }
        let end = start + grapheme.chars().count();
        if char_index < end {
            return start;
        }
        boundary = end;
    }

    boundary
}

pub(crate) fn boundary_at_or_after(text: &str, char_index: usize) -> usize {
    for (start, grapheme) in grapheme_char_indices(text) {
        let end = start + grapheme.chars().count();
        if char_index <= start {
            return start;
        }
        if char_index < end {
            return end;
        }
    }

    text.chars().count()
}

pub(crate) fn next_boundary(text: &str, char_index: usize) -> usize {
    grapheme_char_indices(text)
        .map(|(start, grapheme)| start + grapheme.chars().count())
        .find(|end| *end > char_index)
        .unwrap_or_else(|| text.chars().count())
}

pub(crate) fn previous_boundary(text: &str, char_index: usize) -> usize {
    grapheme_char_indices(text)
        .map(|(start, _)| start)
        .take_while(|start| *start < char_index)
        .last()
        .unwrap_or(0)
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

pub(crate) fn char_index_at_column(text: &str, target_column: usize) -> usize {
    let mut column: usize = 0;
    let mut boundary = 0;

    for (start, grapheme) in grapheme_char_indices(text) {
        let next_column = column.saturating_add(grapheme_width(grapheme, column));
        if next_column > target_column {
            return start;
        }
        column = next_column;
        boundary = start + grapheme.chars().count();
    }

    boundary
}

pub(crate) fn pop_grapheme(text: &mut String) {
    if let Some((byte_index, _)) = text.grapheme_indices(true).next_back() {
        text.truncate(byte_index);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        boundary_at_or_after, boundary_at_or_before, char_index_at_column, grapheme_char_indices,
        measure_width, next_boundary, pop_grapheme, previous_boundary,
    };

    #[test]
    fn boundaries_keep_common_extended_graphemes_whole() {
        let text = "e\u{301}👨‍💻🇺🇸👍🏽✈️界";
        let ranges: Vec<_> = grapheme_char_indices(text)
            .map(|(start, grapheme)| (start, start + grapheme.chars().count()))
            .collect();

        assert_eq!(
            ranges,
            vec![(0, 2), (2, 5), (5, 7), (7, 9), (9, 11), (11, 12)]
        );
        assert_eq!(next_boundary(text, 0), 2);
        assert_eq!(previous_boundary(text, 5), 2);
        assert_eq!(boundary_at_or_before(text, 4), 2);
        assert_eq!(boundary_at_or_after(text, 4), 5);
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
        let text = "a界b";

        assert_eq!(char_index_at_column(text, 0), 0);
        assert_eq!(char_index_at_column(text, 1), 1);
        assert_eq!(char_index_at_column(text, 2), 1);
        assert_eq!(char_index_at_column(text, 3), 2);
        assert_eq!(char_index_at_column(text, 4), 3);
    }

    #[test]
    fn pop_removes_one_complete_grapheme() {
        let mut text = "e\u{301}👨‍💻".to_string();

        pop_grapheme(&mut text);
        assert_eq!(text, "e\u{301}");
        pop_grapheme(&mut text);
        assert_eq!(text, "");
    }
}
