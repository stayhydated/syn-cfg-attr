# Handle malformed attributes

Use a `try_*` method when malformed nested metadata should stop expansion and
produce a `syn::Error`. This is the usual choice for procedural macros because
the error can become a compiler diagnostic.

Fallible expansion validates the structure it consumes: a `cfg_attr` must be a
list attribute, and each nested entry must parse as `syn::Meta`. Guard tokens
are preserved during expansion and are parsed separately by
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
They skip nested entries that cannot be parsed as `syn::Meta` and retain the
other parseable entries:

```rust
# use syn::{Attribute, parse_quote};
# use syn_cfg_attr::AttributeHelpers;
# let attrs: Vec<Attribute> = vec![parse_quote!(
#     #[cfg_attr(feature = "broken", serde + invalid, serde(default))]
# )];
let serde_attrs = attrs.find_attribute("serde");
assert_eq!(serde_attrs.len(), 1);
```

Best-effort expansion discards malformed entries, so use it only when partial
results are part of the caller's intended behavior rather than a way to hide a
user error.

## Handle later parsing errors

Argument and condition parsing remain fallible after expansion:

- `ExpandedAttr::parse_args<T>()` reports a non-list attribute or invalid `T`.
- `ExpandedAttr::parse_condition()` reports a preserved condition that is not a
  supported `CfgPredicate`.

Propagate these errors through the same diagnostic path as fallible expansion.
For full signatures, see the
[`syn-cfg-attr` API documentation](https://docs.rs/syn-cfg-attr/).
