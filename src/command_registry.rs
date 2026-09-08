use crate::commands::Command;

#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    pub name: &'static str,
    pub description: &'static str,
    aliases: &'static [&'static str],
    target: Target,
    argument: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
enum Target {
    Command(Command),
    Character,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Invocation<'a> {
    pub command: Command,
    pub argument: &'a str,
}

const fn command(
    name: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    target: Command,
    argument: Option<&'static str>,
) -> CommandSpec {
    CommandSpec {
        name,
        description,
        aliases,
        target: Target::Command(target),
        argument,
    }
}

pub const COMMANDS: &[CommandSpec] = &[
    command(
        "save-buffer",
        "Save the active buffer",
        &["save"],
        Command::SaveBuffer,
        None,
    ),
    command(
        "find-file",
        "Prompt for a file or directory relative to the active file",
        &[],
        Command::OpenFile,
        None,
    ),
    command(
        "switch-buffer",
        "Prompt for an open buffer",
        &[],
        Command::SwitchBuffer,
        None,
    ),
    command("undo", "Undo the last edit", &[], Command::Undo, None),
    command(
        "redo",
        "Redo the last undone edit",
        &[],
        Command::Redo,
        None,
    ),
    command(
        "quit",
        "Quit, asking before discarding unsaved buffers",
        &[],
        Command::Quit,
        None,
    ),
    command(
        "force-quit",
        "Quit and discard all unsaved buffers",
        &["quit!"],
        Command::ForceQuit,
        None,
    ),
    command(
        "reload-buffer",
        "Reload the active file if the buffer is clean",
        &["reload"],
        Command::ReloadBuffer,
        None,
    ),
    command(
        "search-forward",
        "Search forward for literal text",
        &["search"],
        Command::Search,
        Some("text"),
    ),
    command(
        "repeat-search",
        "Find the next occurrence of the last search",
        &["next"],
        Command::RepeatSearch,
        None,
    ),
    command(
        "open-file",
        "Open a file path relative to the working directory",
        &["open"],
        Command::OpenPath,
        Some("path"),
    ),
    command(
        "execute-extended-command",
        "Prompt for a named command",
        &[],
        Command::OpenCommandLine,
        None,
    ),
    command(
        "help",
        "List commands or describe one command",
        &["commands"],
        Command::Help,
        Some("command"),
    ),
    command(
        "newline",
        "Insert a newline carrying the current indentation",
        &[],
        Command::InsertNewline,
        None,
    ),
    command(
        "indent",
        "Insert spaces to a tab stop or indent selected lines",
        &[],
        Command::Indent,
        None,
    ),
    command(
        "outdent",
        "Remove one indentation level from current or selected lines",
        &[],
        Command::Outdent,
        None,
    ),
    command(
        "delete-backward-char",
        "Delete the previous grapheme",
        &[],
        Command::DeleteBackward,
        None,
    ),
    command(
        "delete-char",
        "Delete the next grapheme",
        &[],
        Command::DeleteForward,
        None,
    ),
    command(
        "forward-char",
        "Move forward one grapheme",
        &[],
        Command::MoveForwardChar,
        None,
    ),
    command(
        "backward-char",
        "Move backward one grapheme",
        &[],
        Command::MoveBackwardChar,
        None,
    ),
    command(
        "next-line",
        "Move down one line",
        &[],
        Command::MoveNextLine,
        None,
    ),
    command(
        "previous-line",
        "Move up one line",
        &[],
        Command::MovePreviousLine,
        None,
    ),
    command(
        "beginning-of-line",
        "Move to the beginning of the line",
        &[],
        Command::MoveToLineStart,
        None,
    ),
    command(
        "end-of-line",
        "Move to the end of the line",
        &[],
        Command::MoveToLineEnd,
        None,
    ),
    command(
        "set-mark",
        "Set the start of a region at point",
        &[],
        Command::SetMark,
        None,
    ),
    command(
        "kill-region",
        "Cut the active region",
        &[],
        Command::KillRegion,
        None,
    ),
    command(
        "kill-line",
        "Cut to the end of the line or the next newline",
        &[],
        Command::KillLine,
        None,
    ),
    command("yank", "Insert the newest kill", &[], Command::Yank, None),
    command(
        "yank-pop",
        "Replace the last yank with an older kill",
        &[],
        Command::YankPop,
        None,
    ),
    CommandSpec {
        name: "self-insert-command",
        description: "Insert one printable character",
        aliases: &[],
        target: Target::Character,
        argument: Some("character"),
    },
];

fn command_text(input: &str) -> &str {
    let input = input.trim_start();
    input.strip_prefix('/').unwrap_or(input).trim_start()
}

pub fn lookup(name: &str) -> Option<&'static CommandSpec> {
    let name = command_text(name);
    COMMANDS
        .iter()
        .find(|spec| spec.name == name || spec.aliases.contains(&name))
}

pub fn parse(input: &str) -> Result<Invocation<'_>, String> {
    let input = input.trim_start();
    let command_text = command_text(input);
    if command_text.trim().is_empty() {
        return Ok(Invocation {
            command: Command::Help,
            argument: "",
        });
    }
    let (name, argument) = command_text
        .split_once(char::is_whitespace)
        .unwrap_or((command_text, ""));
    let spec = lookup(name).ok_or_else(|| format!("Unknown command: {}", input.trim()))?;
    let argument = if matches!(spec.target, Target::Character) && argument.chars().count() == 1 {
        argument
    } else {
        argument.trim()
    };
    if spec.argument.is_none() && !argument.is_empty() {
        return Err(format!("{} takes no arguments", spec.name));
    }
    let command = match spec.target {
        Target::Command(command) => command,
        Target::Character => {
            let mut chars = argument.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) if !ch.is_control() => Command::Insert(ch),
                _ => return Err("Usage: self-insert-command <character>".to_string()),
            }
        }
    };
    Ok(Invocation { command, argument })
}

pub fn matches(input: &str) -> impl Iterator<Item = &'static CommandSpec> + '_ {
    let prefix = command_text(input);
    COMMANDS.iter().filter(move |spec| {
        spec.name.starts_with(prefix) || spec.aliases.iter().any(|alias| alias.starts_with(prefix))
    })
}

pub fn complete(input: &str) -> String {
    let name = command_text(input);
    if name.chars().any(char::is_whitespace) {
        return input.to_string();
    }
    if let Some(spec) = lookup(name) {
        return spec.name.to_string();
    }
    let mut candidates = matches(name);
    let Some(first) = candidates.next() else {
        return input.to_string();
    };
    let mut prefix = first.name;
    for candidate in candidates {
        let len = prefix
            .bytes()
            .zip(candidate.name.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        prefix = &prefix[..len];
    }
    if prefix.is_empty() {
        input.to_string()
    } else {
        prefix.to_string()
    }
}

pub fn help(argument: &str) -> Result<String, String> {
    if argument.is_empty() {
        return Ok(format!(
            "M-x: {}; Tab completes; help <command> describes",
            COMMANDS
                .iter()
                .map(|spec| spec.name)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let spec = lookup(argument).ok_or_else(|| format!("Unknown command: {argument}"))?;
    let usage = spec
        .argument
        .map(|name| format!(" <{name}>"))
        .unwrap_or_default();
    Ok(format!("{}{usage}: {}", spec.name, spec.description))
}

#[cfg(test)]
mod tests {
    use super::{complete, lookup, parse, Target, COMMANDS};
    use crate::{
        commands::Command,
        input::Key,
        keymap::{Keymap, KeymapResult},
    };
    use std::collections::HashSet;

    #[test]
    fn command_names_and_aliases_are_unique_and_have_descriptions() {
        let mut names = HashSet::new();
        for spec in COMMANDS {
            assert!(!spec.description.is_empty());
            for name in std::iter::once(spec.name).chain(spec.aliases.iter().copied()) {
                assert!(
                    name.is_ascii() && !name.is_empty() && !name.chars().any(char::is_whitespace)
                );
                assert!(names.insert(name), "duplicate command or alias: {name}");
            }
        }
    }

    #[test]
    fn names_and_legacy_aliases_resolve_to_existing_actions() {
        for input in ["save-buffer", "save", "/save", "  /save  "] {
            assert_eq!(parse(input).unwrap().command, Command::SaveBuffer);
        }
        assert_eq!(parse("find-file").unwrap().command, Command::OpenFile);
        assert_eq!(parse("/quit!").unwrap().command, Command::ForceQuit);
        assert_eq!(
            parse("execute-extended-command").unwrap().command,
            Command::OpenCommandLine
        );
        assert_eq!(parse("/").unwrap().command, Command::Help);
        assert_eq!(parse("help").unwrap().command, Command::Help);
        assert_eq!(lookup("/next").unwrap().name, "repeat-search");
    }

    #[test]
    fn arguments_preserve_internal_spaces_and_unicode() {
        let open = parse(" /open folder/日本語 file.rs ").unwrap();
        assert_eq!(open.command, Command::OpenPath);
        assert_eq!(open.argument, "folder/日本語 file.rs");
        let search = parse("search-forward café  au lait").unwrap();
        assert_eq!(search.command, Command::Search);
        assert_eq!(search.argument, "café  au lait");
        assert_eq!(
            parse("self-insert-command 界").unwrap().command,
            Command::Insert('界')
        );
        assert_eq!(
            parse("self-insert-command  ").unwrap().command,
            Command::Insert(' ')
        );
    }

    #[test]
    fn character_arguments_preserve_printable_unicode_whitespace() {
        for ch in [' ', '\u{a0}', '\u{2003}', '\u{3000}'] {
            let input = format!("self-insert-command {ch}");
            assert_eq!(parse(&input).unwrap().command, Command::Insert(ch));
        }
    }

    #[test]
    fn invalid_names_and_arguments_cannot_dispatch_an_edit() {
        for input in [
            "does-not-exist",
            "save-buffer extra",
            "quit! extra",
            "self-insert-command",
            "self-insert-command ab",
            "self-insert-command \u{1b}",
        ] {
            assert!(parse(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn completion_uses_names_shared_prefixes_and_legacy_aliases() {
        assert_eq!(complete("sav"), "save-buffer");
        assert_eq!(complete("/save"), "save-buffer");
        assert_eq!(complete("delete"), "delete-");
        assert_eq!(complete("/next"), "repeat-search");
        assert_eq!(complete("no-such-command"), "no-such-command");
        assert_eq!(complete("search-forward save"), "search-forward save");
        assert_eq!(complete("日"), "日");
    }

    #[test]
    fn leading_whitespace_is_accepted_by_parsing_hints_and_completion() {
        for input in ["  sav", " /  sav", "\u{2003}save"] {
            assert_eq!(complete(input), "save-buffer");
            assert_eq!(
                super::matches(input)
                    .map(|spec| spec.name)
                    .collect::<Vec<_>>(),
                ["save-buffer"]
            );
            assert_eq!(
                parse(&complete(input)).unwrap().command,
                Command::SaveBuffer
            );
        }
        assert_eq!(complete("  search-forward save"), "  search-forward save");
    }

    #[test]
    fn every_keybound_editor_action_has_a_registered_name() {
        let keys = (32_u8..=126)
            .flat_map(|ch| {
                [
                    Key::Char(ch as char),
                    Key::Ctrl(ch as char),
                    Key::Meta(ch as char),
                    Key::Command(ch as char),
                ]
            })
            .chain([
                Key::Enter,
                Key::Tab,
                Key::BackTab,
                Key::Backspace,
                Key::Delete,
                Key::Left,
                Key::Right,
                Key::Up,
                Key::Down,
                Key::Ctrl(' '),
                Key::Char('界'),
                Key::Char('\u{a0}'),
                Key::Char('\u{2003}'),
                Key::Char('\u{3000}'),
            ]);
        for key in keys {
            for prefixed in [false, true] {
                let mut keymap = Keymap::new();
                if prefixed {
                    keymap.resolve(Key::Ctrl('x'));
                }
                let KeymapResult::Command(command) = keymap.resolve(key) else {
                    continue;
                };
                let spec = COMMANDS
                    .iter()
                    .find(|spec| match spec.target {
                        Target::Command(target) => command == target,
                        Target::Character => matches!(command, Command::Insert(_)),
                    })
                    .unwrap_or_else(|| panic!("no named command for {command:?}"));
                let input = match command {
                    Command::Insert(ch) => format!("{} {ch}", spec.name),
                    _ => spec.name.to_string(),
                };
                assert_eq!(parse(&input).unwrap().command, command);
            }
        }
    }
}
