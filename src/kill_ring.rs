use std::collections::VecDeque;

const MAX_ENTRIES: usize = 32;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct KillRing {
    entries: VecDeque<String>,
}

impl KillRing {
    pub fn push(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.entries.push_front(text);
        self.entries.truncate(MAX_ENTRIES);
    }

    /// Joins text onto the newest entry, so consecutive kills yank as one.
    pub fn append(&mut self, text: &str, before: bool) {
        match self.entries.front_mut() {
            Some(newest) if before => newest.insert_str(0, text),
            Some(newest) => newest.push_str(text),
            None => self.push(text.to_string()),
        }
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::KillRing;

    #[test]
    fn retains_the_latest_32_complete_nonempty_kills() {
        let mut ring = KillRing::default();
        ring.push(String::new());
        assert_eq!(ring.len(), 0);
        for number in 0..35 {
            ring.push(format!("{number}: λ\r\n\t"));
        }
        ring.push(String::new());
        assert_eq!(ring.len(), 32);
        assert_eq!(ring.get(0), Some("34: λ\r\n\t"));
        assert_eq!(ring.get(31), Some("3: λ\r\n\t"));
        assert_eq!(ring.get(32), None);
    }

    #[test]
    fn append_joins_text_onto_the_newest_entry() {
        let mut ring = KillRing::default();
        ring.append("b", false);
        assert_eq!(ring.get(0), Some("b"));
        ring.append("c", false);
        ring.append("a", true);
        assert_eq!(ring.get(0), Some("abc"));
        assert_eq!(ring.len(), 1);
    }
}
