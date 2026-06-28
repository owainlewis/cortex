# Cortex

A small macOS-only terminal code editor written in Rust.

Cortex is currently in v0.3 development.
The current goal is one fast editing loop: open a file or directory, edit one buffer, save it, and quit cleanly.

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
curl -fsSL https://raw.githubusercontent.com/owainlewis/cortex/main/install.sh | bash -s -- --version v0.1.0
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

You can also install from the current Git repository as a developer fallback:

```sh
cargo install --git https://github.com/owainlewis/cortex.git
```

Existing files open with their current contents.
Missing files open as empty clean buffers attached to the requested path.
Saving can create the target file when its parent directory already exists.
Saving does not create missing parent directories.
Directories open a picker that lists non-hidden entries.
The picker can open regular files.
The picker can expand and collapse directories.

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
| `C-s` | Repeat the previous search |
| `C-Space` | Set the mark |
| `C-w` | Cut the active region |
| `C-k` | Cut to the end of the line |
| `C-y` | Yank the last cut text |
| `C-/` or `C-_` | Undo the last edit |
| `Command-z` | Undo the last edit |
| `C-x C-f` | Open the file picker when the current buffer is clean |
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
| `/search <text>` | Search forward for text |
| `/next` | Repeat the previous search |
| `/undo` | Undo the last edit |
| `/redo` | Redo the last undone edit |
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
| Enter | Open the selected regular file or expand the selected directory |
| Left | Collapse the selected directory or move to its parent row |
| Backspace | Browse to the parent directory |
| Escape | Quit the picker |
| `C-x C-c` | Quit the picker |

## Syntax Highlighting

Cortex highlights Rust, Markdown, JSON, TOML, Python, JavaScript, JSX, TypeScript, TSX, and Ruby files.
Other file types render as plain text.

## Releases And Nightlies

Stable releases are built from tags like `v0.1.0`.
The release workflow publishes a GitHub Release with a macOS tarball and matching `.sha256` checksum.

Nightly builds are unstable test artifacts from `main`.
They are downloadable from the Nightly workflow run artifacts and are not used by the stable installer.

See [docs/release.md](docs/release.md) for the release checklist.

## Known Limitations

Redo is available from the slash command line, but does not have a dedicated keybinding yet.
Cortex has one active buffer at a time.
The directory picker can expand directories, but it is still a minimal picker.
The slash command `/open <path>` opens files only, not directories.
There are no splits, tabs, minibuffer, config, plugins, LSP, AI integration, or embedded terminal pane yet.
Long lines are clipped to the terminal width instead of wrapped.
The renderer uses a simple full-screen redraw rather than diffed rendering.
External file changes are not watched or reloaded.

## License

MIT.
