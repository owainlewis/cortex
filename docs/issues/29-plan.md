# Issue 29: Incremental literal search

## Task and acceptance

C-s and C-r open forward and reverse incremental search with an empty query.
Typing or pasting changes the query without changing text, dirty state, revision, history, or the kill ring.
Backspace removes one grapheme from the query.
Repeating either search key advances in that direction, wrapping once and showing when it wraps.
With an empty query, a repeat recalls the most recent accepted or named search, if any.
A failed query shows no-match and keeps the last useful point.
Enter accepts the current location; Escape or C-g cancels and restores the original View, including both scroll offsets, even after a pending clipboard prefix.
Other editing and navigation keys accept the search and dispatch normally.

The current literal match has a distinct background and renders complete graphemes across horizontal scrolling and resize.
Search state is separate from text storage and remains transient in AppState.
Keep the existing named literal search and repeat aliases while allowing search-forward and search-backward without arguments to open incremental search.

Stream Rope characters through a linear literal matcher with memory proportional to query length.
Support reverse traversal, overlapping occurrences, chunk-spanning matches, empty buffers, Unicode, and matches at both file edges.
Do not copy the whole buffer on search keystrokes.

## Checks

Test query updates, repetition, direction changes, wrapping, backspace, accept, cancel with scroll restoration, no-match, paste, unchanged history, and named command compatibility.
Compare streaming matching against flat-string expectations and verify scan counts stop after a nearby match even in a large buffer.
Test active-match rendering with Unicode and horizontal scrolling.
Run formatting, strict Clippy, the full suite, a release build, and independent review.
Run a release terminal session searching Markdown forward and backward, repeat/wrap, fail and recover, cancel, accept, save, and quit with the shell restored.
