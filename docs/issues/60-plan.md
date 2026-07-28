# Issue 60 plan

## Task

Preserve existing macOS file metadata during atomic saves and define safe symlink behavior.

## Acceptance criteria

- Saving an existing regular file preserves its owner, group, mode, ACLs, and extended attributes.
- Saving through a symlink updates its stable regular-file target without replacing the link.
- Broken, retargeted, non-regular, externally changed, or externally replaced targets fail clearly.
- A metadata or write failure leaves the original file untouched and removes the temporary file.
- Saving a new file keeps the existing secure create-and-rename behavior and umask-derived defaults.
- Existing ordinary-file save, dirty tracking, reload, and terminal cleanup behavior remains intact.

## Implementation

1. Record whether a buffer opened a missing path, regular file, or symlink to a regular file, including the resolved save target.
2. Refuse normal save when that location or its disk stamp no longer matches the open or last-save baseline.
3. Write to the existing securely created sibling temporary file.
4. For an existing file, copy all native macOS metadata from an open source descriptor to the temporary file before the final rename.
5. Recheck the stable location and source descriptor immediately before rename, then clean up the temporary file on any failure.
6. Add focused tests for ordinary files, modes and ownership, extended attributes, symlinks, races, new-file defaults, and failure cleanup.

The atomic `RENAME_EXCL` or `RENAME_SWAP` is the save commit point.
Changes observed before it are save races and are refused or restored.
An identity change observed during immediate post-commit verification returns a conservative save error because it is indistinguishable from pre-commit temporary-path substitution.
Changes after successful identity verification are later external edits and are reported by disk-change tracking.
macOS does not provide an inode-conditional rename or unlink transaction across the later verification and cleanup syscalls.
If interference makes rollback or cleanup ambiguous, Cortex reports the recovery path and does not delete an unverified inode.

## Verification

- Run the focused buffer tests.
- Run `cargo fmt --check`.
- Run `cargo clippy -- -D warnings`.
- Run `cargo test`.
- Run `cargo build`.
- Manually save and exit in a PTY, check executable mode and symlink behavior, and confirm the shell is restored.
