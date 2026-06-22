use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
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
    if event.modifiers.contains(KeyModifiers::ALT)
        || event.modifiers.contains(KeyModifiers::SUPER)
        || event.modifiers.contains(KeyModifiers::META)
    {
        return Key::Unhandled;
    }

    match event.code {
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
    }

    #[test]
    fn maps_invalid_keys_to_unhandled_so_prefixes_can_reset() {
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
            Key::Unhandled
        );
        assert_eq!(
            key_from_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::SUPER)),
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
