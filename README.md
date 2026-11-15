[![progress-banner](https://backend.codecrafters.io/progress/shell/861cb530-5d2a-475c-ab4b-d7c403ce6fd8)](https://app.codecrafters.io/users/baschdik?r=2qF)

# BRUSH – Baschdik's Rust-based Shell

A small Unix shell written in Rust.

## About this project

This is a personal learning project. It started as my solution to the
[**"Build Your Own Shell" challenge**](https://app.codecrafters.io/courses/shell/overview)
on [CodeCrafters](https://codecrafters.io). The challenge leads you stage by stage
from a simple prompt to a shell with quoting, redirection, pipelines, tab
completion, history and background jobs.

- **The shell itself (`src/`) is mostly hand-coded by me.** Writing it was the
  point of the exercise.
- **The integration test suite (`tests/`) was built with
  [Claude Code](https://claude.com/claude-code).** It is modeled on the
  CodeCrafters tester, so the shell can be checked locally, without
  `codecrafters submit`. See [`tests/README.md`](tests/README.md) for how it works.

If you want to build your own shell, try the challenge on
[codecrafters.io](https://codecrafters.io) instead of reading the code here first.

## Features

Every feature below is covered by the integration tests (file in brackets).

### Basics ([`basics.rs`](tests/basics.rs))
- Interactive REPL with a `$ ` prompt
- `<cmd>: command not found` for unknown commands
- Builtins: `echo`, `exit`, `type`, `pwd`, `cd`, `history`, `jobs`
- `type` reports builtins and resolves executables via `$PATH`. Non-executable
  files are skipped and the first match in `$PATH` wins.
- Runs external programs with arguments

### Navigation ([`navigation.rs`](tests/navigation.rs))
- `pwd`
- `cd` with absolute paths, relative paths (`./`, `../`) and `~`, plus an error for
  missing directories

### Quoting ([`quoting.rs`](tests/quoting.rs))
- Single quotes: everything is literal
- Double quotes: a backslash escapes only `"` and `\`
- Backslash escapes outside quotes
- Adjacent quoted parts are joined (`'a''b'` → `ab`)
- Quoted executable and file names with spaces, quotes or backslashes

### Redirection ([`redirection.rs`](tests/redirection.rs))
- `>` / `1>` and `2>`: redirect stdout or stderr to a file
- `>>` / `1>>` and `2>>`: append stdout or stderr to a file

### Pipelines ([`pipelines.rs`](tests/pipelines.rs))
- Pipelines with two or more stages
- Builtins and external programs can be mixed in a pipeline
- Data streams through the pipe while the commands run (e.g. `tail -f file | head -n 5`)
- Pipelines combined with redirection

### Tab completion ([`completion.rs`](tests/completion.rs))
- Completes builtins and executables from `$PATH`, followed by a space
- Rings the bell when nothing matches
- With several matches: completes the longest common prefix. Otherwise the first
  TAB rings the bell and the second lists all matches.

### History ([`history.rs`](tests/history.rs))
- `history` and `history <n>`
- Up/Down arrow keys move through earlier commands
- `history -r <file>`, `history -w <file>`, `history -a <file>`
- `$HISTFILE` is loaded on startup and written on `exit`

### Background jobs ([`jobs.rs`](tests/jobs.rs))
- Start jobs with `&`. The shell prints `[n] <pid>`.
- `jobs` lists jobs with `+` / `-` markers and `Running` / `Done` status
- Finished jobs are reaped and reported by `jobs` or before the next prompt
- Free job numbers are reused

## Building and running

Requires Rust (edition 2024, `cargo` 1.96 or newer) on Linux.

```sh
cargo run --release
```

The shell switches the terminal to raw mode, so it has to run in a real terminal.
Piping input into it does not work.

## Testing

```sh
cargo test
```

This builds the shell and runs the integration tests in a pseudo terminal. For
details on how the tests work and how to run single tests, see
[`tests/README.md`](tests/README.md).
