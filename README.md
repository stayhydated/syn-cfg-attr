# syn-cfg-attr

[![Build Status](https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/github/stayhydated/syn-cfg-attr/graph/badge.svg)](https://codecov.io/github/stayhydated/syn-cfg-attr)
[![Docs](https://docs.rs/syn-cfg-attr/badge.svg)](https://docs.rs/syn-cfg-attr/)
[![Crates.io](https://img.shields.io/crates/v/syn-cfg-attr.svg)](https://crates.io/crates/syn-cfg-attr)

Expand `cfg_attr` so you can treat conditional attributes like regular
attributes while still preserving the guarding `cfg` condition.

This crate gives you:

- Flattening of direct attributes and `cfg_attr`-wrapped attributes in one pass.
- Access to the `cfg` condition for each nested attribute.
- A unified `parse_args` experience for direct and nested list-style attributes.
- A robust token splitter that respects groups and generics.

## Examples

```rust
use syn::{Attribute, parse_quote};
use syn_cfg_attr::{AttributeHelpers, ExpandedAttr};

fn main() -> syn::Result<()> {
    let attrs: Vec<Attribute> = vec![
        parse_quote!(#[serde(default)]),
        parse_quote!(#[cfg_attr(feature = "json", serde(rename = "id"), other)]),
    ];

    for attr in attrs.find_attribute("serde") {
        if let ExpandedAttr::Nested { condition, .. } = &attr {
            println!("guarded by: {}", condition);
        }

        let meta: syn::Meta = attr.parse_args()?;
        println!("serde meta: {:?}", meta);
    }

    Ok(())
}
```

More complete usage is in `examples/usage.rs`.

## Installation

```bash
cargo add syn-cfg-attr
```

Or in `Cargo.toml`:

```toml
[dependencies]
syn-cfg-attr = "0.1"
```

## API at a glance

- `AttributeHelpers::flattened_attributes()` expands `cfg_attr` recursively.
- `AttributeHelpers::find_attribute("name")` filters the flattened list.
- `ExpandedAttr::parse_args<T>()` parses arguments for direct and nested list-style attributes.
- `ExpandedAttr::Nested { condition, original, .. }` exposes the guard and containing `cfg_attr` attribute.

## Notes and limitations

- Conditions are stored as raw `TokenStream` values and are not evaluated.
- Nested `cfg_attr` conditions are not combined; only the immediate condition is stored.
- Nested entries that cannot be parsed as `syn::Meta` are skipped.

## MSRV

Rust 1.88.

## Contributing

See `CONTRIBUTING.md` for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license
