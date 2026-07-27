use ropey::{Rope, RopeSlice};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{self, BufReader, BufWriter, Write},
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const TEMP_FILE_CREATE_ATTEMPTS: usize = 128;
static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct Buffer {
    id: u64,
    text: Rope,
    path: PathBuf,
    dirty: bool,
    clean_text: String,
    revision: u64,
    undo_stack: Vec<Edit>,
    redo_stack: Vec<Edit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    start: usize,
    deleted: String,
    inserted: String,
    point_before: usize,
    point_after: usize,
}

impl Buffer {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let text = match File::open(&path) {
            Ok(file) => Rope::from_reader(BufReader::new(file))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Rope::new(),
            Err(error) => return Err(error),
        };
        let clean_text = text.to_string();

        Ok(Self {
            id: NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed),
            text,
            path,
            dirty: false,
            clean_text,
            revision: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.text.len_lines()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub fn line_for_char(&self, char_idx: usize) -> usize {
        let len_chars = self.len_chars();

        if len_chars == 0 || char_idx >= len_chars {
            return self.len_lines().saturating_sub(1);
        }

        self.text.char_to_line(char_idx)
    }

    pub fn line_start_char(&self, line_idx: usize) -> usize {
        let line_idx = self.clamp_line_idx(line_idx);
        self.text.line_to_char(line_idx)
    }

    pub fn line_end_char(&self, line_idx: usize) -> usize {
        let line_idx = self.clamp_line_idx(line_idx);
        let line = self.text.line(line_idx);
        self.line_start_char(line_idx) + line_content_len_chars(line)
    }

    pub fn line_prefix_text(&self, line_idx: usize, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }

        let line_idx = self.clamp_line_idx(line_idx);
        let line = self.text.line(line_idx);
        let content_len = line_content_len_chars(line);
        line.slice(..content_len.min(max_chars)).to_string()
    }

    pub fn line_changed(&self, line_idx: usize) -> bool {
        let line_idx = self.clamp_line_idx(line_idx);
        self.text.line(line_idx) != clean_line_text(&self.clean_text, line_idx)
    }

    pub fn find_forward(&self, query: &str, start_char: usize) -> Option<usize> {
        if query.is_empty() {
            return None;
        }

        let text = self.text.to_string();
        let start_byte = char_to_byte_idx(&text, start_char.min(self.len_chars()));

        find_byte_from(&text, query, start_byte)
            .or_else(|| find_byte_from(&text, query, 0))
            .map(|byte_idx| text[..byte_idx].chars().count())
    }

    pub fn text(&self) -> String {
        self.text.to_string()
    }

    pub(crate) fn text_prefix_lines(&self, line_count: usize) -> String {
        let end = if line_count >= self.len_lines() {
            self.len_chars()
        } else {
            self.text.line_to_char(line_count)
        };
        self.text.slice(..end).to_string()
    }

    pub fn text_range(&self, char_range: Range<usize>) -> String {
        let start = char_range.start.min(self.len_chars());
        let end = char_range.end.min(self.len_chars());

        if start >= end {
            return String::new();
        }

        self.text.slice(start..end).to_string()
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        let point_after = char_idx + text.chars().count();
        self.replace_with_points(char_idx..char_idx, text, char_idx, point_after);
    }

    pub fn delete(&mut self, char_range: Range<usize>) {
        self.delete_with_points(char_range.clone(), char_range.start, char_range.start);
    }

    pub fn delete_with_points(
        &mut self,
        char_range: Range<usize>,
        point_before: usize,
        point_after: usize,
    ) {
        if char_range.is_empty() {
            return;
        }

        self.replace_with_points(char_range, "", point_before, point_after);
    }

    pub fn undo(&mut self) -> Option<usize> {
        let edit = self.undo_stack.pop()?;
        self.apply_inverse_edit(&edit);
        let point = edit.point_before.min(self.len_chars());
        self.redo_stack.push(edit);
        self.update_dirty();
        Some(point)
    }

    pub fn redo(&mut self) -> Option<usize> {
        let edit = self.redo_stack.pop()?;
        self.apply_edit(&edit);
        let point = edit.point_after.min(self.len_chars());
        self.undo_stack.push(edit);
        self.update_dirty();
        Some(point)
    }

    pub fn save(&mut self) -> io::Result<()> {
        ensure_parent_directory_exists(&self.path)?;
        write_atomically(&self.path, &self.text)?;
        self.clean_text = self.text.to_string();
        self.dirty = false;
        Ok(())
    }

    fn clamp_line_idx(&self, line_idx: usize) -> usize {
        line_idx.min(self.len_lines().saturating_sub(1))
    }

    fn replace_with_points(
        &mut self,
        char_range: Range<usize>,
        inserted: &str,
        point_before: usize,
        point_after: usize,
    ) {
        let deleted = self.text.slice(char_range.clone()).to_string();
        let edit = Edit {
            start: char_range.start,
            deleted,
            inserted: inserted.to_string(),
            point_before,
            point_after,
        };

        self.apply_edit(&edit);
        self.undo_stack.push(edit);
        self.redo_stack.clear();
        self.update_dirty();
    }

    fn apply_edit(&mut self, edit: &Edit) {
        let deleted_len = edit.deleted.chars().count();
        self.apply_change(edit.start, deleted_len, &edit.inserted);
    }

    fn apply_inverse_edit(&mut self, edit: &Edit) {
        let inserted_len = edit.inserted.chars().count();
        self.apply_change(edit.start, inserted_len, &edit.deleted);
    }

    fn apply_change(&mut self, start: usize, remove_len: usize, inserted: &str) {
        if remove_len > 0 {
            self.text.remove(start..start + remove_len);
        }
        if !inserted.is_empty() {
            self.text.insert(start, inserted);
        }
        self.revision = self.revision.wrapping_add(1);
    }

    fn update_dirty(&mut self) {
        self.dirty = self.text != self.clean_text.as_str();
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        Self {
            id: NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed),
            text: self.text.clone(),
            path: self.path.clone(),
            dirty: self.dirty,
            clean_text: self.clean_text.clone(),
            revision: self.revision,
            undo_stack: self.undo_stack.clone(),
            redo_stack: self.redo_stack.clone(),
        }
    }
}

fn line_content_len_chars(line: RopeSlice<'_>) -> usize {
    let len_chars = line.len_chars();

    if len_chars == 0 || line.char(len_chars - 1) != '\n' {
        return len_chars;
    }

    if len_chars >= 2 && line.char(len_chars - 2) == '\r' {
        len_chars - 2
    } else {
        len_chars - 1
    }
}

fn clean_line_text(text: &str, line_idx: usize) -> &str {
    text.split_inclusive('\n').nth(line_idx).unwrap_or_default()
}

fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(text.len())
}

fn find_byte_from(text: &str, query: &str, start_byte: usize) -> Option<usize> {
    text.get(start_byte..)
        .and_then(|suffix| suffix.find(query))
        .map(|offset| start_byte + offset)
}

/// Writes `text` to `path` without ever truncating the existing file in place.
///
/// The contents are written to a temporary sibling file, flushed and fsynced,
/// then atomically renamed over the target. If any step fails the original file
/// is left untouched and the temporary file is removed.
fn write_atomically(path: &Path, text: &Rope) -> io::Result<()> {
    write_atomically_with(path, random_temp_suffix, |file| {
        let mut writer = BufWriter::new(file);
        text.write_to(&mut writer)?;
        writer.flush()?;
        let file = writer.into_inner().map_err(|error| error.into_error())?;
        file.sync_all()
    })
}

fn write_atomically_with<S, W>(path: &Path, suffix: S, write: W) -> io::Result<()>
where
    S: FnMut() -> io::Result<String>,
    W: FnOnce(File) -> io::Result<()>,
{
    let (temp_path, temp_file) = create_temp_file(path, suffix)?;
    let result = write(temp_file).and_then(|()| fs::rename(&temp_path, path));

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

fn create_temp_file<S>(path: &Path, mut suffix: S) -> io::Result<(PathBuf, File)>
where
    S: FnMut() -> io::Result<String>,
{
    for _ in 0..TEMP_FILE_CREATE_ATTEMPTS {
        let temp_path = temp_path_for(path, &suffix()?);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique temporary save file",
    ))
}

fn random_temp_suffix() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        io::Error::other(format!("could not generate a temporary name: {error}"))
    })?;
    Ok(format!("{:032x}", u128::from_ne_bytes(bytes)))
}

fn temp_path_for(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(path.file_name().unwrap_or_else(|| OsStr::new("cortex")));
    name.push(format!(".cortex-{suffix}.tmp"));

    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(name),
        _ => PathBuf::from(name),
    }
}

fn ensure_parent_directory_exists(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    if parent.as_os_str().is_empty() || parent.is_dir() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("parent directory does not exist: {}", parent.display()),
    ))
}

#[cfg(test)]
mod tests {
    use super::{temp_path_for, write_atomically_with, Buffer};
    use std::{
        fs, io,
        io::Write,
        os::unix::fs::symlink,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn loads_existing_files_into_the_buffer() {
        let dir = test_dir("loads-existing-files");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha\nbeta\n").unwrap();

        let buffer = Buffer::open(&path).unwrap();

        assert_eq!(buffer.path(), path.as_path());
        assert_eq!(buffer.text(), "alpha\nbeta\n");
        assert!(!buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn missing_files_open_as_empty_clean_buffers_with_the_requested_path() {
        let dir = test_dir("missing-files");
        let path = dir.join("new.txt");

        let buffer = Buffer::open(&path).unwrap();

        assert_eq!(buffer.path(), path.as_path());
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.len_chars(), 0);
        assert!(!buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn insert_marks_the_buffer_dirty() {
        let dir = test_dir("insert-dirty");
        let path = dir.join("notes.txt");
        fs::write(&path, "helo").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(2, "l");

        assert_eq!(buffer.text(), "hello");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn delete_marks_the_buffer_dirty() {
        let dir = test_dir("delete-dirty");
        let path = dir.join("notes.txt");
        fs::write(&path, "helllo").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.delete(3..4);

        assert_eq!(buffer.text(), "hello");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn line_changed_compares_current_text_to_the_saved_baseline() {
        let dir = test_dir("line-changed");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha\nbeta\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        assert!(!buffer.line_changed(0));
        assert!(!buffer.line_changed(1));

        buffer.insert(0, "x");

        assert!(buffer.line_changed(0));
        assert!(!buffer.line_changed(1));

        buffer.save().unwrap();

        assert!(!buffer.line_changed(0));
        remove_dir(dir);
    }

    #[test]
    fn line_changed_detects_deleted_final_newline() {
        let dir = test_dir("line-changed-delete-final-newline");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.delete(5..6);

        assert!(buffer.line_changed(0));
        remove_dir(dir);
    }

    #[test]
    fn line_changed_detects_added_final_newline() {
        let dir = test_dir("line-changed-add-final-newline");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(5, "\n");

        assert!(buffer.line_changed(0));
        remove_dir(dir);
    }

    #[test]
    fn line_changed_detects_lf_and_crlf_terminator_changes() {
        let dir = test_dir("line-changed-line-terminator");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha\r\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.delete(5..6);

        assert_eq!(buffer.text(), "alpha\n");
        assert!(buffer.line_changed(0));
        remove_dir(dir);
    }

    #[test]
    fn undo_and_redo_restore_final_newline_change_marker() {
        let dir = test_dir("line-changed-undo-redo-final-newline");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha\n").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.delete(5..6);
        assert!(buffer.line_changed(0));

        buffer.undo();
        assert!(!buffer.line_changed(0));

        buffer.redo();
        assert!(buffer.line_changed(0));
        remove_dir(dir);
    }

    #[test]
    fn save_resets_final_newline_change_marker_baseline() {
        let dir = test_dir("line-changed-save-final-newline");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(5, "\n");
        assert!(buffer.line_changed(0));

        buffer.save().unwrap();
        assert!(!buffer.line_changed(0));
        remove_dir(dir);
    }

    #[test]
    fn find_forward_searches_from_point_and_wraps_once() {
        let dir = test_dir("find-forward");
        let path = dir.join("notes.txt");
        fs::write(&path, "alpha beta alpha").unwrap();
        let buffer = Buffer::open(&path).unwrap();

        assert_eq!(buffer.find_forward("alpha", 1), Some(11));
        assert_eq!(buffer.find_forward("alpha", 12), Some(0));
        assert_eq!(buffer.find_forward("missing", 0), None);
        assert_eq!(buffer.find_forward("", 0), None);
        remove_dir(dir);
    }

    #[test]
    fn undo_and_redo_reverse_insertions() {
        let dir = test_dir("undo-redo-insert");
        let path = dir.join("notes.txt");
        fs::write(&path, "ac").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(1, "b");

        assert_eq!(buffer.undo(), Some(1));
        assert_eq!(buffer.text(), "ac");
        assert!(!buffer.is_dirty());

        assert_eq!(buffer.redo(), Some(2));
        assert_eq!(buffer.text(), "abc");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn undo_and_redo_reverse_deletions() {
        let dir = test_dir("undo-redo-delete");
        let path = dir.join("notes.txt");
        fs::write(&path, "abc").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.delete_with_points(1..2, 2, 1);

        assert_eq!(buffer.text(), "ac");
        assert_eq!(buffer.undo(), Some(2));
        assert_eq!(buffer.text(), "abc");
        assert!(!buffer.is_dirty());

        assert_eq!(buffer.redo(), Some(1));
        assert_eq!(buffer.text(), "ac");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn new_edit_after_undo_clears_redo_history() {
        let dir = test_dir("undo-clears-redo");
        let path = dir.join("notes.txt");
        fs::write(&path, "a").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(1, "b");
        assert_eq!(buffer.undo(), Some(1));
        buffer.insert(1, "c");

        assert_eq!(buffer.redo(), None);
        assert_eq!(buffer.text(), "ac");
        remove_dir(dir);
    }

    #[test]
    fn save_resets_the_clean_undo_baseline() {
        let dir = test_dir("undo-save-baseline");
        let path = dir.join("notes.txt");
        fs::write(&path, "a").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(1, "b");
        buffer.save().unwrap();
        assert!(!buffer.is_dirty());

        assert_eq!(buffer.undo(), Some(1));
        assert_eq!(buffer.text(), "a");
        assert!(buffer.is_dirty());

        assert_eq!(buffer.redo(), Some(2));
        assert_eq!(buffer.text(), "ab");
        assert!(!buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn save_writes_buffer_contents_to_disk_and_clears_dirty_state() {
        let dir = test_dir("save-existing");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(buffer.len_chars(), "\nafter");
        buffer.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "before\nafter");
        assert!(!buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn save_creates_the_target_file_when_the_parent_directory_exists() {
        let dir = test_dir("save-creates-file");
        let path = dir.join("created.txt");
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(0, "created");
        buffer.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "created");
        assert!(!buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn save_creates_an_empty_file_for_a_clean_missing_buffer() {
        let dir = test_dir("save-clean-missing-buffer");
        let path = dir.join("empty.txt");
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.save().unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "");
        assert!(!buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn save_fails_clearly_when_the_parent_directory_does_not_exist() {
        let dir = test_dir("save-missing-parent");
        let path = dir.join("missing").join("created.txt");
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(0, "created");
        let error = buffer.save().unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error
            .to_string()
            .contains("parent directory does not exist"));
        assert!(buffer.is_dirty());
        assert!(!path.exists());
        remove_dir(dir);
    }

    #[test]
    fn save_leaves_no_temporary_files_behind() {
        let dir = test_dir("save-no-temp-files");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(buffer.len_chars(), "\nafter");
        buffer.save().unwrap();

        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names, vec!["notes.txt".to_string()]);
        assert_eq!(fs::read_to_string(&path).unwrap(), "before\nafter");
        remove_dir(dir);
    }

    #[test]
    fn save_retries_a_pre_existing_temporary_file_without_changing_it() {
        let dir = test_dir("save-pre-existing-temp-file");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let occupied = temp_path_for(&path, "occupied");
        fs::write(&occupied, "attacker content").unwrap();
        let mut suffixes = ["occupied", "available"].into_iter();

        write_atomically_with(
            &path,
            || Ok(suffixes.next().unwrap().to_string()),
            |mut file| file.write_all(b"saved"),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "saved");
        assert_eq!(fs::read_to_string(&occupied).unwrap(), "attacker content");
        assert!(!temp_path_for(&path, "available").exists());
        remove_dir(dir);
    }

    #[test]
    fn save_does_not_follow_or_remove_a_hostile_temporary_symlink() {
        let dir = test_dir("save-hostile-temp-symlink");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let symlink_target = dir.join("attacker-target.txt");
        fs::write(&symlink_target, "attacker content").unwrap();
        let occupied = temp_path_for(&path, "occupied");
        symlink(&symlink_target, &occupied).unwrap();
        let mut suffixes = ["occupied", "available"].into_iter();

        write_atomically_with(
            &path,
            || Ok(suffixes.next().unwrap().to_string()),
            |mut file| file.write_all(b"saved"),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "saved");
        assert_eq!(
            fs::read_to_string(&symlink_target).unwrap(),
            "attacker content"
        );
        assert!(occupied.is_symlink());
        assert!(!temp_path_for(&path, "available").exists());
        remove_dir(dir);
    }

    #[test]
    fn temporary_name_collisions_are_retried_safely() {
        let dir = test_dir("save-temp-collision");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let occupied = temp_path_for(&path, "occupied");
        fs::write(&occupied, "occupied").unwrap();
        let mut attempts = 0;

        write_atomically_with(
            &path,
            || {
                attempts += 1;
                Ok(if attempts == 1 {
                    "occupied"
                } else {
                    "available"
                }
                .to_string())
            },
            |mut file| file.write_all(b"saved"),
        )
        .unwrap();

        assert_eq!(attempts, 2);
        assert_eq!(fs::read_to_string(&path).unwrap(), "saved");
        assert_eq!(fs::read_to_string(&occupied).unwrap(), "occupied");
        remove_dir(dir);
    }

    #[test]
    fn write_failure_preserves_the_target_and_cleans_up_only_the_created_temp_file() {
        let dir = test_dir("save-write-failure-cleanup");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let occupied = temp_path_for(&path, "occupied");
        fs::write(&occupied, "attacker content").unwrap();
        let created = temp_path_for(&path, "created");
        let mut suffixes = ["occupied", "created"].into_iter();

        let result = write_atomically_with(
            &path,
            || Ok(suffixes.next().unwrap().to_string()),
            |mut file| {
                file.write_all(b"partial")?;
                Err(io::Error::other("injected write failure"))
            },
        );

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_to_string(&occupied).unwrap(), "attacker content");
        assert!(!created.exists());
        remove_dir(dir);
    }

    #[test]
    fn save_failure_after_writing_cleans_up_the_temporary_file() {
        let dir = test_dir("save-failure-cleanup");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(0, "x");

        // Replace the target with a directory so the final rename fails after the
        // temporary file has already been written and synced.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();

        let result = buffer.save();

        assert!(result.is_err());
        assert!(buffer.is_dirty());
        assert!(path.is_dir());
        let leftover_temp = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains(".cortex-"));
        assert!(!leftover_temp, "temporary save file should be cleaned up");
        remove_dir(dir);
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cortex-buffer-test-{}-{name}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }

    fn remove_dir(dir: PathBuf) {
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn text_range_returns_the_requested_chars() {
        let dir = test_dir("text-range");
        let path = dir.join("notes.txt");
        fs::write(&path, "aλcde").unwrap();
        let buffer = Buffer::open(&path).unwrap();

        assert_eq!(buffer.text_range(1..4), "λcd");
        assert_eq!(buffer.text_range(4..99), "e");
        assert_eq!(buffer.text_range(4..2), "");
        fs::remove_dir_all(dir).unwrap();
    }
}
