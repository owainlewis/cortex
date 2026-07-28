use ropey::{Rope, RopeSlice};
use std::{
    ffi::{CString, OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, BufWriter, Seek, Write},
    ops::Range,
    os::unix::ffi::OsStrExt,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const TEMP_FILE_CREATE_ATTEMPTS: usize = 128;
const FILE_READ_ATTEMPTS: usize = 3;
static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);

type Acl = *mut libc::c_void;

unsafe extern "C" {
    fn acl_get_fd_np(fd: libc::c_int, acl_type: libc::c_int) -> Acl;
    fn acl_size(acl: Acl) -> libc::ssize_t;
    fn acl_copy_ext(buffer: *mut libc::c_void, acl: Acl, size: libc::ssize_t) -> libc::ssize_t;
    fn acl_free(object: *mut libc::c_void) -> libc::c_int;
}

const ACL_TYPE_EXTENDED: libc::c_int = 0x00000100;

#[derive(Debug)]
pub struct Buffer {
    id: u64,
    text: Rope,
    path: PathBuf,
    disk_baseline: DiskStamp,
    save_location: SaveLocation,
    disk_changed: bool,
    clean_text: Rope,
    history_state: u64,
    clean_history_state: u64,
    next_history_state: u64,
    revision: u64,
    undo_stack: Vec<Edit>,
    redo_stack: Vec<Edit>,
}

#[derive(Debug)]
pub enum ReloadError {
    Dirty,
    Io(io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiskStamp {
    Missing,
    Present {
        device: u64,
        inode: u64,
        len: u64,
        modified_seconds: i64,
        modified_nanoseconds: i64,
        changed_seconds: i64,
        changed_nanoseconds: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SaveLocation {
    Missing { target: PathBuf },
    Regular { target: PathBuf },
    Symlink { target: PathBuf },
}

#[derive(Debug, PartialEq, Eq)]
struct SemanticMetadata {
    mode: u32,
    uid: u32,
    gid: u32,
    flags: u32,
    acl: Vec<u8>,
    xattrs: Vec<(Vec<u8>, Vec<u8>)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PathIdentity {
    device: u64,
    inode: u64,
}

struct CommitContext<'a> {
    buffer_path: &'a Path,
    expected_location: &'a SaveLocation,
    source: Option<SourceBaseline<'a>>,
}

struct SourceBaseline<'a> {
    file: &'a File,
    metadata: SemanticMetadata,
    text: &'a Rope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Edit {
    start: usize,
    deleted: String,
    inserted: String,
    point_before: usize,
    point_after: usize,
    state_before: u64,
    state_after: u64,
}

impl Buffer {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let (text, disk_baseline, save_location) = load_file(&path, true)?;
        let clean_text = text.clone();

        Ok(Self {
            id: NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed),
            text,
            path,
            disk_baseline,
            save_location,
            disk_changed: false,
            clean_text,
            history_state: 0,
            clean_history_state: 0,
            next_history_state: 1,
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
        self.history_state != self.clean_history_state
    }

    pub fn disk_changed(&self) -> bool {
        self.disk_changed
    }

    pub fn refresh_disk_changed(&mut self) {
        self.disk_changed = current_disk_state(&self.path)
            .map(|(location, stamp)| location != self.save_location || stamp != self.disk_baseline)
            .unwrap_or(true);
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
        let current_line = self.text.line(line_idx);
        self.clean_text.get_line(line_idx).map_or_else(
            || current_line.len_chars() != 0,
            |clean_line| current_line != clean_line,
        )
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

    pub(crate) fn text_prefix_lines(
        &self,
        line_count: usize,
        max_chars_per_line: usize,
    ) -> (String, Vec<usize>) {
        let line_count = line_count.min(self.len_lines());
        let mut text = String::new();
        let mut context_barriers = Vec::new();

        for line_idx in 0..line_count {
            if line_idx > 0 {
                text.push('\n');
            }
            text.push_str(&self.line_prefix_text(line_idx, max_chars_per_line));
            if line_content_len_chars(self.text.line(line_idx)) > max_chars_per_line {
                context_barriers.push(line_idx + 1);
            }
        }

        (text, context_barriers)
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
        Some(point)
    }

    pub fn redo(&mut self) -> Option<usize> {
        let edit = self.redo_stack.pop()?;
        self.apply_edit(&edit);
        let point = edit.point_after.min(self.len_chars());
        self.undo_stack.push(edit);
        Some(point)
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.save_with_hooks(|_| {}, || {})
    }

    #[cfg(test)]
    fn save_with<F>(&mut self, before_commit: F) -> io::Result<()>
    where
        F: FnOnce(&Path),
    {
        self.save_with_hooks(before_commit, || {})
    }

    fn save_with_hooks<F, G>(&mut self, before_commit: F, after_rename: G) -> io::Result<()>
    where
        F: FnOnce(&Path),
        G: FnOnce(),
    {
        ensure_parent_directory_exists(&self.path)?;
        let (disk_baseline, save_location) = write_atomically(
            &self.path,
            &self.save_location,
            self.disk_baseline,
            &self.text,
            &self.clean_text,
            before_commit,
            after_rename,
        )?;
        self.disk_baseline = disk_baseline;
        self.save_location = save_location;
        self.disk_changed = false;
        self.clean_text = self.text.clone();
        self.clean_history_state = self.history_state;
        Ok(())
    }

    pub fn reload(&mut self) -> Result<(), ReloadError> {
        if self.is_dirty() {
            return Err(ReloadError::Dirty);
        }

        let (text, disk_baseline, save_location) =
            load_file(&self.path, false).map_err(ReloadError::Io)?;
        self.clean_text = text.clone();
        self.text = text;
        self.history_state = 0;
        self.clean_history_state = 0;
        self.next_history_state = 1;
        self.revision = self.revision.wrapping_add(1);
        self.disk_baseline = disk_baseline;
        self.save_location = save_location;
        self.disk_changed = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
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
        let state_after = self.allocate_history_state();
        let edit = Edit {
            start: char_range.start,
            deleted,
            inserted: inserted.to_string(),
            point_before,
            point_after,
            state_before: self.history_state,
            state_after,
        };

        self.apply_edit(&edit);
        self.undo_stack.push(edit);
        self.redo_stack.clear();
    }

    fn apply_edit(&mut self, edit: &Edit) {
        let deleted_len = edit.deleted.chars().count();
        self.apply_change(edit.start, deleted_len, &edit.inserted);
        self.history_state = edit.state_after;
    }

    fn apply_inverse_edit(&mut self, edit: &Edit) {
        let inserted_len = edit.inserted.chars().count();
        self.apply_change(edit.start, inserted_len, &edit.deleted);
        self.history_state = edit.state_before;
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

    fn allocate_history_state(&mut self) -> u64 {
        let state = self.next_history_state;
        self.next_history_state = self
            .next_history_state
            .checked_add(1)
            .expect("buffer history state exhausted");
        state
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        Self {
            id: NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed),
            text: self.text.clone(),
            path: self.path.clone(),
            disk_baseline: self.disk_baseline,
            save_location: self.save_location.clone(),
            disk_changed: self.disk_changed,
            clean_text: self.clean_text.clone(),
            history_state: self.history_state,
            clean_history_state: self.clean_history_state,
            next_history_state: self.next_history_state,
            revision: self.revision,
            undo_stack: self.undo_stack.clone(),
            redo_stack: self.redo_stack.clone(),
        }
    }
}

fn load_file(path: &Path, allow_missing: bool) -> io::Result<(Rope, DiskStamp, SaveLocation)> {
    for _ in 0..FILE_READ_ATTEMPTS {
        let save_location = resolve_save_location(path)?;
        let target = save_location.target();
        let file = match &save_location {
            SaveLocation::Missing { .. } if allow_missing => {
                return Ok((Rope::new(), DiskStamp::Missing, save_location));
            }
            SaveLocation::Missing { .. } => return Err(file_missing_error(path)),
            SaveLocation::Regular { .. } | SaveLocation::Symlink { .. } => File::open(target)?,
        };
        let before = stamp_for_metadata(&file.metadata()?);
        let text = Rope::from_reader(BufReader::new(&file))?;
        let after = stamp_for_metadata(&file.metadata()?);
        let current = current_disk_state(path)?;

        if before == after && current == (save_location.clone(), after) {
            return Ok((text, after, save_location));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        "file changed while it was being read",
    ))
}

impl SaveLocation {
    fn target(&self) -> &Path {
        match self {
            Self::Missing { target } | Self::Regular { target } | Self::Symlink { target } => {
                target
            }
        }
    }
}

fn current_disk_state(path: &Path) -> io::Result<(SaveLocation, DiskStamp)> {
    let location = resolve_save_location(path)?;
    let stamp = match &location {
        SaveLocation::Missing { .. } => DiskStamp::Missing,
        SaveLocation::Regular { target } | SaveLocation::Symlink { target } => disk_stamp(target)?,
    };
    Ok((location, stamp))
}

fn resolve_save_location(path: &Path) -> io::Result<SaveLocation> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target = fs::canonicalize(path).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("could not resolve symlink {}: {error}", path.display()),
                )
            })?;
            ensure_regular_file(&target)?;
            Ok(SaveLocation::Symlink { target })
        }
        Ok(metadata) if metadata.is_file() => Ok(SaveLocation::Regular {
            target: fs::canonicalize(path)?,
        }),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("save target is not a regular file: {}", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(SaveLocation::Missing {
            target: missing_target(path)?,
        }),
        Err(error) => Err(error),
    }
}

fn ensure_regular_file(path: &Path) -> io::Result<()> {
    if fs::metadata(path)?.is_file() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("save target is not a regular file: {}", path.display()),
    ))
}

fn missing_target(path: &Path) -> io::Result<PathBuf> {
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("save target has no file name: {}", path.display()),
        )
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());

    match parent {
        Some(parent) => match fs::canonicalize(parent) {
            Ok(parent) => Ok(parent.join(file_name)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(std::env::current_dir()?.join(path))
            }
            Err(error) => Err(error),
        },
        None => Ok(std::env::current_dir()?.join(file_name)),
    }
}

fn file_missing_error(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("file does not exist: {}", path.display()),
    )
}

fn disk_stamp(path: &Path) -> io::Result<DiskStamp> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(stamp_for_metadata(&metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DiskStamp::Missing),
        Err(error) => Err(error),
    }
}

fn stamp_for_metadata(metadata: &fs::Metadata) -> DiskStamp {
    DiskStamp::Present {
        device: metadata.dev(),
        inode: metadata.ino(),
        len: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
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

/// Writes `text` without ever truncating the existing file in place.
///
/// The contents are written to a temporary sibling file, flushed and fsynced,
/// existing metadata is copied when present, and the temporary file is
/// atomically renamed over the stable target. If any step before rename fails,
/// the original file is left untouched and the temporary file is removed.
fn write_atomically<F, G>(
    path: &Path,
    expected_location: &SaveLocation,
    expected_stamp: DiskStamp,
    text: &Rope,
    baseline_text: &Rope,
    before_commit: F,
    after_rename: G,
) -> io::Result<(DiskStamp, SaveLocation)>
where
    F: FnOnce(&Path),
    G: FnOnce(),
{
    let source = validate_save_target(path, expected_location, expected_stamp)?;
    let source_baseline = source
        .as_ref()
        .map(|file| {
            Ok::<SourceBaseline<'_>, io::Error>(SourceBaseline {
                file,
                metadata: semantic_metadata(file)?,
                text: baseline_text,
            })
        })
        .transpose()?;
    let commit_context = CommitContext {
        buffer_path: path,
        expected_location,
        source: source_baseline,
    };
    let expected_target = match expected_stamp {
        DiskStamp::Missing => None,
        stamp @ DiskStamp::Present { .. } => Some(stamp),
    };
    let saved_stamp = write_atomically_with(
        expected_location.target(),
        source.as_ref(),
        expected_target,
        Some(&commit_context),
        random_temp_suffix,
        |file| {
            let mut writer = BufWriter::new(file);
            text.write_to(&mut writer)?;
            writer.flush()
        },
        || {
            if let Some(source) = &source {
                if stamp_for_metadata(&source.metadata()?) != expected_stamp {
                    return Err(disk_changed_error(path));
                }
            }
            validate_save_target(path, expected_location, expected_stamp).map(|_| ())
        },
        before_commit,
        after_rename,
    )?;
    let saved_location = match expected_location {
        SaveLocation::Missing { target } => SaveLocation::Regular {
            target: target.clone(),
        },
        SaveLocation::Regular { .. } | SaveLocation::Symlink { .. } => expected_location.clone(),
    };

    Ok((saved_stamp, saved_location))
}

#[allow(clippy::too_many_arguments)]
fn write_atomically_with<S, W, B, A, V>(
    path: &Path,
    metadata_source: Option<&File>,
    expected_target: Option<DiskStamp>,
    commit_context: Option<&CommitContext<'_>>,
    suffix: S,
    write: W,
    validate: V,
    before_commit: B,
    after_rename: A,
) -> io::Result<DiskStamp>
where
    S: FnMut() -> io::Result<String>,
    W: FnOnce(&mut File) -> io::Result<()>,
    B: FnOnce(&Path),
    A: FnOnce(),
    V: FnOnce() -> io::Result<()>,
{
    write_atomically_with_metadata(
        path,
        metadata_source,
        expected_target,
        commit_context,
        suffix,
        write,
        copy_metadata,
        File::sync_all,
        validate,
        before_commit,
        after_rename,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_atomically_with_metadata<S, W, M, Y, B, A, V>(
    path: &Path,
    metadata_source: Option<&File>,
    expected_target: Option<DiskStamp>,
    commit_context: Option<&CommitContext<'_>>,
    suffix: S,
    write: W,
    preserve_metadata: M,
    sync_file: Y,
    validate: V,
    before_commit: B,
    after_rename: A,
) -> io::Result<DiskStamp>
where
    S: FnMut() -> io::Result<String>,
    W: FnOnce(&mut File) -> io::Result<()>,
    M: FnOnce(&File, &File) -> io::Result<()>,
    Y: FnOnce(&File) -> io::Result<()>,
    B: FnOnce(&Path),
    A: FnOnce(),
    V: FnOnce() -> io::Result<()>,
{
    let (temp_path, mut temp_file) = create_temp_file(path, suffix)?;
    let result = (|| {
        if let Some(source) = metadata_source {
            preserve_metadata(source, &temp_file).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "could not preserve metadata for {}: {error}",
                        path.display()
                    ),
                )
            })?;
        }
        write(&mut temp_file)?;
        if let Some(source) = metadata_source {
            let mode = source.metadata()?.mode();
            temp_file.set_permissions(fs::Permissions::from_mode(mode))?;
        }
        sync_file(&temp_file)?;
        validate()?;
        before_commit(&temp_path);
        let saved_stamp = commit_temp_file(
            &temp_path,
            path,
            &temp_file,
            expected_target,
            commit_context,
            after_rename,
        )?;
        Ok(saved_stamp)
    })();

    if result.is_err() {
        remove_created_temp(&temp_path, &temp_file);
    }

    result
}

fn validate_save_target(
    path: &Path,
    expected_location: &SaveLocation,
    expected_stamp: DiskStamp,
) -> io::Result<Option<File>> {
    let (location, stamp) = current_disk_state(path)?;
    if &location != expected_location || stamp != expected_stamp {
        return Err(disk_changed_error(path));
    }

    match location {
        SaveLocation::Missing { .. } => Ok(None),
        SaveLocation::Regular { target } | SaveLocation::Symlink { target } => {
            let file = File::open(target)?;
            if stamp_for_metadata(&file.metadata()?) != expected_stamp {
                return Err(disk_changed_error(path));
            }
            Ok(Some(file))
        }
    }
}

fn disk_changed_error(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::WouldBlock,
        format!(
            "file changed on disk since it was opened: {}",
            path.display()
        ),
    )
}

fn copy_metadata(source: &File, destination: &File) -> io::Result<()> {
    // SAFETY: both descriptors remain open for the duration of the call, the
    // state pointer is allowed to be null, and COPYFILE_METADATA copies no data.
    let result = unsafe {
        libc::fcopyfile(
            source.as_raw_fd(),
            destination.as_raw_fd(),
            std::ptr::null_mut(),
            libc::COPYFILE_METADATA,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn semantic_metadata(file: &File) -> io::Result<SemanticMetadata> {
    let metadata = file.metadata()?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: stat points to writable storage and file remains open.
    if unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstat initialized stat after returning success.
    let stat = unsafe { stat.assume_init() };

    Ok(SemanticMetadata {
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        flags: stat.st_flags,
        acl: acl_bytes(file)?,
        xattrs: extended_attributes(file)?,
    })
}

fn acl_bytes(file: &File) -> io::Result<Vec<u8>> {
    // SAFETY: file remains open and ACL_TYPE_EXTENDED is valid on macOS.
    let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), ACL_TYPE_EXTENDED) };
    if acl.is_null() {
        let error = io::Error::last_os_error();
        return match error.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::Unsupported => Ok(Vec::new()),
            _ => Err(error),
        };
    }

    // SAFETY: acl is a valid object returned by acl_get_fd_np.
    let size = unsafe { acl_size(acl) };
    if size < 0 {
        let error = io::Error::last_os_error();
        // SAFETY: acl is owned by this function.
        unsafe {
            acl_free(acl);
        }
        return Err(error);
    }

    let mut bytes = vec![0_u8; size as usize];
    // SAFETY: bytes has size bytes of writable storage and acl is valid.
    let copied = unsafe { acl_copy_ext(bytes.as_mut_ptr().cast(), acl, size) };
    // SAFETY: acl is owned by this function and no longer used.
    unsafe {
        acl_free(acl);
    }
    if copied < 0 {
        return Err(io::Error::last_os_error());
    }
    bytes.truncate(copied as usize);
    Ok(bytes)
}

fn extended_attributes(file: &File) -> io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
    for _ in 0..FILE_READ_ATTEMPTS {
        // SAFETY: a null buffer with zero size requests the required length.
        let size = unsafe { libc::flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0, 0) };
        if size < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::Unsupported {
                Ok(Vec::new())
            } else {
                Err(error)
            };
        }

        let mut names = vec![0_u8; size as usize];
        // SAFETY: names provides size writable bytes.
        let read = unsafe {
            libc::flistxattr(file.as_raw_fd(), names.as_mut_ptr().cast(), names.len(), 0)
        };
        if read < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ERANGE) {
                continue;
            }
            return Err(error);
        }
        names.truncate(read as usize);

        let mut attributes = Vec::new();
        for name in names
            .split(|byte| *byte == 0)
            .filter(|name| !name.is_empty())
        {
            attributes.push((name.to_vec(), extended_attribute(file, name)?));
        }
        attributes.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        return Ok(attributes);
    }

    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        "extended attributes changed while saving",
    ))
}

fn extended_attribute(file: &File, name: &[u8]) -> io::Result<Vec<u8>> {
    let name = CString::new(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "extended attribute name contains a null byte",
        )
    })?;

    for _ in 0..FILE_READ_ATTEMPTS {
        // SAFETY: a null buffer with zero size requests the required length.
        let size = unsafe {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                std::ptr::null_mut(),
                0,
                0,
                0,
            )
        };
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut value = vec![0_u8; size as usize];
        // SAFETY: value provides size writable bytes and name is a valid C string.
        let read = unsafe {
            libc::fgetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
                0,
                0,
            )
        };
        if read < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ERANGE) {
                continue;
            }
            return Err(error);
        }
        value.truncate(read as usize);
        return Ok(value);
    }

    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        "extended attribute changed while saving",
    ))
}

fn file_text_matches(file: &File, expected: &Rope) -> io::Result<bool> {
    let mut file = file.try_clone()?;
    file.rewind()?;
    let mut reader = BufReader::new(file);

    for chunk in expected.chunks() {
        let mut expected_bytes = chunk.as_bytes();
        while !expected_bytes.is_empty() {
            let actual = reader.fill_buf()?;
            if actual.is_empty() {
                return Ok(false);
            }
            let compared = actual.len().min(expected_bytes.len());
            if actual[..compared] != expected_bytes[..compared] {
                return Ok(false);
            }
            reader.consume(compared);
            expected_bytes = &expected_bytes[compared..];
        }
    }

    Ok(reader.fill_buf()?.is_empty())
}

fn commit_temp_file<A>(
    temp_path: &Path,
    target_path: &Path,
    temp_file: &File,
    expected_target: Option<DiskStamp>,
    context: Option<&CommitContext<'_>>,
    after_rename: A,
) -> io::Result<DiskStamp>
where
    A: FnOnce(),
{
    let mut after_rename = Some(after_rename);
    let Some(expected_target) = expected_target else {
        rename_with_flags(temp_path, target_path, libc::RENAME_EXCL)?;
        let saved_stamp = stamp_for_metadata(&temp_file.metadata()?);
        let committed_identity = path_identity(target_path)?;
        after_rename.take().unwrap()();
        if let Err(error) = verify_committed_save(target_path, temp_file, context, false) {
            return rollback_exclusive_rename(temp_path, target_path, committed_identity, error)
                .and(Ok(saved_stamp));
        }
        return Ok(saved_stamp);
    };

    rename_with_flags(temp_path, target_path, libc::RENAME_SWAP)?;
    let saved_stamp = stamp_for_metadata(&temp_file.metadata()?);
    let committed_identity = path_identity(target_path)?;
    let displaced_identity = path_identity(temp_path)?;
    after_rename.take().unwrap()();
    let replaced_stamp = regular_file_stamp(temp_path);

    if !matches!(
        replaced_stamp,
        Ok(stamp) if same_file_version_after_rename(stamp, expected_target)
    ) {
        return rollback_swap(
            temp_path,
            target_path,
            committed_identity,
            displaced_identity,
            disk_changed_error(target_path),
        )
        .and(Ok(saved_stamp));
    }

    if let Err(error) = verify_committed_save(target_path, temp_file, context, true) {
        return rollback_swap(
            temp_path,
            target_path,
            committed_identity,
            displaced_identity,
            error,
        )
        .and(Ok(saved_stamp));
    }

    if !matches!(
        path_identity(target_path),
        Ok(identity) if identity == committed_identity
    ) || !matches!(
        path_identity(temp_path),
        Ok(identity) if identity == displaced_identity
    ) {
        return Err(recovery_path_error(
            "original target changed before cleanup",
            temp_path,
        ));
    }
    if let Err(error) = fs::remove_file(temp_path) {
        return rollback_swap(
            temp_path,
            target_path,
            committed_identity,
            displaced_identity,
            error,
        )
        .and(Ok(saved_stamp));
    }

    Ok(saved_stamp)
}

fn verify_committed_save(
    target_path: &Path,
    temp_file: &File,
    context: Option<&CommitContext<'_>>,
    replaced_existing: bool,
) -> io::Result<()> {
    if !same_file(target_path, temp_file)? {
        return Err(io::Error::other(
            "temporary save file changed before it could be committed",
        ));
    }

    let Some(context) = context else {
        return Ok(());
    };

    validate_committed_location(context.buffer_path, context.expected_location)?;
    if let Some(source) = &context.source {
        if !replaced_existing
            || semantic_metadata(source.file)? != source.metadata
            || !file_text_matches(source.file, source.text)?
        {
            return Err(disk_changed_error(context.buffer_path));
        }
    }
    Ok(())
}

fn validate_committed_location(path: &Path, expected: &SaveLocation) -> io::Result<()> {
    let committed = resolve_save_location(path)?;
    let matches = match (expected, committed) {
        (
            SaveLocation::Missing {
                target: expected_target,
            },
            SaveLocation::Regular {
                target: committed_target,
            },
        )
        | (
            SaveLocation::Regular {
                target: expected_target,
            },
            SaveLocation::Regular {
                target: committed_target,
            },
        )
        | (
            SaveLocation::Symlink {
                target: expected_target,
            },
            SaveLocation::Symlink {
                target: committed_target,
            },
        ) => expected_target == &committed_target,
        _ => false,
    };

    if matches {
        Ok(())
    } else {
        Err(disk_changed_error(path))
    }
}

fn same_file(path: &Path, file: &File) -> io::Result<bool> {
    let path_metadata = fs::symlink_metadata(path)?;
    let file_metadata = file.metadata()?;
    Ok(path_metadata.dev() == file_metadata.dev() && path_metadata.ino() == file_metadata.ino())
}

fn rollback_exclusive_rename(
    temp_path: &Path,
    target_path: &Path,
    committed_identity: PathIdentity,
    cause: io::Error,
) -> io::Result<()> {
    if !matches!(
        path_identity(target_path),
        Ok(identity) if identity == committed_identity
    ) {
        return Err(io::Error::new(
            cause.kind(),
            format!("{cause}; save target changed again, so no rollback was attempted"),
        ));
    }

    match rename_with_flags(target_path, temp_path, libc::RENAME_EXCL) {
        Ok(()) => Err(cause),
        Err(rollback_error) => Err(io::Error::new(
            rollback_error.kind(),
            format!(
                "{cause}; could not remove the unverified save target: {rollback_error}; inspect {}",
                target_path.display()
            ),
        )),
    }
}

fn same_file_version_after_rename(actual: DiskStamp, expected: DiskStamp) -> bool {
    match (actual, expected) {
        (
            DiskStamp::Present {
                device: actual_device,
                inode: actual_inode,
                len: actual_len,
                modified_seconds: actual_modified_seconds,
                modified_nanoseconds: actual_modified_nanoseconds,
                ..
            },
            DiskStamp::Present {
                device: expected_device,
                inode: expected_inode,
                len: expected_len,
                modified_seconds: expected_modified_seconds,
                modified_nanoseconds: expected_modified_nanoseconds,
                ..
            },
        ) => {
            actual_device == expected_device
                && actual_inode == expected_inode
                && actual_len == expected_len
                && actual_modified_seconds == expected_modified_seconds
                && actual_modified_nanoseconds == expected_modified_nanoseconds
        }
        _ => false,
    }
}

fn regular_file_stamp(path: &Path) -> io::Result<DiskStamp> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("save target is not a regular file: {}", path.display()),
        ));
    }
    Ok(stamp_for_metadata(&metadata))
}

fn path_identity(path: &Path) -> io::Result<PathIdentity> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(PathIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn rollback_swap(
    temp_path: &Path,
    target_path: &Path,
    committed_identity: PathIdentity,
    displaced_identity: PathIdentity,
    cause: io::Error,
) -> io::Result<()> {
    if !matches!(
        path_identity(target_path),
        Ok(identity) if identity == committed_identity
    ) || !matches!(
        path_identity(temp_path),
        Ok(identity) if identity == displaced_identity
    ) {
        return Err(recovery_path_error(
            &format!("{cause}; save paths changed again, so no rollback was attempted"),
            temp_path,
        ));
    }

    match rename_with_flags(temp_path, target_path, libc::RENAME_SWAP) {
        Ok(()) => Err(cause),
        Err(rollback_error) => Err(io::Error::new(
            rollback_error.kind(),
            format!(
                "{cause}; could not restore the original target: {rollback_error}; recover it from {}",
                temp_path.display()
            ),
        )),
    }
}

fn recovery_path_error(message: &str, recovery_path: &Path) -> io::Error {
    io::Error::other(format!(
        "{message}; recover the original target from {}",
        recovery_path.display()
    ))
}

fn rename_with_flags(from: &Path, to: &Path, flags: libc::c_uint) -> io::Result<()> {
    let from = CString::new(from.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "save path contains an interior null byte",
        )
    })?;
    let to = CString::new(to.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "save path contains an interior null byte",
        )
    })?;

    // SAFETY: both paths are valid C strings and AT_FDCWD resolves the absolute
    // stable target paths selected when the buffer was opened.
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            flags,
        )
    };

    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn remove_created_temp(path: &Path, file: &File) {
    let Ok(open_metadata) = file.metadata() else {
        return;
    };
    let Ok(path_metadata) = fs::symlink_metadata(path) else {
        return;
    };

    if open_metadata.dev() == path_metadata.dev()
        && open_metadata.ino() == path_metadata.ino()
        && fs::remove_file(path).is_err()
    {
        // Metadata copying may have made Cortex's own temporary inode
        // immutable. Clear file flags only after proving the pathname still
        // names that open inode, then retry cleanup.
        // SAFETY: file remains open and fchflags only affects that descriptor.
        unsafe {
            libc::fchflags(file.as_raw_fd(), 0);
        }
        let _ = fs::remove_file(path);
    }
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
    use super::{
        disk_stamp, temp_path_for, write_atomically_with, write_atomically_with_metadata, Buffer,
    };
    use std::{
        fs::{self, FileTimes},
        io,
        io::Write,
        os::unix::fs::{symlink, MetadataExt, PermissionsExt},
        path::PathBuf,
        process::Command,
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
    fn disk_changed_tracks_external_writes_deletion_and_creation() {
        let dir = test_dir("disk-changed");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.refresh_disk_changed();
        assert!(!buffer.disk_changed());

        fs::write(&path, "after with a different length").unwrap();
        buffer.refresh_disk_changed();
        assert!(buffer.disk_changed());

        fs::remove_file(&path).unwrap();
        buffer.refresh_disk_changed();
        assert!(buffer.disk_changed());

        let missing_path = dir.join("created.txt");
        let mut missing_buffer = Buffer::open(&missing_path).unwrap();
        fs::write(&missing_path, "created outside Cortex").unwrap();
        missing_buffer.refresh_disk_changed();
        assert!(missing_buffer.disk_changed());
        remove_dir(dir);
    }

    #[test]
    fn disk_changed_detects_same_length_edits_with_restored_mtime() {
        let dir = test_dir("disk-changed-restored-mtime");
        let path = dir.join("notes.txt");
        fs::write(&path, "AAAA").unwrap();
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        fs::write(&path, "BBBB").unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(modified))
            .unwrap();
        buffer.refresh_disk_changed();

        assert!(buffer.disk_changed());
        remove_dir(dir);
    }

    #[test]
    fn reload_replaces_a_clean_buffer_and_resets_disk_and_edit_history() {
        let dir = test_dir("reload-clean");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(0, "x");
        buffer.undo();
        let revision = buffer.revision();
        fs::write(&path, "after with a different length").unwrap();
        buffer.refresh_disk_changed();

        buffer.reload().unwrap();

        assert_eq!(buffer.text(), "after with a different length");
        assert!(!buffer.is_dirty());
        assert!(!buffer.disk_changed());
        assert_eq!(buffer.revision(), revision.wrapping_add(1));
        assert_eq!(buffer.undo(), None);
        assert_eq!(buffer.redo(), None);
        remove_dir(dir);
    }

    #[test]
    fn reload_refuses_to_replace_unsaved_edits() {
        let dir = test_dir("reload-dirty");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(0, "local ");
        fs::write(&path, "external change").unwrap();

        assert!(matches!(buffer.reload(), Err(super::ReloadError::Dirty)));
        assert_eq!(buffer.text(), "local before");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn save_refuses_to_overwrite_an_external_change() {
        let dir = test_dir("save-disk-baseline");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        fs::write(&path, "external change").unwrap();
        buffer.refresh_disk_changed();
        assert!(buffer.disk_changed());

        buffer.insert(0, "local ");
        let error = buffer.save().unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(error.to_string().contains("file changed on disk"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "external change");
        assert!(buffer.is_dirty());
        assert!(buffer.disk_changed());
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
    fn editing_near_eof_of_a_large_buffer_uses_one_history_state() {
        let dir = test_dir("large-eof-dirty-state");
        let path = dir.join("notes.txt");
        fs::write(&path, "line\n".repeat(100_000)).unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        let eof = buffer.len_chars();

        buffer.insert(eof, "tail");

        assert_eq!(buffer.history_state, 1);
        assert_eq!(buffer.next_history_state, 2);
        assert!(buffer.is_dirty());

        assert_eq!(buffer.undo(), Some(eof));
        assert_eq!(buffer.history_state, 0);
        assert!(!buffer.is_dirty());

        assert_eq!(buffer.redo(), Some(eof + 4));
        assert_eq!(buffer.history_state, 1);
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
    fn line_changed_tracks_inserted_and_deleted_lines() {
        let dir = test_dir("line-changed-structure");
        let inserted_path = dir.join("inserted.txt");
        let deleted_path = dir.join("deleted.txt");
        fs::write(&inserted_path, "alpha\nbeta\ngamma\n").unwrap();
        fs::write(&deleted_path, "alpha\nbeta\ngamma\n").unwrap();
        let mut inserted = Buffer::open(&inserted_path).unwrap();
        let mut deleted = Buffer::open(&deleted_path).unwrap();

        inserted.insert(inserted.line_start_char(1), "new\n");
        deleted.delete(deleted.line_start_char(1)..deleted.line_start_char(2));

        assert!(!inserted.line_changed(0));
        assert!(inserted.line_changed(1));
        assert!(inserted.line_changed(2));
        assert!(inserted.line_changed(3));
        assert!(!inserted.line_changed(4));

        assert!(!deleted.line_changed(0));
        assert!(deleted.line_changed(1));
        assert!(deleted.line_changed(2));
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
        buffer.refresh_disk_changed();
        assert!(!buffer.disk_changed());
        remove_dir(dir);
    }

    #[test]
    fn save_preserves_existing_mode_and_ownership() {
        let dir = test_dir("save-mode-ownership");
        let path = dir.join("script.sh");
        fs::write(&path, "#!/bin/sh\necho before\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o6751)).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(FileTimes::new().set_modified(UNIX_EPOCH))
            .unwrap();
        let before = fs::metadata(&path).unwrap();
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(buffer.len_chars(), "echo after\n");
        buffer.save().unwrap();

        let after = fs::metadata(&path).unwrap();
        assert_eq!(after.mode() & 0o7777, before.mode() & 0o7777);
        assert_eq!(after.uid(), before.uid());
        assert_eq!(after.gid(), before.gid());
        assert_ne!(after.modified().unwrap(), UNIX_EPOCH);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "#!/bin/sh\necho before\necho after\n"
        );
        remove_dir(dir);
    }

    #[test]
    fn save_preserves_extended_attributes_and_acls() {
        let dir = test_dir("save-macos-metadata");
        let path = dir.join("notes.txt");
        fs::write(&path, "before").unwrap();
        command(&["xattr", "-w", "com.cortex.save-test", "preserved"], &path);
        command(&["chmod", "+a", "everyone allow read"], &path);
        let acl_before = acl_entries(&path);
        assert!(!acl_before.is_empty());
        let mut buffer = Buffer::open(&path).unwrap();

        buffer.insert(buffer.len_chars(), " after");
        buffer.save().unwrap();

        assert_eq!(
            command_output(&["xattr", "-p", "com.cortex.save-test"], &path),
            "preserved"
        );
        assert_eq!(acl_entries(&path), acl_before);
        remove_dir(dir);
    }

    #[test]
    fn save_through_a_symlink_updates_its_target_and_keeps_the_link() {
        let dir = test_dir("save-symlink");
        let target = dir.join("target.txt");
        let link = dir.join("link.txt");
        fs::write(&target, "before").unwrap();
        symlink("target.txt", &link).unwrap();
        let mut buffer = Buffer::open(&link).unwrap();

        buffer.insert(buffer.len_chars(), " after");
        buffer.save().unwrap();

        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_link(&link).unwrap(), PathBuf::from("target.txt"));
        assert_eq!(fs::read_to_string(&target).unwrap(), "before after");
        remove_dir(dir);
    }

    #[test]
    fn save_refuses_a_retargeted_symlink() {
        let dir = test_dir("save-retargeted-symlink");
        let original = dir.join("original.txt");
        let redirected = dir.join("redirected.txt");
        let link = dir.join("link.txt");
        fs::write(&original, "original").unwrap();
        fs::write(&redirected, "redirected").unwrap();
        symlink("original.txt", &link).unwrap();
        let mut buffer = Buffer::open(&link).unwrap();
        buffer.insert(buffer.len_chars(), " local");
        fs::remove_file(&link).unwrap();
        symlink("redirected.txt", &link).unwrap();

        let error = buffer.save().unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
        assert_eq!(fs::read_to_string(&redirected).unwrap(), "redirected");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn opening_a_broken_symlink_fails_clearly() {
        let dir = test_dir("open-broken-symlink");
        let link = dir.join("link.txt");
        symlink("missing.txt", &link).unwrap();

        let error = Buffer::open(&link).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("could not resolve symlink"));
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        remove_dir(dir);
    }

    #[test]
    fn save_race_preserves_an_external_replacement_and_cleans_up() {
        let dir = test_dir("save-race");
        let path = dir.join("notes.txt");
        let replacement = dir.join("replacement.txt");
        fs::write(&path, "original").unwrap();
        fs::write(&replacement, "external replacement").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(buffer.len_chars(), " local");

        let error = buffer
            .save_with(|_| fs::rename(&replacement, &path).unwrap())
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(fs::read_to_string(&path).unwrap(), "external replacement");
        assert!(buffer.is_dirty());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn save_race_preserves_same_length_external_content_with_restored_mtime() {
        let dir = test_dir("save-content-race");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(buffer.len_chars(), " local");

        let error = buffer
            .save_with(|_| {
                fs::write(&path, "changed!").unwrap();
                fs::File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_times(FileTimes::new().set_modified(modified))
                    .unwrap();
            })
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(fs::read_to_string(&path).unwrap(), "changed!");
        assert!(buffer.is_dirty());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn save_race_preserves_external_metadata_changes() {
        let dir = test_dir("save-metadata-race");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(buffer.len_chars(), " local");

        let error = buffer
            .save_with(|_| {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
                command(&["xattr", "-w", "com.cortex.race-test", "external"], &path);
            })
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o640);
        assert_eq!(
            command_output(&["xattr", "-p", "com.cortex.race-test"], &path),
            "external"
        );
        assert!(buffer.is_dirty());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn save_race_preserves_a_final_symlink_retarget() {
        let dir = test_dir("save-final-symlink-race");
        let original = dir.join("original.txt");
        let redirected = dir.join("redirected.txt");
        let link = dir.join("link.txt");
        fs::write(&original, "original").unwrap();
        fs::write(&redirected, "redirected").unwrap();
        symlink("original.txt", &link).unwrap();
        let mut buffer = Buffer::open(&link).unwrap();
        buffer.insert(buffer.len_chars(), " local");

        let error = buffer
            .save_with(|_| {
                fs::remove_file(&link).unwrap();
                symlink("redirected.txt", &link).unwrap();
            })
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(fs::read_to_string(&original).unwrap(), "original");
        assert_eq!(fs::read_to_string(&redirected).unwrap(), "redirected");
        assert_eq!(
            fs::read_link(&link).unwrap(),
            PathBuf::from("redirected.txt")
        );
        assert!(buffer.is_dirty());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn save_never_commits_a_replaced_temporary_path() {
        let dir = test_dir("save-temp-path-race");
        let path = dir.join("notes.txt");
        let moved_temp = dir.join("moved-cortex-temp");
        fs::write(&path, "original").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(buffer.len_chars(), " local");
        let mut foreign_temp = None;

        let error = buffer
            .save_with(|temp_path| {
                fs::rename(temp_path, &moved_temp).unwrap();
                fs::write(temp_path, "foreign").unwrap();
                foreign_temp = Some(temp_path.to_path_buf());
            })
            .unwrap_err();

        assert!(error.to_string().contains("temporary save file changed"));
        let foreign_temp = foreign_temp.unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_to_string(&foreign_temp).unwrap(), "foreign");
        assert_eq!(fs::read_to_string(&moved_temp).unwrap(), "original local");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn existing_save_does_not_rollback_over_a_post_rename_replacement() {
        let dir = test_dir("existing-post-rename-race");
        let path = dir.join("notes.txt");
        let replacement = dir.join("replacement.txt");
        fs::write(&path, "original").unwrap();
        fs::write(&replacement, "external").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(buffer.len_chars(), " local");
        let temp_path = std::cell::RefCell::new(None);

        let error = buffer
            .save_with_hooks(
                |path| *temp_path.borrow_mut() = Some(path.to_path_buf()),
                || fs::rename(&replacement, &path).unwrap(),
            )
            .unwrap_err();

        let recovery = temp_path.into_inner().unwrap();
        assert!(error.to_string().contains("no rollback was attempted"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert_eq!(fs::read_to_string(&recovery).unwrap(), "original");
        assert!(buffer.is_dirty());
        remove_dir(dir);
    }

    #[test]
    fn new_save_does_not_rollback_over_a_post_rename_replacement() {
        let dir = test_dir("new-post-rename-race");
        let path = dir.join("notes.txt");
        let replacement = dir.join("replacement.txt");
        fs::write(&replacement, "external").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(0, "local");

        let error = buffer
            .save_with_hooks(|_| {}, || fs::rename(&replacement, &path).unwrap())
            .unwrap_err();

        assert!(error.to_string().contains("no rollback was attempted"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert!(buffer.is_dirty());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn post_commit_in_place_edit_is_reported_as_a_later_disk_change() {
        let dir = test_dir("post-commit-in-place-edit");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(buffer.len_chars(), " local");

        buffer
            .save_with_hooks(|_| {}, || fs::write(&path, "external").unwrap())
            .unwrap();
        buffer.refresh_disk_changed();

        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert!(!buffer.is_dirty());
        assert!(buffer.disk_changed());
        remove_dir(dir);
    }

    #[test]
    fn new_file_commit_race_preserves_the_external_file_and_cleans_up() {
        let dir = test_dir("new-file-save-race");
        let path = dir.join("notes.txt");
        let mut buffer = Buffer::open(&path).unwrap();
        buffer.insert(0, "local");

        let error = buffer
            .save_with(|_| fs::write(&path, "external").unwrap())
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
        assert!(buffer.is_dirty());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn metadata_failure_preserves_the_target_and_cleans_up() {
        let dir = test_dir("save-metadata-failure");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let source = fs::File::open(&path).unwrap();
        let expected = disk_stamp(&path).unwrap();

        let error = write_atomically_with_metadata(
            &path,
            Some(&source),
            Some(expected),
            None,
            || Ok("metadata-failure".to_string()),
            |file| file.write_all(b"saved"),
            |_, _| {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected metadata failure",
                ))
            },
            fs::File::sync_all,
            || Ok(()),
            |_| {},
            || {},
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("could not preserve metadata"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    #[test]
    fn sync_failure_preserves_the_target_and_cleans_up() {
        let dir = test_dir("save-sync-failure");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();
        let source = fs::File::open(&path).unwrap();
        let expected = disk_stamp(&path).unwrap();

        let error = write_atomically_with_metadata(
            &path,
            Some(&source),
            Some(expected),
            None,
            || Ok("sync-failure".to_string()),
            |file| file.write_all(b"saved"),
            super::copy_metadata,
            |_| Err(io::Error::other("injected sync failure")),
            || Ok(()),
            |_| {},
            || {},
        )
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_no_cortex_temp_files(&dir);
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
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o111, 0);
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
            None,
            Some(disk_stamp(&path).unwrap()),
            None,
            || Ok(suffixes.next().unwrap().to_string()),
            |file| file.write_all(b"saved"),
            || Ok(()),
            |_| {},
            || {},
        )
        .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "saved");
        assert_eq!(fs::read_to_string(&occupied).unwrap(), "attacker content");
        assert!(!temp_path_for(&path, "available").exists());
        remove_dir(dir);
    }

    #[test]
    fn atomic_write_returns_the_stamp_of_the_saved_file() {
        let dir = test_dir("save-returned-stamp");
        let path = dir.join("notes.txt");
        fs::write(&path, "original").unwrap();

        let saved_stamp = write_atomically_with(
            &path,
            None,
            Some(disk_stamp(&path).unwrap()),
            None,
            || Ok("saved".to_string()),
            |file| file.write_all(b"saved"),
            || Ok(()),
            |_| {},
            || {},
        )
        .unwrap();

        assert_eq!(saved_stamp, disk_stamp(&path).unwrap());
        fs::remove_file(&path).unwrap();
        fs::write(&path, "external replacement").unwrap();
        assert_ne!(saved_stamp, disk_stamp(&path).unwrap());
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
            None,
            Some(disk_stamp(&path).unwrap()),
            None,
            || Ok(suffixes.next().unwrap().to_string()),
            |file| file.write_all(b"saved"),
            || Ok(()),
            |_| {},
            || {},
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
            None,
            Some(disk_stamp(&path).unwrap()),
            None,
            || {
                attempts += 1;
                Ok(if attempts == 1 {
                    "occupied"
                } else {
                    "available"
                }
                .to_string())
            },
            |file| file.write_all(b"saved"),
            || Ok(()),
            |_| {},
            || {},
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
            None,
            Some(disk_stamp(&path).unwrap()),
            None,
            || Ok(suffixes.next().unwrap().to_string()),
            |file| {
                file.write_all(b"partial")?;
                Err(io::Error::other("injected write failure"))
            },
            || Ok(()),
            |_| {},
            || {},
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

        let result = buffer.save_with(|_| {
            // Change the target after validation, writing, metadata preservation,
            // and sync so the atomic commit must detect and roll back the race.
            fs::remove_file(&path).unwrap();
            fs::create_dir(&path).unwrap();
        });

        assert!(result.is_err());
        assert!(buffer.is_dirty());
        assert!(path.is_dir());
        assert_no_cortex_temp_files(&dir);
        remove_dir(dir);
    }

    fn command(args: &[&str], path: &std::path::Path) {
        let status = Command::new(args[0])
            .args(&args[1..])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success(), "{} failed", args[0]);
    }

    fn command_output(args: &[&str], path: &std::path::Path) -> String {
        let output = Command::new(args[0])
            .args(&args[1..])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success(), "{} failed", args[0]);
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn acl_entries(path: &std::path::Path) -> Vec<String> {
        command_output(&["ls", "-lde"], path)
            .lines()
            .filter(|line| line.trim_start().starts_with("0:"))
            .map(str::to_string)
            .collect()
    }

    fn assert_no_cortex_temp_files(dir: &std::path::Path) {
        let leftover_temp = fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains(".cortex-"));
        assert!(!leftover_temp, "temporary save file should be cleaned up");
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
