# Cortex

A small macOS-only terminal code editor written in Rust.

Cortex is currently in v0.1 development.
The v0.1 goal is one plain editing loop: open one file, edit it, save it, and quit cleanly.

## Platform

Cortex is currently macOS-only.
Other platforms are not a v0.1 or v1 goal.

## Run

Install Rust, then run Cortex with exactly one file path:

```sh
cargo run -- path/to/file.txt
```

For a temporary smoke test file:

```sh
cargo run -- /tmp/cortex-smoke.txt
```

Existing files open with their current contents.
Missing files open as empty clean buffers attached to the requested path.
Saving can create the target file when its parent directory already exists.
Saving does not create missing parent directories.

## v0.1 Scope

The current editor supports a single file in the terminal alternate screen.
It uses raw terminal mode while running and should restore the shell after exit.
It shows file text, cursor position, dirty state, save errors, and short status messages in a modeline.

## Keybindings

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

If the buffer is dirty, `C-x C-c` asks whether to quit without saving.
Press `y` to confirm.
Press `n` or Escape to cancel.

## Known Limitations

Cortex only opens one file per process.
There are no splits, tabs, minibuffer, search, kill ring, undo, syntax highlighting, config, plugins, LSP, AI integration, or embedded terminal pane yet.
Long lines are clipped to the terminal width instead of wrapped.
The renderer uses a simple full-screen redraw rather than diffed rendering.
External file changes are not watched or reloaded.

## License

MIT.
