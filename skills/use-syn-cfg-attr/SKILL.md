---
name: use-syn-cfg-attr
description: >
  Use when writing, reviewing, or documenting Rust procedural macros, derives,
  source analyzers, and code generators that consume syn-cfg-attr. Covers
  recursive cfg_attr expansion from syn attribute vectors, identifier filtering,
  uniform list-argument parsing, preserved nested guards, caller-owned
  CfgPredicate evaluation, and fallible versus best-effort discovery. Excludes
  generic syn parsing unrelated to this crate and maintenance of syn-cfg-attr
  itself.
---

# Use syn-cfg-attr

## Apply the workflow

1. Locate the consumer-owned `Vec<syn::Attribute>` and import
   `syn_cfg_attr::AttributeHelpers`.
2. Decide whether one malformed nested entry must become a diagnostic or may be
   skipped deliberately.
3. Expand all attributes or filter one identifier with the corresponding
   method below.
4. Parse list arguments through `ExpandedAttr::parse_args<T>()` so direct and
   nested forms share one code path.
5. Preserve raw guards with `condition()`, or parse them with
   `parse_condition()` when structured inspection is required.
6. Evaluate parsed guards only against configuration state supplied by the
   consumer.

Add the dependency with `cargo add syn-cfg-attr` when the consumer does not
already declare it.

## Choose expansion behavior

| Required behavior | Method |
|---|---|
| Expand all and report malformed nested entries | `try_flattened_attributes()` |
| Expand parseable entries and skip malformed nested entries | `flattened_attributes()` |
| Filter one identifier and report malformed nested entries | `try_find_attribute(name)` |
| Filter one identifier and skip malformed nested entries | `find_attribute(name)` |

Prefer a `try_*` method for procedural-macro input. Use a best-effort method
only when partial discovery is an explicit product decision.

The filtered methods use single-identifier matching. Use an expansion method
and filter `ExpandedAttr::path()` yourself for qualified or consumer-defined
path rules.

## Expand and parse attributes

Use fallible filtered expansion for the common procedural-macro path:

```rust
use syn::{Attribute, Meta};
use syn_cfg_attr::AttributeHelpers;

fn parse_serde(attrs: &Vec<Attribute>) -> syn::Result<Vec<(Meta, Option<String>)>> {
    attrs
        .try_find_attribute("serde")?
        .into_iter()
        .map(|attr| {
            let condition = attr.condition().map(ToString::to_string);
            let args = attr.parse_args::<Meta>()?;
            Ok((args, condition))
        })
        .collect()
}
```

Treat `parse_args<T>()` as a list-attribute operation. Path-only and name-value
metadata do not contain list arguments and return an error.

Match `ExpandedAttr` when the consumer needs the direct `Attribute`, nested
`Meta`, or containing `cfg_attr`. During recursive expansion, `original` can be
the immediate nested wrapper rather than the outer source attribute; use the
combined `condition` for the complete guard.

## Preserve or evaluate conditions

Prefer the raw condition when forwarding syntax or attaching a later guard:

```rust
if let Some(condition) = expanded.condition() {
    generated.extend(quote::quote!(#[cfg(#condition)]));
}
```

Parse and evaluate only with an explicit cfg source. Treat direct attributes as
unconditionally enabled:

```rust
use syn_cfg_attr::{CfgOption, ExpandedAttr};

fn is_enabled(
    expanded: &ExpandedAttr,
    mut cfg_enabled: impl FnMut(CfgOption<'_>) -> bool,
) -> syn::Result<bool> {
    Ok(match expanded.parse_condition()? {
        Some(predicate) => predicate.evaluate(&mut cfg_enabled),
        None => true,
    })
}
```

Back `cfg_enabled` with the configuration for the code being inspected. Do not
infer target, feature, or custom cfg state from the host running a procedural
macro.

## Propagate diagnostics

Fallible expansion reports a non-list `cfg_attr` or a nested entry that cannot
parse as `syn::Meta`. It preserves condition tokens without validating them.
Propagate `syn::Error` from expansion, `parse_args`, and `parse_condition`
through the consumer's existing diagnostic path.

## Verify consumer changes

Run the narrowest command that covers the edited integration:

```bash
cargo check -p <consumer-package>
cargo test -p <consumer-package> <focused-test-name>
```

Include both direct and `cfg_attr`-wrapped input in focused tests when the
consumer promises uniform parsing. Add a nested `cfg_attr` case when combined
conditions affect generated output or evaluation.
