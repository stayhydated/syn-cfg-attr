# syn-cfg-attr

[![Build Status](https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/github/stayhydated/syn-cfg-attr/graph/badge.svg)](https://codecov.io/github/stayhydated/syn-cfg-attr)
[![Docs](https://docs.rs/syn-cfg-attr/badge.svg)](https://docs.rs/syn-cfg-attr/)
[![Crates.io](https://img.shields.io/crates/v/syn-cfg-attr.svg)](https://crates.io/crates/syn-cfg-attr)

`syn-cfg-attr` recursively expands `cfg_attr` entries from
`Vec<syn::Attribute>`. Procedural macros and code generators can inspect direct
and conditional attributes through one API without losing their guard
conditions.

## Install

```bash
cargo add syn-cfg-attr
```

## Use

```rust
use syn::{Attribute, parse_quote};
use syn::Meta;
use syn_cfg_attr::AttributeHelpers;

fn main() -> syn::Result<()> {
    let attrs: Vec<Attribute> = vec![
        parse_quote!(#[serde(default)]),
        parse_quote!(#[cfg_attr(feature = "json", serde(rename = "id"))]),
    ];

    let serde_attrs = attrs.try_find_attribute("serde")?;
    assert_eq!(serde_attrs.len(), 2);
    assert_eq!(
        serde_attrs
            .iter()
            .filter(|attr| attr.condition().is_some())
            .count(),
        1
    );

    for attr in serde_attrs {
        let _: Meta = attr.parse_args()?;
    }

    Ok(())
}
```

`try_find_attribute` expands recursively, filters by one attribute identifier,
and reports malformed nested entries as `syn::Error`. The returned
`ExpandedAttr` values expose preserved conditions and parse list arguments for
both direct and nested attributes.

## Documentation

- Follow the [user guide](https://stayhydated.github.io/syn-cfg-attr/book/)
  for method selection, condition evaluation, and error handling.
- Use the [API reference](https://docs.rs/syn-cfg-attr/) for exact signatures.
- Run the [complete example](examples/usage.rs) for direct, conditional, and
  recursively nested attributes.
