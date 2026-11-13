//! Single quotes, double quotes, backslashes, quoted executables.
mod common;
use common::Shell;

#[test]
fn single_quotes() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("echo 'hello    world'"), "hello    world\n");
    assert_eq!(sh.run("echo 'shell''test' 'a'b"), "shelltest ab\n");
    assert_eq!(sh.run(r#"echo 'keep "double" \n'"#), "keep \"double\" \\n\n");
}

#[test]
fn single_quoted_file_names() {
    let mut sh = Shell::spawn();
    sh.write_file("f 1", "one\n");
    sh.write_file("f's", "two\n");
    assert_eq!(sh.run(r#"cat 'f 1' "f's""#), "one\ntwo\n");
}

#[test]
fn double_quotes() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r#"echo "hello    world""#), "hello    world\n");
    assert_eq!(sh.run(r#"echo "it's" "a""b""#), "it's ab\n");
}

#[test]
fn backslash_outside_quotes() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r"echo world\ \ \ \ script"), "world    script\n");
    assert_eq!(sh.run(r#"echo \'\x\"\\"#), "'x\"\\\n");
}

#[test]
fn backslash_within_single_quotes() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r"echo 'a\\b\nc'"), "a\\\\b\\nc\n");
}

#[test]
fn backslash_within_double_quotes() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r#"echo "a\"b\\c\d""#), "a\"b\\c\\d\n");
}

#[test]
fn quoted_executable() {
    let mut sh = Shell::spawn();
    sh.add_executable("exe with space", "echo spaced");
    sh.add_executable("exe'with'quote", "echo quoted");
    assert_eq!(sh.run("'exe with space'"), "spaced\n");
    assert_eq!(sh.run(r#""exe'with'quote""#), "quoted\n");
}

// ---- exact cases from the Codecrafters stages ----

#[test]
fn backslash_outside_quotes_codecrafters_cases() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r"echo hello\ \ \ \ \ \ example"), "hello      example\n");
    assert_eq!(sh.run(r#"echo \'\"example shell\"\'"#), "'\"example shell\"'\n");
    assert_eq!(sh.run(r"echo shell\nworld"), "shellnworld\n");
}

#[test]
fn backslash_outside_quotes_in_file_names() {
    let mut sh = Shell::spawn();
    sh.write_file("_ignored_67", "orange grape.");
    sh.write_file("ignore_16", "banana raspberry.");
    sh.write_file(r"just_one_\_66", "mango banana.\n");
    assert_eq!(
        sh.run(r"cat \_ignored_67 ignore_\16 just_one_\\_66"),
        "orange grape.banana raspberry.mango banana.\n"
    );
}

#[test]
fn backslash_within_single_quotes_codecrafters_cases() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r"echo 'world\\nexample'"), "world\\\\nexample\n");
    assert_eq!(
        sh.run(r#"echo 'shell\"scripttest\"world'"#),
        "shell\\\"scripttest\\\"world\n"
    );
}

#[test]
fn backslash_within_single_quotes_in_file_names() {
    let mut sh = Shell::spawn();
    sh.write_file("no slash 2", "raspberry strawberry.");
    sh.write_file(r"one slash \1", "strawberry pear.");
    sh.write_file(r"two slashes \73\", "pear pineapple.\n");
    assert_eq!(
        sh.run(r"cat 'no slash 2' 'one slash \1' 'two slashes \73\'"),
        "raspberry strawberry.strawberry pear.pear pineapple.\n"
    );
}

#[test]
fn backslash_within_double_quotes_codecrafters_cases() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r#"echo "test'shell'\\'example""#), "test'shell'\\'example\n");
    assert_eq!(sh.run(r#"echo "test\"insidequotes"shell\""#), "test\"insidequotesshell\"\n");
    assert_eq!(sh.run(r#"echo "mixed\"quote'world'\\""#), "mixed\"quote'world'\\\n");
}

#[test]
fn backslash_within_double_quotes_in_file_names() {
    let mut sh = Shell::spawn();
    sh.write_file("number 93", "orange banana.");
    sh.write_file("doublequote \" 44", "pineapple orange.");
    sh.write_file(r"backslash \ 61", "blueberry orange.\n");
    assert_eq!(
        sh.run(r#"cat "number 93" "doublequote \" 44" "backslash \\ 61""#),
        "orange banana.pineapple orange.blueberry orange.\n"
    );
}

#[test]
fn double_quotes_codecrafters_cases() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run(r#"echo "shell  example"  "test""hello""#), "shell  example testhello\n");
    assert_eq!(sh.run(r#"echo "test"  "script's"  shell""example"#), "test script's shellexample\n");
    sh.write_file("f 62", "blueberry strawberry.");
    sh.write_file("f   66", "raspberry strawberry.");
    sh.write_file("f's27", "pineapple raspberry.\n");
    assert_eq!(
        sh.run(r#"cat "f 62" "f   66" "f's27""#),
        "blueberry strawberry.raspberry strawberry.pineapple raspberry.\n"
    );
}

#[test]
fn single_quotes_codecrafters_cases() {
    let mut sh = Shell::spawn();
    assert_eq!(
        sh.run("echo 'test     world' 'hello''shell' script''example"),
        "test     world helloshell scriptexample\n"
    );
    sh.write_file("f   95", "pear blueberry.");
    sh.write_file("f   25", "grape apple.");
    sh.write_file("f   4", "raspberry blueberry.\n");
    assert_eq!(
        sh.run("cat 'f   95' 'f   25' 'f   4'"),
        "pear blueberry.grape apple.raspberry blueberry.\n"
    );
}

#[test]
fn quoted_executables_with_special_characters() {
    let mut sh = Shell::spawn();
    for name in [
        "exe  with  space",
        "exe with \"quotes\"",
        "exe with 'single quotes'",
        r"exe with \ backslash",
    ] {
        sh.add_executable(name, r#"cat "$@""#);
    }
    sh.write_file("f1", "mango banana.\n");
    sh.write_file("f2", "banana pineapple.\n");
    sh.write_file("f3", "orange blueberry.\n");
    sh.write_file("f4", "blueberry apple.\n");
    assert_eq!(sh.run("'exe  with  space' f1"), "mango banana.\n");
    assert_eq!(sh.run(r#"'exe with "quotes"' f2"#), "banana pineapple.\n");
    assert_eq!(sh.run(r#""exe with 'single quotes'" f3"#), "orange blueberry.\n");
    assert_eq!(sh.run(r#""exe with \\ backslash" f4"#), "blueberry apple.\n");
}
