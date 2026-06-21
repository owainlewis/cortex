use ropey::{Rope, RopeSlice};
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Buffer {
    text: Rope,
    path: PathBuf,
    dirty: bool,
}

impl Buffer {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let text = match File::open(&path) {
            Ok(file) => Rope::from_reader(BufReader::new(file))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Rope::new(),
            Err(error) => return Err(error),
        };

        Ok(Self {
            text,
            path,
            dirty: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
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

    pub fn text(&self) -> String {
        self.text.to_string()
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        self.text.insert(char_idx, text);
        self.dirty = true;
    }

    pub fn delete(&mut self, char_range: Range<usize>) {
        if char_range.is_empty() {
            return;
        }

        self.text.remove(char_range);
        self.dirty = true;
    }

    pub fn save(&mut self) -> io::Result<()> {
        ensure_parent_directory_exists(&self.path)?;

        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        self.text.write_to(&mut writer)?;
        writer.flush()?;
        self.dirty = false;
        Ok(())
    }

    fn clamp_line_idx(&self, line_idx: usize) -> usize {
        line_idx.min(self.len_lines().saturating_sub(1))
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
    use super::Buffer;
    use std::{
        fs,
        io,
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
        assert!(error.to_string().contains("parent directory does not exist"));
        assert!(buffer.is_dirty());
        assert!(!path.exists());
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
}
