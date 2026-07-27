use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Meta(char),
    Command(char),
    Enter,
    Escape,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Unhandled,
}

pub fn key_from_event(event: KeyEvent) -> Key {
    let meta_modifiers = KeyModifiers::ALT | KeyModifiers::META;
    let allowed_meta_modifiers = meta_modifiers | KeyModifiers::SHIFT;

    match event.code {
        KeyCode::Char(ch)
            if event.modifiers.intersects(meta_modifiers)
                && event
                    .modifiers
                    .difference(allowed_meta_modifiers)
                    .is_empty() =>
        {
            Key::Meta(ch.to_ascii_lowercase())
        }
        _ if event.modifiers.intersects(meta_modifiers) => Key::Unhandled,
        KeyCode::Null => Key::Ctrl(' '),
        KeyCode::Char(ch) if event.modifiers.contains(KeyModifiers::SUPER) => {
            Key::Command(ch.to_ascii_lowercase())
        }
        KeyCode::Char(ch) if event.modifiers.contains(KeyModifiers::CONTROL) => {
            Key::Ctrl(ch.to_ascii_lowercase())
        }
        KeyCode::Char(ch) if printable_char(ch, event.modifiers) => Key::Char(ch),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        _ => Key::Unhandled,
    }
}

fn printable_char(ch: char, modifiers: KeyModifiers) -> bool {
    let allowed_modifiers = KeyModifiers::NONE | KeyModifiers::SHIFT;
    !ch.is_control() && modifiers.difference(allowed_modifiers).is_empty()
}

#[cfg(test)]
mod tests {
    use super::{key_from_event, Key};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn maps_printable_characters() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Key::Char('a')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            Key::Char('A')
        );
    }

    #[test]
    fn maps_control_characters_case_insensitively() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::CONTROL)),
            Key::Ctrl('x')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL)),
            Key::Ctrl('/')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('_'), KeyModifiers::CONTROL)),
            Key::Ctrl('_')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL)),
            Key::Ctrl(' ')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)),
            Key::Ctrl(' ')
        );
    }

    #[test]
    fn maps_command_characters_case_insensitively() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::SUPER)),
            Key::Command('z')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(
                KeyCode::Char('Z'),
                KeyModifiers::SUPER | KeyModifiers::SHIFT
            )),
            Key::Command('z')
        );
    }

    #[test]
    fn maps_meta_characters_case_insensitively() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
            Key::Meta('x')
        );
        assert_eq!(
            key_from_event(KeyEvent::new(
                KeyCode::Char('X'),
                KeyModifiers::META | KeyModifiers::SHIFT
            )),
            Key::Meta('x')
        );
    }

    #[test]
    fn maps_meta_modified_non_characters_to_unhandled() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT)),
            Key::Unhandled
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Left, KeyModifiers::META)),
            Key::Unhandled
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::ALT)),
            Key::Unhandled
        );
    }

    #[test]
    fn maps_meta_characters_with_extra_modifiers_to_unhandled() {
        assert_eq!(
            key_from_event(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            Key::Unhandled
        );
        assert_eq!(
            key_from_event(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::SUPER | KeyModifiers::META
            )),
            Key::Unhandled
        );
    }

    #[test]
    fn maps_escape() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Key::Escape
        );
    }

    #[test]
    fn maps_editing_and_arrow_keys() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Key::Enter
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            Key::Backspace
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
            Key::Delete
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Key::Left
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            Key::Right
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Key::Up
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            Key::Down
        );
    }
}
