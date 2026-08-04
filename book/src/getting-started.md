# Get started

This walkthrough finds `serde` attributes written directly or inside
`cfg_attr`, then parses both forms through the same method.

## Prerequisites

- Rust 1.96 or newer.
- A parser, procedural macro, or code generator that collects attributes in a
  `Vec<syn::Attribute>`.

## Add the dependency

Add `syn-cfg-attr` beside the `syn` dependency used by your project:

```bash
cargo add syn-cfg-attr
```

## Expand and parse attributes

Import `AttributeHelpers`, which is implemented for `Vec<syn::Attribute>`:

```rust
use syn::{Attribute, Meta, parse_quote};
use syn_cfg_attr::AttributeHelpers;

fn main() -> syn::Result<()> {
    let attrs: Vec<Attribute> = vec![
        parse_quote!(#[serde(default)]),
        parse_quote!(#[cfg_attr(feature = "json", serde(rename = "id"))]),
    ];

    let serde_attrs = attrs.try_find_attribute("serde")?;
    assert_eq!(serde_attrs.len(), 2);
    assert!(serde_attrs[0].condition().is_none());
    assert!(serde_attrs[1].condition().is_some());

    for attr in serde_attrs {
        let _: Meta = attr.parse_args()?;
    }

    Ok(())
}
```

`try_find_attribute` expands recursively before filtering. The result contains
both the direct `serde` attribute and the `serde` entry guarded by
`feature = "json"`. A successful run completes both `Meta` parses.

Next, learn how to [choose an expansion method](expand-and-filter.md),
[evaluate preserved conditions](evaluate-conditions.md), or
[handle malformed input](error-handling.md).
