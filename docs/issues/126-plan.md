# Issues 126 and 127 modeline priority plan

## Task

Keep critical modeline state visible when the terminal is too narrow for the ordinary modeline.
This covers dirty-quit confirmation from issue #126 and the external disk-change warning from issue #127.

## Acceptance criteria

- A dirty quit at 34 columns clearly asks for confirmation and shows the accepted `y` and `n` keys.
- Prompt state takes priority over keycast, file name, dirty state, position, language, disk state, and other informational fields when the ordinary modeline does not fit.
- `[disk-changed]` remains visible at every width that can display the complete marker.
- Clean or modified state remains beside `[disk-changed]` when the width can display both.
- Long Unicode file names, position, language, and other lower-priority fields yield before the disk-change marker.
- The existing wide modeline field order, content, and styling remain unchanged.
- `y`, `n`, and Escape retain their current dirty-quit behavior.
- Dirty-buffer quit does not write in-memory edits.
- Resize and quit restore the alternate screen, cursor, terminal modes, and a reusable shell.

## Implementation

Keep ordinary modeline composition unchanged.
Add a fitting step that uses the ordinary modeline whenever it fits.
When it does not fit, render prompt state first, then disk-change and buffer state, before falling back to the existing right-clipped modeline for informational states.
Keep the change inside renderer composition and fitting.

## Checks

- Add a focused renderer regression for a 34-column dirty-quit prompt with a keycast and long file name.
- Add focused renderer regressions for narrow clean and dirty disk-changed buffers with long Unicode file names.
- Extend the app dirty-quit test to prove prompt state and `y`, `n`, and Escape behavior.
- Run focused renderer, app, and buffer reload tests.
- Run `cargo fmt --all -- --check`.
- Run `cargo test --all-targets`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo build --all-targets`.
- In a synchronized PTY, resize a dirty editor from 92 to 34 columns, verify visible confirmation, cancel once, confirm once, verify disk non-persistence, compare terminal state before and after, and reuse the shell.
