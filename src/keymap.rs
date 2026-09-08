use crate::{commands::Command, input::Key};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keymap {
    pending_prefix: Option<Prefix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefix {
    CtrlX,
    CtrlC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapResult {
    Command(Command),
    PendingPrefix,
    Unbound,
}

impl Keymap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending_label(&self) -> Option<&'static str> {
        self.pending_prefix.map(|prefix| match prefix {
            Prefix::CtrlX => "C-x",
            Prefix::CtrlC => "C-c",
        })
    }

    pub fn clipboard_prefix_pending(&self) -> bool {
        self.pending_prefix == Some(Prefix::CtrlC)
    }

    pub fn resolve(&mut self, key: Key) -> KeymapResult {
        if let Some(prefix) = self.pending_prefix.take() {
            return resolve_prefixed(prefix, key);
        }

        match key {
            Key::Ctrl('x') => {
                self.pending_prefix = Some(Prefix::CtrlX);
                KeymapResult::PendingPrefix
            }
            Key::Ctrl('c') => {
                self.pending_prefix = Some(Prefix::CtrlC);
                KeymapResult::PendingPrefix
            }
            Key::Meta('w') => KeymapResult::Command(Command::CopyRegion),
            Key::Char(ch) => KeymapResult::Command(Command::Insert(ch)),
            Key::Enter => KeymapResult::Command(Command::InsertNewline),
            Key::Tab => KeymapResult::Command(Command::Indent),
            Key::BackTab => KeymapResult::Command(Command::Outdent),
            Key::Backspace => KeymapResult::Command(Command::DeleteBackward),
            Key::Delete | Key::Ctrl('d') => KeymapResult::Command(Command::DeleteForward),
            Key::Ctrl('k') => KeymapResult::Command(Command::KillLine),
            Key::Ctrl('w') => KeymapResult::Command(Command::KillRegion),
            Key::Ctrl('y') => KeymapResult::Command(Command::Yank),
            Key::Meta('f') => KeymapResult::Command(Command::MoveForwardWord),
            Key::Meta('b') => KeymapResult::Command(Command::MoveBackwardWord),
            Key::Meta('<') => KeymapResult::Command(Command::MoveToBufferStart),
            Key::Meta('>') => KeymapResult::Command(Command::MoveToBufferEnd),
            Key::Meta('d') => KeymapResult::Command(Command::KillWord),
            Key::MetaBackspace => KeymapResult::Command(Command::BackwardKillWord),
            Key::Ctrl('v') | Key::PageDown => KeymapResult::Command(Command::PageDown),
            Key::Meta('v') | Key::PageUp => KeymapResult::Command(Command::PageUp),
            Key::Meta('y') => KeymapResult::Command(Command::YankPop),
            Key::Right | Key::Ctrl('f') => KeymapResult::Command(Command::MoveForwardChar),
            Key::Left | Key::Ctrl('b') => KeymapResult::Command(Command::MoveBackwardChar),
            Key::Down | Key::Ctrl('n') => KeymapResult::Command(Command::MoveNextLine),
            Key::Up | Key::Ctrl('p') => KeymapResult::Command(Command::MovePreviousLine),
            Key::Ctrl('a') => KeymapResult::Command(Command::MoveToLineStart),
            Key::Ctrl('e') => KeymapResult::Command(Command::MoveToLineEnd),
            Key::Ctrl('s') => KeymapResult::Command(Command::RepeatSearch),
            Key::Ctrl(' ') => KeymapResult::Command(Command::SetMark),
            Key::Meta('x') => KeymapResult::Command(Command::OpenCommandLine),
            Key::Command('z') => KeymapResult::Command(Command::Undo),
            // Crossterm decodes the legacy C-/ and C-_ byte (0x1f) as C-7.
            Key::Ctrl('/') | Key::Ctrl('_') | Key::Ctrl('7') => {
                KeymapResult::Command(Command::Undo)
            }
            _ => KeymapResult::Unbound,
        }
    }
}

fn resolve_prefixed(prefix: Prefix, key: Key) -> KeymapResult {
    match (prefix, key) {
        (Prefix::CtrlC, Key::Ctrl('v')) => KeymapResult::Command(Command::ClipboardPaste),
        (Prefix::CtrlX, Key::Ctrl('s')) => KeymapResult::Command(Command::SaveBuffer),
        (Prefix::CtrlX, Key::Ctrl('c')) => KeymapResult::Command(Command::Quit),
        (Prefix::CtrlX, Key::Ctrl('f')) => KeymapResult::Command(Command::OpenFile),
        (Prefix::CtrlX, Key::Ctrl('r')) => KeymapResult::Command(Command::ReloadBuffer),
        (Prefix::CtrlX, Key::Char('b')) => KeymapResult::Command(Command::SwitchBuffer),
        (Prefix::CtrlX, Key::Char('u')) => KeymapResult::Command(Command::Undo),
        _ => KeymapResult::Unbound,
    }
}

#[cfg(test)]
mod tests {
    use super::{Keymap, KeymapResult};
    use crate::{commands::Command, input::Key};

    #[test]
    fn word_buffer_and_page_keys_resolve_to_navigation_commands() {
        for (key, command) in [
            (Key::Meta('f'), Command::MoveForwardWord),
            (Key::Meta('b'), Command::MoveBackwardWord),
            (Key::Meta('<'), Command::MoveToBufferStart),
            (Key::Meta('>'), Command::MoveToBufferEnd),
            (Key::Meta('d'), Command::KillWord),
            (Key::MetaBackspace, Command::BackwardKillWord),
            (Key::Ctrl('v'), Command::PageDown),
            (Key::PageDown, Command::PageDown),
            (Key::Meta('v'), Command::PageUp),
            (Key::PageUp, Command::PageUp),
        ] {
            assert_eq!(Keymap::new().resolve(key), KeymapResult::Command(command));
        }
    }

    #[test]
    fn clipboard_bindings_are_explicit_and_do_not_change_the_quit_prefix() {
        let mut keymap = Keymap::new();
        assert_eq!(
            keymap.resolve(Key::Meta('w')),
            KeymapResult::Command(Command::CopyRegion)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('v')),
            KeymapResult::Command(Command::PageDown)
        );
        assert_eq!(keymap.resolve(Key::Ctrl('c')), KeymapResult::PendingPrefix);
        assert_eq!(keymap.pending_label(), Some("C-c"));
        assert_eq!(
            keymap.resolve(Key::Ctrl('v')),
            KeymapResult::Command(Command::ClipboardPaste)
        );
        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Ctrl('c')),
            KeymapResult::Command(Command::Quit)
        );
    }

    #[test]
    fn resolves_printable_and_editing_keys_to_commands() {
        let mut keymap = Keymap::new();

        assert_eq!(
            keymap.resolve(Key::Char('a')),
            KeymapResult::Command(Command::Insert('a'))
        );
        assert_eq!(
            keymap.resolve(Key::Char('/')),
            KeymapResult::Command(Command::Insert('/'))
        );
        assert_eq!(
            keymap.resolve(Key::Enter),
            KeymapResult::Command(Command::InsertNewline)
        );
        assert_eq!(
            keymap.resolve(Key::Backspace),
            KeymapResult::Command(Command::DeleteBackward)
        );
        assert_eq!(
            keymap.resolve(Key::Delete),
            KeymapResult::Command(Command::DeleteForward)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('d')),
            KeymapResult::Command(Command::DeleteForward)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('k')),
            KeymapResult::Command(Command::KillLine)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('w')),
            KeymapResult::Command(Command::KillRegion)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('y')),
            KeymapResult::Command(Command::Yank)
        );
    }

    #[test]
    fn resolves_arrow_and_control_movement_keys() {
        let mut keymap = Keymap::new();

        assert_eq!(
            keymap.resolve(Key::Right),
            KeymapResult::Command(Command::MoveForwardChar)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('f')),
            KeymapResult::Command(Command::MoveForwardChar)
        );
        assert_eq!(
            keymap.resolve(Key::Left),
            KeymapResult::Command(Command::MoveBackwardChar)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('b')),
            KeymapResult::Command(Command::MoveBackwardChar)
        );
        assert_eq!(
            keymap.resolve(Key::Down),
            KeymapResult::Command(Command::MoveNextLine)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('n')),
            KeymapResult::Command(Command::MoveNextLine)
        );
        assert_eq!(
            keymap.resolve(Key::Up),
            KeymapResult::Command(Command::MovePreviousLine)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('p')),
            KeymapResult::Command(Command::MovePreviousLine)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('a')),
            KeymapResult::Command(Command::MoveToLineStart)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('e')),
            KeymapResult::Command(Command::MoveToLineEnd)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('s')),
            KeymapResult::Command(Command::RepeatSearch)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl(' ')),
            KeymapResult::Command(Command::SetMark)
        );
        assert_eq!(
            keymap.resolve(Key::Meta('x')),
            KeymapResult::Command(Command::OpenCommandLine)
        );
        assert_eq!(
            keymap.resolve(Key::Command('z')),
            KeymapResult::Command(Command::Undo)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('/')),
            KeymapResult::Command(Command::Undo)
        );
        assert_eq!(
            keymap.resolve(Key::Ctrl('_')),
            KeymapResult::Command(Command::Undo)
        );
    }

    #[test]
    fn resolves_ctrl_x_prefixed_commands() {
        let mut keymap = Keymap::new();

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Ctrl('s')),
            KeymapResult::Command(Command::SaveBuffer)
        );

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Ctrl('c')),
            KeymapResult::Command(Command::Quit)
        );

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Ctrl('f')),
            KeymapResult::Command(Command::OpenFile)
        );

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Ctrl('r')),
            KeymapResult::Command(Command::ReloadBuffer)
        );

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Char('b')),
            KeymapResult::Command(Command::SwitchBuffer)
        );

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(
            keymap.resolve(Key::Char('u')),
            KeymapResult::Command(Command::Undo)
        );
    }

    #[test]
    fn resets_prefix_after_invalid_key() {
        let mut keymap = Keymap::new();

        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert_eq!(keymap.resolve(Key::Unhandled), KeymapResult::Unbound);
        assert_eq!(
            keymap.resolve(Key::Char('a')),
            KeymapResult::Command(Command::Insert('a'))
        );
    }

    #[test]
    fn unbound_control_keys_do_not_leave_prefix_state() {
        let mut keymap = Keymap::new();

        assert_eq!(keymap.resolve(Key::Ctrl('z')), KeymapResult::Unbound);
        assert_eq!(
            keymap.resolve(Key::Char('a')),
            KeymapResult::Command(Command::Insert('a'))
        );
    }
}
