//! BRUSH - Baschdik's Rust-based Shell
//!
//! A simple Unix shell implementation

mod builtin_cmd;
mod cmd_parser;
mod external_cmd;
mod history;
mod output;
mod pipeline;
mod tokenizer;
mod userinput;

use crate::{history::HistHandling, output::OutputStrings, pipeline::run_pipeline};
use userinput::get_userinput;

fn main() {
    let mut cmd_history = history::CmdHistory::init();
    loop {
        let user_input = get_userinput(&mut cmd_history);
        if user_input.is_empty() {
            continue;
        }

        let user_input_tokens = tokenizer::tokenize(&user_input);

        match cmd_parser::parse_cmd_set(&user_input_tokens) {
            Err(e) => OutputStrings::error(e.to_string()).write(),
            Ok(cmds) => {
                for pipeline in cmds {
                    run_pipeline(&pipeline, &mut cmd_history).write();
                }
            }
        }
    }
}
