//! Prompt, REPL, invalid commands, exit, echo, type, running programs.
mod common;
use common::Shell;

#[test]
fn prints_prompt() {
    // Shell::spawn already waits for the initial "$ ".
    let _sh = Shell::spawn();
}

#[test]
fn invalid_command() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("invalid_banana"), "invalid_banana: command not found\n");
}

#[test]
fn repl_handles_multiple_commands() {
    let mut sh = Shell::spawn();
    for cmd in ["invalid_a", "invalid_b", "invalid_c"] {
        assert_eq!(sh.run(cmd), format!("{cmd}: command not found\n"));
    }
}

#[test]
fn exit_terminates_shell() {
    let mut sh = Shell::spawn();
    sh.run("invalid_pear_command");
    // exit() also asserts exit status 0
    assert_eq!(sh.exit(), "", "no output expected after exit");
}

#[test]
fn echo() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("echo hello world"), "hello world\n");
    assert_eq!(sh.run("echo   many    spaces  "), "many spaces\n");
}

#[test]
fn type_builtins() {
    let mut sh = Shell::spawn();
    for builtin in ["echo", "exit", "type", "pwd", "cd", "history", "jobs"] {
        assert_eq!(
            sh.run(&format!("type {builtin}")),
            format!("{builtin} is a shell builtin\n")
        );
    }
}

#[test]
fn type_executable_and_missing() {
    let mut sh = Shell::spawn();
    let exe = sh.add_executable("my_exe", "echo hi");
    assert_eq!(sh.run("type my_exe"), format!("my_exe is {}\n", exe.display()));
    assert_eq!(sh.run("type nonexistent_xyz"), "nonexistent_xyz: not found\n");
}

#[test]
fn type_skips_non_executable_files() {
    let mut sh = Shell::spawn();
    sh.write_file("bin/not_exec", "plain file");
    assert_eq!(sh.run("type not_exec"), "not_exec: not found\n");
}

#[test]
fn runs_program_with_arguments() {
    let mut sh = Shell::spawn();
    sh.add_executable(
        "custom_exe",
        r#"echo "Program was passed $# args (including program name)."
echo "Arg #0 (program name): $(basename "$0")"
i=1; for a in "$@"; do echo "Arg #$i: $a"; i=$((i+1)); done"#,
    );
    assert_eq!(
        sh.run("custom_exe alice bob"),
        "Program was passed 2 args (including program name).\n\
         Arg #0 (program name): custom_exe\n\
         Arg #1: alice\n\
         Arg #2: bob\n"
    );
}

#[test]
fn type_system_executables() {
    let mut sh = Shell::spawn();
    for exe in ["cat", "cp", "mkdir"] {
        let out = sh.run(&format!("type {exe}"));
        let path = out
            .strip_prefix(&format!("{exe} is "))
            .and_then(|p| p.strip_suffix('\n'))
            .unwrap_or_else(|| panic!("unexpected output: {out:?}"));
        assert!(path.ends_with(&format!("/{exe}")), "{out:?}");
        assert!(std::path::Path::new(path).is_file(), "{out:?}");
    }
}

#[test]
fn type_uses_first_executable_in_path() {
    // PATH: rat:bee:fox – only bee/my_exe is executable.
    let mut sh = Shell::builder().path_dirs(&["rat", "bee", "fox"]).spawn();
    sh.write_file("rat/my_exe", "#!/bin/sh\necho rat\n");
    sh.write_file("fox/my_exe", "#!/bin/sh\necho fox\n");
    let exe = sh.add_executable_at("bee/my_exe", "echo bee");
    assert_eq!(sh.run("type my_exe"), format!("my_exe is {}\n", exe.display()));
    assert_eq!(sh.run("my_exe"), "bee\n");
}

#[test]
fn type_prefers_earlier_path_entry() {
    let mut sh = Shell::builder().path_dirs(&["first", "second"]).spawn();
    let first = sh.add_executable_at("first/dup_exe", "echo first");
    sh.add_executable_at("second/dup_exe", "echo second");
    assert_eq!(sh.run("type dup_exe"), format!("dup_exe is {}\n", first.display()));
    assert_eq!(sh.run("dup_exe"), "first\n");
}
