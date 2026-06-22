use std::{ops::Range, path::Path};
use tree_sitter_highlight::{
    HighlightConfiguration, HighlightEvent, Highlighter as TreeSitterHighlighter,
};

const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "boolean",
    "comment",
    "constant",
    "constructor",
    "function",
    "keyword",
    "markup",
    "number",
    "operator",
    "property",
    "punctuation",
    "string",
    "tag",
    "type",
    "variable",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Attribute,
    Boolean,
    Comment,
    Constant,
    Constructor,
    Function,
    Keyword,
    Markup,
    Number,
    Operator,
    Property,
    Punctuation,
    String,
    Tag,
    Type,
    Variable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub range: Range<usize>,
    pub kind: HighlightKind,
}

pub struct SyntaxHighlighter {
    highlighter: TreeSitterHighlighter,
    languages: Vec<LanguageDefinition>,
}

struct LanguageDefinition {
    extensions: &'static [&'static str],
    config: HighlightConfiguration,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let languages = [
            rust_definition(),
            markdown_definition(),
            json_definition(),
            toml_definition(),
        ]
        .into_iter()
        .flatten()
        .collect();

        Self {
            highlighter: TreeSitterHighlighter::new(),
            languages,
        }
    }

    pub fn highlight_visible_lines(
        &mut self,
        path: &Path,
        lines: &[String],
    ) -> Vec<Vec<HighlightSpan>> {
        let mut highlighted_lines = vec![Vec::new(); lines.len()];
        let Some(language_idx) = self.language_idx_for_path(path) else {
            return highlighted_lines;
        };
        let source = lines.join("\n");
        if source.is_empty() {
            return highlighted_lines;
        }

        let language = &self.languages[language_idx];
        let line_starts = line_start_offsets(lines);
        let events = self
            .highlighter
            .highlight(&language.config, source.as_bytes(), None, |_| None);
        let Ok(events) = events else {
            return highlighted_lines;
        };

        let mut highlight_stack = Vec::new();
        for event in events {
            let Ok(event) = event else {
                return vec![Vec::new(); lines.len()];
            };

            match event {
                HighlightEvent::Source { start, end } => {
                    if start < end {
                        if let Some(kind) = highlight_stack.last().copied().and_then(highlight_kind)
                        {
                            push_span(&mut highlighted_lines, &line_starts, start, end, kind);
                        }
                    }
                }
                HighlightEvent::HighlightStart(highlight) => highlight_stack.push(highlight.0),
                HighlightEvent::HighlightEnd => {
                    highlight_stack.pop();
                }
            }
        }

        highlighted_lines
    }

    fn language_idx_for_path(&self, path: &Path) -> Option<usize> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        self.languages
            .iter()
            .position(|language| language.extensions.contains(&extension.as_str()))
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn rust_definition() -> Option<LanguageDefinition> {
    language_definition(
        &["rs"],
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
    )
}

fn markdown_definition() -> Option<LanguageDefinition> {
    language_definition(
        &["md", "markdown"],
        tree_sitter_md::LANGUAGE.into(),
        "markdown",
        tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
        tree_sitter_md::INJECTION_QUERY_BLOCK,
    )
}

fn json_definition() -> Option<LanguageDefinition> {
    language_definition(
        &["json"],
        tree_sitter_json::LANGUAGE.into(),
        "json",
        tree_sitter_json::HIGHLIGHTS_QUERY,
        "",
    )
}

fn toml_definition() -> Option<LanguageDefinition> {
    language_definition(
        &["toml"],
        tree_sitter_toml_ng::LANGUAGE.into(),
        "toml",
        tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
        "",
    )
}

fn language_definition(
    extensions: &'static [&'static str],
    language: tree_sitter::Language,
    name: &str,
    highlights_query: &str,
    injection_query: &str,
) -> Option<LanguageDefinition> {
    let mut config =
        HighlightConfiguration::new(language, name, highlights_query, injection_query, "").ok()?;
    config.configure(HIGHLIGHT_NAMES);
    Some(LanguageDefinition { extensions, config })
}

fn highlight_kind(index: usize) -> Option<HighlightKind> {
    match HIGHLIGHT_NAMES.get(index).copied()? {
        "attribute" => Some(HighlightKind::Attribute),
        "boolean" => Some(HighlightKind::Boolean),
        "comment" => Some(HighlightKind::Comment),
        "constant" => Some(HighlightKind::Constant),
        "constructor" => Some(HighlightKind::Constructor),
        "function" => Some(HighlightKind::Function),
        "keyword" => Some(HighlightKind::Keyword),
        "markup" => Some(HighlightKind::Markup),
        "number" => Some(HighlightKind::Number),
        "operator" => Some(HighlightKind::Operator),
        "property" => Some(HighlightKind::Property),
        "punctuation" => Some(HighlightKind::Punctuation),
        "string" => Some(HighlightKind::String),
        "tag" => Some(HighlightKind::Tag),
        "type" => Some(HighlightKind::Type),
        "variable" => Some(HighlightKind::Variable),
        _ => None,
    }
}

fn line_start_offsets(lines: &[String]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(lines.len() + 1);
    let mut offset = 0;
    for line in lines {
        offsets.push(offset);
        offset += line.len() + 1;
    }
    offsets.push(offset);
    offsets
}

fn push_span(
    highlighted_lines: &mut [Vec<HighlightSpan>],
    line_starts: &[usize],
    start: usize,
    end: usize,
    kind: HighlightKind,
) {
    let first_line = line_for_offset(line_starts, start);
    let last_line = line_for_offset(line_starts, end.saturating_sub(1));

    for line_idx in first_line..=last_line {
        let line_start = line_starts[line_idx];
        let line_end = line_starts[line_idx + 1].saturating_sub(1);
        let span_start = start.max(line_start).saturating_sub(line_start);
        let span_end = end.min(line_end).saturating_sub(line_start);

        if span_start < span_end {
            highlighted_lines[line_idx].push(HighlightSpan {
                range: span_start..span_end,
                kind,
            });
        }
    }
}

fn line_for_offset(line_starts: &[usize], offset: usize) -> usize {
    line_starts
        .partition_point(|line_start| *line_start <= offset)
        .saturating_sub(1)
        .min(line_starts.len().saturating_sub(2))
}

#[cfg(test)]
mod tests {
    use super::{HighlightKind, SyntaxHighlighter};
    use std::path::Path;

    #[test]
    fn highlights_known_rust_files() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = vec!["fn main() {".to_string(), "    let value = 42;".to_string()];

        let highlighted = highlighter.highlight_visible_lines(Path::new("main.rs"), &lines);

        assert!(highlighted
            .iter()
            .flatten()
            .any(|span| span.kind == HighlightKind::Keyword));
        assert!(highlighted.iter().flatten().next().is_some());
    }

    #[test]
    fn highlights_required_file_types() {
        let cases = [
            (Path::new("notes.md"), vec!["# Heading".to_string()]),
            (Path::new("data.json"), vec!["{\"enabled\": true}".to_string()]),
            (Path::new("config.toml"), vec!["enabled = true".to_string()]),
        ];

        for (path, lines) in cases {
            let mut highlighter = SyntaxHighlighter::new();
            let highlighted = highlighter.highlight_visible_lines(path, &lines);

            assert!(
                highlighted.iter().flatten().next().is_some(),
                "{} should produce at least one highlight",
                path.display()
            );
        }
    }

    #[test]
    fn unknown_extensions_return_plain_lines() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = vec!["fn main() {}".to_string()];

        let highlighted = highlighter.highlight_visible_lines(Path::new("notes.unknown"), &lines);

        assert_eq!(highlighted, vec![Vec::new()]);
    }

    #[test]
    fn invalid_source_still_returns_without_panicking() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = vec!["fn {".to_string(), "\"unterminated".to_string()];

        let highlighted = highlighter.highlight_visible_lines(Path::new("broken.rs"), &lines);

        assert_eq!(highlighted.len(), 2);
    }

    #[test]
    fn large_visible_inputs_return_without_panicking() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = (0..1000)
            .map(|idx| format!("let value_{idx} = {idx};"))
            .collect::<Vec<_>>();

        let highlighted = highlighter.highlight_visible_lines(Path::new("large.rs"), &lines);

        assert_eq!(highlighted.len(), lines.len());
    }
}
