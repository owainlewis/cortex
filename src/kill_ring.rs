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
}
