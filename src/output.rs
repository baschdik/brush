use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use crate::cmd_parser::OutputDirection;

/// Holds out and err strings
#[derive(Debug)]
pub struct OutputStrings {
    pub out: String,
    pub err: String,
    pub out_direction: OutputDirection,
    pub err_direction: OutputDirection,
}

impl Default for OutputStrings {
    fn default() -> Self {
        OutputStrings {
            out: "".to_string(),
            err: "".to_string(),
            out_direction: OutputDirection::Terminal,
            err_direction: OutputDirection::Terminal,
        }
    }
}

impl OutputStrings {
    /// Writes Output and Error independently in the given direction
    pub fn write(&self) {
        match &self.out_direction {
            OutputDirection::Terminal => print!("{}", self.out),
            OutputDirection::File(path) => {
                fs::write(path, &self.out).expect("Failed to write to file")
            }
            OutputDirection::Append(path) => Self::append_file(&self.out, path),
        }
        match &self.err_direction {
            OutputDirection::Terminal => eprint!("{}", self.err),
            OutputDirection::File(path) => {
                fs::write(path, &self.err).expect("Failed to write to file")
            }
            OutputDirection::Append(path) => Self::append_file(&self.err, path),
        }
    }
    fn append_file(str: &str, path: &PathBuf) {
        let mut f = File::options()
            .append(true)
            .create(true)
            .open(path)
            .expect("Failed to open file!");
        write!(&mut f, "{}", str).expect("Failed to append to file");
    }
    /// Creates OutputStrings struct if only Output is given (no error)
    pub fn from_out(out: &str, out_direction: &OutputDirection) -> Self {
        let line_end = if !out.is_empty() && !out.ends_with("\n") {
            "\n"
        } else {
            ""
        };
        Self {
            out: out.to_string() + line_end,
            err: "".to_string(),
            out_direction: out_direction.to_owned(),
            err_direction: OutputDirection::Terminal,
        }
    }
    /// Creates OutputStrings struct if only error is given (no output)
    pub fn from_err(err: &str, err_direction: &OutputDirection) -> Self {
        let line_end = if !err.is_empty() && !err.ends_with("\n") {
            "\n"
        } else {
            ""
        };
        Self {
            out: "".to_string(),
            err: err.to_string() + line_end,
            out_direction: OutputDirection::Terminal,
            err_direction: err_direction.to_owned(),
        }
    }
}
