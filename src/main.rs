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

pub struct ShellState {
    pub cmd_history: history::CmdHistory,
    pub bg_jobs: jobs::JobsList,
}

impl ShellState {
    fn init() -> Self {
        ShellState {
            cmd_history: history::CmdHistory::init(),
            bg_jobs: jobs::JobsList::new(),
        }
    }
}

fn main() {
    let mut shell_state = ShellState::init();

    loop {
        let reaped_bg_jobs = shell_state.bg_jobs.reap_finished();
        if let Some(jobs) = reaped_bg_jobs {
            print!("{}", jobs.show());
        };
        let user_input = get_userinput(&mut shell_state.cmd_history);
        if user_input.is_empty() {
            continue;
        }

        match tokenizer::tokenize(&user_input) {
            Err(e) => eprintln!("{}", e), //OutputStrings::error(e.to_string()).write(),
            Ok(cmds) => {
                for pipeline in cmds {
                    match pipeline.mode {
                        RunMode::Foreground => {
                            let starting_result = start_pipeline(&pipeline.cmds, &mut shell_state);
                            match starting_result {
                                Ok(pipeline) => pipeline.wait(),
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                        RunMode::Background => {
                            let starting_result = start_pipeline(&pipeline.cmds, &mut shell_state);
                            match starting_result {
                                Ok(pipeline) => shell_state.bg_jobs.add(
                                    pipeline,
                                    shell_state
                                        .cmd_history
                                        .get_latest(0)
                                        .expect("Cant be empty here"),
                                ),
                                Err(e) => eprintln!("{}", e),
                            }
                        }
                    }
                }
            }
        }
    }
}
