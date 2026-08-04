# AGENTS.md

This is the working guide for contributors and coding agents in the
`syn-cfg-attr` workspace.

Use it to decide:

1. where documentation belongs,
1. whether a surface is user-facing, public integration, or internal,
1. which related docs and examples must change together,
1. which validation command should run before handoff.

For most code changes, start with `src/`.

For usage guidance, start with `README.md` and `examples/usage.rs`.
For implementation behavior, use `src/` and focused tests.
For repo commands, start with `just --list`; `justfile` is the local command index.

## Project Summary

`syn-cfg-attr` is a focused Rust attribute-expansion helper built on `syn`.

Its priorities are:

1. **`cfg_attr` expansion**: treat conditional attributes as if they were written directly.
1. **Condition preservation**: keep the guarding `cfg` predicate alongside each expanded attribute.
1. **Ergonomics**: provide helpers that mirror `syn` parsing for both direct and nested attributes.

## Quick Decision Flow

Before editing, classify the change:

1. **Find the surface in the workspace map.** Use its audience label to decide
   how much public explanation the change needs.
1. **Place documentation by content, not by surface audience.** The README,
   public rustdoc, and example are user-facing. Implementation details belong
   near the code, tests, or concise rustdoc. Contributor conventions belong in
   `AGENTS.md`.
1. **Sync public workflow and Rust compatibility changes.** If API behavior,
   parsing semantics, examples, documented behavior boundaries, the Cargo
   edition, or the declared `rust-version` changes, update the README, example,
   rustdoc, tests, and contributor guidance in the same change when applicable.
1. **Validate narrowly.** Run the smallest command that proves the edited
   behavior or documentation surface is still sound.

## Audience Labels

These labels describe the surface itself, not the documentation file being
edited:

- **User-facing**: normal entry points for crate consumers and macro authors.
- **Public integration**: public APIs intended to be composed into procedural macro or code-generation crates.
- **Internal**: implementation details, tests, and contributor tooling.

## Documentation Placement

### User-Facing Documentation

Treat these surfaces as user-facing:

- `README.md`,
- public API rustdoc,
- `examples/usage.rs`.

User-facing documentation should explain:

- who the crate is for,
- what `cfg_attr` behavior is expanded,
- what condition context is preserved,
- which behavior boundaries users need to account for.

Keep user-facing documentation example-first. Prefer compact Rust snippets over
prose-only explanations.

### Internal and Contributor Documentation

Treat these surfaces as contributor-facing rather than user-facing:

- `AGENTS.md`.

Use them for workspace conventions, review expectations, and validation rules.
Do not use them as substitutes for user docs.

Keep implementation detail close to the implementation:

- use rustdoc for public API behavior and behavior boundaries,
- use focused comments for non-obvious local code,
- use tests for parsing and token-splitting edge cases.

## Synchronization Rules

When a substantive change modifies public API behavior, parsing semantics,
`cfg_attr` expansion behavior, documented behavior boundaries, examples,
the Cargo edition, or the declared `rust-version`:

1. Update `README.md` when user-facing behavior or positioning changes.
1. Update `examples/usage.rs` when usage patterns or recommended APIs change.
1. Update rustdoc when public items or edge cases need clearer API documentation.
1. Update tests when expansion, parsing, or token-splitting behavior changes.
1. Keep `.rustfmt.toml`'s `edition` setting aligned when the Cargo edition changes.
1. Update `AGENTS.md` when contributor workflow or workspace rules changed.
1. Keep book chapters and `skills/use-syn-cfg-attr/` aligned when public usage
   guidance or recommended API choices change.
1. Keep `web/`, `xtask/`, the three shared dependency revisions, Pages
   workflows, and root web recipes aligned when site assembly changes.
1. Keep these surfaces aligned in the same change unless there is a documented reason not to.

## Workspace Map

### Crate and Documentation Surfaces

- `src/lib.rs`
  Audience: **Public integration**
  Role: crate root, rustdoc, public exports, and API-level tests for flattening and parsing `cfg_attr` while keeping span and condition context.

- `src/splitter.rs`
  Audience: **Internal**
  Role: token splitting helper used by `cfg_attr` expansion; changes here should be validated with focused splitter tests plus any affected expansion tests.

- `README.md`
  Audience: **User-facing**
  Role: top-level introduction, installation instructions, API overview, examples, and behavior notes.

- `examples/usage.rs`
  Audience: **User-facing**
  Role: runnable usage example for direct and nested attribute expansion.

- `book/`
  Audience: **User-facing**
  Role: task-oriented mdBook documentation for adopting the crate, expanding
  attributes, evaluating preserved conditions, and choosing error behavior.

- `skills/use-syn-cfg-attr/`
  Audience: **Public integration**
  Role: reusable coding-agent guidance for applying the public API in syn-based
  procedural macros and code-generation tools.

- `web/`
  Audience: **User-facing**
  Role: demo-less single-page Dioxus project portal, project-owned
  destinations, and route manifest. Shared owns its generic styles and Pages
  assembly assets.

- `xtask/`
  Audience: **Internal**
  Role: repository commands that build the book, llms outputs, and Pages site,
  and preview the assembled static artifact.

- `.github/workflows/gh-pages.yml`
  Audience: **Internal**
  Role: GitHub Pages build and deployment through the shared reusable workflow.

- `.github/workflows/update-shared-revisions.yml`
  Audience: **Internal**
  Role: scheduled shared-revision update workflow for the three synchronized
  stayhydated dependencies.

### Internal Guidance

- `AGENTS.md`
  Audience: **Internal**
  Role: workspace map and documentation placement rules for coding agents and contributors.

## Validation and Editing Rules

### Validation After Changes

- Validation is the default after code or workflow changes.
- Run the narrowest command that proves the edited behavior works for the
  affected API, docs, example, or tooling surface.
- Prefer targeted `cargo test`, `cargo check`, example, or docs checks before broader validation.
- Use `just --list` to inspect repository recipes; use `just check`,
  `just test`, or a more specific `justfile` recipe when the change spans
  multiple surfaces.
- Use `just test-docs` for README, rustdoc, or example documentation changes,
  and `just test-publish` for package metadata or publishability-sensitive
  changes.
- Use `cargo xtask build book` for mdBook source changes and `just web-build`
  for site, sitemap, generated llms, or Pages assembly changes.
- Validate `skills/use-syn-cfg-attr/` with the skill-creator
  `scripts/quick_validate.py` helper after changing its frontmatter or guidance.
- Use `cargo test -p web --lib --locked` for the consumer-owned portal and
  route-manifest contract without a browser build.
- CI runs formatting checks, locked Rust tests, clippy, docs, package dry-run,
  cargo-machete, coverage, and Codecov publishing from `.github/workflows/ci.yml`.
- If validation cannot be run, state why and what remains unvalidated.
- Do not claim a change works unless it was validated or the remaining risk is explicitly documented.

### When Editing Docs

- Keep the README and example user-facing.
- Keep implementation detail close to code, rustdoc, or focused tests.
- Prefer Rust snippets over prose-only explanations.
- Sync `README.md`, `examples/usage.rs`, and rustdoc when public usage patterns change.
- Update `AGENTS.md` when contributor workflow or repo conventions change.

### When Editing Rust Code

- Treat the current public API shape as the contract; when it changes, update
  the README, examples, rustdoc, and tests with the new shape.
- Keep code within the declared `rust-version` in `Cargo.toml`.
- Treat `cfg_attr` condition tokens as preserved syntax. Use `CfgPredicate::evaluate`
  with caller-provided `cfg` state when behavior depends on evaluation.

### When Editing Attribute Expansion or Parsing Behavior

- Keep direct attributes and nested `cfg_attr` attributes behaviorally aligned where the public API promises a unified parsing experience.
- Preserve combined condition context for nested attributes.
- Keep `CfgPredicate` parsing and evaluation aligned with condition-token preservation.
- Add focused tests or examples for token-splitting edge cases involving groups, generics, name-value attributes, or nested attributes.

### When Writing Tests

- Prefer focused tests near the parsing or expansion behavior being changed.
- Use readable multiline Rust snippets over heavily escaped single-line literals.
- Include direct and `cfg_attr`-wrapped versions when a behavior should be consistent across both paths.

### When Editing Formatting or Tooling

- Use the existing `justfile` recipes for formatting and validation.
- `just fmt` runs `cargo sort-derives`, `cargo fmt`, `taplo fmt`, and `rumdl fmt .`.
