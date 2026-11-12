use std::mem;

use crate::cmd_parser::{CommandErrors, UserCommand};

pub type CmdPipelines = Vec<Pipeline>;

#[derive(Default)]
pub enum RunMode {
    #[default]
    Foreground,
    Background,
}

#[derive(Default)]
pub struct Pipeline {
    pub cmds: Vec<UserCommand>,
    pub mode: RunMode,
}

impl Pipeline {
    fn push(&mut self, cmd: UserCommand) {
        self.cmds.push(cmd);
    }
    fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }
}

#[derive(Debug)]
enum Quotation {
    No,
    Single,
    Double,
}

/// Splits the user input into words and groups them into commands, which are
/// separated by `|` into pipelines, which are separated by `;`.
/// Quoted or escaped `|` and `;` are ordinary characters.
pub fn tokenize(input: &str) -> Result<CmdPipelines, CommandErrors> {
    let mut pipelines = CmdPipelines::new();
    let mut pipeline = Pipeline::default();
    // Words of the current command; the last one is the word being read.
    let mut processed = vec!["".to_string()];
    let mut qoutation_status = Quotation::No;

    let mut input_iter = input.chars().peekable();
    while let Some(c) = input_iter.next() {
        match qoutation_status {
            Quotation::No => {
                if c == '\\' {
                    //escaping
                    processed
                        .last_mut()
                        .unwrap()
                        .push(input_iter.next().unwrap_or_default());
                } else if c.is_whitespace() {
                    processed.push("".to_string());
                } else if c == '|' {
                    pipeline.push(parse_command(&mut processed)?);
                } else if c == ';' {
                    pipeline.push(parse_command(&mut processed)?);
                    pipelines.push(mem::take(&mut pipeline));
                } else if c == '&' {
                    pipeline.push(parse_command(&mut processed)?);
                    pipeline.mode = RunMode::Background;
                    pipelines.push(mem::take(&mut pipeline));
                } else if c == '\'' {
                    //Ignore Empty quotes '':
                    if input_iter.next_if(|c| *c == '\'').is_some() {
                        continue;
                    }
                    qoutation_status = Quotation::Single;
                } else if c == '"' {
                    //Ignore Empty quotes "":
                    if input_iter.next_if(|c| *c == '"').is_some() {
                        continue;
                    }
                    qoutation_status = Quotation::Double;
                } else {
                    processed.last_mut().unwrap().push(c);
                }
            }
            Quotation::Single => {
                if c == '\'' {
                    //Concatenate adjacent quoted strings:
                    if input_iter.next_if(|c| *c == '\'').is_some() {
                        continue;
                    }
                    qoutation_status = Quotation::No;
                } else {
                    processed.last_mut().unwrap().push(c);
                }
            }
            Quotation::Double => {
                if c == '\\' {
                    //Escape only " and \, otherwise the backslash is kept literally
                    processed.last_mut().unwrap().push(
                        input_iter
                            .next_if(|c| matches!(c, '"' | '\\'))
                            .unwrap_or('\\'),
                    );
                } else if c == '"' {
                    //Concatenate adjacent quoted strings:
                    if input_iter.next_if(|c| *c == '\'').is_some() {
                        continue;
                    }
                    qoutation_status = Quotation::No;
                } else {
                    processed.last_mut().unwrap().push(c);
                }
            }
        }
    }

    // Accept a trailing semicolon.
    let trailing_semicolon =
        !pipelines.is_empty() && pipeline.is_empty() && processed.iter().all(String::is_empty);
    if !trailing_semicolon {
        pipeline.push(parse_command(&mut processed)?);
        pipelines.push(pipeline);
    }
    Ok(pipelines)
}

/// Parses the words collected so far into a command and starts a new one.
fn parse_command(words: &mut Vec<String>) -> Result<UserCommand, CommandErrors> {
    let words: Vec<String> = mem::replace(words, vec!["".to_string()])
        .into_iter()
        .filter(|word| !word.is_empty())
        .collect();
    UserCommand::parse(&words)
}
