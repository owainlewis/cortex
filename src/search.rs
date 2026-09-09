use std::ops::Range;

use ropey::RopeSlice;

use crate::{buffer::Buffer, view::View};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchMatch {
    pub range: Range<usize>,
    pub wrapped: bool,
}

// KMP streams the rope in either direction. Its only retained text is the query.
struct LiteralPattern {
    chars: Vec<char>,
    fallback: Vec<usize>,
    direction: Direction,
}

impl LiteralPattern {
    fn new(query: &str, direction: Direction) -> Self {
        let mut chars: Vec<_> = query.chars().collect();
        if direction == Direction::Backward {
            chars.reverse();
        }
        let mut fallback = vec![0; chars.len()];
        let mut matched = 0;
        for index in 1..chars.len() {
            while matched > 0 && chars[matched] != chars[index] {
                matched = fallback[matched - 1];
            }
            if chars[matched] == chars[index] {
                matched += 1;
            }
            fallback[index] = matched;
        }
        Self {
            chars,
            fallback,
            direction,
        }
    }

    fn find_in(&self, text: RopeSlice<'_>, range: Range<usize>) -> (Option<Range<usize>>, usize) {
        if self.chars.is_empty() || range.len() < self.chars.len() {
            return (None, 0);
        }
        let chars = match self.direction {
            Direction::Forward => text.chars_at(range.start),
            Direction::Backward => text.chars_at(range.end).reversed(),
        };
        let mut matched = 0;
        for (offset, ch) in chars.take(range.len()).enumerate() {
            while matched > 0 && self.chars[matched] != ch {
                matched = self.fallback[matched - 1];
            }
            if self.chars[matched] == ch {
                matched += 1;
            }
            if matched == self.chars.len() {
                let start = match self.direction {
                    Direction::Forward => range.start + offset + 1 - matched,
                    Direction::Backward => range.end - offset - 1,
                };
                return (Some(start..start + matched), offset + 1);
            }
        }
        (None, range.len())
    }
}

pub(crate) fn find_literal(
    text: RopeSlice<'_>,
    query: &str,
    start: usize,
    direction: Direction,
) -> Option<SearchMatch> {
    let pattern = LiteralPattern::new(query, direction);
    let start = start.min(text.len_chars());
    let range = match direction {
        Direction::Forward => start..text.len_chars(),
        Direction::Backward => {
            0..start
                .saturating_add(pattern.chars.len())
                .min(text.len_chars())
        }
    };
    if let (Some(range), _) = pattern.find_in(text, range) {
        return Some(SearchMatch {
            range,
            wrapped: false,
        });
    }
    pattern
        .find_in(text, 0..text.len_chars())
        .0
        .map(|range| SearchMatch {
            range,
            wrapped: true,
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IncrementalSearch {
    pub query: String,
    pub direction: Direction,
    pub original_view: View,
    pub active_match: Option<SearchMatch>,
    anchor: usize,
}

impl IncrementalSearch {
    pub fn new(direction: Direction, view: &View) -> Self {
        Self {
            query: String::new(),
            direction,
            original_view: view.clone(),
            active_match: None,
            anchor: view.point(),
        }
    }

    pub fn refresh(&mut self, buffer: &Buffer, view: &mut View) {
        if self.query.is_empty() {
            self.active_match = None;
            self.anchor = self.original_view.point();
            *view = self.original_view.clone();
            return;
        }
        self.active_match = buffer.find_literal(&self.query, self.anchor, self.direction);
        if let Some(found) = &self.active_match {
            view.set_point(found.range.start, buffer);
        }
    }

    pub fn repeat(
        &mut self,
        direction: Direction,
        last_query: Option<&str>,
        buffer: &Buffer,
        view: &mut View,
    ) {
        self.direction = direction;
        if self.query.is_empty() {
            self.query = last_query.unwrap_or("").to_string();
            self.refresh(buffer, view);
            return;
        }
        let mut forced_wrap = false;
        if let Some(found) = &self.active_match {
            self.anchor = match direction {
                Direction::Forward => found.range.start + 1,
                Direction::Backward => found.range.start.checked_sub(1).unwrap_or_else(|| {
                    forced_wrap = true;
                    buffer.len_chars()
                }),
            };
        }
        self.refresh(buffer, view);
        if let Some(found) = &mut self.active_match {
            found.wrapped |= forced_wrap;
            self.anchor = found.range.start;
        }
    }

    pub fn prompt(&self) -> String {
        let direction = match self.direction {
            Direction::Forward => "I-search",
            Direction::Backward => "I-search backward",
        };
        let state = match &self.active_match {
            None if !self.query.is_empty() => " [no match]",
            Some(found) if found.wrapped => " [wrapped]",
            _ => "",
        };
        format!("{direction}{state}: {}", self.query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn literal_matches_agree_with_flat_text_in_both_directions() {
        for value in [
            "α café α café e\u{301}👨‍💻",
            "aaaaabaaaabaa",
            "",
            "a\r\nb\n",
            "界界界",
        ] {
            let chars: Vec<_> = value.chars().collect();
            let rope = Rope::from_str(value);
            for query in [
                "α",
                "café",
                "\u{301}",
                "👨‍💻",
                "aaa",
                "aaab",
                "\r\nb",
                "界界",
                "missing",
                "",
            ] {
                let query_chars: Vec<_> = query.chars().collect();
                let matches: Vec<_> = if query.is_empty() {
                    vec![]
                } else {
                    chars
                        .windows(query_chars.len())
                        .enumerate()
                        .filter(|(_, window)| *window == query_chars)
                        .map(|(index, _)| index)
                        .collect()
                };
                for start in 0..=chars.len() {
                    for direction in [Direction::Forward, Direction::Backward] {
                        let direct = match direction {
                            Direction::Forward => {
                                matches.iter().copied().find(|index| *index >= start)
                            }
                            Direction::Backward => {
                                matches.iter().copied().rev().find(|index| *index <= start)
                            }
                        };
                        let wrapped = match direction {
                            Direction::Forward => matches.first().copied(),
                            Direction::Backward => matches.last().copied(),
                        };
                        let expected = direct.or(wrapped).map(|index| SearchMatch {
                            range: index..index + query_chars.len(),
                            wrapped: direct.is_none(),
                        });
                        assert_eq!(
                            find_literal(rope.slice(..), query, start, direction),
                            expected,
                            "{value:?}, {query:?}, {start}, {direction:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn literal_matching_crosses_rope_chunks_and_stops_after_nearby_matches() {
        let rope = Rope::from_str(&format!(
            "{}café needle{}",
            "界".repeat(1_000),
            "x".repeat(2_000_000)
        ));
        let query = format!("{}café", "界".repeat(999));
        assert_eq!(
            find_literal(rope.slice(..), &query, 0, Direction::Forward)
                .unwrap()
                .range,
            1..1_004
        );
        for direction in [Direction::Forward, Direction::Backward] {
            let pattern = LiteralPattern::new("needle", direction);
            let range = match direction {
                Direction::Forward => 1_005..rope.len_chars(),
                Direction::Backward => 0..1_011,
            };
            let (found, visited) = pattern.find_in(rope.slice(..), range);
            assert_eq!(found, Some(1_005..1_011));
            assert_eq!(visited, 6);
            assert_eq!(pattern.chars.len(), 6);
            assert_eq!(pattern.fallback.len(), 6);
        }
    }
}
