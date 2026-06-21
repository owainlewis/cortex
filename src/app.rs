use crate::terminal::TerminalSession;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    queue,
    style::Print,
    terminal::{self, ClearType},
};
use std::{
    io::{self, Write},
    path::Path,
};

pub fn run(path: &Path) -> io::Result<()> {
    let mut terminal = TerminalSession::enter(io::stdout())?;

    draw(terminal.writer_mut(), path)?;

    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
                && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
            {
                break;
            }
        }
    }

    Ok(())
}

fn draw<W: Write>(writer: &mut W, path: &Path) -> io::Result<()> {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let width = cols.saturating_sub(1) as usize;
    let last_row = rows.saturating_sub(1);

    queue!(
        writer,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        Print(fit_line("Cortex v0.1", width)),
        cursor::MoveTo(0, 2),
        Print(fit_line(&format!("File: {}", path.display()), width)),
        cursor::MoveTo(0, 4),
        Print(fit_line("Terminal lifecycle shell. Press q or Esc to quit.", width)),
        cursor::MoveTo(0, last_row),
        Print(fit_line("raw mode + alternate screen active", width)),
    )?;
    writer.flush()
}

fn fit_line(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::fit_line;

    #[test]
    fn fit_line_truncates_to_terminal_width() {
        assert_eq!(fit_line("abcdef", 3), "abc");
    }

    #[test]
    fn fit_line_allows_empty_width() {
        assert_eq!(fit_line("abcdef", 0), "");
    }
}
