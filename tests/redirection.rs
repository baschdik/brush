//! >, 1>, 2>, >>, 1>>, 2>>
mod common;
use common::Shell;

#[test]
fn redirect_stdout() {
    let mut sh = Shell::spawn();
    sh.write_file("in.txt", "content\n");
    assert_eq!(sh.run("cat in.txt > out1.txt"), "");
    assert_eq!(sh.read_file("out1.txt"), "content\n");
    assert_eq!(sh.run("echo hello 1> out2.txt"), "");
    assert_eq!(sh.read_file("out2.txt"), "hello\n");
}

#[test]
fn redirect_stdout_keeps_stderr_on_terminal() {
    let mut sh = Shell::spawn();
    assert_eq!(
        sh.run("cat missing_file > out.txt"),
        "cat: missing_file: No such file or directory\n"
    );
    assert_eq!(sh.read_file("out.txt"), "");
}

#[test]
fn redirect_stderr() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("cat missing_file 2> err.txt"), "");
    assert_eq!(sh.read_file("err.txt"), "cat: missing_file: No such file or directory\n");
    // stdout of builtins still goes to the terminal
    assert_eq!(sh.run("echo visible 2> err2.txt"), "visible\n");
    assert_eq!(sh.read_file("err2.txt"), "");
}

#[test]
fn append_stdout() {
    let mut sh = Shell::spawn();
    sh.run("echo one >> out.txt");
    sh.run("echo two 1>> out.txt");
    sh.run("echo three >> out.txt");
    assert_eq!(sh.read_file("out.txt"), "one\ntwo\nthree\n");
}

#[test]
fn append_stderr() {
    let mut sh = Shell::spawn();
    sh.run("cat missing_a 2>> err.txt");
    sh.run("cat missing_b 2>> err.txt");
    assert_eq!(
        sh.read_file("err.txt"),
        "cat: missing_a: No such file or directory\ncat: missing_b: No such file or directory\n"
    );
}

#[test]
fn redirect_overwrites_existing_file() {
    let mut sh = Shell::spawn();
    sh.write_file("out.txt", "old content\n");
    sh.run("echo new > out.txt");
    assert_eq!(sh.read_file("out.txt"), "new\n");
}

#[test]
fn redirect_into_subdirectory() {
    let mut sh = Shell::spawn();
    std::fs::create_dir(sh.path("sub")).unwrap();
    sh.run("echo deep > sub/out.txt");
    assert_eq!(sh.read_file("sub/out.txt"), "deep\n");
}

#[test]
fn redirect_stdout_with_partial_failure() {
    // stdout of the successful part goes to the file, the error to the terminal
    let mut sh = Shell::spawn();
    sh.write_file("orange", "orange\n");
    assert_eq!(
        sh.run("cat orange nonexistent 1> out.txt"),
        "cat: nonexistent: No such file or directory\n"
    );
    assert_eq!(sh.read_file("out.txt"), "orange\n");
}

#[test]
fn redirect_stderr_with_partial_success() {
    // stdout stays on the terminal, only the error goes to the file
    let mut sh = Shell::spawn();
    sh.write_file("strawberry", "strawberry\n");
    assert_eq!(sh.run("cat strawberry nonexistent 2> err.txt"), "strawberry\n");
    assert_eq!(sh.read_file("err.txt"), "cat: nonexistent: No such file or directory\n");
}

#[test]
fn append_stdout_keeps_stderr_on_terminal() {
    let mut sh = Shell::spawn();
    assert_eq!(
        sh.run("cat nonexistent >> out.txt"),
        "cat: nonexistent: No such file or directory\n"
    );
    assert_eq!(sh.read_file("out.txt"), "");
}

#[test]
fn append_stderr_of_builtin_creates_empty_file() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("echo Maria says Error 2>> err.txt"), "Maria says Error\n");
    assert_eq!(sh.read_file("err.txt"), "");
    // later errors are still appended
    sh.run("cat nonexistent 2>> err.txt");
    assert_eq!(sh.read_file("err.txt"), "cat: nonexistent: No such file or directory\n");
}

#[test]
fn append_external_output_after_builtin_overwrite() {
    let mut sh = Shell::spawn();
    for f in ["orange", "raspberry", "strawberry"] {
        sh.write_file(&format!("bee/{f}"), f);
    }
    sh.run("echo List of files: > out.txt");
    sh.run("ls -1 bee >> out.txt");
    assert_eq!(
        sh.read_file("out.txt"),
        "List of files:\norange\nraspberry\nstrawberry\n"
    );
}
