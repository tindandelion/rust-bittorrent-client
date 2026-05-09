---
name: final-check-post
description: Perform a final pass on Jekyll blog posts: fix grammar and minor style issues, resolve Rust link placeholders, and verify no links are missing. Use when editing files under `_posts/` or `_drafts/` and the user asks for a final review, copyedit, proofreading, or link check.
disable-model-invocation: true
---

# final-check-post

## Purpose

Run a publish-ready pass on a markdown post with two goals:

1. Correct grammar and minor stylistic issues without changing meaning or voice.
2. Ensure links are complete and validly formed, with no missing references.

## Scope

- Target files: markdown posts under `_posts/` or `_drafts/`.
- Keep edits minimal and surgical.
- Do not rewrite structure unless the user asks for broader edits.

## Workflow

1. Read the target post.
2. Scan for unresolved Rust placeholders:
   - `[crate-link?]`
   - `[doc-link?]`
3. Resolve placeholders as inline links:
   - `[crate-link?]` -> `https://docs.rs/<crate-name>/latest/<module_path>/`
   - `[doc-link?]` -> `https://doc.rust-lang.org/std/...`
4. If a `[doc-link?]` target is ambiguous, ask the user before editing.
5. Copyedit prose for:
   - grammar and punctuation
   - awkward phrasing and readability
   - minor consistency issues (for example, article use, prepositions, and hyphenation)
6. Verify link integrity:
   - no unresolved placeholders remain
   - every reference-style link used in body (`[text][label]`) has a matching definition (`[label]: ...`)
   - no obviously malformed markdown links
7. Run lints for changed files and fix issues introduced by edits when practical.
8. Report back with:
   - what was changed
   - whether missing links/placeholders were found
   - any remaining warnings intentionally left unchanged

## Editing rules

- Preserve technical accuracy and the author's tone.
- Prefer concise wording changes over large rewrites.
- Keep existing markdown style (inline vs reference links) unless a placeholder must be replaced.
- Do not touch unrelated files.

## Output expectations

Provide a short completion summary:

- grammar/style pass status
- link check status
- notable remaining risks (if any)

## Additional resources

- For concrete invocation patterns, see [examples.md](examples.md).
