mod app;
pub mod buffer;
mod cli;
mod commands;
pub mod highlighter;
mod input;
mod keymap;
mod picker;
pub mod renderer;
mod terminal;
mod update;
pub mod view;

use cli::{parse_args, ParseResult, USAGE};
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match parse_args(env::args().skip(1)) {
        Ok(ParseResult::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Version) => {
            println!("cortex {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(ParseResult::CheckUpdate) => match update::check_latest(env!("CARGO_PKG_VERSION")) {
            Ok(status) if status.has_update() => {
                println!(
                    "Update available: {} -> {}",
                    env!("CARGO_PKG_VERSION"),
                    status.latest_tag().unwrap_or("unknown")
                );
                println!("Run install.sh again to install the latest release.");
                ExitCode::SUCCESS
            }
            Ok(status) => {
                if let Some(latest_tag) = status.latest_tag() {
                    println!(
                        "cortex {} is up to date (latest {})",
                        env!("CARGO_PKG_VERSION"),
                        latest_tag
                    );
                } else {
                    println!(
                        "No GitHub releases found; cortex {} is the current local version",
                        env!("CARGO_PKG_VERSION")
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: failed to check for updates: {error}");
                ExitCode::FAILURE
            }
        },
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
