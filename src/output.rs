use std::{
    fs::{self, File},
    io::{self, Write},
};

use crate::cmd_parser::{OutputDirection, UserCommand};

/// Output of a command (or pipeline) together with where it has to go.
#[derive(Debug, Default)]
pub struct OutputStrings {
    pub out: String,
    pub err: String,
    pub out_direction: OutputDirection,
    pub err_direction: OutputDirection,
}

impl OutputStrings {
    /// Output that goes where the redirections of `cmd` point to.
    pub fn new(out: String, err: String, cmd: &UserCommand) -> Self {
        Self {
            out,
            err,
            out_direction: cmd.out_direct.clone(),
            err_direction: cmd.err_direct.clone(),
        }
    }

    /// An error message of the shell itself, printed to the terminal.
    pub fn error(err: impl Into<String>) -> Self {
        Self {
            err: ensure_newline(err.into()),
            ..Default::default()
        }
    }

    /// Writes output and error independently in their direction.
    /// A redirection target file is created even if the text is empty.
    pub fn write(&self) {
        write_to(&self.out, &self.out_direction, &mut io::stdout());
        write_to(&self.err, &self.err_direction, &mut io::stderr());
    }
}

fn write_to(text: &str, direction: &OutputDirection, terminal: &mut impl Write) {
    match direction {
        OutputDirection::Terminal => {
            let _ = terminal.write_all(text.as_bytes());
        }
        OutputDirection::File(path) => fs::write(path, text).expect("Failed to write to file"),
        OutputDirection::Append(path) => {
            let mut f = File::options()
                .append(true)
                .create(true)
                .open(path)
                .expect("Failed to open file!");
            f.write_all(text.as_bytes())
                .expect("Failed to append to file");
        }
    }
}

/// Appends a line break to non-empty text that doesn't end with one.
pub fn ensure_newline(mut text: String) -> String {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
