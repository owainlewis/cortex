# Cortex

A small macOS-only terminal code editor written in Rust.

Cortex is currently in v0.3 development.
The current goal is one fast editing loop: open files or a directory, edit and switch buffers, search, save, and quit cleanly.

## Platform

Cortex is currently macOS-only.
Other platforms are not a current or v1 goal.

## Install

Install the latest stable release with the shell installer:

```sh
curl -fsSL https://raw.githubusercontent.com/owainlewis/cortex/main/install.sh | bash
```

The installer downloads the latest GitHub Release for macOS arm64, verifies the checksum, and installs `cortex` to `~/.local/bin` by default.
Set `CORTEX_INSTALL_DIR` to choose another install directory.

```sh
curl -fsSL https://raw.githubusercontent.com/owainlewis/cortex/main/install.sh | CORTEX_INSTALL_DIR="$HOME/bin" bash
```

To install a specific stable release, pass its tag:

```sh
curl -fsSL https://raw.githubusercontent.com/owainlewis/cortex/main/install.sh | bash -s -- --version v0.2.0
```

You can also download the release tarball and `.sha256` file directly from GitHub Releases.
Verify the checksum before running the binary.

## Update

Run the installer again to update to the latest stable release:

```sh
curl -fsSL https://raw.githubusercontent.com/owainlewis/cortex/main/install.sh | bash
```

Check the installed binary version with:

```sh
cortex --version
```

Check whether GitHub has a newer stable release with:

```sh
cortex --check-update
```

`cortex --check-update` never updates the binary by itself.
It only reports release status.

## Developer Run

Install Rust, then run Cortex.
Pass a file path to open that file directly:

```sh
cargo run -- path/to/file.txt
```

Pass a directory path to open the directory picker:

```sh
cargo run -- .
```

Run Cortex with no path to open the current directory in the picker:

```sh
cargo run
```

For a temporary smoke test file:

```sh
cargo run -- /tmp/cortex-smoke.txt
```

You can also install from the current Git repository as a developer fallback:

```sh
cargo install --git https://github.com/owainlewis/cortex.git
```

Existing files open with their current contents.
Missing files open as empty clean buffers attached to the requested path.
Saving can create the target file when its parent directory already exists.
Saving does not create missing parent directories.
Inside Cortex, `C-x C-f` accepts an existing or missing file path.
Enter a missing path, edit the empty buffer, and save to create the file.
Enter a directory path to browse it in the directory picker.
Directories open a picker that lists non-hidden entries.
The picker can open regular files.
The picker can expand and collapse directories.

## Current Scope

The current editor supports multiple file buffers in the terminal alternate screen.
It uses raw terminal mode while running and should restore the shell after exit.
It shows file text, cursor position, dirty state, save errors, and short status messages in a modeline.
It includes shared prompts for commands and buffer navigation, a directory picker, forward search, mark and cut/yank editing, undo and redo, visual polish, and syntax highlighting for supported file types.

## Editor Keybindings

| Key | Action |
| --- | --- |
| Printable character | Insert character |
| Enter | Insert newline |
| Backspace | Delete backward |
| Delete or `C-d` | Delete forward |
| Left or `C-b` | Move backward one character |
| Right or `C-f` | Move forward one character |
| Up or `C-p` | Move to previous line |
| Down or `C-n` | Move to next line |
| `C-a` | Move to start of line |
| `C-e` | Move to end of line |
| `C-s` | Repeat the previous search |
| `C-Space` | Set the mark |
| `C-w` | Cut the active region |
| `C-k` | Cut to the end of the line |
| `C-y` | Yank the newest cut |
| `M-y` | Replace the last yank with an older cut |
| `M-w` | Copy the active region to the macOS clipboard |
| `C-c C-v` | Paste the macOS clipboard into the buffer or active prompt |
| `C-/` or `C-_` | Undo the last edit |
| `C-x u` | Undo the last edit |
| `Command-z` | Undo the last edit |
| `C-x C-f` | Enter a path to find or create a file buffer |
| `C-x b` | Switch to an open buffer by path or unique file name |
| `C-x C-r` | Reload a clean buffer from disk |
| `C-x C-s` | Save the file |
| `C-x C-c` | Quit |
| Tab / Shift-Tab | Indent or outdent the active region; otherwise insert to a tab stop or outdent the current line |
| `M-x` | Open the named command prompt |

If any open buffer is dirty, `C-x C-c` asks whether to quit without saving.
Press `y` to confirm.
Press `n` or Escape to cancel.

Tab inserts spaces to the next four-column stop.
Enter carries leading spaces and tabs from the current line, limited to indentation before point.
Newlines retain the current line's LF or CRLF style; an unterminated final line uses the previous line's style, and a new file uses LF.
With a region active, Tab adds four spaces to each selected line and Shift-Tab removes up to four columns of leading indentation.
Each region change is one undo step and keeps the region active for another indentation command.
A selection ending at the start of a line excludes that line.
Shift-Tab without a region outdents the current line and preserves any remaining tabs.

Terminal paste uses bracketed paste mode.
Pasted text, including tabs and line endings, is inserted literally as one undo step and clears the active selection.
Paste does not run keybindings or add automatic indentation.
In command, file, and buffer prompts, line breaks and tabs become spaces; CRLF becomes one space and other control characters are removed.
Pasting does not submit a prompt or confirm a dirty quit.
Pasted text is ignored in the directory picker.

## Named commands

Press `M-x`, type a command name, and press Enter.
The minibuffer shows matching names as you type.
Tab completes a unique match or the shared prefix of several matches.
Use `help <command>` for a description.
Escape cancels the prompt; unknown names and invalid arguments report an error without changing text.

| Command | Action | Existing alias |
| --- | --- | --- |
| `save-buffer` | Save the current file | `/save` |
| `reload-buffer` | Reload a clean buffer from disk | `/reload` |
| `find-file` | Prompt for a file or directory relative to the active file | |
| `switch-buffer` | Prompt for an open buffer | |
| `open-file <path>` | Open a file relative to the working directory | `/open <path>` |
| `search-forward <text>` | Search forward for literal text | `/search <text>` |
| `repeat-search` | Repeat the previous search | `/next` |
| `undo`, `redo` | Undo or redo an edit | `/undo`, `/redo` |
| `quit` | Quit with the dirty-buffer guard | `/quit` |
| `force-quit` | Quit without saving | `/quit!` |
| `forward-char`, `backward-char` | Move one grapheme | |
| `next-line`, `previous-line` | Move one line | |
| `beginning-of-line`, `end-of-line` | Move to a line boundary | |
| `newline` | Insert a newline carrying leading indentation | |
| `indent`, `outdent` | Apply Tab or Shift-Tab behavior | |
| `delete-char`, `delete-backward-char` | Delete one grapheme | |
| `set-mark`, `kill-region`, `kill-line`, `yank` | Select, cut, or insert cut text | |
| `yank-pop` | Replace the last yank with an older cut | |
| `copy-region` | Copy the region without deleting it | `clipboard-copy` |
| `clipboard-paste` | Insert literal clipboard text | |
| `self-insert-command <character>` | Insert one printable character | |
| `execute-extended-command` | Open the command prompt | |
| `help`, `help <command>` | List commands or describe one | `/help`, `/commands` |

Aliases also work without a leading slash.
`open-file` and `/open` reject directories and open another buffer without discarding unsaved changes.
Use `find-file` to browse a directory.
Typing `/` in the editor still inserts a slash.

## Directory Picker Keybindings

| Key | Action |
| --- | --- |
| Down or `C-n` | Move to next entry |
| Up or `C-p` | Move to previous entry |
| Enter | Open the selected regular file or expand the selected directory |
| Left | Collapse the selected directory or move to its parent row |
| Backspace | Browse to the parent directory |
| Escape | Quit the picker |
| `C-x C-c` | Quit the picker |

## Cuts and history

Cortex keeps the last 32 nonempty cuts in memory.
`C-y` inserts the newest cut; repeated `M-y` cycles through older cuts and wraps to the newest.
Movement, edits, save, buffer changes, and undo/redo end yank-pop, so it cannot replace unrelated text.
`M-x yank-pop` also works after a yank.
Each kill, yank, and yank-pop is one undo step.
The ring is local to this process and does not read or write the system clipboard.

Clipboard commands run only when requested, using macOS `pbcopy` and `pbpaste`.
Copy keeps the region and buffer unchanged; clipboard paste is one literal undo step and clears the selection.
In editable prompts, `C-c C-v` flattens line breaks and tabs without submitting the prompt.
Clipboard commands time out after two seconds and reject transfers above 16 MiB or invalid UTF-8, leaving buffer text intact.
Clipboard errors in a prompt remain visible until the next prompt key; the entered text is preserved.

Contiguous typing and same-direction deletion undo as a group.
A pause of at least 750 ms, movement, save, prompt entry, buffer switch, or deliberate edit starts a new group.
Paste, indentation, kills, and yanks remain separate undo steps.
History retains up to 16 MiB of inserted and deleted text per buffer, evicting the oldest whole groups and always keeping the newest group even if it exceeds that limit.

## Syntax Highlighting

Cortex highlights Rust, Markdown, JSON, TOML, Python, JavaScript, JSX, TypeScript, TSX, Ruby, and OCaml files.
Other file types render as plain text.

## Releases And Nightlies

Stable releases are built from tags like `v0.2.0`.
The release workflow publishes an arm64 macOS tarball and matching `.sha256` checksum.

Nightly builds are unstable test artifacts from `main`.
They are downloadable from the Nightly workflow run artifacts and are not used by the stable installer.

See [docs/release.md](docs/release.md) for the release checklist.

## Known Limitations

Redo is available through `M-x redo` and has no dedicated keybinding.
Cortex shows one active buffer at a time.
The switch-buffer prompt requires an exact path or a unique file name and does not offer completion yet.
Search is forward-only through `search-forward <text>` or `/search <text>`, with `C-s` or `/next` repeating the last search.
Incremental and reverse search are not implemented yet.
The directory picker can expand directories, but it is still a minimal picker.
The slash command `/open <path>` opens files only, not directories.
Internal splits, tabs, and an embedded terminal are deferred in favour of external terminal panes.
There is no config, plugin system, LSP, or AI integration.
Long lines are clipped to the terminal width instead of wrapped.
External file changes are not watched automatically.
The modeline marks detected disk changes, and `C-x C-r` or `/reload` reloads the file.
Reload refuses to replace a buffer with unsaved edits.

## License

MIT.
