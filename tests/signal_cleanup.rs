use std::{
    fs::{self, File},
    io::{self, Read, Write},
    mem::MaybeUninit,
    os::fd::{AsRawFd, FromRawFd, RawFd},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

const ALT_SCREEN_ENTER: &[u8] = b"\x1b[?1049h";
const ALT_SCREEN_LEAVE: &[u8] = b"\x1b[?1049l";
const CURSOR_HIDE: &[u8] = b"\x1b[?25l";
const CURSOR_SHOW: &[u8] = b"\x1b[?25h";
const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_INTERVAL: Duration = Duration::from_millis(10);
const QUIET_INTERVAL: Duration = Duration::from_millis(50);
static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn editor_restores_terminal_after_termination_signals() {
    let fixture = Fixture::new("editor");
    let path = fixture.path().join("file.txt");
    fs::write(&path, "before\n").expect("write editor fixture");

    for signal in [libc::SIGTERM, libc::SIGHUP] {
        let mut session = PtySession::spawn(&path);
        session.wait_for_output(ALT_SCREEN_ENTER);
        session.wait_for_output(CURSOR_HIDE);
        session.wait_for_quiet();

        let output_len = session.output.len();
        session.master.write_all(b"x").expect("send editor input");
        session.wait_for_output_growth(output_len);

        session.send_signal(signal);
        if signal == libc::SIGTERM {
            session.send_signal(signal);
        }
        let status = session.wait_for_exit();

        assert!(!status.success(), "signal shutdown must be non-zero");
        session.assert_raw_mode_restored();
        assert_contains(&session.output, ALT_SCREEN_LEAVE, "alternate screen leave");
        assert_contains(&session.output, CURSOR_SHOW, "cursor show");
        assert_eq!(
            fs::read_to_string(&path).expect("read editor fixture"),
            "before\n",
            "signal shutdown must not save dirty in-memory edits"
        );
    }
}

#[test]
fn picker_restores_terminal_after_termination_signals() {
    let fixture = Fixture::new("picker");

    for signal in [libc::SIGTERM, libc::SIGHUP] {
        let mut session = PtySession::spawn(fixture.path());
        session.wait_for_output(ALT_SCREEN_ENTER);
        session.wait_for_output(CURSOR_HIDE);
        session.wait_for_quiet();
        session.send_signal(signal);
        let status = session.wait_for_exit();

        assert!(!status.success(), "signal shutdown must be non-zero");
        session.assert_raw_mode_restored();
        assert_contains(&session.output, ALT_SCREEN_LEAVE, "alternate screen leave");
        assert_contains(&session.output, CURSOR_SHOW, "cursor show");
    }
}

#[test]
fn nested_picker_restores_terminal_after_mixed_termination_signals() {
    let fixture = Fixture::new("nested-picker");
    let current_path = fixture.path().join("current.txt");
    fs::write(&current_path, "before\n").expect("write current editor fixture");
    fs::write(fixture.path().join("nested-entry.txt"), "").expect("write picker entry");

    for first_signal in [libc::SIGTERM, libc::SIGHUP] {
        let mut session = PtySession::spawn(&current_path);
        session.wait_for_output(ALT_SCREEN_ENTER);
        session.wait_for_output(CURSOR_HIDE);
        session.wait_for_quiet();

        let output_len = session.output.len();
        session.master.write_all(b"x").expect("send editor input");
        session.wait_for_output_growth(output_len);
        let output_len = session.output.len();
        session.master.write_all(&[0x18]).expect("start key chord");
        session.wait_for_output_growth(output_len);
        session
            .master
            .write_all(&[0x06])
            .expect("open find-file prompt");
        session.wait_for_output(b"Find file:");
        session.wait_for_quiet();
        session
            .master
            .write_all(fixture.path().as_os_str().as_encoded_bytes())
            .expect("enter picker directory");
        session.master.write_all(b"\r").expect("submit picker path");
        session.wait_for_output(b"nested-entry.txt");
        session.wait_for_quiet();

        session.send_signal(first_signal);
        let second_signal = if first_signal == libc::SIGTERM {
            libc::SIGHUP
        } else {
            libc::SIGTERM
        };
        session.send_signal(second_signal);
        let status = session.wait_for_exit();

        assert!(!status.success(), "signal shutdown must be non-zero");
        session.assert_raw_mode_restored();
        assert_contains(&session.output, ALT_SCREEN_LEAVE, "alternate screen leave");
        assert_contains(&session.output, CURSOR_SHOW, "cursor show");
        assert_eq!(
            fs::read_to_string(&current_path).expect("read current editor fixture"),
            "before\n",
            "nested picker signal shutdown must not save dirty edits"
        );
    }
}

struct PtySession {
    child: Child,
    master: File,
    original_termios: libc::termios,
    output: Vec<u8>,
}

impl PtySession {
    fn spawn(path: &Path) -> Self {
        let (master, slave, original_termios) = open_pty().expect("open PTY");
        let stdin = slave.try_clone().expect("clone PTY slave for stdin");
        let stdout = slave.try_clone().expect("clone PTY slave for stdout");
        let mut command = Command::new(env!("CARGO_BIN_EXE_cortex"));
        command
            .arg(path)
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(slave));

        let child = command.spawn().expect("spawn Cortex in PTY");
        set_nonblocking(master.as_raw_fd()).expect("make PTY master nonblocking");

        Self {
            child,
            master,
            original_termios,
            output: Vec::new(),
        }
    }

    fn wait_for_output(&mut self, needle: &[u8]) {
        let description = format!("terminal output containing {needle:?}");
        self.wait_until(|session| contains(&session.output, needle), &description);
    }

    fn wait_for_output_growth(&mut self, prior_len: usize) {
        self.wait_until(
            |session| session.output.len() > prior_len,
            "editor render after input",
        );
    }

    fn wait_for_quiet(&mut self) {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        let mut previous_len = self.output.len();
        loop {
            thread::sleep(QUIET_INTERVAL);
            self.read_available();
            if self.output.len() == previous_len {
                return;
            }
            previous_len = self.output.len();
            assert!(
                Instant::now() < deadline,
                "terminal output did not become quiet"
            );
        }
    }

    fn send_signal(&self, signal: i32) {
        let result = unsafe { libc::kill(self.child.id() as i32, signal) };
        assert_eq!(result, 0, "send signal {signal}");
    }

    fn wait_for_exit(&mut self) -> ExitStatus {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
            self.read_available();
            if let Some(status) = self.child.try_wait().expect("poll Cortex process") {
                self.read_available();
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "Cortex did not exit after signal"
            );
            thread::sleep(WAIT_INTERVAL);
        }
    }

    fn assert_raw_mode_restored(&self) {
        let restored = read_termios(self.master.as_raw_fd()).expect("read restored PTY state");
        assert_raw_termios_eq(&self.original_termios, &restored);
    }

    fn wait_until(&mut self, condition: impl Fn(&Self) -> bool, description: &str) {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        while !condition(self) {
            self.read_available();
            assert!(
                self.child
                    .try_wait()
                    .expect("poll Cortex process")
                    .is_none(),
                "Cortex exited before {description}"
            );
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}; output tail: {:?}",
                String::from_utf8_lossy(&self.output[self.output.len().saturating_sub(500)..])
            );
            thread::sleep(WAIT_INTERVAL);
        }
    }

    fn read_available(&mut self) {
        let mut buffer = [0_u8; 8192];
        loop {
            match self.master.read(&mut buffer) {
                Ok(0) => return,
                Ok(read) => self.output.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                Err(error) => panic!("read PTY output: {error}"),
            }
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn open_pty() -> io::Result<(File, File, libc::termios)> {
    let mut master_fd = -1;
    let mut slave_fd = -1;
    let mut window_size = libc::winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = unsafe {
        libc::openpty(
            &mut master_fd,
            &mut slave_fd,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut window_size,
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    let original_termios = read_termios(slave_fd)?;
    let master = unsafe { File::from_raw_fd(master_fd) };
    let slave = unsafe { File::from_raw_fd(slave_fd) };
    Ok((master, slave, original_termios))
}

fn read_termios(fd: RawFd) -> io::Result<libc::termios> {
    let mut state = MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(fd, state.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { state.assume_init() })
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn assert_raw_termios_eq(expected: &libc::termios, actual: &libc::termios) {
    let input_mask = libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON;
    let control_mask = libc::CSIZE | libc::PARENB;
    let local_mask = libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG;

    assert_eq!(
        expected.c_iflag & input_mask,
        actual.c_iflag & input_mask,
        "raw-mode input flags"
    );
    assert_eq!(
        expected.c_oflag & libc::OPOST,
        actual.c_oflag & libc::OPOST,
        "raw-mode output flags"
    );
    assert_eq!(
        expected.c_cflag & control_mask,
        actual.c_cflag & control_mask,
        "raw-mode control flags"
    );
    assert_eq!(
        expected.c_lflag & local_mask,
        actual.c_lflag & local_mask,
        "raw-mode local flags"
    );
    assert_eq!(
        expected.c_cc[libc::VMIN],
        actual.c_cc[libc::VMIN],
        "raw-mode minimum input"
    );
    assert_eq!(
        expected.c_cc[libc::VTIME],
        actual.c_cc[libc::VTIME],
        "raw-mode input timeout"
    );
}

fn assert_contains(output: &[u8], needle: &[u8], description: &str) {
    assert!(contains(output, needle), "missing {description} sequence");
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("cortex-signal-{name}-{}-{id}", std::process::id()));
        fs::create_dir(&path).expect("create signal fixture");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("remove signal fixture");
    }
}
