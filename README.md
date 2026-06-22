# Cortex

A small macOS-only terminal code editor written in Rust.

Cortex is currently in v0.2 development.
The current goal is one fast editing loop: open a file or directory, edit one buffer, save it, and quit cleanly.

## Platform

Cortex is currently macOS-only.
Other platforms are not a current or v1 goal.

## Run

Install Rust, then run Cortex with exactly one path.
Pass a file path to open that file directly:

```sh
cargo run -- path/to/file.txt
```

Pass a directory path to open the directory picker:

```sh
cargo run -- .
```

For a temporary smoke test file:

```sh
cargo run -- /tmp/cortex-smoke.txt
```

Existing files open with their current contents.
Missing files open as empty clean buffers attached to the requested path.
Saving can create the target file when its parent directory already exists.
Saving does not create missing parent directories.
Directories open a picker that lists non-hidden entries.
The picker can open regular files.
It does not descend into directories yet.

## Current Scope

The current editor supports one buffer in the terminal alternate screen.
It uses raw terminal mode while running and should restore the shell after exit.
It shows file text, cursor position, dirty state, save errors, and short status messages in a modeline.
It includes a directory picker, a slash command line, visual theme and modeline polish, and syntax highlighting for supported file types.

## Editor Keybindings

| Key | Action |
| --- | --- |
| Printable character | Insert character |
| Enter | Insert newline |
| Backspace | Delete backward |
| Delete | Delete forward |
| Left or `C-b` | Move backward one character |
| Right or `C-f` | Move forward one character |
| Up or `C-p` | Move to previous line |
| Down or `C-n` | Move to next line |
| `C-a` | Move to start of line |
| `C-e` | Move to end of line |
| `C-x C-s` | Save the file |
| `C-x C-c` | Quit |
| `/` | Open the slash command line |

If the buffer is dirty, `C-x C-c` asks whether to quit without saving.
Press `y` to confirm.
Press `n` or Escape to cancel.

## Slash Commands

| Command | Action |
| --- | --- |
| `/save` | Save the current file |
| `/quit` | Quit, using the same dirty-buffer prompt as `C-x C-c` |
| `/quit!` | Quit without saving |
| `/open <path>` | Open a file path when the current buffer is clean |
| `/help` | Show the available slash commands |

Escape cancels the command line.
Unknown slash commands leave the editor open and show an error message.
`/open <path>` rejects directories and keeps the current buffer in place when it has unsaved changes.

## Directory Picker Keybindings

| Key | Action |
| --- | --- |
| Down or `C-n` | Move to next entry |
| Up or `C-p` | Move to previous entry |
| Enter | Open the selected regular file |
| Escape | Quit the picker |
| `C-x C-c` | Quit the picker |

## Syntax Highlighting

Cortex highlights Rust, Markdown, JSON, and TOML files.
Other file types render as plain text.
JavaScript and TypeScript highlighting are not implemented yet.

## Known Limitations

Cortex has one active buffer at a time.
The directory picker opens regular files only and does not navigate into directories.
The slash command `/open <path>` opens files only, not directories.
There are no splits, tabs, minibuffer, search, kill ring, undo, config, plugins, LSP, AI integration, or embedded terminal pane yet.
Long lines are clipped to the terminal width instead of wrapped.
The renderer uses a simple full-screen redraw rather than diffed rendering.
External file changes are not watched or reloaded.

## License

MIT.
