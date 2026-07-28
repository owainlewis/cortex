use crate::{
    buffer::Buffer,
    highlighter::{HighlightKind, SyntaxHighlighter},
    renderer::{Renderer, TerminalSize},
    view::View,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    buffer: Buffer,
    directory: PathBuf,
}

impl Fixture {
    fn new(name: &str, text: &str) -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("cortex-performance-{}-{id}", std::process::id()));
        fs::create_dir(&directory).expect("create performance fixture directory");
        let path = directory.join(name);
        fs::write(&path, text).expect("write performance fixture");
        let buffer = Buffer::open(path).expect("open performance fixture");

        Self { buffer, directory }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).expect("remove performance fixture directory");
    }
}

fn measured<T>(label: &str, run: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = run();
    eprintln!("{label}: {:?}", start.elapsed());
    result
}

fn contains_kind(lines: &[Vec<crate::highlighter::HighlightSpan>], kind: HighlightKind) -> bool {
    lines
        .iter()
        .flatten()
        .any(|highlight| highlight.kind == kind)
}

fn highlight_deep_viewport(
    highlighter: &mut SyntaxHighlighter,
    buffer: &Buffer,
) -> Vec<Vec<crate::highlighter::HighlightSpan>> {
    let end = buffer.len_lines().saturating_sub(1);
    highlighter.highlight_visible_lines(buffer, end.saturating_sub(40)..end)
}

#[test]
#[ignore = "run with the documented local performance command"]
fn large_rope_edits() {
    let mut fixture = Fixture::new("large.txt", &"line\n".repeat(200_000));
    let original_len = fixture.buffer.len_chars();

    measured("large rope edits", || {
        for _ in 0..1_000 {
            let end = fixture.buffer.len_chars();
            fixture.buffer.insert(end, "x");
            assert_eq!(fixture.buffer.undo(), Some(end));

            fixture.buffer.delete(end - 1..end);
            assert_eq!(fixture.buffer.undo(), Some(end - 1));
        }
    });

    assert_eq!(fixture.buffer.len_chars(), original_len);
    assert!(!fixture.buffer.is_dirty());
}

#[test]
#[ignore = "run with the documented local performance command"]
fn large_viewport_rendering() {
    let text = (0..200_000)
        .map(|line| format!("line {line:06} with viewport text\n"))
        .collect::<String>();
    let fixture = Fixture::new("large.txt", &text);
    let renderer = Renderer::new();
    let size = TerminalSize {
        cols: 100,
        rows: 30,
    };
    let mut view = View::new();
    let mut output = Vec::new();
    let mut largest_frame = 0;

    measured("large viewport rendering", || {
        for line in (10_000..200_000).step_by(3_000) {
            view.set_point(fixture.buffer.line_start_char(line), &fixture.buffer);
            view.ensure_point_visible(
                &fixture.buffer,
                renderer.viewport_height(size),
                renderer.viewport_width(&fixture.buffer, size),
            );
            renderer
                .render(
                    &mut output,
                    &fixture.buffer,
                    &view,
                    size,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .expect("render large viewport");
            largest_frame = largest_frame.max(output.len());
            output.clear();
        }
    });

    eprintln!("largest rendered frame: {largest_frame} bytes");
    assert!(largest_frame > 0);
    assert!(
        largest_frame < 256 * 1024,
        "rendering a viewport emitted {largest_frame} bytes"
    );
}

#[test]
#[ignore = "run with the documented local performance command"]
fn large_visible_highlighting() {
    let rust = (0..20_000)
        .map(|line| format!("fn item_{line}() {{ let value = {line}; }}\n"))
        .collect::<String>();
    let markdown = (0..20_000)
        .map(|line| format!("# Heading {line}\n\nParagraph with `code_{line}`.\n"))
        .collect::<String>();
    let rust_fixture = Fixture::new("large.rs", &rust);
    let markdown_fixture = Fixture::new("large.md", &markdown);
    let mut highlighter = SyntaxHighlighter::new();

    let (rust_lines, markdown_lines) = measured("large visible highlighting", || {
        (
            highlight_deep_viewport(&mut highlighter, &rust_fixture.buffer),
            highlight_deep_viewport(&mut highlighter, &markdown_fixture.buffer),
        )
    });

    assert_eq!(rust_lines.len(), 40);
    assert_eq!(markdown_lines.len(), 40);
    assert!(contains_kind(&rust_lines, HighlightKind::Keyword));
    assert!(contains_kind(&markdown_lines, HighlightKind::MarkupHeading));
}

#[test]
#[ignore = "run with the documented local performance command"]
fn large_buffer_search() {
    let prefix = "haystack\n".repeat(200_000);
    let expected = prefix.chars().count();
    let fixture = Fixture::new("large.txt", &(prefix + "UNIQUE_NEEDLE\n"));

    measured("large buffer search", || {
        for _ in 0..25 {
            assert_eq!(
                fixture.buffer.find_forward("UNIQUE_NEEDLE", 0),
                Some(expected)
            );
            assert_eq!(
                fixture.buffer.find_forward("UNIQUE_NEEDLE", expected + 1),
                Some(expected)
            );
        }
    });
}
