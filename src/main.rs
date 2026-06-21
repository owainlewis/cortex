mod app;
pub mod buffer;
mod cli;
mod terminal;
pub mod view;

use cli::{parse_args, ParseResult, USAGE};
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match parse_args(env::args().skip(1)) {
        Ok(ParseResult::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Run(args)) => {
            if let Err(error) = app::run(&args.path) {
                eprintln!("error: failed to run editor: {error}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
