//! Candidates for the minibuffer menu: commands, paths, and open buffers.

use crate::{command_registry, keymap};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Directory reads stop after this many matching entries so a huge
/// directory cannot stall a keypress.
const MAX_PATH_CANDIDATES: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Prompt text after accepting this candidate.
    pub replacement: String,
    pub label: String,
    pub keys: &'static str,
    pub detail: String,
    /// A directory, which Enter descends into instead of opening.
    pub directory: bool,
}

/// Candidates for `M-x` input: command names first, then path arguments.
pub fn commands(input: &str) -> Vec<Candidate> {
    let text = command_registry::command_text(input);
    if let Some((name, argument)) = text.split_once(char::is_whitespace) {
        let Some(spec) = command_registry::lookup(name) else {
            return Vec::new();
        };
        if !spec.takes_path() {
            return Vec::new();
        }
        let argument = argument.trim_start();
        return paths(argument)
            .into_iter()
            .map(|candidate| Candidate {
                replacement: format!("{} {}", spec.name, candidate.replacement),
                ..candidate
            })
            .collect();
    }

    let needle = text.to_lowercase();
    let mut prefixed = Vec::new();
    let mut contained = Vec::new();
    for spec in command_registry::COMMANDS
        .iter()
        .filter(|spec| spec.listed())
    {
        let names = std::iter::once(spec.name).chain(spec.aliases().iter().copied());
        let target = if names.clone().any(|name| name.starts_with(&needle)) {
            &mut prefixed
        } else if spec.name.contains(&needle) {
            &mut contained
        } else {
            continue;
        };
        target.push(Candidate {
            replacement: if spec.takes_argument() {
                format!("{} ", spec.name)
            } else {
                spec.name.to_string()
            },
            label: spec.name.to_string(),
            keys: spec.command().and_then(keymap::binding_label).unwrap_or(""),
            detail: spec.description.to_string(),
            directory: false,
        });
    }
    prefixed.extend(contained);
    prefixed
}

/// Entries of the directory named by `input`, filtered by its last component.
/// Hidden entries appear only when that component starts with a dot.
pub fn paths(input: &str) -> Vec<Candidate> {
    let input = effective_path(input);
    let (directory_text, prefix) = match input.rfind('/') {
        Some(index) => input.split_at(index + 1),
        None => ("", input),
    };
    let directory = if directory_text.is_empty() {
        PathBuf::from(".")
    } else {
        expand_home(directory_text)
    };
    let Ok(entries) = fs::read_dir(&directory) else {
        return Vec::new();
    };
    let prefix_lower = prefix.to_lowercase();
    let mut candidates: Vec<Candidate> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with('.') && !prefix.starts_with('.') {
                return None;
            }
            if !name.to_lowercase().starts_with(&prefix_lower) {
                return None;
            }
            // Follow symlinks so a link to a directory can be descended.
            let is_dir = fs::metadata(entry.path()).is_ok_and(|metadata| metadata.is_dir());
            let label = if is_dir { format!("{name}/") } else { name };
            Some(Candidate {
                replacement: format!("{directory_text}{label}"),
                label,
                keys: "",
                detail: String::new(),
                directory: is_dir,
            })
        })
        .take(MAX_PATH_CANDIDATES)
        .collect();
    candidates.sort_by(|a, b| {
        (a.label.to_lowercase(), &a.label).cmp(&(b.label.to_lowercase(), &b.label))
    });
    candidates
}

/// Open buffers whose path contains `input`, ignoring case.
pub fn buffers(input: &str, names: &[String]) -> Vec<Candidate> {
    let needle = input.to_lowercase();
    names
        .iter()
        .filter(|name| name.to_lowercase().contains(&needle))
        .map(|name| Candidate {
            replacement: name.clone(),
            label: name.clone(),
            keys: "",
            detail: String::new(),
            directory: false,
        })
        .collect()
}

/// Lets a new absolute or home path be typed after a prefilled directory,
/// as in Emacs: `a/b//etc` means `/etc` and `a/b/~/x` means `~/x`.
pub fn effective_path(input: &str) -> &str {
    let restart = [
        input.rfind("//").map(|i| i + 1),
        input.rfind("/~/").map(|i| i + 1),
    ]
    .into_iter()
    .flatten()
    .max();
    restart.map_or(input, |index| &input[index..])
}

/// Expands a leading `~` to the home directory.
pub fn expand_home(text: &str) -> PathBuf {
    let text = effective_path(text);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match (text.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home,
        (Some(rest), Some(home)) if rest.starts_with('/') => home.join(&rest[1..]),
        _ => PathBuf::from(text),
    }
}

/// The directory to prefill in find-file: the active file's parent, written
/// with `~` for the home directory and a trailing slash.
pub fn directory_prompt(active_path: &Path) -> String {
    let Some(parent) = active_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return String::new();
    };
    let text = parent.to_string_lossy();
    let text = match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && text == home => "~".to_string(),
        Ok(home) if !home.is_empty() && text.starts_with(&format!("{home}/")) => {
            format!("~{}", &text[home.len()..])
        }
        _ => text.into_owned(),
    };
    if text.ends_with('/') {
        text
    } else {
        format!("{text}/")
    }
}

/// Removes the last path component or word, keeping the separator before it.
pub fn delete_component(input: &mut String) {
    let trimmed = input.trim_end_matches(['/', ' ']);
    let keep = trimmed.rfind(['/', ' ']).map_or(0, |index| index + 1);
    input.truncate(keep);
}

#[cfg(test)]
mod tests {
    use super::{buffers, commands, delete_component, directory_prompt, expand_home, paths};
    use std::{fs, path::Path};

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cortex-completion-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn command_candidates_list_prefix_matches_before_substring_matches() {
        let names: Vec<_> = commands("open")
            .into_iter()
            .map(|candidate| candidate.label)
            .collect();
        assert_eq!(names[..2], ["open-file", "open-directory"]);

        let names: Vec<_> = commands("buffer")
            .into_iter()
            .map(|candidate| candidate.label)
            .collect();
        assert!(names.contains(&"save-buffer".to_string()));
        assert!(names.contains(&"switch-buffer".to_string()));
    }

    #[test]
    fn command_candidates_show_bindings_and_leave_room_for_arguments() {
        let save = commands("save")
            .into_iter()
            .find(|candidate| candidate.label == "save-buffer")
            .unwrap();
        assert_eq!(save.keys, "C-x C-s");
        assert_eq!(save.replacement, "save-buffer");

        let open = commands("open-f").remove(0);
        assert_eq!(open.replacement, "open-file ");
        assert!(commands("self-insert").is_empty());
    }

    #[test]
    fn first_command_candidate_completes_names_and_legacy_aliases() {
        let first = |input: &str| commands(input).first().map(|c| c.replacement.clone());
        assert_eq!(first("sav").as_deref(), Some("save-buffer"));
        assert_eq!(first("/save").as_deref(), Some("save-buffer"));
        assert_eq!(first("/next").as_deref(), Some("repeat-search"));
        for input in ["  sav", " /  sav", "\u{2003}save"] {
            assert_eq!(first(input).as_deref(), Some("save-buffer"), "{input:?}");
        }
        assert_eq!(first("no-such-command"), None);
        assert_eq!(first("日"), None);
        assert_eq!(first("search-forward save"), None);
    }

    #[test]
    fn empty_command_input_lists_every_listed_command() {
        assert!(commands("").len() > 30);
    }

    #[test]
    fn path_candidates_complete_directories_and_hide_dotfiles() {
        let dir = test_dir("paths");
        fs::create_dir(dir.join("src")).unwrap();
        fs::write(dir.join("Cargo.toml"), "").unwrap();
        fs::write(dir.join("cargo.lock"), "").unwrap();
        fs::write(dir.join(".hidden"), "").unwrap();
        let base = format!("{}/", dir.display());

        let labels: Vec<_> = paths(&base).into_iter().map(|c| c.label).collect();
        assert_eq!(labels, ["cargo.lock", "Cargo.toml", "src/"]);

        let found = paths(&format!("{base}Ca"));
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].replacement, format!("{base}Cargo.toml"));

        let found = paths(&format!("{base}s"));
        assert!(found[0].directory);
        assert_eq!(found[0].replacement, format!("{base}src/"));

        let labels: Vec<_> = paths(&format!("{base}."))
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, [".hidden"]);
        assert!(paths(&format!("{base}missing/")).is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn path_arguments_complete_inside_the_command_menu() {
        let dir = test_dir("menu-paths");
        fs::write(dir.join("notes.md"), "").unwrap();
        let found = commands(&format!("open {}/no", dir.display()));
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].replacement,
            format!("open-file {}/notes.md", dir.display())
        );
        assert!(commands("goto-line 1").is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn typing_a_new_root_after_a_prefilled_directory_restarts_the_path() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_home("~/code//etc/hosts"), Path::new("/etc/hosts"));
        assert_eq!(
            expand_home("/tmp/a/~/notes"),
            Path::new(&home).join("notes")
        );
        assert_eq!(expand_home("src/main.rs"), Path::new("src/main.rs"));
        let dir = test_dir("restart");
        fs::write(dir.join("file.txt"), "").unwrap();
        let found = paths(&format!("~/somewhere/{}/fi", dir.display()));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].replacement, format!("{}/file.txt", dir.display()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn home_expansion_only_applies_to_a_leading_tilde() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_home("~"), Path::new(&home));
        assert_eq!(expand_home("~/code"), Path::new(&home).join("code"));
        assert_eq!(expand_home("a/~"), Path::new("a/~"));
        assert_eq!(expand_home("~other"), Path::new("~other"));
    }

    #[test]
    fn find_file_prompt_starts_in_the_active_directory() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(directory_prompt(Path::new("notes.md")), "");
        assert_eq!(directory_prompt(Path::new("src/main.rs")), "src/");
        assert_eq!(directory_prompt(Path::new("/tmp/x.rs")), "/tmp/");
        assert_eq!(
            directory_prompt(&Path::new(&home).join("code/x.rs")),
            "~/code/"
        );
    }

    #[test]
    fn delete_component_removes_one_path_part_or_word() {
        for (before, after) in [
            ("~/code/cortex/", "~/code/"),
            ("~/code/cor", "~/code/"),
            ("open-file ~/", "open-file "),
            ("open-file ", ""),
            ("save", ""),
        ] {
            let mut input = before.to_string();
            delete_component(&mut input);
            assert_eq!(input, after, "{before}");
        }
    }

    #[test]
    fn buffer_candidates_match_any_part_of_the_path() {
        let names = vec!["src/app.rs".to_string(), "README.md".to_string()];
        let labels: Vec<_> = buffers("app", &names)
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["src/app.rs"]);
        assert_eq!(buffers("", &names).len(), 2);
        assert_eq!(buffers("readme", &names)[0].label, "README.md");
    }
}
