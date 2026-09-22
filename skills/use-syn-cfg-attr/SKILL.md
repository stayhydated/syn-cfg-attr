---
name: use-syn-cfg-attr
description: >
  Integrate syn-cfg-attr into Rust procedural macros, derives, source analyzers,
  and code generators. Use when expanding conditional attributes, preserving or
  evaluating their guards, or choosing diagnostics in consumer code. Excludes
  generic syn parsing and maintenance of syn-cfg-attr itself.
---

# Use syn-cfg-attr

Locate the consumer's `Vec<syn::Attribute>` and import
`syn_cfg_attr::AttributeHelpers`. `syn-cfg-attr` 0.3 uses `syn` 3 and requires
Rust 1.98. Check the consumer's versions before changing an integration; its
attribute types must come from the same major `syn` version.

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
path rules. Fallible filtering expands the entire vector first, so malformed
nested metadata can fail the call even when it has another attribute name.

## Expand and parse attributes

Use fallible filtered expansion for the common procedural-macro path:

```rust
use proc_macro2::TokenStream;
use syn::{Attribute, Meta};
use syn_cfg_attr::AttributeHelpers;

fn parse_serde(attrs: &Vec<Attribute>) -> syn::Result<Vec<(Meta, Option<TokenStream>)>> {
    attrs
        .try_find_attribute("serde")?
        .into_iter()
        .map(|attr| {
            let condition = attr.condition().cloned();
            let args = attr.parse_args::<Meta>()?;
            Ok((args, condition))
        })
        .collect()
}
```

Treat `parse_args<T>()` as a list-attribute operation. Path-only and name-value
metadata return an error. This example accepts one `Meta` per argument list;
use the consumer's `Parse` implementation for a different grammar.

Match `ExpandedAttr` when the consumer needs the direct `Attribute`, nested
`Meta`, or containing `cfg_attr`. During recursive expansion, `original` can be
the immediate nested wrapper rather than the outer source attribute; use the
combined `condition` for the complete guard.

## Preserve or evaluate conditions

Prefer the raw condition when forwarding syntax or attaching a later guard:

```rust
use proc_macro2::TokenStream;
use quote::quote;
use syn_cfg_attr::ExpandedAttr;

fn cfg_guard(expanded: &ExpandedAttr) -> Option<TokenStream> {
    expanded.condition().map(|condition| quote!(#[cfg(#condition)]))
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
macro. Evaluation short-circuits `all` and `any`, so the callback must answer
configuration queries without relying on every option being visited.

## Propagate diagnostics

Fallible expansion reports a non-list `cfg_attr` or a nested entry that cannot
parse as `syn::Meta`. It preserves condition tokens for later parsing and treats
an empty `cfg_attr()` list as producing no entries. Apply any additional
consumer syntax requirements separately.

Propagate `syn::Error` from expansion, `parse_args`, and `parse_condition`
through the consumer's existing diagnostic path.

## Verify consumer changes

Use the consumer's existing checks. Include both direct and `cfg_attr`-wrapped
input in focused tests when the consumer promises uniform parsing. Add a nested
`cfg_attr` case when combined conditions affect generated output or evaluation.
