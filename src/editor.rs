use crate::{buffer::Buffer, view::View};
use std::{
    ffi::{CString, OsStr, OsString},
    fs, io,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug)]
struct BufferEntry {
    buffer: Buffer,
    view: View,
    identity: PathBuf,
}

#[derive(Debug)]
pub struct Editor {
    buffers: Vec<BufferEntry>,
    active: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenResult {
    Opened,
    AlreadyOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchError {
    Ambiguous,
    NotFound,
}

impl Editor {
    pub fn new(buffer: Buffer) -> io::Result<Self> {
        let identity = path_identity(buffer.path())?;
        Ok(Self {
            buffers: vec![BufferEntry {
                buffer,
                view: View::new(),
                identity,
            }],
            active: 0,
        })
    }

    pub fn active(&self) -> (&Buffer, &View) {
        let entry = &self.buffers[self.active];
        (&entry.buffer, &entry.view)
    }

    pub fn active_mut(&mut self) -> (&mut Buffer, &mut View) {
        let entry = &mut self.buffers[self.active];
        (&mut entry.buffer, &mut entry.view)
    }

    pub fn open(&mut self, path: &Path) -> io::Result<OpenResult> {
        let identity = path_identity(path)?;
        if let Some(index) = self.index_for_identity(&identity) {
            self.active = index;
            return Ok(OpenResult::AlreadyOpen);
        }

        self.buffers.push(BufferEntry {
            buffer: Buffer::open(path)?,
            view: View::new(),
            identity,
        });
        self.active = self.buffers.len() - 1;
        Ok(OpenResult::Opened)
    }

    pub fn switch_to(&mut self, name: &str) -> Result<(), SwitchError> {
        let path_matches: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (entry.buffer.path().to_string_lossy() == name).then_some(index)
            })
            .collect();

        if let Some(index) = unique_match(&path_matches)? {
            self.active = index;
            return Ok(());
        }

        if let Ok(identity) = path_identity(Path::new(name)) {
            if let Some(index) = self.index_for_identity(&identity) {
                self.active = index;
                return Ok(());
            }
        }

        let file_name_matches: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry
                    .buffer
                    .path()
                    .file_name()
                    .is_some_and(|file_name| file_name.to_string_lossy() == name)
                    .then_some(index)
            })
            .collect();

        let index = unique_match(&file_name_matches)?.ok_or(SwitchError::NotFound)?;
        self.active = index;
        Ok(())
    }

    pub fn any_dirty(&self) -> bool {
        self.buffers.iter().any(|entry| entry.buffer.is_dirty())
    }

    pub fn names(&self) -> String {
        self.buffers
            .iter()
            .map(|entry| entry.buffer.path().display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn index_for_identity(&self, identity: &Path) -> Option<usize> {
        self.buffers
            .iter()
            .position(|entry| entry.identity == identity)
    }
}

fn unique_match(matches: &[usize]) -> Result<Option<usize>, SwitchError> {
    match matches {
        [] => Ok(None),
        [index] => Ok(Some(*index)),
        _ => Err(SwitchError::Ambiguous),
    }
}

fn path_identity(path: &Path) -> io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut missing_components: Vec<OsString> = Vec::new();

    loop {
        match fs::canonicalize(ancestor) {
            Ok(canonical) => {
                let mut identity = normalize_existing_path(&canonical)?;
                let case_sensitive = volume_is_case_sensitive(&canonical)?;
                for component in missing_components.iter().rev() {
                    identity.push(normalize_path_component(component, case_sensitive));
                }
                return Ok(identity);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let Some(file_name) = ancestor.file_name() else {
                    return Err(error);
                };
                missing_components.push(file_name.to_os_string());
                let Some(parent) = ancestor.parent() else {
                    return Err(error);
                };
                ancestor = parent;
            }
            Err(error) => return Err(error),
        }
    }
}

fn normalize_existing_path(path: &Path) -> io::Result<PathBuf> {
    let mut actual_parent = PathBuf::new();
    let mut identity = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                actual_parent.push(prefix.as_os_str());
                identity.push(prefix.as_os_str());
            }
            Component::RootDir => {
                actual_parent.push(Path::new("/"));
                identity.push(Path::new("/"));
            }
            Component::CurDir => {}
            Component::ParentDir => {
                actual_parent.pop();
                identity.pop();
            }
            Component::Normal(part) => {
                let case_sensitive = volume_is_case_sensitive(&actual_parent)?;
                identity.push(normalize_path_component(part, case_sensitive));
                actual_parent.push(part);
            }
        }
    }

    Ok(identity)
}

fn volume_is_case_sensitive(path: &Path) -> io::Result<bool> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "file path contains an interior NUL byte",
        )
    })?;

    // SAFETY: `path` is a valid NUL-terminated C string, and `_PC_CASE_SENSITIVE`
    // is a read-only query supported by macOS for existing paths.
    let result = unsafe { libc::pathconf(path.as_ptr(), libc::_PC_CASE_SENSITIVE) };
    match result {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::Error::last_os_error()),
    }
}

fn normalize_path_component(component: &OsStr, case_sensitive: bool) -> OsString {
    if case_sensitive {
        component.to_os_string()
    } else {
        component
            .to_str()
            .map(|text| text.nfd().case_fold().nfd().collect::<String>())
            .map(OsString::from)
            .unwrap_or_else(|| component.to_os_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{Editor, OpenResult, SwitchError};
    use crate::buffer::Buffer;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn opening_and_switching_buffers_preserves_unsaved_text_and_view() {
        let dir = test_dir("preserve");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        {
            let (buffer, view) = editor.active_mut();
            buffer.insert(0, "dirty ");
            view.set_point(3, buffer);
        }

        assert_eq!(editor.open(&second).unwrap(), OpenResult::Opened);
        assert_eq!(editor.active().0.text(), "second");

        editor.switch_to("first.txt").unwrap();
        assert_eq!(editor.active().0.text(), "dirty first");
        assert!(editor.active().0.is_dirty());
        assert_eq!(editor.active().1.point(), 3);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn switching_buffers_preserves_each_horizontal_view() {
        let dir = test_dir("horizontal-view");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "abcdefghij").unwrap();
        fs::write(&second, "klmnopqrst").unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        {
            let (buffer, view) = editor.active_mut();
            view.move_to_line_end(buffer);
            view.ensure_point_visible(buffer, 1, 6);
            assert_eq!(view.scroll_column(), 5);
        }

        editor.open(&second).unwrap();
        assert_eq!(editor.active().1.scroll_column(), 0);
        editor.switch_to("first.txt").unwrap();
        assert_eq!(editor.active().1.scroll_column(), 5);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn opening_an_existing_buffer_switches_without_reloading_it() {
        let dir = test_dir("already-open");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.open(&second).unwrap();
        fs::write(&first, "changed on disk").unwrap();

        assert_eq!(editor.open(&first).unwrap(), OpenResult::AlreadyOpen);
        assert_eq!(editor.active().0.text(), "first");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_relative_and_absolute_paths_share_one_buffer() {
        let dir = test_dir("missing-alias");
        let first = dir.join("first.txt");
        fs::write(&first, "first").unwrap();
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let relative = PathBuf::from(format!(
            "cortex-missing-alias-{}-{nanos}-{unique}.txt",
            std::process::id()
        ));
        let absolute = std::env::current_dir().unwrap().join(&relative);

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        assert_eq!(editor.open(&relative).unwrap(), OpenResult::Opened);
        editor.active_mut().0.insert(0, "unsaved");
        assert_eq!(editor.open(&absolute).unwrap(), OpenResult::AlreadyOpen);
        assert_eq!(editor.active().0.text(), "unsaved");
        assert_eq!(editor.active().0.path(), relative);
        assert!(!absolute.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_path_case_identity_follows_the_volume() {
        let dir = test_dir("missing-case");
        let first = dir.join("first.txt");
        let upper = dir.join("Future.swift");
        let lower = dir.join("future.swift");
        fs::write(&first, "first").unwrap();
        let case_sensitive = super::volume_is_case_sensitive(&dir).unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        assert_eq!(editor.open(&upper).unwrap(), OpenResult::Opened);
        editor.active_mut().0.insert(0, "unsaved");
        let result = editor.open(&lower).unwrap();

        if case_sensitive {
            assert_eq!(result, OpenResult::Opened);
            assert_eq!(editor.active().0.text(), "");
        } else {
            assert_eq!(result, OpenResult::AlreadyOpen);
            assert_eq!(editor.active().0.text(), "unsaved");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_component_case_folding_is_volume_aware() {
        let component = std::ffi::OsStr::new("Future-é-Σ.swift");
        assert_eq!(
            super::normalize_path_component(component, false),
            "future-e\u{301}-σ.swift"
        );
        assert_eq!(
            super::normalize_path_component(component, true),
            "Future-é-Σ.swift"
        );
    }

    #[test]
    fn missing_unicode_aliases_share_a_buffer_on_case_insensitive_volumes() {
        let dir = test_dir("missing-unicode-case");
        let first = dir.join("first.txt");
        fs::write(&first, "first").unwrap();
        let case_sensitive = super::volume_is_case_sensitive(&dir).unwrap();
        let aliases = [
            (dir.join("é.swift"), dir.join("e\u{301}.swift")),
            (dir.join("Σ.swift"), dir.join("ς.swift")),
        ];

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        for (left, right) in aliases {
            assert_eq!(editor.open(&left).unwrap(), OpenResult::Opened);
            editor.active_mut().0.insert(0, "unsaved");
            let result = editor.open(&right).unwrap();

            if case_sensitive {
                assert_eq!(result, OpenResult::Opened);
                assert_eq!(editor.active().0.text(), "");
            } else {
                assert_eq!(result, OpenResult::AlreadyOpen);
                assert_eq!(editor.active().0.text(), "unsaved");
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saved_missing_file_keeps_the_same_buffer_identity() {
        let dir = test_dir("saved-missing-identity");
        let first = dir.join("first.txt");
        let created = dir.join("Future.swift");
        let case_alias = dir.join("future.swift");
        fs::write(&first, "first").unwrap();
        let case_sensitive = super::volume_is_case_sensitive(&dir).unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        assert_eq!(editor.open(&created).unwrap(), OpenResult::Opened);
        editor.active_mut().0.insert(0, "saved");
        editor.active_mut().0.save().unwrap();
        editor.active_mut().0.insert(0, "unsaved ");

        assert_eq!(editor.open(&created).unwrap(), OpenResult::AlreadyOpen);
        assert_eq!(editor.active().0.text(), "unsaved saved");
        if !case_sensitive {
            assert_eq!(editor.open(&case_alias).unwrap(), OpenResult::AlreadyOpen);
            assert_eq!(editor.active().0.text(), "unsaved saved");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_paths_open_as_empty_buffers_and_save_creates_them() {
        let dir = test_dir("create");
        let first = dir.join("first.txt");
        let created = dir.join("created.txt");
        fs::write(&first, "first").unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.open(&created).unwrap();
        assert_eq!(editor.active().0.text(), "");
        assert_eq!(editor.active().0.path(), created);

        editor.active_mut().0.insert(0, "new file");
        editor.active_mut().0.save().unwrap();
        assert_eq!(fs::read_to_string(&created).unwrap(), "new file");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn dirty_state_is_reported_across_inactive_buffers() {
        let dir = test_dir("dirty");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.active_mut().0.insert(0, "dirty ");
        editor.open(&second).unwrap();

        assert!(editor.any_dirty());
        assert!(!editor.active().0.is_dirty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saving_writes_only_the_active_buffer() {
        let dir = test_dir("save-active");
        let first = dir.join("first.txt");
        let second = dir.join("second.txt");
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.active_mut().0.insert(0, "dirty ");
        editor.open(&second).unwrap();
        editor.active_mut().0.insert(0, "saved ");
        editor.active_mut().0.save().unwrap();

        assert_eq!(fs::read_to_string(&first).unwrap(), "first");
        assert_eq!(fs::read_to_string(&second).unwrap(), "saved second");
        assert!(editor.any_dirty());
        assert!(!editor.active().0.is_dirty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn switch_buffer_requires_a_unique_path_or_file_name() {
        let dir = test_dir("switch");
        let first = dir.join("a").join("notes.txt");
        let second = dir.join("b").join("notes.txt");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(&first, "first").unwrap();
        fs::write(&second, "second").unwrap();
        let first_alias_dir = dir.join("first-alias");
        std::os::unix::fs::symlink(first.parent().unwrap(), &first_alias_dir).unwrap();
        let first_alias = first_alias_dir.join("notes.txt");

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.open(&second).unwrap();

        assert_eq!(editor.switch_to("notes.txt"), Err(SwitchError::Ambiguous));
        assert_eq!(editor.switch_to(&first_alias.to_string_lossy()), Ok(()));
        assert_eq!(editor.active().0.text(), "first");
        assert_eq!(editor.switch_to(&first.to_string_lossy()), Ok(()));
        assert_eq!(editor.active().0.text(), "first");
        assert_eq!(editor.switch_to("missing.txt"), Err(SwitchError::NotFound));
        fs::remove_dir_all(dir).unwrap();
    }

    fn test_dir(label: &str) -> PathBuf {
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cortex-editor-{label}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
