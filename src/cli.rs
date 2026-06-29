use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub path: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseResult {
    Run(Args),
    Help,
    Version,
    CheckUpdate,
}

pub const USAGE: &str = "\
Cortex

Usage:
  cortex [path]
  cortex --version
  cortex --check-update

Opens a file or directory in the Cortex terminal editor.
Defaults to the current directory when no path is given.
";

pub fn parse_args<I, S>(args: I) -> Result<ParseResult, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let values: Vec<String> = args.into_iter().map(Into::into).collect();

    if values.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(ParseResult::Help);
    }

    match values.as_slice() {
        [flag] if flag == "--version" || flag == "-V" => Ok(ParseResult::Version),
        [flag] if flag == "--check-update" => Ok(ParseResult::CheckUpdate),
        [path] => Ok(ParseResult::Run(Args {
            path: PathBuf::from(path),
        })),
        [] => Ok(ParseResult::Run(Args {
            path: PathBuf::from("."),
        })),
        _ => Err(format!("expected exactly one file path\n\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Args, ParseResult};
    use std::path::PathBuf;

    #[test]
    fn parses_one_file_path() {
        assert_eq!(
            parse_args(["notes.txt"]),
            Ok(ParseResult::Run(Args {
                path: PathBuf::from("notes.txt"),
            }))
        );
    }

    #[test]
    fn accepts_help_flag() {
        assert_eq!(parse_args(["--help"]), Ok(ParseResult::Help));
        assert_eq!(parse_args(["-h"]), Ok(ParseResult::Help));
    }

    #[test]
    fn accepts_version_flag() {
        assert_eq!(parse_args(["--version"]), Ok(ParseResult::Version));
        assert_eq!(parse_args(["-V"]), Ok(ParseResult::Version));
    }

    #[test]
    fn accepts_check_update_flag() {
        assert_eq!(parse_args(["--check-update"]), Ok(ParseResult::CheckUpdate));
    }

    #[test]
    fn defaults_to_current_directory() {
        assert_eq!(
            parse_args(std::iter::empty::<&str>()),
            Ok(ParseResult::Run(Args {
                path: PathBuf::from("."),
            }))
        );
    }

    #[test]
    fn rejects_multiple_file_paths() {
        let error = parse_args(["a.txt", "b.txt"]).unwrap_err();
        assert!(error.contains("expected exactly one file path"));
    }
}
