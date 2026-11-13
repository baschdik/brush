//! TAB completion.
mod common;
use common::Shell;

#[test]
fn completes_builtins() {
    let mut sh = Shell::spawn();
    sh.type_text("ech\t");
    sh.expect_input_line("echo ");
    sh.type_text("hello");
    assert_eq!(sh.enter(), "hello\n");

    sh.type_text("exi\t");
    sh.expect_input_line("exit ");
}

#[test]
fn completion_with_arguments() {
    let mut sh = Shell::spawn();
    sh.type_text("ech\t");
    sh.expect_input_line("echo ");
    sh.type_text("foo bar");
    assert_eq!(sh.enter(), "foo bar\n");
}

#[test]
fn missing_completion_rings_bell() {
    let mut sh = Shell::spawn();
    sh.type_text("xyz_no_such\t");
    sh.expect_bell();
    assert_eq!(sh.enter(), "xyz_no_such: command not found\n");
}

#[test]
fn completes_executable() {
    let mut sh = Shell::spawn();
    sh.add_executable("custom_exe_3889", "echo custom ran");
    sh.type_text("custom_e\t");
    sh.expect_input_line("custom_exe_3889 ");
    assert_eq!(sh.enter(), "custom ran\n");
}

#[test]
fn multiple_completions_bell_then_list() {
    let mut sh = Shell::spawn();
    for exe in ["xyz_rat", "xyz_pig", "xyz_owl"] {
        sh.add_executable(exe, "true");
    }
    sh.type_text("xyz_\t");
    sh.expect_bell();
    sh.type_text("\t");
    sh.expect_raw("xyz_owl  xyz_pig  xyz_rat");
    sh.expect_input_line("xyz_");
}

#[test]
fn partial_completions() {
    let mut sh = Shell::spawn();
    for exe in ["xyz_foo", "xyz_foo_bar", "xyz_foo_bar_baz"] {
        sh.add_executable(exe, "true");
    }
    sh.type_text("xyz_\t");
    sh.expect_input_line("xyz_foo");
    sh.type_text("_\t");
    sh.expect_input_line("xyz_foo_bar");
    sh.type_text("_\t");
    sh.expect_input_line("xyz_foo_bar_baz ");
}
