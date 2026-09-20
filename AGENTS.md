# AGENTS.md

`syn-cfg-attr` expands direct and nested `syn` attributes while preserving their
combined `cfg_attr` guards. Start with `src/lib.rs` for API behavior and
`examples/usage.rs` for consumer usage. Use `just --list` for repository recipes.

## Where to edit

| Surface | Ownership |
| --- | --- |
| `src/lib.rs` | Public integration API, rustdoc, expansion and predicate logic, and API tests. |
| `src/splitter.rs` | Internal comma splitting and focused token tests. |
| `README.md`, `examples/usage.rs` | Consumer introduction and runnable usage. |
| `book/src/` | Consumer tasks: expansion, parsing, conditions, and diagnostics. `SUMMARY.md` owns navigation. |
| `skills/use-syn-cfg-attr/` | Reusable guidance for agents integrating the crate into consumers. |
| `web/src/lib.rs` | Project-owned portal destinations, route manifest, and contract tests. |
| `xtask/src/` | Book, llms, and static-site generation and preview commands. |

## Changes that belong together

- When public parsing, expansion, or predicate behavior changes, update the
  affected rustdoc, API tests, `examples/usage.rs`, README, book chapters, and
  consumer skill in the same change.
- Keep implementation explanations beside the owning code and tests. Use the
  README and book for consumer decisions and examples.
- Preserve combined guards during recursive expansion. `condition()` owns raw
  syntax; `parse_condition()` and `CfgPredicate::evaluate` provide structured
  parsing and evaluation with caller-supplied configuration.
- Cover direct and `cfg_attr`-wrapped input when changing shared parsing behavior.
  For splitter changes, check the focused group, generic, and comma cases plus
  affected expansion tests in `src/lib.rs`.
- When site assembly changes, keep `web/`, `xtask/`, Pages workflows, and the root
  web recipes aligned. The three `stayhydated-*` dependencies in `Cargo.toml`
  share one revision; `.github/workflows/update-shared-revisions.yml` updates
  them together. Shared owns generic site styles and assembly assets.
- Generate published book and llms artifacts from `book/src/` through `xtask`.
  Edit the source chapters and project configuration rather than generated site
  output.

## Validation

Choose the check for the edited surface:

| Change | Check |
| --- | --- |
| Expansion, predicates, or splitter | `cargo test -p syn-cfg-attr --lib --locked` with a focused test filter when useful. |
| Runnable example | `cargo run -p syn-cfg-attr --example usage --locked`. |
| Rustdoc examples | `cargo test -p syn-cfg-attr --doc --locked`. |
| API documentation rendering | `cargo doc -p syn-cfg-attr --no-deps --locked`. `just test-docs` builds workspace documentation and opens it in a browser. |
| Markdown | `rumdl check README.md AGENTS.md book/src skills/use-syn-cfg-attr`. |
| Book | `MDBOOK_BUILD__CREATE_MISSING=false cargo xtask build book`. |
| Consumer skill | Run the skill-creator `scripts/quick_validate.py` helper against `skills/use-syn-cfg-attr/` when available. |
| Portal destinations or route manifest | `cargo test -p web --lib --locked`. |
| Site, sitemap, or llms assembly | `MDBOOK_BUILD__CREATE_MISSING=false just web-build`. |

README and book snippets need their own compilation checks when changed; a
`cargo doc` build renders rustdoc. Use `just check`, `just test`, or `just clippy`
when a change spans the workspace. Report which checks ran and any failed or
unavailable checks.
