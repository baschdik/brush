//! history builtin, arrow navigation and HISTFILE persistence.
mod common;
use common::Shell;

#[test]
fn type_history() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("type history"), "history is a shell builtin\n");
}

#[test]
fn lists_history() {
    let mut sh = Shell::spawn();
    sh.run("echo orange");
    sh.run("echo mango");
    assert_eq!(
        sh.run("history"),
        "    1 echo orange\n    2 echo mango\n    3 history\n"
    );
}

#[test]
fn limits_history() {
    let mut sh = Shell::spawn();
    sh.run("echo a");
    sh.run("echo b");
    sh.run("echo c");
    assert_eq!(sh.run("history 2"), "    3 echo c\n    4 history 2\n");
}

#[test]
fn up_and_down_arrows() {
    let mut sh = Shell::spawn();
    sh.run("echo first");
    sh.run("echo second");
    sh.press_up();
    sh.expect_input_line("echo second");
    sh.press_up();
    sh.expect_input_line("echo first");
    sh.press_down();
    sh.expect_input_line("echo second");
    assert_eq!(sh.enter(), "second\n");
}

#[test]
fn read_history_from_file() {
    let mut sh = Shell::spawn();
    sh.write_file("hist.txt", "echo from file 1\necho from file 2\n");
    assert_eq!(sh.run("history -r hist.txt"), "");
    assert_eq!(
        sh.run("history"),
        "    1 history -r hist.txt\n    2 echo from file 1\n    3 echo from file 2\n    4 history\n"
    );
}

#[test]
fn write_history_to_file() {
    let mut sh = Shell::spawn();
    sh.run("echo hello");
    sh.run("history -w hist.txt");
    assert_eq!(sh.read_file("hist.txt"), "echo hello\nhistory -w hist.txt\n");
}

#[test]
fn append_history_to_file() {
    let mut sh = Shell::spawn();
    sh.write_file("hist.txt", "old command\n");
    sh.run("echo one");
    sh.run("history -a hist.txt");
    sh.run("echo two");
    sh.run("history -a hist.txt");
    assert_eq!(
        sh.read_file("hist.txt"),
        "old command\necho one\nhistory -a hist.txt\necho two\nhistory -a hist.txt\n"
    );
}

#[test]
fn histfile_is_loaded_on_startup() {
    let builder = Shell::builder();
    let histfile = builder.dir().join("histfile");
    std::fs::write(&histfile, "echo saved 1\necho saved 2\n").unwrap();
    let mut sh = builder.env("HISTFILE", &histfile).spawn();
    assert_eq!(
        sh.run("history"),
        "    1 echo saved 1\n    2 echo saved 2\n    3 history\n"
    );
}

#[test]
fn histfile_is_written_on_exit() {
    let builder = Shell::builder();
    let histfile = builder.dir().join("histfile");
    let mut sh = builder.env("HISTFILE", &histfile).spawn();
    sh.run("echo persisted");
    sh.exit();
    assert_eq!(
        std::fs::read_to_string(&histfile).unwrap(),
        "echo persisted\nexit\n"
    );
}
