//! pwd and cd.
mod common;
use common::Shell;

fn canonical(sh: &Shell, rel: &str) -> String {
    sh.path(rel).canonicalize().unwrap().display().to_string()
}

#[test]
fn pwd_prints_working_directory() {
    let mut sh = Shell::spawn();
    assert_eq!(sh.run("pwd"), format!("{}\n", canonical(&sh, "")));
}

#[test]
fn cd_absolute_path() {
    let mut sh = Shell::spawn();
    std::fs::create_dir_all(sh.path("a/b")).unwrap();
    let target = canonical(&sh, "a/b");
    assert_eq!(sh.run(&format!("cd {target}")), "");
    assert_eq!(sh.run("pwd"), format!("{target}\n"));
}

#[test]
fn cd_nonexistent_directory() {
    let mut sh = Shell::spawn();
    assert_eq!(
        sh.run("cd /non-existing-directory"),
        "cd: /non-existing-directory: No such file or directory\n"
    );
}

#[test]
fn cd_relative_paths() {
    let mut sh = Shell::spawn();
    std::fs::create_dir_all(sh.path("a/b/c")).unwrap();
    sh.run("cd ./a/b");
    assert_eq!(sh.run("pwd"), format!("{}\n", canonical(&sh, "a/b")));
    sh.run("cd c");
    assert_eq!(sh.run("pwd"), format!("{}\n", canonical(&sh, "a/b/c")));
    sh.run("cd ../../");
    assert_eq!(sh.run("pwd"), format!("{}\n", canonical(&sh, "a")));
}

#[test]
fn cd_home() {
    let mut sh = Shell::spawn();
    std::fs::create_dir_all(sh.path("sub")).unwrap();
    sh.run("cd sub");
    sh.run("cd ~");
    assert_eq!(sh.run("pwd"), format!("{}\n", sh.dir().display()));
}
