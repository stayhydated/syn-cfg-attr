# Handle malformed attributes

Use a `try_*` method when malformed nested metadata should stop expansion and
produce a `syn::Error`. This is the usual choice for procedural macros because
the error can become a compiler diagnostic.

Fallible expansion requires list-style `cfg_attr` syntax and parses each nested
entry as `syn::Meta`. It preserves guard tokens for later parsing with
`ExpandedAttr::parse_condition()`.

```rust
use syn::{Attribute, parse_quote};
use syn_cfg_attr::AttributeHelpers;

let attrs: Vec<Attribute> = vec![parse_quote!(
    #[cfg_attr(feature = "broken", serde + invalid, serde(default))]
)];

assert!(attrs.try_flattened_attributes().is_err());
```

## Keep parseable entries deliberately

Use the best-effort methods when the caller explicitly wants partial discovery.
They skip malformed nested entries and non-list `cfg_attr` attributes, retaining
the other parseable entries:

```rust
# use syn::{Attribute, parse_quote};
# use syn_cfg_attr::AttributeHelpers;
# let attrs: Vec<Attribute> = vec![parse_quote!(
#     #[cfg_attr(feature = "broken", serde + invalid, serde(default))]
# )];
let serde_attrs = attrs.find_attribute("serde");
assert_eq!(serde_attrs.len(), 1);
```

## Handle later parsing errors

Argument and condition parsing remain fallible after expansion:

- `ExpandedAttr::parse_args<T>()` reports a non-list attribute or invalid `T`.
- `ExpandedAttr::parse_condition()` reports a preserved condition that is not a
  supported `CfgPredicate`.

Expansion is not a complete Rust attribute validator. For example, both
expansion modes produce an empty result for `#[cfg_attr()]`. Validate any
additional syntax requirements in your consumer.

Propagate these errors through the same diagnostic path as fallible expansion.
For full signatures, see the
[`syn-cfg-attr` API documentation](https://docs.rs/syn-cfg-attr/).
