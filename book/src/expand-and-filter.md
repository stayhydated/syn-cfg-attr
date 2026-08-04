# Expand and filter attributes

Choose an expansion method based on whether malformed nested entries should
become a diagnostic and whether you need every attribute or one identifier.

| Goal | Method |
|---|---|
| Expand every attribute and report malformed nested entries | `try_flattened_attributes()` |
| Expand every parseable attribute and skip malformed nested entries | `flattened_attributes()` |
| Expand, filter by one identifier, and report malformed nested entries | `try_find_attribute(name)` |
| Expand, filter by one identifier, and skip malformed nested entries | `find_attribute(name)` |

Prefer a fallible method for procedural-macro input so invalid nested metadata
can become a compiler diagnostic:

```rust
use syn::Attribute;
use syn_cfg_attr::{AttributeHelpers, ExpandedAttr};

fn serde_attributes(attrs: Vec<Attribute>) -> syn::Result<Vec<ExpandedAttr>> {
    attrs.try_find_attribute("serde")
}
```

The two `find_attribute` methods use `ExpandedAttr::is_ident`, so they match a
single-segment path such as `serde`. Expand first and filter with
`ExpandedAttr::path()` when your integration accepts qualified or custom paths.

## Understand recursive expansion

Expansion is recursive. For this input:

```text
#[cfg_attr(
    feature = "serde",
    cfg_attr(target_os = "linux", serde(default))
)]
```

the nested `serde` attribute has the combined condition:

```text
all(feature = "serde", target_os = "linux")
```

Read the combined guard through `ExpandedAttr::condition()` or the
`ExpandedAttr::Nested` variant. Direct attributes return `None` from
`condition()`.

`ExpandedAttr::Nested` also exposes the inner `syn::Meta` as `attr` and the
containing `cfg_attr` as `original`. During recursive expansion, `original` can
represent the immediate nested `cfg_attr` rather than the outer source
attribute. Use it for local wrapper inspection and diagnostics; keep the
combined `condition` when later behavior depends on the complete guard.
