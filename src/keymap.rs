use crate::{commands::Command, input::Key};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keymap {
    pending_prefix: Option<Prefix>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefix {
    CtrlX,
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

    pub fn has_pending_prefix(&self) -> bool {
        self.pending_prefix.is_some()
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
            Key::Char(ch) => KeymapResult::Command(Command::Insert(ch)),
            Key::Enter => KeymapResult::Command(Command::InsertNewline),
            Key::Backspace => KeymapResult::Command(Command::DeleteBackward),
            Key::Delete => KeymapResult::Command(Command::DeleteForward),
            Key::Right | Key::Ctrl('f') => KeymapResult::Command(Command::MoveForwardChar),
            Key::Left | Key::Ctrl('b') => KeymapResult::Command(Command::MoveBackwardChar),
            Key::Down | Key::Ctrl('n') => KeymapResult::Command(Command::MoveNextLine),
            Key::Up | Key::Ctrl('p') => KeymapResult::Command(Command::MovePreviousLine),
            Key::Ctrl('a') => KeymapResult::Command(Command::MoveToLineStart),
            Key::Ctrl('e') => KeymapResult::Command(Command::MoveToLineEnd),
            _ => KeymapResult::Unbound,
        }
    }
}

fn resolve_prefixed(prefix: Prefix, key: Key) -> KeymapResult {
    match (prefix, key) {
        (Prefix::CtrlX, Key::Ctrl('s')) => KeymapResult::Command(Command::SaveBuffer),
        (Prefix::CtrlX, Key::Ctrl('c')) => KeymapResult::Command(Command::Quit),
        _ => KeymapResult::Unbound,
    }
}

#[cfg(test)]
mod tests {
    use super::{Keymap, KeymapResult};
    use crate::{commands::Command, input::Key};

    #[test]
    fn resolves_printable_and_editing_keys_to_commands() {
        let mut keymap = Keymap::new();

        assert_eq!(
            keymap.resolve(Key::Char('a')),
            KeymapResult::Command(Command::Insert('a'))
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

    #[test]
    fn reports_pending_prefix_state() {
        let mut keymap = Keymap::new();

        assert!(!keymap.has_pending_prefix());
        assert_eq!(keymap.resolve(Key::Ctrl('x')), KeymapResult::PendingPrefix);
        assert!(keymap.has_pending_prefix());
        assert_eq!(keymap.resolve(Key::Unhandled), KeymapResult::Unbound);
        assert!(!keymap.has_pending_prefix());
    }
}
