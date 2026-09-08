# Issue 162: correct existing syntax highlighting

## Goal

Restore ordinary TypeScript/TSX syntax categories and keep Markdown formatting outside code blocks.
Do not change text editing, add grammars, or alter the highlighter cache policy.

## Evidence and approach

The new TypeScript regression fails on `const` before the fix because only the TypeScript supplemental query is configured.
Combine JavaScript captures with TypeScript captures and add JSX captures for TSX.
Tree-sitter Highlight 0.26.11 gives later matching patterns precedence on the same node, so the base query comes first and the language-specific queries follow it.
Keep JavaScript injection support for template literals.

The new Markdown regressions fail because a second inline pass interprets fenced and indented source as prose.
Removing that pass alone loses valid inline markup: the upstream Markdown injection query excludes child nodes by default.
Use Markdown injection queries that include child content for inline, pipe-table-cell, and fenced-code nodes.
Keep the other existing HTML and frontmatter injection rules.
The block parser now supplies the code/prose boundary and multiline inline context; remove the redundant line-based parser and its unused helpers.

## Acceptance and proof

- Assert exact TypeScript keyword, type, string, comment, function, and number categories.
- Assert TSX keyword, type, tag, attribute, string, and property categories.
- Check backtick, tilde, and quoted Rust fences retain string styles when source contains Markdown-looking emphasis.
- Check unlabelled, unsupported-language, and indented code do not acquire inline Markdown formatting.
- Preserve multiline emphasis and existing prose links, headings, inline code, and table-cell formatting.
- Verify the actual retained renderer cell uses the Rust string style inside a Markdown fence.
- Run all highlighter and renderer tests, formatting, Clippy, and the complete suite.
- Run a release-build terminal smoke with temporary TS, TSX, and Markdown fixtures, checking emitted colours and shell restoration.

## Delivery

Update roadmap state for the already merged direction task and superseded layout tickets.
Open one PR for #162, obtain independent review, pass CI, and merge under the owner's batch authorization.
Performance redesign and additional language support remain separate tasks.
