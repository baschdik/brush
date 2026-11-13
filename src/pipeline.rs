//! Runs one pipeline (`cmd1 | cmd2 | ...`) of builtin and external commands.
//!
//! A pipeline is split into sections of consecutive builtins or consecutive
//! external commands. Each section gets the output of the previous section as
//! input. Builtins never read their input and run inside the shell process,
//! while a section of external commands is spawned as one chain of processes
//! connected by OS pipes.

use std::{
    io::Write,
    process::{Child, Stdio},
    thread::{self, JoinHandle},
};

use crate::{
    ShellState,
    cmd_parser::{CommandKind, UserCommand},
    output::{self, ensure_newline},
};

/// Where the stdout of the last process in an external chain goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainStdout {
    /// Redirect depending on the current Cmd.
    AsRedirected,
    /// Thrown away, because a builtin (which ignores stdin) comes next.
    /// The pipe is closed right away, so a writer gets SIGPIPE like in bash
    /// (`yes | echo hi` terminates).
    Discard,
}

pub fn start_pipeline(
    pipeline: &[UserCommand],
    shall_state: &mut ShellState,
) -> Result<RunningPipeline, String> {
    let in_pipe = pipeline.len() > 1;
    let sections: Vec<_> = pipeline
        .chunk_by(|a, b| a.is_builtin() == b.is_builtin())
        .collect();

    let mut running_pipeline = RunningPipeline::new();

    let mut prev: Option<String> = None;
    for (i, section) in sections.iter().enumerate() {
        let is_last = i == sections.len() - 1;
        let output = if section[0].is_builtin() {
            // Builtins ignore stdin, so only the last one of a section matters.
            let cmd = section.last().expect("chunks are never empty");
            let out = run_builtin(cmd, in_pipe, shall_state);
            if is_last {
                match output::get_file_from_direction(&cmd.out_direct) {
                    None => print!("{}", out),
                    Some(mut file) => file
                        .write_all(out.as_bytes())
                        .expect("Failed to write to file"),
                };
                None
            } else {
                Some(out)
            }
        } else {
            // A non-last external section is always followed by a builtin.
            let stdout = if !is_last {
                ChainStdout::Discard
            } else {
                ChainStdout::AsRedirected
            };
            let input = prev.as_deref();
            match start_chain(section, input, stdout) {
                Ok(mut pipeline) => {
                    running_pipeline.append(&mut pipeline);
                    None
                }
                Err(msg) => Some(msg),
            }
        };
        prev = output;
    }

    prev.map_or(Ok(running_pipeline), Err)
}

/// Runs a builtin in the shell process.
///
/// Inside a pipeline, builtins with side effects (`cd`, `exit`, ...) do nothing,
/// similar to bash running them in a subshell.
fn run_builtin(cmd: &UserCommand, in_pipe: bool, shell_state: &mut ShellState) -> String {
    let CommandKind::Builtin(builtin) = &cmd.cmd else {
        unreachable!("run_builtin called with external command");
    };
    // Builtins write nothing to stderr, but like in bash a `2>` target is
    // still created (or truncated).
    let _ = output::get_file_from_direction(&cmd.err_direct);
    if in_pipe && builtin.has_sideeffects() {
        String::new()
    } else {
        ensure_newline(builtin.execute(&cmd.args, shell_state))
    }
}

pub struct RunningPipeline {
    children: Vec<Child>,
    feeders: Vec<JoinHandle<()>>,
}

impl RunningPipeline {
    pub fn new() -> Self {
        RunningPipeline {
            children: Vec::new(),
            feeders: Vec::new(),
        }
    }
    fn append(&mut self, other: &mut Self) {
        self.children.append(&mut other.children);
        self.feeders.append(&mut other.feeders);
    }
    pub fn wait(self) {
        wait_on_last(self.children);
        for feeder in self.feeders {
            let _ = feeder.join();
        }
    }

    pub fn try_wait(&mut self) -> bool {
        let last_child_status = self
            .children
            .last_mut()
            .expect("Last is never empty")
            .try_wait();
        match last_child_status {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => panic!("error attempting to wait: {e}, error case not yet handled"),
        }
    }
    pub fn get_pid(&self) -> u32 {
        self.children.last().expect("Pipe is never empty").id()
    }
}

/// Runs consecutive external commands connected by pipes.
///
/// `input` is fed to the first process; without input it reads from the terminal.
/// Returns only Error Strings.
fn start_chain(
    cmds: &[UserCommand],
    input: Option<&str>,
    stdout: ChainStdout,
) -> Result<RunningPipeline, String> {
    let mut children = spawn_chain(cmds, input.is_some(), stdout)?;

    // Feed the input from a separate thread: the first process may block on
    // writing its own output before it has read all of its input.
    let feeder = input.map(|text| {
        let mut stdin = children[0]
            .stdin
            .take()
            .expect("stdin of first stage is piped");
        let text = text.to_owned();
        thread::spawn(move || {
            let _ = stdin.write_all(text.as_bytes());
        })
    });
    let feeders = if let Some(feeder) = feeder {
        vec![feeder]
    } else {
        Vec::new()
    };
    Ok(RunningPipeline { children, feeders })
}

/// Spawns all processes of a chain and connects stdout of each one to stdin
/// of the next. Only the last process may write to the terminal.
fn spawn_chain(
    cmds: &[UserCommand],
    pipe_stdin: bool,
    stdout: ChainStdout,
) -> Result<Vec<Child>, String> {
    let mut children: Vec<Child> = Vec::with_capacity(cmds.len());
    for (i, cmd) in cmds.iter().enumerate() {
        let is_last = i == cmds.len() - 1;
        let mut process = cmd.to_command();

        process.stdin(match children.last_mut() {
            Some(prev) => Stdio::from(prev.stdout.take().expect("inner stages are piped")),
            None if pipe_stdin => Stdio::piped(),
            None => Stdio::inherit(),
        });
        process.stdout(if is_last && stdout == ChainStdout::AsRedirected {
            match output::get_file_from_direction(&cmd.out_direct) {
                None => Stdio::inherit(),
                Some(file) => Stdio::from(file),
            }
        } else {
            Stdio::piped()
        });

        // stderr is never chained, so every stage may redirect it.
        if let Some(file) = output::get_file_from_direction(&cmd.err_direct) {
            process.stderr(Stdio::from(file));
        }

        match process.spawn() {
            Ok(child) => children.push(child),
            Err(e) => {
                reap(children);
                return Err(format!("{}: {e}", process.get_program().display()));
            }
        }
    }

    if stdout == ChainStdout::Discard
        && let Some(last) = children.last_mut()
    {
        drop(last.stdout.take());
    }
    Ok(children)
}

/// Waits for the last process and collects its captured stdout and stderr.
fn wait_on_last(mut children: Vec<Child>) {
    if !children.is_empty() {
        let mut last = children.pop().expect("chain is never empty");
        let _ = last.wait();
        reap(children);
    }
}

/// Stops earlier stages that are still running (e.g. `tail -f`) once the last
/// stage has finished.
fn reap(children: Vec<Child>) {
    for mut child in children {
        let _ = child.kill();
        let _ = child.wait();
    }
}
