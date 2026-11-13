//! Shared test harness for the integration tests.
//!
//! The shell switches the terminal into raw mode, so it can't be driven through
//! plain pipes. Every test therefore runs the real binary inside a pseudo
//! terminal (PTY) – the same way the Codecrafters tester does – types input and
//! inspects what the shell prints.
//!
//! Every shell gets its own temporary directory which is used as working
//! directory and `$HOME`. It contains a `bin/` directory that is prepended to
//! `$PATH`, so tests can provide their own executables.

#![allow(dead_code)] // not every test file uses every helper

use rexpect::process::WaitStatus;
use rexpect::session::{PtySession, spawn_command};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Prompt printed by the shell before every input line.
pub const PROMPT: &str = "$ ";

/// Max time to wait for any expected output before the test fails.
const TIMEOUT_MS: u64 = 5_000;

pub struct ShellBuilder {
    dir: TempDir,
    envs: Vec<(String, String)>,
    path_dirs: Vec<String>,
}

impl ShellBuilder {
    /// Adds directories (relative to the temp dir) to the front of `$PATH`,
    /// in the given order and before the default `bin/`. They are created
    /// on spawn.
    pub fn path_dirs(mut self, dirs: &[&str]) -> Self {
        self.path_dirs.extend(dirs.iter().map(|d| d.to_string()));
        self
    }

    /// Sets an additional environment variable for the shell process.
    pub fn env(mut self, key: &str, value: impl AsRef<Path>) -> Self {
        self.envs
            .push((key.to_string(), value.as_ref().display().to_string()));
        self
    }

    /// The temporary directory the shell will run in (before spawning).
    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    pub fn spawn(self) -> Shell {
        let root = self.dir.path().to_path_buf();
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();

        let mut path_entries = Vec::new();
        for dir in &self.path_dirs {
            let dir = root.join(dir);
            fs::create_dir_all(&dir).unwrap();
            path_entries.push(dir.display().to_string());
        }
        path_entries.push(bin.display().to_string());
        path_entries.push(std::env::var("PATH").unwrap_or_default());

        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codecrafters-shell"));
        cmd.current_dir(&root)
            .env("HOME", &root)
            .env("PATH", path_entries.join(":"))
            .env("TERM", "dumb")
            .env("LC_ALL", "C")
            .env_remove("HISTFILE");
        for (key, value) in &self.envs {
            cmd.env(key, value);
        }

        let mut session = spawn_command(cmd, Some(TIMEOUT_MS)).expect("failed to spawn shell");
        session
            .exp_string(PROMPT)
            .expect("shell did not print the initial prompt");

        Shell {
            session,
            dir: self.dir,
        }
    }
}

pub struct Shell {
    session: PtySession,
    dir: TempDir,
}

impl Shell {
    /// Starts a shell with default settings.
    pub fn spawn() -> Shell {
        Shell::builder().spawn()
    }

    /// Allows configuring the environment before the shell is started.
    pub fn builder() -> ShellBuilder {
        ShellBuilder {
            dir: tempfile::tempdir().expect("failed to create temp dir"),
            envs: Vec::new(),
            path_dirs: Vec::new(),
        }
    }

    /// Working directory / `$HOME` of the shell.
    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    /// Path to a file inside the shell's working directory.
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Creates an executable shell script `name` in a directory on `$PATH`.
    pub fn add_executable(&self, name: &str, script_body: &str) -> PathBuf {
        self.add_executable_at(&format!("bin/{name}"), script_body)
    }

    /// Creates an executable shell script at a path relative to the temp dir.
    pub fn add_executable_at(&self, rel_path: &str, script_body: &str) -> PathBuf {
        let path = self.write_file(rel_path, &format!("#!/bin/sh\n{script_body}\n"));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Creates a named pipe (FIFO) relative to the working dir.
    /// A `cat` on it blocks until the test writes to / closes the pipe.
    pub fn mkfifo(&self, name: &str) -> PathBuf {
        let path = self.path(name);
        let status = Command::new("mkfifo").arg(&path).status().unwrap();
        assert!(status.success(), "mkfifo failed");
        path
    }

    /// Writes `text` into a FIFO and closes it (the reader then sees EOF).
    /// Blocks until a reader has opened the FIFO.
    pub fn write_fifo(&self, name: &str, text: &str) {
        fs::write(self.path(name), text).unwrap();
    }

    /// Appends text to a file relative to the working dir.
    pub fn append_file(&self, name: &str, content: &str) {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .append(true)
            .open(self.path(name))
            .unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// Creates a file (and its parent directories) relative to the working dir.
    pub fn write_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.path(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    /// Reads a file relative to the working dir.
    pub fn read_file(&self, name: &str) -> String {
        fs::read_to_string(self.path(name))
            .unwrap_or_else(|e| panic!("could not read {name}: {e}"))
    }

    /// Types `line`, presses enter and returns everything the shell printed
    /// (stdout and stderr) until the next prompt, normalized to plain text.
    pub fn run(&mut self, line: &str) -> String {
        self.session.send_line(line).unwrap();
        self.read_until_prompt()
    }

    /// Types `line` and presses enter without waiting for anything.
    /// Use with `expect_raw` / `wait_for_prompt` for long-running commands.
    pub fn send_line(&mut self, line: &str) {
        self.session.send_line(line).unwrap();
    }

    /// Returns everything (normalized) printed until the next prompt.
    /// Unlike `run`, nothing is skipped – the caller decides what was consumed.
    pub fn wait_for_prompt(&mut self) -> String {
        match self.session.exp_string(PROMPT) {
            Ok(raw) => normalize(&raw),
            Err(e) => panic!("shell did not return to the prompt: {e}"),
        }
    }

    /// Types text without pressing enter (`\t` presses TAB).
    pub fn type_text(&mut self, text: &str) {
        self.session.send(text).unwrap();
        self.session.flush().unwrap();
    }

    /// Sends raw bytes, e.g. escape sequences for arrow keys.
    pub fn press_up(&mut self) {
        self.type_text("\x1b[A");
    }

    pub fn press_down(&mut self) {
        self.type_text("\x1b[B");
    }

    /// Presses enter on the current input line and returns the command output.
    pub fn enter(&mut self) -> String {
        self.type_text("\r");
        self.read_until_prompt()
    }

    /// Waits until the raw terminal output contains `expected`.
    /// Everything printed before it is consumed.
    pub fn expect_raw(&mut self, expected: &str) {
        if let Err(e) = self.session.exp_string(expected) {
            panic!("did not receive {expected:?}: {e}");
        }
    }

    /// Waits for the terminal bell.
    pub fn expect_bell(&mut self) {
        self.expect_raw("\x07");
    }

    /// Waits until the prompt line has been (re)drawn as `$ {input}`.
    pub fn expect_input_line(&mut self, input: &str) {
        self.expect_raw(&format!("{PROMPT}{input}"));
    }

    /// Sends `exit`, asserts the shell terminates with status 0 and
    /// returns everything it printed after the `exit` line.
    pub fn exit(&mut self) -> String {
        self.session.send_line("exit").unwrap();
        let raw = self
            .session
            .exp_eof()
            .expect("shell did not terminate after `exit`");
        let status = self.session.process().wait().unwrap();
        assert!(
            matches!(status, WaitStatus::Exited(_, 0)),
            "unexpected exit status: {status:?}"
        );
        let after_echo = raw.split_once("\r\n").map_or("", |(_, rest)| rest);
        normalize(after_echo)
    }

    fn read_until_prompt(&mut self) -> String {
        // Skip the echoed input line (it may contain redraws like "$ ..." from
        // tab completion or history) – the output starts after its line break.
        if let Err(e) = self.session.exp_string("\r\n") {
            panic!("shell did not accept the input line: {e}");
        }
        let raw = match self.session.exp_string(PROMPT) {
            Ok(raw) => raw,
            Err(e) => panic!("shell did not return to the prompt: {e}"),
        };
        normalize(&raw)
    }
}

/// Removes ANSI escape sequences, bells and `\r`.
fn normalize(raw: &str) -> String {
    strip_ansi(raw).replace(['\r', '\x07'], "")
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                // CSI: parameters until a final byte in 0x40..=0x7E
                for c in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        break;
                    }
                }
            } else {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Collapses runs of whitespace to single spaces in every line.
/// Useful for output with column padding (e.g. `jobs`, `wc`).
pub fn squash_ws(s: &str) -> String {
    s.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses the "[n] <pid>" line printed when a background job starts,
/// asserts the job number and returns the pid.
pub fn parse_job_start(out: &str, job_number: usize) -> u32 {
    let prefix = format!("[{job_number}] ");
    let pid = out
        .strip_prefix(&prefix)
        .and_then(|r| r.strip_suffix('\n'))
        .unwrap_or_else(|| panic!("expected \"{prefix}<pid>\", got {out:?}"));
    pid.parse()
        .unwrap_or_else(|_| panic!("not a pid: {pid:?} in {out:?}"))
}

/// Name of the executable running as `pid` (from /proc/<pid>/comm).
pub fn process_name(pid: u32) -> String {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .unwrap_or_else(|e| panic!("no process with pid {pid}: {e}"))
        .trim()
        .to_string()
}

/// Waits until `pid` has terminated (it may stay a zombie until the shell
/// reaps it). Makes "job finished" tests deterministic.
pub fn wait_until_exited(pid: u32) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(TIMEOUT_MS);
    loop {
        match fs::read_to_string(format!("/proc/{pid}/stat")) {
            Err(_) => return, // already gone
            Ok(stat) => {
                // state is the first field after the ")" closing the comm
                let state = stat.rsplit_once(')').map(|(_, r)| r.trim_start());
                if state.is_some_and(|s| s.starts_with('Z')) {
                    return;
                }
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "process {pid} did not exit in time"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
