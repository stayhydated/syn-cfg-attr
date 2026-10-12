# syn-cfg-attr

[![CI][ci-badge]][ci]
[![Codecov][codecov-badge]][codecov]
[![Book][book-badge]][book]
[![crates.io: syn-cfg-attr][crate-badge]][crate]

`syn-cfg-attr` recursively expands `cfg_attr` entries from
`Vec<syn::Attribute>`. Procedural macros and code generators can inspect direct
and conditional attributes through one API without losing their guard
conditions.

Requires Rust 1.99 or newer. Attribute values must come from `syn` 3, matching
the crate's parser dependency. See the [API reference][api] for expansion and
predicate semantics.

## Example

This self-contained example filters direct and conditional `serde` attributes
without evaluating their guards, then parses each list argument as `syn::Meta`.

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

Use `condition()` to forward preserved guards as Rust tokens, or
`parse_condition()` and `CfgPredicate::evaluate` with configuration supplied by
your tool. The best-effort `find_attribute` and `flattened_attributes` helpers
skip malformed nested entries; prefer the fallible helpers when diagnostics
matter.

The [complete example][example] shows recursive expansion,
condition evaluation, and error handling.

[ci-badge]: https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml/badge.svg?branch=master
[ci]: https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml
[codecov-badge]: https://codecov.io/github/stayhydated/syn-cfg-attr/graph/badge.svg?branch=master
[codecov]: https://codecov.io/github/stayhydated/syn-cfg-attr
[book-badge]: https://img.shields.io/badge/Book-mdBook-blue
[book]: https://stayhydated.github.io/syn-cfg-attr/book/
[crate-badge]: https://img.shields.io/crates/v/syn-cfg-attr.svg?label=syn-cfg-attr
[crate]: https://crates.io/crates/syn-cfg-attr
[example]: https://github.com/stayhydated/syn-cfg-attr/blob/master/examples/usage.rs
[api]: https://docs.rs/syn-cfg-attr/latest/syn_cfg_attr/
