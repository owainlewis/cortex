# Issue 18 Plan

## Goal

Add a small slash command line in the bottom chrome for named editor commands.
The command line should support `/open <path>`, `/save`, `/quit`, `/quit!`, and a compact `/help` message if it stays simple.
The command line should reuse the existing command dispatch behavior for save and normal quit.

## Current repo context

Cortex already routes editor keys through `Keymap` into `commands::dispatch`.
`AppState` owns dirty quit prompt state and transient status messages.
`Renderer` renders the file viewport and bottom modeline.
`Buffer::open` already has the desired startup file behavior for existing and missing files.
Directory paths currently use the startup directory picker from issue 17.

## Proposed implementation

Add command-line state to `AppState`.
Use `/` in normal editor mode to enter command-line input instead of inserting a slash into the buffer.
While command-line input is active, printable characters edit the command text, Backspace deletes, Enter submits, and Escape cancels.
Render active command-line input in the existing bottom chrome.
Dispatch `/save` and `/quit` through `commands::dispatch`.
Implement `/quit!` as an explicit forced quit action that bypasses dirty confirmation.
Implement `/open <path>` by opening the requested path with `Buffer::open`, replacing the active buffer only after open succeeds, and resetting the view.
Reject `/open <directory>` with a clear status message for this issue.
Show a clear status message for unknown or malformed commands and keep the editor open.

## Acceptance criteria

- Pressing `/` opens a command line at the bottom of the editor.
- Escape cancels the command line without editing the buffer.
- `/save` saves the current file and shows the existing save status.
- `/quit` follows the same clean and dirty quit rules as `C-x C-c`.
- `/quit!` exits without saving when dirty.
- `/open <path>` opens a file path using the same buffer/file behavior as normal startup.
- Unknown commands show a clear status message and keep the editor open.
- Keybindings continue to work as before outside command-line mode.

## Verification steps

- Run `cargo fmt -- --check`.
- Run `cargo test`.
- Manually test `/save`, `/quit`, `/quit!`, `/open <path>`, Escape cancellation, and an unknown command in a PTY.
- Manually confirm the shell is usable after quitting.

## Out of scope

Completion is out of scope.
Command history is out of scope.
Fuzzy command search is out of scope.
User-defined commands are out of scope.
Scripting and shell command execution are out of scope.
Plugin or agent integration is out of scope.
Mid-session directory picker support for `/open <directory>` is out of scope.
