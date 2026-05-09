---
name: rust-links
description: Resolve Rust link placeholders in Jekyll blog posts by converting `[crate-link?]` to docs.rs crate links and `[doc-link?]` to Rust standard library documentation links. Use when posts under `_posts/` or `_drafts/` contain either placeholder, or when the user asks to fill in Rust links.
---

# rust-links

Convert Rust link placeholders in Jekyll markdown posts into inline `[text](url)` links.

This skill handles two placeholder types:

- `[crate-link?]` for external crate docs on [docs.rs](https://docs.rs/)
- `[doc-link?]` for Rust standard library docs on [doc.rust-lang.org/std](https://doc.rust-lang.org/std/)

## When to apply

Apply this skill whenever a markdown file in this repo contains either literal placeholder:

- `[crate-link?]`
- `[doc-link?]`

Each placeholder marks a link the author expects the agent to resolve.

## Output conventions

- Use inline markdown links only: `[text](url)`.
- Preserve visible link text exactly (including `**...**`, backticks, and other formatting).
- Do not introduce reference-style definitions (`[label]: url`).
- Only edit placeholder matches exactly; leave unrelated links untouched.

## Placeholder type A: `[crate-link?]`

Resolve to docs.rs crate documentation.

### URL format

```
https://docs.rs/<crate-name>/latest/<module-path>/
```

- `<crate-name>`: crates.io name, preserving hyphens.
- `<module-path>`: same name with hyphens replaced by underscores.

Examples:

- `[**tokio**][crate-link?]` -> `[**tokio**](https://docs.rs/tokio/latest/tokio/)`
- `[async-std][crate-link?]` -> `[async-std](https://docs.rs/async-std/latest/async_std/)`
- ``[`mio`][crate-link?]`` -> ``[`mio`](https://docs.rs/mio/latest/mio/)``

### Fallback

If the crate has no docs.rs page, use:

```
https://crates.io/crates/<crate-name>
```

Mention this fallback to the user.

## Placeholder type B: `[doc-link?]`

Resolve to Rust standard library documentation in `https://doc.rust-lang.org/std/`.

### Resolution targets

- Types/traits/enums/structs/type aliases: canonical type page.
- Free functions: function page.
- Methods: owning type or trait page with method anchor when available.
- Primitive types: primitive docs page.
- Macros: macro docs page.

Examples:

- ``[`Future`][doc-link?]`` -> ``[`Future`](https://doc.rust-lang.org/std/future/trait.Future.html)``
- ``[`Future::poll()`][doc-link?]`` -> ``[`Future::poll()`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll)``
- `[_waker_][doc-link?]` -> `[_waker_](https://doc.rust-lang.org/std/task/struct.Waker.html)`

### Ambiguity rule (mandatory)

If multiple plausible std items match the text, do not guess. Ask the user to clarify before editing.

Common ambiguous cases:

- Same name in different modules (for example `take`, `from`, `new`).
- Bare method name without clear owner.
- Trait method vs inherent method uncertainty.
- Descriptive prose that does not identify a specific std item.

## Workflow

1. Find all `[crate-link?]` and `[doc-link?]` occurrences.
2. Resolve each placeholder according to its type-specific rules.
3. If any `[doc-link?]` is ambiguous, pause and ask for clarification.
4. Rewrite each placeholder pair as an inline link, preserving text exactly.
5. Verify:
   - No unresolved placeholder remains.
   - No new reference-style link definitions were added.
   - Each produced URL opens and matches the intended item.

## Notes

- For `[doc-link?]`, prefer `std` pages over `core`/`alloc` unless the user asks otherwise.
- For `[crate-link?]`, use `/latest/` unless the user explicitly asks to pin a version.
