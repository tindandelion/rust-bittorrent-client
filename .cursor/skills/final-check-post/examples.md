# final-check-post examples

## Example 1: Single post final pass

User prompt:

`Final-check this post: _posts/2026-05-06-intro-to-async-rust.md. Fix grammar and minor style only, and make sure no links are missing.`

Expected behavior:

- Run a light copyedit pass.
- Resolve any `[crate-link?]` / `[doc-link?]` placeholders.
- Verify all reference-style links are defined.
- Report a short summary of edits and link-check status.

## Example 2: Draft before publishing

User prompt:

`Please run final-check-post on _drafts/2026-05-06-going-async-1.md before I publish it.`

Expected behavior:

- Keep author voice and structure.
- Fix grammar, punctuation, and awkward phrasing.
- Confirm no unresolved placeholders or missing link definitions remain.

## Example 3: Minimal edits only

User prompt:

`Use final-check-post on _posts/2026-04-10-non-blocking-request-file.md, but keep edits minimal and avoid rewording technical paragraphs unless grammar is wrong.`

Expected behavior:

- Make only surgical wording fixes.
- Avoid broad rewrites.
- Validate links and placeholders as usual.

## Example 4: Batch check multiple posts

User prompt:

`Run final-check-post on these files: _posts/2026-05-06-intro-to-async-rust.md and _drafts/2026-05-06-going-async-1.md.`

Expected behavior:

- Process each file in turn with the same checklist.
- Provide a per-file summary of edits and link status.
- Call out any ambiguity that requires user clarification.
