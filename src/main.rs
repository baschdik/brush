//! BRUSH - Baschdik's Rust-based Shell
//!
//! A simple Unix shell implementation

mod builtin_cmd;
mod cmd_parser;
mod external_cmd;
mod history;
mod jobs;
mod output;
mod pipeline;
mod tokenizer;
mod userinput;

use crate::{
    history::HistHandling, output::OutputStrings, pipeline::start_pipeline, tokenizer::RunMode,
};
use userinput::get_userinput;

fn main() {
    let mut cmd_history = history::CmdHistory::init();
    let mut bg_jobs_list = jobs::JobsList::new();
    loop {
        let user_input = get_userinput(&mut cmd_history);
        if user_input.is_empty() {
            continue;
        }

        match tokenizer::tokenize(&user_input) {
            Err(e) => eprintln!("{}", e), //OutputStrings::error(e.to_string()).write(),
            Ok(cmds) => {
                for pipeline in cmds {
                    match pipeline.mode {
                        RunMode::Foreground => {
                            let starting_result = start_pipeline(&pipeline.cmds, &mut cmd_history);
                            match starting_result {
                                Ok(pipeline) => pipeline.wait(),
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                        RunMode::Background => {
                            let starting_result = start_pipeline(&pipeline.cmds, &mut cmd_history);
                            match starting_result {
                                Ok(pipeline) => bg_jobs_list.add(pipeline), //TODO here: bg_jobs_list.add () ...
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                    }
                }
            }
        }
    }
}
