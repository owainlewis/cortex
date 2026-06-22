use crate::input::Key;
use std::{
    cmp::Ordering,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    name: String,
    path: PathBuf,
    kind: DirectoryEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryEntryKind {
    File,
    Directory,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPicker {
    directory: PathBuf,
    entries: Vec<DirectoryEntry>,
    selected: usize,
    pending_ctrl_x: bool,
    status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryPickerAction {
    Continue,
    Quit,
    Open(PathBuf),
}

impl DirectoryEntry {
    pub(crate) fn new(name: String, path: PathBuf, kind: DirectoryEntryKind) -> Self {
        Self { name, path, kind }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> DirectoryEntryKind {
        self.kind
    }

    pub fn is_file(&self) -> bool {
        self.kind == DirectoryEntryKind::File
    }

    pub fn is_directory(&self) -> bool {
        self.kind == DirectoryEntryKind::Directory
    }
}

impl DirectoryPicker {
    pub fn read(directory: &Path) -> io::Result<Self> {
        Ok(Self::new(
            directory.to_path_buf(),
            read_directory_entries(directory)?,
        ))
    }

    pub fn new(directory: PathBuf, entries: Vec<DirectoryEntry>) -> Self {
        Self {
            directory,
            entries,
            selected: 0,
            pending_ctrl_x: false,
            status_message: None,
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn entries(&self) -> &[DirectoryEntry] {
        &self.entries
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_entry(&self) -> Option<&DirectoryEntry> {
        self.entries.get(self.selected)
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn handle_key(&mut self, key: Key) -> DirectoryPickerAction {
        if self.pending_ctrl_x {
            self.pending_ctrl_x = false;
            return match key {
                Key::Ctrl('c') => DirectoryPickerAction::Quit,
                _ => {
                    self.status_message = None;
                    DirectoryPickerAction::Continue
                }
            };
        }

        match key {
            Key::Down | Key::Ctrl('n') => {
                self.move_next();
                DirectoryPickerAction::Continue
            }
            Key::Up | Key::Ctrl('p') => {
                self.move_previous();
                DirectoryPickerAction::Continue
            }
            Key::Enter => self.open_selected(),
            Key::Escape => DirectoryPickerAction::Quit,
            Key::Ctrl('x') => {
                self.pending_ctrl_x = true;
                self.status_message = Some("C-x".to_string());
                DirectoryPickerAction::Continue
            }
            _ => {
                self.status_message = None;
                DirectoryPickerAction::Continue
            }
        }
    }

    fn move_next(&mut self) {
        self.status_message = None;
        self.selected = self
            .selected
            .saturating_add(1)
            .min(self.entries.len().saturating_sub(1));
    }

    fn move_previous(&mut self) {
        self.status_message = None;
        self.selected = self.selected.saturating_sub(1);
    }

    fn open_selected(&mut self) -> DirectoryPickerAction {
        let Some(entry) = self.selected_entry() else {
            self.status_message = Some("No files in directory".to_string());
            return DirectoryPickerAction::Continue;
        };

        if entry.is_file() {
            DirectoryPickerAction::Open(entry.path().to_path_buf())
        } else {
            self.status_message = Some("Only regular files can be opened".to_string());
            DirectoryPickerAction::Continue
        }
    }
}

fn read_directory_entries(directory: &Path) -> io::Result<Vec<DirectoryEntry>> {
    let entries = fs::read_dir(directory).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read directory {}: {error}", directory.display()),
        )
    })?;
    let mut visible = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "failed to read directory entry in {}: {error}",
                    directory.display()
                ),
            )
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();

        if name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to inspect {}: {error}", entry.path().display()),
            )
        })?;
        let kind = if file_type.is_file() {
            DirectoryEntryKind::File
        } else if file_type.is_dir() {
            DirectoryEntryKind::Directory
        } else {
            DirectoryEntryKind::Other
        };

        visible.push(DirectoryEntry::new(name, entry.path(), kind));
    }

    visible.sort_by(compare_entries);
    Ok(visible)
}

fn compare_entries(left: &DirectoryEntry, right: &DirectoryEntry) -> Ordering {
    left.name
        .to_lowercase()
        .cmp(&right.name.to_lowercase())
        .then_with(|| left.name.cmp(&right.name))
}

#[cfg(test)]
mod tests {
    use super::{DirectoryEntry, DirectoryEntryKind, DirectoryPicker, DirectoryPickerAction};
    use crate::input::Key;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn reads_non_hidden_entries_in_stable_order() {
        let dir = test_dir("entries");
        fs::write(dir.join("zeta.txt"), "").unwrap();
        fs::write(dir.join(".hidden"), "").unwrap();
        fs::create_dir(dir.join("alpha")).unwrap();
        fs::write(dir.join("Beta.txt"), "").unwrap();

        let picker = DirectoryPicker::read(&dir).unwrap();
        let names: Vec<&str> = picker.entries().iter().map(DirectoryEntry::name).collect();

        assert_eq!(names, vec!["alpha", "Beta.txt", "zeta.txt"]);
        assert!(picker.entries()[0].is_directory());
        assert!(picker.entries()[1].is_file());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn selection_moves_down_and_up_with_arrows_and_control_keys() {
        let mut picker = picker_with_entries(["a.txt", "b.txt", "c.txt"]);

        picker.handle_key(Key::Down);
        assert_eq!(picker.selected(), 1);
        picker.handle_key(Key::Ctrl('n'));
        assert_eq!(picker.selected(), 2);
        picker.handle_key(Key::Down);
        assert_eq!(picker.selected(), 2);
        picker.handle_key(Key::Up);
        assert_eq!(picker.selected(), 1);
        picker.handle_key(Key::Ctrl('p'));
        assert_eq!(picker.selected(), 0);
        picker.handle_key(Key::Up);
        assert_eq!(picker.selected(), 0);
    }

    #[test]
    fn enter_opens_regular_file_only() {
        let mut picker = DirectoryPicker::new(
            PathBuf::from("/tmp"),
            vec![
                DirectoryEntry::new(
                    "src".to_string(),
                    PathBuf::from("/tmp/src"),
                    DirectoryEntryKind::Directory,
                ),
                DirectoryEntry::new(
                    "main.rs".to_string(),
                    PathBuf::from("/tmp/main.rs"),
                    DirectoryEntryKind::File,
                ),
            ],
        );

        assert_eq!(
            picker.handle_key(Key::Enter),
            DirectoryPickerAction::Continue
        );
        assert_eq!(
            picker.status_message(),
            Some("Only regular files can be opened")
        );
        picker.handle_key(Key::Down);
        assert_eq!(
            picker.handle_key(Key::Enter),
            DirectoryPickerAction::Open(PathBuf::from("/tmp/main.rs"))
        );
    }

    #[test]
    fn escape_and_ctrl_x_ctrl_c_quit_picker() {
        let mut picker = picker_with_entries(["a.txt"]);

        assert_eq!(picker.handle_key(Key::Escape), DirectoryPickerAction::Quit);

        let mut picker = picker_with_entries(["a.txt"]);
        assert_eq!(
            picker.handle_key(Key::Ctrl('x')),
            DirectoryPickerAction::Continue
        );
        assert_eq!(picker.status_message(), Some("C-x"));
        assert_eq!(
            picker.handle_key(Key::Ctrl('c')),
            DirectoryPickerAction::Quit
        );
    }

    fn picker_with_entries<const N: usize>(names: [&str; N]) -> DirectoryPicker {
        DirectoryPicker::new(
            PathBuf::from("/tmp"),
            names
                .into_iter()
                .map(|name| {
                    DirectoryEntry::new(
                        name.to_string(),
                        PathBuf::from("/tmp").join(name),
                        DirectoryEntryKind::File,
                    )
                })
                .collect(),
        )
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cortex-picker-test-{}-{name}-{unique}-{counter}",
            std::process::id(),
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
}
