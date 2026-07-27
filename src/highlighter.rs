use crate::buffer::Buffer;
use std::{collections::HashMap, ops::Range, path::Path};
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
    "markup.bold",
    "markup.heading",
    "markup.italic",
    "markup.link",
    "markup.link.url",
    "markup.list",
    "markup.quote",
    "markup.raw",
    "markup.raw.block",
    "markup.raw.inline",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.escape",
    "tag",
    "text.emphasis",
    "text.literal",
    "text.reference",
    "text.strong",
    "text.title",
    "text.uri",
    "type",
    "variable",
];

const RUST_EXTENSIONS: &[&str] = &["rs"];
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown"];
const JSON_EXTENSIONS: &[&str] = &["json"];
const TOML_EXTENSIONS: &[&str] = &["toml"];
const PYTHON_EXTENSIONS: &[&str] = &["py", "pyw"];
const JAVASCRIPT_EXTENSIONS: &[&str] = &["js", "jsx", "mjs", "cjs"];
const TYPESCRIPT_EXTENSIONS: &[&str] = &["ts", "mts", "cts"];
const TYPESCRIPT_TSX_EXTENSIONS: &[&str] = &["tsx"];
const RUBY_EXTENSIONS: &[&str] = &["rb", "rake", "gemspec"];
const OCAML_EXTENSIONS: &[&str] = &["ml"];
const OCAML_INTERFACE_EXTENSIONS: &[&str] = &["mli"];
const HIGHLIGHT_READ_AHEAD_LINES: usize = 256;

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
    MarkupBold,
    MarkupHeading,
    MarkupItalic,
    MarkupLink,
    MarkupLinkUrl,
    MarkupList,
    MarkupQuote,
    MarkupRaw,
    MarkupRawBlock,
    MarkupRawInline,
    Number,
    Operator,
    Property,
    Punctuation,
    PunctuationDelimiter,
    PunctuationSpecial,
    String,
    StringEscape,
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
    documents: HashMap<u64, HighlightedDocument>,
    #[cfg(test)]
    document_parse_count: usize,
}

struct LanguageDefinition {
    extensions: &'static [&'static str],
    config: HighlightConfiguration,
}

struct HighlightedDocument {
    revision: u64,
    covered_lines: usize,
    lines: Vec<Vec<HighlightSpan>>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let languages = [
            rust_definition(),
            markdown_definition(),
            markdown_inline_definition(),
            json_definition(),
            toml_definition(),
            python_definition(),
            javascript_definition(),
            typescript_definition(),
            typescript_tsx_definition(),
            ruby_definition(),
            ocaml_definition(),
            ocaml_interface_definition(),
        ]
        .into_iter()
        .flatten()
        .collect();

        Self {
            highlighter: TreeSitterHighlighter::new(),
            languages,
            documents: HashMap::new(),
            #[cfg(test)]
            document_parse_count: 0,
        }
    }

    pub fn highlight_visible_lines(
        &mut self,
        buffer: &Buffer,
        line_range: Range<usize>,
    ) -> Vec<Vec<HighlightSpan>> {
        let buffer_id = buffer.id();
        let revision = buffer.revision();
        let visible_end = line_range.end.min(buffer.len_lines());
        let needs_refresh = self.documents.get(&buffer_id).is_none_or(|document| {
            document.revision != revision || document.covered_lines < visible_end
        });

        if needs_refresh {
            let covered_lines = visible_end
                .saturating_add(HIGHLIGHT_READ_AHEAD_LINES)
                .min(buffer.len_lines());
            let highlighted_lines =
                self.highlight_document(buffer.path(), &buffer.text_prefix_lines(covered_lines));
            self.documents.insert(
                buffer_id,
                HighlightedDocument {
                    revision,
                    covered_lines,
                    lines: highlighted_lines,
                },
            );
        }

        let document = &self.documents[&buffer_id];
        document.lines
            [line_range.start.min(document.lines.len())..line_range.end.min(document.lines.len())]
            .to_vec()
    }

    fn highlight_document(&mut self, path: &Path, source: &str) -> Vec<Vec<HighlightSpan>> {
        #[cfg(test)]
        {
            self.document_parse_count += 1;
        }

        let lines = document_lines(source);
        let mut highlighted_lines = vec![Vec::new(); lines.len()];
        let Some(language_idx) = self.language_idx_for_path(path) else {
            return highlighted_lines;
        };
        let source = lines.join("\n");
        if source.is_empty() {
            return highlighted_lines;
        }

        let languages = &self.languages;
        let language = &languages[language_idx];
        let line_starts = line_start_offsets(&lines);
        {
            let events =
                self.highlighter
                    .highlight(&language.config, source.as_bytes(), None, |name| {
                        language_config_for_name(languages, name)
                    });
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
                            if let Some(kind) =
                                highlight_stack.last().copied().and_then(highlight_kind)
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
        }

        if is_markdown_path(path) {
            self.highlight_markdown_inline_lines(&lines, &mut highlighted_lines);
        }

        highlighted_lines
    }

    fn language_idx_for_path(&self, path: &Path) -> Option<usize> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        self.languages
            .iter()
            .position(|language| language.extensions.contains(&extension.as_str()))
    }

    fn language_idx_for_name(&self, name: &str) -> Option<usize> {
        self.languages
            .iter()
            .position(|language| language.config.language_name == name)
    }

    fn highlight_markdown_inline_lines(
        &mut self,
        lines: &[String],
        highlighted_lines: &mut [Vec<HighlightSpan>],
    ) {
        let Some(language_idx) = self.language_idx_for_name("markdown_inline") else {
            return;
        };

        let languages = &self.languages;
        let language = &languages[language_idx];

        for (line_idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }

            let events =
                self.highlighter
                    .highlight(&language.config, line.as_bytes(), None, |name| {
                        language_config_for_name(languages, name)
                    });
            let Ok(events) = events else {
                continue;
            };

            let mut highlight_stack = Vec::new();
            for event in events {
                let Ok(event) = event else {
                    break;
                };

                match event {
                    HighlightEvent::Source { start, end } => {
                        if start < end {
                            if let Some(kind) =
                                highlight_stack.last().copied().and_then(highlight_kind)
                            {
                                highlighted_lines[line_idx].push(HighlightSpan {
                                    range: start..end,
                                    kind,
                                });
                            }
                        }
                    }
                    HighlightEvent::HighlightStart(highlight) => highlight_stack.push(highlight.0),
                    HighlightEvent::HighlightEnd => {
                        highlight_stack.pop();
                    }
                }
            }
        }
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

pub fn language_label_for_path(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();

    [
        ("RUST", RUST_EXTENSIONS),
        ("MARKDOWN", MARKDOWN_EXTENSIONS),
        ("JSON", JSON_EXTENSIONS),
        ("TOML", TOML_EXTENSIONS),
        ("PYTHON", PYTHON_EXTENSIONS),
        ("JAVASCRIPT", JAVASCRIPT_EXTENSIONS),
        ("TYPESCRIPT", TYPESCRIPT_EXTENSIONS),
        ("TYPESCRIPT", TYPESCRIPT_TSX_EXTENSIONS),
        ("RUBY", RUBY_EXTENSIONS),
        ("OCAML", OCAML_EXTENSIONS),
        ("OCAML", OCAML_INTERFACE_EXTENSIONS),
    ]
    .into_iter()
    .find(|(_, extensions)| extensions.contains(&extension.as_str()))
    .map(|(label, _)| label)
}

fn rust_definition() -> Option<LanguageDefinition> {
    language_definition(
        RUST_EXTENSIONS,
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
    )
}

fn markdown_definition() -> Option<LanguageDefinition> {
    language_definition(
        MARKDOWN_EXTENSIONS,
        tree_sitter_md::LANGUAGE.into(),
        "markdown",
        tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
        tree_sitter_md::INJECTION_QUERY_BLOCK,
    )
}

fn markdown_inline_definition() -> Option<LanguageDefinition> {
    language_definition(
        &[],
        tree_sitter_md::INLINE_LANGUAGE.into(),
        "markdown_inline",
        tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
        tree_sitter_md::INJECTION_QUERY_INLINE,
    )
}

fn json_definition() -> Option<LanguageDefinition> {
    language_definition(
        JSON_EXTENSIONS,
        tree_sitter_json::LANGUAGE.into(),
        "json",
        tree_sitter_json::HIGHLIGHTS_QUERY,
        "",
    )
}

fn toml_definition() -> Option<LanguageDefinition> {
    language_definition(
        TOML_EXTENSIONS,
        tree_sitter_toml_ng::LANGUAGE.into(),
        "toml",
        tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
        "",
    )
}

fn python_definition() -> Option<LanguageDefinition> {
    language_definition(
        PYTHON_EXTENSIONS,
        tree_sitter_python::LANGUAGE.into(),
        "python",
        tree_sitter_python::HIGHLIGHTS_QUERY,
        "",
    )
}

fn javascript_definition() -> Option<LanguageDefinition> {
    let highlights = format!(
        "{}\n{}",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
    );

    language_definition(
        JAVASCRIPT_EXTENSIONS,
        tree_sitter_javascript::LANGUAGE.into(),
        "javascript",
        &highlights,
        tree_sitter_javascript::INJECTIONS_QUERY,
    )
}

fn typescript_definition() -> Option<LanguageDefinition> {
    language_definition(
        TYPESCRIPT_EXTENSIONS,
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "typescript",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
    )
}

fn typescript_tsx_definition() -> Option<LanguageDefinition> {
    language_definition(
        TYPESCRIPT_TSX_EXTENSIONS,
        tree_sitter_typescript::LANGUAGE_TSX.into(),
        "tsx",
        tree_sitter_typescript::HIGHLIGHTS_QUERY,
        "",
    )
}

fn ruby_definition() -> Option<LanguageDefinition> {
    language_definition(
        RUBY_EXTENSIONS,
        tree_sitter_ruby::LANGUAGE.into(),
        "ruby",
        tree_sitter_ruby::HIGHLIGHTS_QUERY,
        "",
    )
}

fn ocaml_definition() -> Option<LanguageDefinition> {
    language_definition(
        OCAML_EXTENSIONS,
        tree_sitter_ocaml::LANGUAGE_OCAML.into(),
        "ocaml",
        tree_sitter_ocaml::HIGHLIGHTS_QUERY,
        "",
    )
}

fn ocaml_interface_definition() -> Option<LanguageDefinition> {
    // The shared OCaml highlights query references a `shebang` node that exists
    // only in the implementation grammar, so strip it for interface (`.mli`) files.
    let highlights = tree_sitter_ocaml::HIGHLIGHTS_QUERY.replace("(shebang)", "");

    language_definition(
        OCAML_INTERFACE_EXTENSIONS,
        tree_sitter_ocaml::LANGUAGE_OCAML_INTERFACE.into(),
        "ocaml_interface",
        &highlights,
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
        "markup.bold" | "text.strong" => Some(HighlightKind::MarkupBold),
        "markup.heading" | "text.title" => Some(HighlightKind::MarkupHeading),
        "markup.italic" | "text.emphasis" => Some(HighlightKind::MarkupItalic),
        "markup.link" | "text.reference" => Some(HighlightKind::MarkupLink),
        "markup.link.url" | "text.uri" => Some(HighlightKind::MarkupLinkUrl),
        "markup.list" => Some(HighlightKind::MarkupList),
        "markup.quote" => Some(HighlightKind::MarkupQuote),
        "markup.raw" | "text.literal" => Some(HighlightKind::MarkupRaw),
        "markup.raw.block" => Some(HighlightKind::MarkupRawBlock),
        "markup.raw.inline" => Some(HighlightKind::MarkupRawInline),
        "number" => Some(HighlightKind::Number),
        "operator" => Some(HighlightKind::Operator),
        "property" => Some(HighlightKind::Property),
        "punctuation" => Some(HighlightKind::Punctuation),
        "punctuation.delimiter" => Some(HighlightKind::PunctuationDelimiter),
        "punctuation.special" => Some(HighlightKind::PunctuationSpecial),
        "string" => Some(HighlightKind::String),
        "string.escape" => Some(HighlightKind::StringEscape),
        "tag" => Some(HighlightKind::Tag),
        "type" => Some(HighlightKind::Type),
        "variable" => Some(HighlightKind::Variable),
        _ => None,
    }
}

fn language_config_for_name<'a>(
    languages: &'a [LanguageDefinition],
    name: &str,
) -> Option<&'a HighlightConfiguration> {
    languages
        .iter()
        .find(|language| language.config.language_name == name)
        .map(|language| &language.config)
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn document_lines(source: &str) -> Vec<String> {
    source
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect()
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
    use super::{language_label_for_path, HighlightKind, SyntaxHighlighter};
    use crate::buffer::Buffer;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn highlights_known_rust_files() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = ["fn main() {".to_string(), "    let value = 42;".to_string()];

        let highlighted = highlighter.highlight_document(Path::new("main.rs"), &lines.join("\n"));

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
            (
                Path::new("data.json"),
                vec!["{\"enabled\": true}".to_string()],
            ),
            (Path::new("config.toml"), vec!["enabled = true".to_string()]),
        ];

        for (path, lines) in cases {
            let mut highlighter = SyntaxHighlighter::new();
            let highlighted = highlighter.highlight_document(path, &lines.join("\n"));

            assert!(
                highlighted.iter().flatten().next().is_some(),
                "{} should produce at least one highlight",
                path.display()
            );
        }
    }

    #[test]
    fn language_labels_follow_highlighted_file_types() {
        assert_eq!(language_label_for_path(Path::new("main.rs")), Some("RUST"));
        assert_eq!(
            language_label_for_path(Path::new("notes.markdown")),
            Some("MARKDOWN")
        );
        assert_eq!(
            language_label_for_path(Path::new("data.json")),
            Some("JSON")
        );
        assert_eq!(
            language_label_for_path(Path::new("config.toml")),
            Some("TOML")
        );
        assert_eq!(language_label_for_path(Path::new("app.py")), Some("PYTHON"));
        assert_eq!(
            language_label_for_path(Path::new("index.jsx")),
            Some("JAVASCRIPT")
        );
        assert_eq!(
            language_label_for_path(Path::new("app.tsx")),
            Some("TYPESCRIPT")
        );
        assert_eq!(
            language_label_for_path(Path::new("task.rake")),
            Some("RUBY")
        );
        assert_eq!(language_label_for_path(Path::new("lib.ml")), Some("OCAML"));
        assert_eq!(language_label_for_path(Path::new("lib.mli")), Some("OCAML"));
        assert_eq!(language_label_for_path(Path::new("notes.txt")), None);
    }

    #[test]
    fn highlights_primary_language_files() {
        let cases = [
            (
                Path::new("app.py"),
                vec![
                    "def greet(name):".to_string(),
                    "    return f\"hi {name}\"".to_string(),
                ],
            ),
            (
                Path::new("index.js"),
                vec![
                    "export function greet(name) {".to_string(),
                    "  return `hi ${name}`;".to_string(),
                ],
            ),
            (
                Path::new("component.jsx"),
                vec![
                    "export function Button() {".to_string(),
                    "  return <button>Save</button>;".to_string(),
                ],
            ),
            (
                Path::new("app.ts"),
                vec![
                    "type User = { name: string };".to_string(),
                    "const user: User = { name: \"Ada\" };".to_string(),
                ],
            ),
            (
                Path::new("component.tsx"),
                vec![
                    "type Props = { label: string };".to_string(),
                    "export const Button = ({ label }: Props) => <button>{label}</button>;"
                        .to_string(),
                ],
            ),
            (
                Path::new("app.rb"),
                vec![
                    "def greet(name)".to_string(),
                    "  \"hi #{name}\"".to_string(),
                    "end".to_string(),
                ],
            ),
            (
                Path::new("lib.ml"),
                vec![
                    "let greet name =".to_string(),
                    "  Printf.sprintf \"hi %s\" name".to_string(),
                ],
            ),
            (
                Path::new("lib.mli"),
                vec!["val greet : string -> string".to_string()],
            ),
        ];

        for (path, lines) in cases {
            let mut highlighter = SyntaxHighlighter::new();
            let highlighted = highlighter.highlight_document(path, &lines.join("\n"));

            assert!(
                highlighted.iter().flatten().next().is_some(),
                "{} should produce at least one highlight",
                path.display()
            );
        }
    }

    #[test]
    fn highlights_markdown_document_structure() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = [
            "# Heading".to_string(),
            "> quoted".to_string(),
            "- item".to_string(),
        ];

        let highlighted = highlighter.highlight_document(Path::new("notes.md"), &lines.join("\n"));

        assert!(line_has_kind(&highlighted[0], HighlightKind::MarkupHeading));
        assert!(line_has_kind(
            &highlighted[0],
            HighlightKind::PunctuationSpecial
        ));
        assert!(line_has_kind(
            &highlighted[1],
            HighlightKind::PunctuationSpecial
        ));
        assert!(line_has_kind(
            &highlighted[2],
            HighlightKind::PunctuationSpecial
        ));
    }

    #[test]
    fn highlights_markdown_inline_markup() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = ["Use **bold**, *em*, `code`, and [link](https://example.com).".to_string()];

        let highlighted = highlighter.highlight_document(Path::new("notes.md"), &lines.join("\n"));

        assert!(line_has_kind(&highlighted[0], HighlightKind::MarkupBold));
        assert!(line_has_kind(&highlighted[0], HighlightKind::MarkupItalic));
        assert!(line_has_kind(&highlighted[0], HighlightKind::MarkupRaw));
        assert!(line_has_kind(&highlighted[0], HighlightKind::MarkupLink));
        assert!(line_has_kind(&highlighted[0], HighlightKind::MarkupLinkUrl));
    }

    #[test]
    fn highlights_markdown_fenced_code_by_language() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = [
            "```rust".to_string(),
            "fn main() {}".to_string(),
            "```".to_string(),
        ];

        let highlighted = highlighter.highlight_document(Path::new("notes.md"), &lines.join("\n"));

        assert!(line_has_kind(&highlighted[1], HighlightKind::Keyword));
    }

    #[test]
    fn viewport_inside_rust_block_comment_keeps_document_context() {
        let (buffer, dir) = buffer_with_text(
            "comment.rs",
            "fn main() {\n    /* opener\n    comment one\n    comment two\n    */\n}\n",
        );
        let mut highlighter = SyntaxHighlighter::new();

        let highlighted = highlighter.highlight_visible_lines(&buffer, 2..4);

        assert_eq!(highlighted.len(), 2);
        assert!(highlighted
            .iter()
            .all(|line| line_has_kind(line, HighlightKind::Comment)));
        remove_dir(dir);
    }

    #[test]
    fn viewport_inside_rust_multiline_string_keeps_document_context() {
        let (buffer, dir) = buffer_with_text(
            "string.rs",
            "fn main() {\n    let text = \"first\nsecond\nthird\";\n}\n",
        );
        let mut highlighter = SyntaxHighlighter::new();

        let highlighted = highlighter.highlight_visible_lines(&buffer, 2..3);

        assert_eq!(highlighted.len(), 1);
        assert!(line_has_kind(&highlighted[0], HighlightKind::String));
        remove_dir(dir);
    }

    #[test]
    fn viewport_inside_markdown_fence_keeps_injected_language_context() {
        let (buffer, dir) = buffer_with_text(
            "notes.md",
            "Before\n\n```rust\nfn main() {\n    let answer = 42;\n}\n```\n",
        );
        let mut highlighter = SyntaxHighlighter::new();

        let highlighted = highlighter.highlight_visible_lines(&buffer, 3..6);

        assert_eq!(highlighted.len(), 3);
        assert!(line_has_kind(&highlighted[0], HighlightKind::Keyword));
        remove_dir(dir);
    }

    #[test]
    fn scrolling_reuses_cached_document_highlights() {
        let (buffer, dir) = buffer_with_text("cached.rs", "fn main() {\n    let answer = 42;\n}\n");
        let mut highlighter = SyntaxHighlighter::new();

        highlighter.highlight_visible_lines(&buffer, 0..2);
        highlighter.highlight_visible_lines(&buffer, 1..3);

        assert_eq!(highlighter.document_parse_count, 1);
        remove_dir(dir);
    }

    #[test]
    fn editing_invalidates_only_the_changed_buffer_cache() {
        let (mut rust_buffer, rust_dir) =
            buffer_with_text("cached.rs", "fn main() {\n    let answer = 42;\n}\n");
        let (markdown_buffer, markdown_dir) =
            buffer_with_text("cached.md", "# Heading\n\nParagraph\n");
        let mut highlighter = SyntaxHighlighter::new();

        highlighter.highlight_visible_lines(&rust_buffer, 0..2);
        highlighter.highlight_visible_lines(&markdown_buffer, 0..2);
        rust_buffer.insert(0, "// changed\n");
        highlighter.highlight_visible_lines(&rust_buffer, 0..2);
        highlighter.highlight_visible_lines(&markdown_buffer, 1..3);

        assert_eq!(highlighter.document_parse_count, 3);
        remove_dir(rust_dir);
        remove_dir(markdown_dir);
    }

    #[test]
    fn independent_buffers_for_the_same_path_do_not_share_cached_spans() {
        let (rust_buffer, dir) = buffer_with_text("same.rs", "fn main() {}\n");
        let mut highlighter = SyntaxHighlighter::new();
        let rust_highlights = highlighter.highlight_visible_lines(&rust_buffer, 0..1);

        fs::write(rust_buffer.path(), "plain words\n").unwrap();
        let plain_buffer = Buffer::open(rust_buffer.path()).unwrap();
        let plain_highlights = highlighter.highlight_visible_lines(&plain_buffer, 0..1);

        assert_ne!(rust_buffer.id(), plain_buffer.id());
        assert!(line_has_kind(&rust_highlights[0], HighlightKind::Keyword));
        assert!(!line_has_kind(&plain_highlights[0], HighlightKind::Keyword));
        assert_eq!(highlighter.document_parse_count, 2);
        remove_dir(dir);
    }

    #[test]
    fn editing_near_start_of_large_buffer_keeps_highlight_work_bounded() {
        let text = (0..5000)
            .map(|idx| format!("let value_{idx} = {idx};"))
            .collect::<Vec<_>>()
            .join("\n");
        let (mut buffer, dir) = buffer_with_text("large.rs", &text);
        let mut highlighter = SyntaxHighlighter::new();

        highlighter.highlight_visible_lines(&buffer, 0..20);
        let document = &highlighter.documents[&buffer.id()];
        assert_eq!(
            document.covered_lines,
            20 + super::HIGHLIGHT_READ_AHEAD_LINES
        );
        assert!(document.lines.len() <= document.covered_lines + 1);

        buffer.insert(0, "// edit\n");
        highlighter.highlight_visible_lines(&buffer, 0..20);

        assert_eq!(highlighter.document_parse_count, 2);
        assert_eq!(
            highlighter.documents[&buffer.id()].covered_lines,
            20 + super::HIGHLIGHT_READ_AHEAD_LINES
        );
        remove_dir(dir);
    }

    #[test]
    fn unknown_extensions_return_plain_lines() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = ["fn main() {}".to_string()];

        let highlighted =
            highlighter.highlight_document(Path::new("notes.unknown"), &lines.join("\n"));

        assert_eq!(highlighted, vec![Vec::new()]);
    }

    #[test]
    fn invalid_source_still_returns_without_panicking() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = ["fn {".to_string(), "\"unterminated".to_string()];

        let highlighted = highlighter.highlight_document(Path::new("broken.rs"), &lines.join("\n"));

        assert_eq!(highlighted.len(), 2);
    }

    #[test]
    fn large_visible_inputs_return_without_panicking() {
        let mut highlighter = SyntaxHighlighter::new();
        let lines = (0..1000)
            .map(|idx| format!("let value_{idx} = {idx};"))
            .collect::<Vec<_>>();

        let highlighted = highlighter.highlight_document(Path::new("large.rs"), &lines.join("\n"));

        assert_eq!(highlighted.len(), lines.len());
    }

    fn line_has_kind(spans: &[super::HighlightSpan], kind: HighlightKind) -> bool {
        spans.iter().any(|span| span.kind == kind)
    }

    fn buffer_with_text(file_name: &str, text: &str) -> (Buffer, PathBuf) {
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cortex-highlighter-test-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(file_name);
        fs::write(&path, text).unwrap();
        (Buffer::open(path).unwrap(), dir)
    }

    fn remove_dir(path: PathBuf) {
        fs::remove_dir_all(path).unwrap();
    }
}
