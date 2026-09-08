# Issue 46: Named editor commands

## Task

Give every current editor action a stable command name through a small static registry.
`M-x` opens an empty command prompt, accepts canonical names and existing slash aliases, and shares dispatch with keybindings.
Keep existing save, dirty quit, file opening, buffer switching, search, and history behavior.

## Implementation

Store names, descriptions, aliases, argument rules, and typed targets in one inspectable registry.
Keep the keymap's typed commands and route them through the same application dispatcher used by named commands.
Show prefix matches in the minibuffer and complete an unambiguous name or shared prefix with Tab.
`help <command>` describes a command; the README lists the complete surface.
Retain `/open` as a file-only path command and `find-file` as the existing interactive file/directory prompt.
Do not add remapping, scripting, fuzzy completion, or command history.

## Acceptance and checks

Known names and aliases must execute equivalent actions.
Unknown names, invalid arguments, help, and cancellation must leave text unchanged.
Argument parsing must preserve spaces inside file paths and search text.
Completion must keep the cursor at the editable input and must not change text in the editor.
Add focused registry, application, keymap, and renderer tests.
Run the full suite, formatting, Clippy, release build, and a real PTY flow covering save, open, search, undo, quit cancel, unknown input, prompt cancel, and shell restoration.

## Verification

The final local suite passed 341 unit tests and eight terminal integration tests.
The two redirected-input cases fail under the local sandbox and require the unchanged GitHub checks.
Formatting, all-target Clippy with warnings denied, and the release build passed.
A release PTY session verified named movement, insertion, undo, dirty quit cancellation, Tab completion and cursor placement, save, search, unknown commands, prompt cancellation, find/open paths containing spaces, buffer switching, help, and restored shell settings.
Independent review approved after Unicode whitespace insertion was made equivalent to normal typing.
