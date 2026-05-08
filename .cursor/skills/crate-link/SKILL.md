---
name: crate-link
description: Resolve `[crate-link?]` placeholders in Jekyll blog posts into inline links pointing to docs.rs. Use when a post under `_posts/` or `_drafts/` contains `[crate-link?]` markers next to Rust crate names, or when the user asks to fill in crate links.
---

# crate-link

Convert `[crate-link?]` placeholders in Jekyll markdown posts into inline `[text](url)` links pointing to [docs.rs](https://docs.rs/).

## When to apply

Apply this skill whenever a markdown file in this repo contains the literal string `[crate-link?]`. Typical occurrences look like:

```markdown
* [**tokio**][crate-link?] - the most popular general-purpose runtime...
* [**async-std**][crate-link?] - an alternative runtime...
* [**smol**][crate-link?] (and related ecosystem crates)...
```

Each `[crate-link?]` is a placeholder the author left for the agent to fill in.

## Conventions in this repo

Crate links use the inline markdown form `[text](url)` and point to `docs.rs`:

```
https://docs.rs/<crate-name>/latest/<module-path>/
```

- `<crate-name>` is the crate name as published on crates.io, **with hyphens preserved** (e.g. `async-std`, `pin-project`, `tokio`, `serde_bytes`).
- `<module-path>` is the same name **with every hyphen replaced by an underscore** (e.g. `async_std`, `pin_project`, `tokio`, `serde_bytes`). Rust module names cannot contain hyphens, so docs.rs serves the inner path under the underscored form.

For crates whose names contain no hyphens, the two parts are identical (e.g. `https://docs.rs/tokio/latest/tokio/`). For hyphenated crates they differ (e.g. `https://docs.rs/async-std/latest/async_std/`).

Do **not** introduce reference-style links (`[label]: url` at the bottom of the file) when resolving these placeholders. Keep the link inline.

## Workflow

1. **Find placeholders.** Search the target file for `[crate-link?]`.

2. **Extract the crate name.** For each placeholder, the crate name is the text inside the `[...]` immediately preceding `[crate-link?]`. Strip any inline markdown:
   - `[**tokio**][crate-link?]` → crate name `tokio`
   - `` [`mio`][crate-link?] `` → crate name `mio`
   - `[async-std][crate-link?]` → crate name `async-std`

   If the preceding text is not a single bare crate name (e.g. it's a phrase like "the runtime"), pause and ask the user which crate it should point to instead of guessing.

3. **Rewrite as an inline link.** Replace the whole `[text][crate-link?]` pair with a single inline link `[text](https://docs.rs/<crate-name>/latest/<module-path>/)`, where `<module-path>` is `<crate-name>` with every hyphen replaced by an underscore (see Conventions). Preserve the original visible text exactly, including `**` or backtick formatting:

   ```
   [**tokio**][crate-link?]   →   [**tokio**](https://docs.rs/tokio/latest/tokio/)
   [`mio`][crate-link?]       →   [`mio`](https://docs.rs/mio/latest/mio/)
   [async-std][crate-link?]   →   [async-std](https://docs.rs/async-std/latest/async_std/)
   [pin-project][crate-link?] →   [pin-project](https://docs.rs/pin-project/latest/pin_project/)
   ```

4. **Verify.** After editing:
   - Confirm zero remaining occurrences of `[crate-link?]` in the file.
   - Confirm no stray reference definitions (`[crate-...]: ...`) were added.
   - Leave any unrelated existing reference-style links in the file untouched.
   - Check that the link is working: try to open the webpage by the link URL and make sure it references the correct crate.

## Example

**Before** (`_posts/2026-05-06-going-async.md`):

```markdown
* [**tokio**][crate-link?] - the most popular general-purpose runtime...
* [**async-std**][crate-link?] - an alternative runtime...
* [**smol**][crate-link?] (and related ecosystem crates)...
```

**After:**

```markdown
* [**tokio**](https://docs.rs/tokio/latest/tokio/) - the most popular general-purpose runtime...
* [**async-std**](https://docs.rs/async-std/latest/async_std/) - an alternative runtime...
* [**smol**](https://docs.rs/smol/latest/smol/) (and related ecosystem crates)...
```

Note how `async-std` becomes `async_std` in the inner module path, while the outer crate-name segment keeps the hyphen.

## Notes and edge cases

- **Crate name has underscores vs hyphens.** The outer `<crate-name>` segment of the URL keeps the spelling from crates.io (so `serde_bytes` stays `serde_bytes` and `async-std` stays `async-std`). The inner `<module-path>` segment always uses underscores (so `async-std` becomes `async_std` there). See Conventions for the full rule.
- **Crate not on docs.rs.** Rare, but if docs.rs has no page (e.g. a non-published or renamed crate), fall back to `https://crates.io/crates/<name>` and mention this to the user.
- **Versioned links.** Existing posts sometimes pin to a specific version (e.g. `https://docs.rs/serde/1.0.228/...`). Do not pin versions when resolving `[crate-link?]` — always use `/latest/` unless the user explicitly asks for a pinned version.
- **Don't touch other links.** Only modify placeholders that match `[crate-link?]` exactly. Leave existing inline links and other reference labels alone.
