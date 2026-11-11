//! Runs one pipeline (`cmd1 | cmd2 | ...`) of builtin and external commands.
//!
//! A pipeline is split into sections of consecutive builtins or consecutive
//! external commands. Each section gets the output of the previous section as
//! input. Builtins never read their input and run inside the shell process,
//! while a section of external commands is spawned as one chain of processes
//! connected by OS pipes.

use std::{
    io::{Read, Write},
    process::{Child, Stdio},
    thread,
};

use crate::{
    cmd_parser::{CommandKind, OutputDirection, UserCommand},
    history::CmdHistory,
    output::{OutputStrings, ensure_newline},
};

/// Where the stdout of the last process in an external chain goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainStdout {
    /// Directly to the terminal. The process sees a tty and line-buffers.
    Terminal,
    /// Read by the shell and returned, e.g. for redirection into a file.
    Capture,
    /// Thrown away, because a builtin (which ignores stdin) comes next.
    /// The pipe is closed right away, so a writer gets SIGPIPE like in bash
    /// (`yes | echo hi` terminates).
    Discard,
}

pub fn run_pipeline(pipeline: &[UserCommand], cmd_history: &mut CmdHistory) -> OutputStrings {
    let in_pipe = pipeline.len() > 1;
    let sections: Vec<_> = pipeline
        .chunk_by(|a, b| a.is_builtin() == b.is_builtin())
        .collect();

    let mut prev: Option<OutputStrings> = None;
    for (i, section) in sections.iter().enumerate() {
        let is_last = i == sections.len() - 1;
        let output = if section[0].is_builtin() {
            // Builtins ignore stdin, so only the last one of a section matters.
            let cmd = section.last().expect("chunks are never empty");
            run_builtin(cmd, in_pipe, cmd_history)
        } else {
            // A non-last external section is always followed by a builtin.
            let stdout = if !is_last {
                ChainStdout::Discard
            } else if section.last().expect("chunks are never empty").out_direct
                == OutputDirection::Terminal
            {
                ChainStdout::Terminal
            } else {
                ChainStdout::Capture
            };
            let input = prev.as_ref().map(|p| p.out.as_str());
            run_chain(section, input, stdout)
        };
        prev = Some(output);
    }
    prev.unwrap_or_default()
}

/// Runs a builtin in the shell process.
///
/// Inside a pipeline, builtins with side effects (`cd`, `exit`, ...) do nothing,
/// similar to bash running them in a subshell.
fn run_builtin(cmd: &UserCommand, in_pipe: bool, cmd_history: &mut CmdHistory) -> OutputStrings {
    let CommandKind::Builtin(builtin) = &cmd.cmd else {
        unreachable!("run_builtin called with external command");
    };
    let out = if in_pipe && builtin.has_sideeffects() {
        String::new()
    } else {
        ensure_newline(builtin.execute(&cmd.args, cmd_history))
    };
    OutputStrings::new(out, String::new(), cmd)
}

/// Runs consecutive external commands connected by pipes.
///
/// `input` is fed to the first process; without input it reads from the terminal.
fn run_chain(cmds: &[UserCommand], input: Option<&str>, stdout: ChainStdout) -> OutputStrings {
    let last_cmd = cmds.last().expect("chain is never empty");
    let mut children = match spawn_chain(cmds, input.is_some(), stdout) {
        Ok(children) => children,
        Err(msg) => return OutputStrings::error(msg),
    };
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

    let output = collect_output(children, last_cmd);
    if let Some(feeder) = feeder {
        let _ = feeder.join();
    }
    output
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
        process.stdout(if is_last && stdout == ChainStdout::Terminal {
            Stdio::inherit()
        } else {
            Stdio::piped()
        });
        if is_last && cmd.err_direct != OutputDirection::Terminal {
            process.stderr(Stdio::piped());
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
fn collect_output(mut children: Vec<Child>, last_cmd: &UserCommand) -> OutputStrings {
    let mut last = children.pop().expect("chain is never empty");

    // stderr in its own thread, otherwise reading stdout and stderr one after
    // the other can deadlock when the child fills the stderr pipe buffer.
    let err_reader = last
        .stderr
        .take()
        .map(|e| thread::spawn(move || read_all(e)));
    // None if stdout goes to the terminal (or was discarded).
    let out = last.stdout.take().map(read_all).unwrap_or_default();
    let _ = last.wait();

    reap(children);

    let err = err_reader
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    OutputStrings::new(
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
        last_cmd,
    )
}

/// Stops earlier stages that are still running (e.g. `tail -f`) once the last
/// stage has finished.
fn reap(children: Vec<Child>) {
    for mut child in children {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn read_all(mut reader: impl Read) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf);
    buf
}
