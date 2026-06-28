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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPickerRow {
    name: String,
    path: PathBuf,
    kind: DirectoryEntryKind,
    depth: usize,
    expanded: bool,
    selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectoryNode {
    entry: DirectoryEntry,
    children: Option<Vec<DirectoryNode>>,
    expanded: bool,
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
    roots: Vec<DirectoryNode>,
    selected: usize,
    pending_ctrl_x: bool,
    status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryPickerAction {
    Continue,
    Quit,
    Browse(PathBuf),
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

impl DirectoryPickerRow {
    fn from_node(node: &DirectoryNode, depth: usize, selected: bool) -> Self {
        Self {
            name: node.entry.name.clone(),
            path: node.entry.path.clone(),
            kind: node.entry.kind,
            depth,
            expanded: node.expanded,
            selected,
        }
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

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn is_directory(&self) -> bool {
        self.kind == DirectoryEntryKind::Directory
    }
}

impl DirectoryNode {
    fn new(entry: DirectoryEntry) -> Self {
        Self {
            entry,
            children: None,
            expanded: false,
        }
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
            roots: entries.into_iter().map(DirectoryNode::new).collect(),
            selected: 0,
            pending_ctrl_x: false,
            status_message: None,
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn visible_rows(&self) -> Vec<DirectoryPickerRow> {
        let mut rows = Vec::new();
        let mut visible_idx = 0;
        for node in &self.roots {
            push_visible_row(node, 0, self.selected, &mut visible_idx, &mut rows);
        }
        rows
    }

    pub fn visible_len(&self) -> usize {
        visible_node_count(&self.roots)
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_row(&self) -> Option<DirectoryPickerRow> {
        self.visible_rows().into_iter().nth(self.selected)
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
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
            Key::Right => {
                self.expand_selected();
                DirectoryPickerAction::Continue
            }
            Key::Left => {
                self.collapse_selected_or_move_to_parent();
                DirectoryPickerAction::Continue
            }
            Key::Backspace => self.browse_parent(),
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
            .min(self.visible_len().saturating_sub(1));
    }

    fn move_previous(&mut self) {
        self.status_message = None;
        self.selected = self.selected.saturating_sub(1);
    }

    fn open_selected(&mut self) -> DirectoryPickerAction {
        let Some(row) = self.selected_row() else {
            self.status_message = Some("No files in directory".to_string());
            return DirectoryPickerAction::Continue;
        };

        match row.kind() {
            DirectoryEntryKind::File => DirectoryPickerAction::Open(row.path().to_path_buf()),
            DirectoryEntryKind::Directory => {
                self.toggle_selected_directory();
                DirectoryPickerAction::Continue
            }
            DirectoryEntryKind::Other => {
                self.status_message =
                    Some("Only regular files and directories can be opened".to_string());
                DirectoryPickerAction::Continue
            }
        }
    }

    fn browse_parent(&mut self) -> DirectoryPickerAction {
        self.status_message = None;
        parent_directory(&self.directory)
            .map(DirectoryPickerAction::Browse)
            .unwrap_or(DirectoryPickerAction::Continue)
    }

    fn expand_selected(&mut self) {
        let Some(path) = self.selected_node_path() else {
            return;
        };

        let Some(node) = node_mut_at_path(&mut self.roots, &path) else {
            return;
        };

        if !node.entry.is_directory() {
            self.status_message = None;
            return;
        }

        match ensure_children_loaded(node) {
            Ok(()) => {
                node.expanded = true;
                self.status_message = None;
            }
            Err(error) => {
                self.status_message = Some(format!("Open failed: {error}"));
            }
        }
    }

    fn collapse_selected_or_move_to_parent(&mut self) {
        let Some(path) = self.selected_node_path() else {
            return;
        };

        if let Some(node) = node_mut_at_path(&mut self.roots, &path) {
            if node.expanded {
                node.expanded = false;
                self.status_message = None;
                return;
            }
        }

        if path.len() > 1 {
            let parent_path = &path[..path.len() - 1];
            if let Some(parent_idx) = visible_index_for_path(&self.roots, parent_path) {
                self.selected = parent_idx;
            }
        }
        self.status_message = None;
    }

    fn toggle_selected_directory(&mut self) {
        let Some(path) = self.selected_node_path() else {
            return;
        };

        let Some(node) = node_mut_at_path(&mut self.roots, &path) else {
            return;
        };

        if !node.entry.is_directory() {
            self.status_message = None;
            return;
        }

        if node.expanded {
            node.expanded = false;
            self.status_message = None;
            return;
        }

        match ensure_children_loaded(node) {
            Ok(()) => {
                node.expanded = true;
                self.status_message = None;
            }
            Err(error) => {
                self.status_message = Some(format!("Open failed: {error}"));
            }
        }
    }

    fn selected_node_path(&self) -> Option<Vec<usize>> {
        let mut visible_idx = 0;
        node_path_for_visible_index(&self.roots, self.selected, &mut visible_idx)
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

    let mut entries_to_sort = Vec::new();

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

        entries_to_sort.push(DirectoryEntry::new(name, entry.path(), kind));
    }

    entries_to_sort.sort_by(compare_entries);
    visible.extend(entries_to_sort);
    Ok(visible)
}

fn read_directory_nodes(directory: &Path) -> io::Result<Vec<DirectoryNode>> {
    read_directory_entries(directory)
        .map(|entries| entries.into_iter().map(DirectoryNode::new).collect())
}

fn parent_directory(directory: &Path) -> Option<PathBuf> {
    directory
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| {
            if directory.parent().is_some() {
                Some(directory.join(".."))
            } else {
                None
            }
        })
}

fn compare_entries(left: &DirectoryEntry, right: &DirectoryEntry) -> Ordering {
    entry_kind_order(left.kind)
        .cmp(&entry_kind_order(right.kind))
        .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        .then_with(|| left.name.cmp(&right.name))
}

fn entry_kind_order(kind: DirectoryEntryKind) -> u8 {
    match kind {
        DirectoryEntryKind::Directory => 0,
        DirectoryEntryKind::File => 1,
        DirectoryEntryKind::Other => 2,
    }
}

fn push_visible_row(
    node: &DirectoryNode,
    depth: usize,
    selected: usize,
    visible_idx: &mut usize,
    rows: &mut Vec<DirectoryPickerRow>,
) {
    rows.push(DirectoryPickerRow::from_node(
        node,
        depth,
        *visible_idx == selected,
    ));
    *visible_idx += 1;

    if node.expanded {
        if let Some(children) = node.children.as_ref() {
            for child in children {
                push_visible_row(child, depth + 1, selected, visible_idx, rows);
            }
        }
    }
}

fn visible_node_count(nodes: &[DirectoryNode]) -> usize {
    nodes
        .iter()
        .map(|node| {
            1 + if node.expanded {
                node.children
                    .as_deref()
                    .map(visible_node_count)
                    .unwrap_or(0)
            } else {
                0
            }
        })
        .sum()
}

fn node_path_for_visible_index(
    nodes: &[DirectoryNode],
    target: usize,
    visible_idx: &mut usize,
) -> Option<Vec<usize>> {
    for (idx, node) in nodes.iter().enumerate() {
        if *visible_idx == target {
            return Some(vec![idx]);
        }
        *visible_idx += 1;

        if node.expanded {
            if let Some(children) = node.children.as_ref() {
                if let Some(mut path) = node_path_for_visible_index(children, target, visible_idx) {
                    path.insert(0, idx);
                    return Some(path);
                }
            }
        }
    }

    None
}

fn visible_index_for_path(nodes: &[DirectoryNode], target: &[usize]) -> Option<usize> {
    let mut visible_idx = 0;
    visible_index_for_path_inner(nodes, target, &mut visible_idx)
}

fn visible_index_for_path_inner(
    nodes: &[DirectoryNode],
    target: &[usize],
    visible_idx: &mut usize,
) -> Option<usize> {
    for (idx, node) in nodes.iter().enumerate() {
        if target == [idx] {
            return Some(*visible_idx);
        }
        *visible_idx += 1;

        if node.expanded {
            if let Some(children) = node.children.as_ref() {
                if target.first() == Some(&idx) {
                    return visible_index_for_path_inner(children, &target[1..], visible_idx);
                }
                *visible_idx += visible_node_count(children);
            }
        }
    }

    None
}

fn node_mut_at_path<'a>(
    nodes: &'a mut [DirectoryNode],
    path: &[usize],
) -> Option<&'a mut DirectoryNode> {
    let (first, rest) = path.split_first()?;
    let node = nodes.get_mut(*first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        node.children
            .as_deref_mut()
            .and_then(|children| node_mut_at_path(children, rest))
    }
}

fn ensure_children_loaded(node: &mut DirectoryNode) -> io::Result<()> {
    if node.children.is_none() {
        node.children = Some(read_directory_nodes(node.entry.path())?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        parent_directory, DirectoryEntry, DirectoryEntryKind, DirectoryPicker,
        DirectoryPickerAction, DirectoryPickerRow,
    };
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
        let rows = picker.visible_rows();
        let names: Vec<&str> = rows.iter().map(DirectoryPickerRow::name).collect();

        assert_eq!(names, vec!["alpha", "Beta.txt", "zeta.txt"]);
        assert!(rows[0].is_directory());
        assert_eq!(rows[1].kind(), DirectoryEntryKind::File);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn parent_directory_supports_relative_directories() {
        assert_eq!(
            parent_directory(PathBuf::from(".").as_path()),
            Some(PathBuf::from("./.."))
        );
        assert_eq!(
            parent_directory(PathBuf::from("src").as_path()),
            Some(PathBuf::from("src/.."))
        );
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
    fn enter_expands_directories_and_opens_regular_files() {
        let dir = test_dir("tree-enter");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("main.rs"), "").unwrap();
        fs::write(dir.join("README.md"), "").unwrap();
        let mut picker = DirectoryPicker::read(&dir).unwrap();

        assert_eq!(
            picker.handle_key(Key::Enter),
            DirectoryPickerAction::Continue
        );
        let rows = picker.visible_rows();
        let names: Vec<&str> = rows.iter().map(DirectoryPickerRow::name).collect();
        assert_eq!(names, vec!["src", "main.rs", "README.md"]);
        assert!(rows[0].is_expanded());
        assert_eq!(rows[1].depth(), 1);

        picker.handle_key(Key::Down);
        assert_eq!(
            picker.handle_key(Key::Enter),
            DirectoryPickerAction::Open(src.join("main.rs"))
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn left_collapses_directories_or_moves_to_parent_row() {
        let dir = test_dir("tree-left");
        let src = dir.join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("main.rs"), "").unwrap();
        let mut picker = DirectoryPicker::read(&dir).unwrap();

        picker.handle_key(Key::Enter);
        picker.handle_key(Key::Down);
        assert_eq!(picker.selected(), 1);

        picker.handle_key(Key::Left);
        assert_eq!(picker.selected(), 0);
        assert!(picker.visible_rows()[0].is_expanded());

        picker.handle_key(Key::Left);
        assert!(!picker.visible_rows()[0].is_expanded());
        assert_eq!(picker.visible_len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn backspace_browses_to_parent_directory() {
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
            picker.handle_key(Key::Backspace),
            DirectoryPickerAction::Browse(PathBuf::from("/"))
        );
    }

    #[test]
    fn enter_rejects_other_entry_kinds() {
        let mut picker = DirectoryPicker::new(
            PathBuf::from("/tmp"),
            vec![DirectoryEntry::new(
                "pipe".to_string(),
                PathBuf::from("/tmp/pipe"),
                DirectoryEntryKind::Other,
            )],
        );

        assert_eq!(
            picker.handle_key(Key::Enter),
            DirectoryPickerAction::Continue
        );
        assert_eq!(
            picker.status_message(),
            Some("Only regular files and directories can be opened")
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
