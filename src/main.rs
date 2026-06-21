mod cli;

use cli::{parse_args, ParseResult, USAGE};
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match parse_args(env::args().skip(1)) {
        Ok(ParseResult::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Run(args)) => {
            println!("cortex: editor UI is not implemented yet");
            println!("file: {}", args.path.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
