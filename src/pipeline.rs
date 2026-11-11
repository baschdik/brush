use std::{
    io::{Read, Write},
    process::Stdio,
    thread,
};

use crate::{
    cmd_parser::{CommandKind, OutputDirection, UserCommand},
    history::CmdHistory,
    output::OutputStrings,
};

pub fn run_pipeline(pipeline: Vec<UserCommand>, cmd_history: &mut CmdHistory) -> OutputStrings {
    if pipeline.len() == 1
        && let Some(cmd) = pipeline.first()
        && let CommandKind::Builtin(cmd_builtin) = &cmd.cmd
    {
        return OutputStrings::from_out(
            &cmd_builtin.execute(&cmd.args, cmd_history),
            &cmd.out_direct,
        );
    };

    let mut output = None;
    let drop_external_output = pipeline.last().is_some_and(|a| a.is_builtin());
    for pipe_section in pipeline.chunk_by(|a, b| a.is_builtin() == b.is_builtin()) {
        if pipe_section[0].is_builtin() {
            let last_cmd = pipe_section.iter().rfind(|cmd| !cmd.cmd.has_sideeffects());
            if let Some(cmd) = last_cmd
                && let CommandKind::Builtin(cmd_builtin) = &cmd.cmd
            {
                output = Some(OutputStrings::from_out(
                    &cmd_builtin.execute(&cmd.args, cmd_history),
                    &cmd.out_direct,
                ));
            }
        } else {
            output = Some(run_pipeline_for_external_cmd(
                pipe_section,
                output.map(|output| output.out),
                drop_external_output,
            ))
        }
    }
    output.unwrap_or_default()
}

fn run_pipeline_for_external_cmd(
    pipeline: &[UserCommand],
    prev_input: Option<String>,
    drop_output: bool,
) -> OutputStrings {
    if let Some(cmd) = pipeline.first()
        && let CommandKind::Builtin(_) = &cmd.cmd
    {
        panic!("Run 'run_pipeline_for_external_cmd' with build-in cmd  ")
    };

    let last_cmd_no = pipeline.len() - 1;
    let last_cmd = pipeline.last().expect("Pipeline is empty!");
    let out_redirected = !(last_cmd.out_direct == OutputDirection::Terminal);
    let err_redirected = !(last_cmd.err_direct == OutputDirection::Terminal);

    let mut prev_stdout = None;
    let mut children = Vec::new();

    for (i, cmd) in pipeline.iter().enumerate() {
        let is_last = i == last_cmd_no;
        let mut process = cmd.to_command();

        if i == 0
            && let Some(_) = &prev_input
        {
            process.stdin(Stdio::piped());
        } else if let Some(prev) = prev_stdout.take() {
            process.stdin(Stdio::from(prev));
        }

        if is_last && !out_redirected && !drop_output {
            // last command now sees a terminal and is therefore line-buffered
            process.stdout(Stdio::inherit());
        } else {
            process.stdout(Stdio::piped());
        }

        if is_last {
            process.stderr(if err_redirected {
                Stdio::piped()
            } else {
                Stdio::inherit()
            });
        }

        let mut child = process.spawn().expect("Spawning failed");
        if i == 0
            && let Some(prev) = &prev_input
        {
            let mut child_stdin = child.stdin.take().expect("Stdin was not piped");
            let _ = child_stdin.write_all(prev.as_bytes());
        }
        if !is_last {
            prev_stdout = child.stdout.take();
        }
        children.push(child);
    }

    let mut last_child = children.pop().expect("No Child in Children");

    // stderr in its own thread, otherwise reading stdout and stderr one after
    // the other can deadlock when the child fills the stderr pipe buffer.
    let err_thread = last_child.stderr.take().map(|mut e| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = e.read_to_end(&mut buf);
            buf
        })
    });

    // Only Some(..) if stdout was redirected (piped). With Stdio::inherit()
    // the child writes directly to the terminal and this is None.
    let mut captured: Vec<u8> = Vec::new();
    if let Some(mut child_out) = last_child.stdout.take() {
        let _ = child_out.read_to_end(&mut captured);
    }

    let _ = last_child.wait();

    // Clean up the earlier stages
    for mut child in children {
        let _ = child.kill();
        let _ = child.wait();
    }

    let err_bytes = err_thread
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();

    OutputStrings {
        out: String::from_utf8_lossy(&captured).into_owned(), // empty if inherited
        err: String::from_utf8_lossy(&err_bytes).into_owned(),
        out_direction: last_cmd.out_direct.to_owned(),
        err_direction: last_cmd.err_direct.to_owned(),
    }
}
