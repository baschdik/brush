//! Background jobs: `&`, `jobs`, reaping.
//!
//! Like the Codecrafters tester, jobs that should finish at a defined moment
//! are `cat <fifo> &`: they block until the test writes to / closes the FIFO.
//! Whitespace inside job lines is compared loosely via `squash_ws`.
mod common;
use common::{Shell, parse_job_start, process_name, squash_ws, wait_until_exited};

/// Starts `cmd &`, asserts "[n] <pid>" and that the pid really runs `exe`.
fn start_job(sh: &mut Shell, cmd: &str, job_number: usize, exe: &str) -> u32 {
    let pid = parse_job_start(&sh.run(&format!("{cmd} &")), job_number);
    assert_eq!(process_name(pid), exe, "pid {pid} is not running {exe}");
    pid
}

/// Lets a `cat <fifo>` job finish and waits until its process has exited.
fn finish_fifo_job(sh: &Shell, fifo: &str, pid: u32) {
    sh.write_fifo(fifo, "");
    wait_until_exited(pid);
}

#[test]
fn type_jobs() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("type jobs"), "jobs is a shell builtin\n");
}

#[test]
fn no_jobs_prints_nothing() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("jobs"), "");
}

#[test]
fn starts_background_job() {
    let mut sh = Shell::spawn();
    start_job(&mut sh, "sleep 10", 1, "sleep");
    start_job(&mut sh, "sleep 10", 2, "sleep");
}

#[test]
fn lists_single_job() {
    let mut sh = Shell::spawn();
    start_job(&mut sh, "sleep 10", 1, "sleep");
    assert_eq!(squash_ws(&sh.run("jobs")), "[1]+ Running sleep 10 &");
}

#[test]
fn lists_multiple_jobs_with_markers() {
    let mut sh = Shell::spawn();
    start_job(&mut sh, "sleep 10", 1, "sleep");
    start_job(&mut sh, "sleep 11", 2, "sleep");
    assert_eq!(
        squash_ws(&sh.run("jobs")),
        "[1]- Running sleep 10 &\n[2]+ Running sleep 11 &"
    );
    start_job(&mut sh, "sleep 12", 3, "sleep");
    // job without marker is padded like bash: "[1]   Running"
    let out = sh.run("jobs");
    assert!(out.starts_with("[1]   Running"), "{out:?}");
    assert_eq!(
        squash_ws(&out),
        "[1] Running sleep 10 &\n[2]- Running sleep 11 &\n[3]+ Running sleep 12 &"
    );
}

#[test]
fn background_job_output_is_printed() {
    let mut sh = Shell::spawn();
    sh.mkfifo("fifo");
    start_job(&mut sh, "cat fifo", 1, "cat");
    sh.write_fifo("fifo", "Hello from background\n");
    sh.expect_raw("Hello from background");
}

#[test]
fn background_output_while_foreground_job_runs() {
    let mut sh = Shell::spawn();
    sh.mkfifo("bg_fifo");
    sh.mkfifo("fg_fifo");
    let bg_pid = start_job(&mut sh, "cat bg_fifo", 1, "cat");

    // foreground job blocks until fg_fifo is written
    sh.send_line("cat fg_fifo");
    sh.expect_raw("cat fg_fifo\r\n");

    sh.write_fifo("bg_fifo", "Hello from FIFO #1\n");
    sh.expect_raw("Hello from FIFO #1\r\n");
    wait_until_exited(bg_pid);

    sh.write_fifo("fg_fifo", "Hello from FIFO #2\n");
    // foreground output, then the reaped background job, then the prompt
    assert_eq!(
        squash_ws(&sh.wait_for_prompt()),
        "Hello from FIFO #2\n[1]+ Done cat bg_fifo"
    );
}

#[test]
fn jobs_reaps_single_finished_job() {
    let mut sh = Shell::spawn();
    sh.mkfifo("fifo");
    let pid = start_job(&mut sh, "cat fifo", 1, "cat");
    assert_eq!(squash_ws(&sh.run("jobs")), "[1]+ Running cat fifo &");
    finish_fifo_job(&sh, "fifo", pid);
    assert_eq!(squash_ws(&sh.run("jobs")), "[1]+ Done cat fifo");
    assert_eq!(sh.run("jobs"), "");
}

#[test]
fn jobs_reaps_multiple_jobs() {
    let mut sh = Shell::spawn();
    sh.mkfifo("fifo_a");
    sh.mkfifo("fifo_b");
    start_job(&mut sh, "sleep 10", 1, "sleep");
    let pid_a = start_job(&mut sh, "cat fifo_a", 2, "cat");
    let pid_b = start_job(&mut sh, "cat fifo_b", 3, "cat");

    // the middle job finishes
    finish_fifo_job(&sh, "fifo_a", pid_a);
    assert_eq!(
        squash_ws(&sh.run("jobs")),
        "[1] Running sleep 10 &\n[2]- Done cat fifo_a\n[3]+ Running cat fifo_b &"
    );

    // markers are recalculated after job 2 was reaped
    finish_fifo_job(&sh, "fifo_b", pid_b);
    assert_eq!(
        squash_ws(&sh.run("jobs")),
        "[1]- Running sleep 10 &\n[3]+ Done cat fifo_b"
    );
    assert_eq!(squash_ws(&sh.run("jobs")), "[1]+ Running sleep 10 &");
}

#[test]
fn reaps_before_next_prompt() {
    let mut sh = Shell::spawn();
    sh.mkfifo("fifo");
    start_job(&mut sh, "sleep 10", 1, "sleep");
    let pid = start_job(&mut sh, "cat fifo", 2, "cat");
    assert_eq!(
        squash_ws(&sh.run("jobs")),
        "[1]- Running sleep 10 &\n[2]+ Running cat fifo &"
    );
    finish_fifo_job(&sh, "fifo", pid);
    assert_eq!(
        squash_ws(&sh.run("echo orange")),
        "orange\n[2]+ Done cat fifo"
    );
    assert_eq!(squash_ws(&sh.run("jobs")), "[1]+ Running sleep 10 &");
}

#[test]
fn recycles_job_number_one() {
    let mut sh = Shell::spawn();
    sh.mkfifo("fifo");
    let pid = start_job(&mut sh, "cat fifo", 1, "cat");
    finish_fifo_job(&sh, "fifo", pid);
    assert_eq!(squash_ws(&sh.run("echo pear")), "pear\n[1]+ Done cat fifo");
    assert_eq!(sh.run("echo pineapple"), "pineapple\n");
    assert_eq!(sh.run("jobs"), "");
    start_job(&mut sh, "sleep 10", 1, "sleep");
    assert_eq!(squash_ws(&sh.run("jobs")), "[1]+ Running sleep 10 &");
}

#[test]
fn recycles_job_number_after_last_job_finished() {
    let mut sh = Shell::spawn();
    sh.mkfifo("fifo");
    start_job(&mut sh, "sleep 10", 1, "sleep");
    let pid = start_job(&mut sh, "cat fifo", 2, "cat");
    finish_fifo_job(&sh, "fifo", pid);
    assert_eq!(squash_ws(&sh.run("echo grape")), "grape\n[2]+ Done cat fifo");
    // job 2 is free again
    start_job(&mut sh, "sleep 11", 2, "sleep");
    assert_eq!(
        squash_ws(&sh.run("jobs")),
        "[1]- Running sleep 10 &\n[2]+ Running sleep 11 &"
    );
}
