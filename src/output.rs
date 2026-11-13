use std::fs::File;

use crate::cmd_parser::OutputDirection;

pub fn get_file_from_direction(direction: &OutputDirection) -> Option<File> {
    match direction {
        OutputDirection::Terminal => None,
        OutputDirection::File(path) => Some(File::create(path).expect("Failed to open file!")),
        OutputDirection::Append(path) => Some(
            File::options()
                .append(true)
                .create(true)
                .open(path)
                .expect("Failed to open file!"),
        ),
    }
}

/// Appends a line break to non-empty text that doesn't end with one.
pub fn ensure_newline(mut text: String) -> String {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
