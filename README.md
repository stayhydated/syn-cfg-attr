# syn-cfg-attr

[![CI][ci-badge]][ci]
[![Codecov][codecov-badge]][codecov]
[![Book][book-badge]][book]
[![crates.io][crate-badge]][crate]

`syn-cfg-attr` recursively expands `cfg_attr` entries from
`Vec<syn::Attribute>`. Procedural macros and code generators can inspect direct
and conditional attributes through one API without losing their guard
conditions.

## Example

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

`try_find_attribute` expands recursively, filters by one attribute identifier,
and reports malformed nested entries as `syn::Error`. The returned
`ExpandedAttr` values expose preserved conditions and parse list arguments for
both direct and nested attributes.

Nested guards combine as `all(parent, child)`. Use `condition()` to forward
the guard as Rust tokens, or `parse_condition()` and `CfgPredicate::evaluate`
to evaluate it against configuration supplied by your tool.

The [complete example](examples/usage.rs) shows recursive expansion,
condition evaluation, and error handling.

[ci-badge]: https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml/badge.svg?branch=master
[ci]: https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml
[codecov-badge]: https://codecov.io/github/stayhydated/syn-cfg-attr/graph/badge.svg
[codecov]: https://codecov.io/github/stayhydated/syn-cfg-attr
[book-badge]: https://img.shields.io/badge/book-online-blue
[book]: https://stayhydated.github.io/syn-cfg-attr/book/
[crate-badge]: https://img.shields.io/crates/v/syn-cfg-attr.svg
[crate]: https://crates.io/crates/syn-cfg-attr
