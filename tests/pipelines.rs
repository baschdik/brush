//! Pipelines with externals and builtins.
mod common;
use common::{Shell, squash_ws};

#[test]
fn dual_command_pipeline() {
    let mut sh = Shell::spawn();
    sh.write_file("f.txt", "raspberry orange\nbanana grape\nblueberry pear\n");
    assert_eq!(squash_ws(&sh.run("cat f.txt | wc")), "3 6 45");
}

#[test]
fn multi_command_pipeline() {
    let mut sh = Shell::spawn();
    sh.write_file("f.txt", "pear\nbanana\npineapple\norange\nraspberry\n");
    assert_eq!(squash_ws(&sh.run("cat f.txt | head -n 3 | wc")), "3 3 22");
    assert_eq!(sh.run("cat f.txt | tail -n 4 | head -n 2 | grep apple"), "pineapple\n");
}

#[test]
fn pipeline_with_builtins() {
    let mut sh = Shell::spawn();
    assert_eq!(squash_ws(&sh.run("echo orange-banana | wc")), "1 1 14");
    assert_eq!(sh.run("ls | type exit"), "exit is a shell builtin\n");
    assert_eq!(sh.run("echo abc | cat | cat"), "abc\n");
}

#[test]
fn pipeline_does_not_wait_for_infinite_producer() {
    // `head` closes the pipe; the shell must not hang on `yes`.
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("yes | head -n 2"), "y\ny\n");
}

#[test]
fn stderr_of_pipeline_stage_reaches_terminal() {
    let mut sh = Shell::spawn();
    assert_eq!(
        sh.run("cat missing_file | wc -l"),
        "cat: missing_file: No such file or directory\n0\n"
    );
}

#[test]
fn pipeline_with_redirection() {
    let mut sh = Shell::spawn();
    sh.run("echo piped | cat > out.txt");
    assert_eq!(sh.read_file("out.txt"), "piped\n");
}

#[test]
fn streaming_pipeline_with_tail_follow() {
    // Data must flow through the pipe while `tail -f` keeps running, and the
    // shell must return once `head` has its 5 lines.
    let mut sh = Shell::spawn();
    sh.write_file(
        "f.txt",
        "1. banana apple\n2. orange mango\n3. strawberry grape\n",
    );
    sh.send_line("tail -f f.txt | head -n 5");
    sh.expect_raw("3. strawberry grape");
    sh.append_file("f.txt", "4. banana orange\n");
    sh.expect_raw("4. banana orange");
    sh.append_file("f.txt", "5. blueberry pear\n");
    sh.expect_raw("5. blueberry pear");
    sh.wait_for_prompt();
    // shell is usable again
    assert_eq!(sh.run("echo done"), "done\n");
}
