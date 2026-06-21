use crate::{
    buffer::Buffer,
    commands,
    input::key_from_event,
    keymap::{Keymap, KeymapResult},
    renderer::{Renderer, TerminalSize},
    terminal::TerminalSession,
    view::View,
};
use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal,
};
use std::{
    io,
    path::Path,
};

pub fn run(path: &Path) -> io::Result<()> {
    let mut buffer = Buffer::open(path)?;
    let mut terminal = TerminalSession::enter(io::stdout())?;
    let mut view = View::new();
    let mut keymap = Keymap::new();
    let renderer = Renderer::new();

    render(&renderer, terminal.writer_mut(), &buffer, &mut view)?;

    loop {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    let key = key_from_event(key);
                    match keymap.resolve(key) {
                        KeymapResult::Command(command) => {
                            let outcome = commands::dispatch(command, &mut buffer, &mut view);
                            if outcome.quit {
                                break;
                            }
                            render(&renderer, terminal.writer_mut(), &buffer, &mut view)?;
                        }
                        KeymapResult::PendingPrefix | KeymapResult::Unbound => {}
                    }
                }
            }
            Event::Resize(_, _) => render(&renderer, terminal.writer_mut(), &buffer, &mut view)?,
            _ => {}
        }
    }

    Ok(())
}

fn render<W: io::Write>(
    renderer: &Renderer,
    writer: &mut W,
    buffer: &Buffer,
    view: &mut View,
) -> io::Result<()> {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let size = TerminalSize { cols, rows };
    view.ensure_point_visible(buffer, renderer.viewport_height(size));
    renderer.render(writer, buffer, view, size)
}
