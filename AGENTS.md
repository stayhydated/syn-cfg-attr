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
There is no standalone architecture document in this workspace.

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
   `AGENTS.md`, `CONTRIBUTING.md`, or `CLAUDE.md`.
1. **Sync public workflow changes.** If API behavior, parsing semantics,
   examples, limitations, or MSRV changes, update the README, example,
   rustdoc, and contributor guidance in the same change when applicable.
1. **Validate narrowly.** Run the smallest command that proves the edited
   behavior or documentation surface is still sound.

## Audience Labels

These labels describe the surface itself, not the documentation file being
edited:

- **User-facing**: normal entry points for crate consumers and macro authors.
- **Public integration**: public APIs intended to be composed into procedural macro or code-generation crates.
- **Internal**: implementation details, tests, examples-as-maintenance, and contributor tooling.

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
- what limitations remain.

Keep user-facing documentation example-first. Prefer compact Rust snippets over
prose-only explanations.

### Internal and Contributor Documentation

Treat these surfaces as contributor-facing rather than user-facing:

- `AGENTS.md`,
- `CONTRIBUTING.md`,
- `CLAUDE.md`.

Use them for workspace conventions, review expectations, and validation rules.
Do not use them as substitutes for user docs.

Keep implementation detail close to the implementation:

- use rustdoc for public API behavior and limitations,
- use focused comments for non-obvious local code,
- use tests for parsing and token-splitting edge cases,
- do not add standalone architecture docs unless the project intentionally
  restores that documentation surface.

## Synchronization Rules

When a substantive change modifies public API behavior, parsing semantics,
`cfg_attr` expansion behavior, limitations, examples, or MSRV:

1. Update `README.md` when user-facing behavior or positioning changes.
1. Update `examples/usage.rs` when usage patterns or recommended APIs change.
1. Update rustdoc when public items or edge cases need clearer API documentation.
1. Update tests when expansion, parsing, or token-splitting behavior changes.
1. Update `CONTRIBUTING.md`, `AGENTS.md`, or `CLAUDE.md` when contributor workflow or workspace rules changed.
1. Keep these surfaces aligned in the same change unless there is a documented reason not to.

## Workspace Map

### Main User-Facing Entry Points

- `src`
  Audience: **User-facing**
  Role: crate source exposing `ExpandedAttr` and `AttributeHelpers` for flattening and parsing `cfg_attr` while keeping span and condition context.

- `src/splitter.rs`
  Audience: **Internal**
  Role: token splitting helper used by `cfg_attr` expansion; changes here should be validated with focused splitter tests plus any affected expansion tests.

- `README.md`
  Audience: **User-facing**
  Role: top-level introduction, installation instructions, API overview, examples, limitations, and MSRV.

- `examples/usage.rs`
  Audience: **User-facing**
  Role: runnable usage example for direct and nested attribute expansion.

### Internal and Tooling Surfaces

- `justfile`
  Audience: **Internal**
  Role: formatting, linting, checking, testing, docs, and publish dry-run workflow.

- `CONTRIBUTING.md`
  Audience: **Internal**
  Role: contributor expectations and validation guidance.

- `CLAUDE.md`
  Audience: **Internal**
  Role: companion contributor guidance for Claude-based coding agents.

- `AGENTS.md`
  Audience: **Internal**
  Role: workspace map and documentation placement rules for coding agents and contributors.

## Validation and Editing Rules

### Validation After Changes

- Validation is the default after code or workflow changes.
- Run the narrowest command that proves the edited behavior works for the
  affected API, docs, example, or tooling surface.
- Prefer targeted `cargo test`, `cargo check`, example, or docs checks before broader validation.
- Use `just check`, `just test`, or a more specific `justfile` recipe when the change spans multiple surfaces.
- If validation cannot be run, state why and what remains unvalidated.
- Do not claim a change works unless it was validated, generated from a source of truth, or the remaining risk is explicitly documented.

### When Editing Docs

- Keep the README and example user-facing.
- Keep implementation detail close to code, rustdoc, or focused tests.
- Prefer Rust snippets over prose-only explanations.
- Sync `README.md`, `examples/usage.rs`, and rustdoc when public usage patterns change.
- Update `AGENTS.md`, `CONTRIBUTING.md`, or `CLAUDE.md` when contributor workflow or repo conventions change.

### When Editing Rust Code

- Use `cargo` for build, test, and run tasks.
- Keep dependency versions in `Cargo.toml`.
- Preserve the public API shape unless the task explicitly changes it.
- Keep code within the declared MSRV in `Cargo.toml`.
- Treat `cfg_attr` condition tokens as preserved syntax, not evaluated configuration.

### When Editing Attribute Expansion or Parsing Behavior

- Keep direct attributes and nested `cfg_attr` attributes behaviorally aligned where the public API promises a unified parsing experience.
- Preserve condition context for nested attributes.
- Be explicit when conditions are intentionally not combined or evaluated.
- Add focused tests or examples for token-splitting edge cases involving groups, generics, name-value attributes, or nested attributes.

### When Writing Tests

- Prefer focused tests near the parsing or expansion behavior being changed.
- Use readable multiline Rust snippets over heavily escaped single-line literals.
- Include direct and `cfg_attr`-wrapped versions when a behavior should be consistent across both paths.

### When Editing Formatting or Tooling

- Use the existing `justfile` recipes for formatting and validation.
- `just fmt` runs `cargo sort-derives`, `cargo fmt`, `taplo fmt`, and `uvx mdformat .`.
