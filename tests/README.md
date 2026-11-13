# Integration tests

End-to-end tests that drive the real shell binary the same way the
[Codecrafters](https://codecrafters.io) tester does: type input, read what the
shell prints, compare.

## Running

```bash
cargo test                       # everything (builds the shell first)
cargo test --test jobs           # one file
cargo test cd_relative           # tests whose name contains the string
cargo test -- --nocapture        # show println! output
```

## Strategy

- **Black-box only.** Tests talk to the compiled binary
  (`CARGO_BIN_EXE_codecrafters-shell`); no shell internals are used.
- **One file per feature area**, mirroring the Codecrafters stages:
  `basics`, `navigation`, `quoting`, `redirection`, `pipelines`,
  `completion`, `history`, `jobs`.
- **Codecrafters cases first.** Each stage from the Codecrafters test log is
  covered, partly with the exact same inputs, plus a few extra edge cases
  (e.g. `yes | head`, history `-r/-w/-a`, `HISTFILE`).
- **Isolated and parallel.** Every test gets its own temp directory, so tests
  run in parallel without interfering.

## Techniques

### Pseudo terminal (PTY)
The shell switches the terminal into raw mode, which panics on plain pipes.
Tests therefore run it inside a PTY via the
[`rexpect`](https://crates.io/crates/rexpect) crate and wait for expected
output with a timeout (5 s), so a hanging shell fails instead of blocking.

### Harness (`common/mod.rs`)
- `Shell::spawn()` / `Shell::builder()` start the shell with:
  - a temp dir as working directory and `$HOME`,
  - a `bin/` dir (and optional extra `path_dirs`) prepended to `$PATH`,
  - `HISTFILE` unset unless set via `.env(...)`.
- `run(cmd)` types a line and returns the output up to the next `$ `.
  The echoed input line is skipped; ANSI escapes, bells and `\r` are removed.
- Interactive input: `type_text("ech\t")`, `press_up()`, `enter()`,
  `expect_input_line("echo ")`, `expect_bell()`, `expect_raw(...)`.
- Long-running commands: `send_line(...)` + `expect_raw(...)` +
  `wait_for_prompt()`.
- `exit()` asserts exit status 0 and returns any output after `exit`.
- Fixtures: `add_executable(name, script)`, `write_file`, `read_file`,
  `append_file`, `mkfifo`, `write_fifo`.

### Deterministic background jobs
Like the Codecrafters tester, jobs that must finish at a defined moment are
`cat <fifo> &`: they block until the test writes to the FIFO. After that,
`wait_until_exited(pid)` polls `/proc/<pid>/stat` until the process is gone or
a zombie, so the next command reliably sees the job as finished — no sleeps.
`process_name(pid)` checks that the printed PID really belongs to the job.

### Loose whitespace where layout is not the point
`squash_ws` collapses column padding for `jobs` and `wc` output, so tests check
content, not the exact alignment (one test explicitly checks the
`[1]   Running` padding).

## Requirements
Linux (uses PTYs and `/proc`), plus `mkfifo`, `tail`, `cat`, `wc`, etc. from
coreutils.
