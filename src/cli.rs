use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub path: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseResult {
    Run(Args),
    Help,
}

pub const USAGE: &str = "\
Cortex

Usage:
  cortex <file>

Opens one file in the Cortex terminal editor.
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
        [path] => Ok(ParseResult::Run(Args {
            path: PathBuf::from(path),
        })),
        [] => Err(format!("missing file path\n\n{USAGE}")),
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
    fn rejects_missing_file_path() {
        let error = parse_args(std::iter::empty::<&str>()).unwrap_err();
        assert!(error.contains("missing file path"));
    }

    #[test]
    fn rejects_multiple_file_paths() {
        let error = parse_args(["a.txt", "b.txt"]).unwrap_err();
        assert!(error.contains("expected exactly one file path"));
    }
}
