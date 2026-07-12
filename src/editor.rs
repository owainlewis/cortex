use crate::{buffer::Buffer, view::View};
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

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
            Ok(mut canonical) => {
                for component in missing_components.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
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

        let mut editor = Editor::new(Buffer::open(&first).unwrap()).unwrap();
        editor.open(&second).unwrap();

        assert_eq!(editor.switch_to("notes.txt"), Err(SwitchError::Ambiguous));
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
