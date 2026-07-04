# syn-cfg-attr

[![Build Status](https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/syn-cfg-attr/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/github/stayhydated/syn-cfg-attr/graph/badge.svg)](https://codecov.io/github/stayhydated/syn-cfg-attr)
[![Docs](https://docs.rs/syn-cfg-attr/badge.svg)](https://docs.rs/syn-cfg-attr/)
[![Crates.io](https://img.shields.io/crates/v/syn-cfg-attr.svg)](https://crates.io/crates/syn-cfg-attr)

Expand `cfg_attr` so you can treat conditional attributes like regular
attributes while still preserving the guarding `cfg` condition.

This crate gives you:

- Flattening of direct attributes and `cfg_attr`-wrapped attributes in one pass.
- Access to the combined `cfg` condition for each nested attribute.
- Parsed `cfg` predicates that can be evaluated with caller-provided option state.
- A unified `parse_args` experience for direct and nested list-style attributes.
- A robust token splitter that respects groups and generics.

## Examples

```rust
use syn::{Attribute, parse_quote};
use syn_cfg_attr::{AttributeHelpers, CfgOption, ExpandedAttr};

fn main() -> syn::Result<()> {
    let attrs: Vec<Attribute> = vec![
        parse_quote!(#[serde(default)]),
        parse_quote!(
            #[cfg_attr(all(feature = "json", not(target_os = "wasm32")), serde(rename = "id"), other)]
        ),
    ];

    for attr in attrs.try_find_attribute("serde")? {
        if let ExpandedAttr::Nested { condition, .. } = &attr {
            println!("guarded by: {}", condition);
        }

        if let Some(condition) = attr.parse_condition()? {
            let enabled = condition.evaluate(|option| match option {
                CfgOption::NameValue { name, value } => {
                    name == "feature" && value.value() == "json"
                }
                CfgOption::Flag(_) => false,
            });
            println!("enabled in this example context: {enabled}");
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
syn-cfg-attr = "0.2"
```

## API at a glance

- `AttributeHelpers::try_flattened_attributes()` expands `cfg_attr` recursively and reports malformed nested entries.
- `AttributeHelpers::try_find_attribute("name")` filters the fallible flattened list.
- `AttributeHelpers::flattened_attributes()` and `find_attribute("name")` provide best-effort versions that skip malformed nested entries.
- `ExpandedAttr::parse_args<T>()` parses arguments for direct and nested list-style attributes.
- `ExpandedAttr::parse_condition()` parses nested guards as `CfgPredicate`.
- `CfgPredicate::evaluate()` evaluates parsed guards with caller-provided `cfg` option lookup.
- `ExpandedAttr::Nested { condition, original, .. }` exposes the combined guard and containing `cfg_attr` attribute.

## Notes

- Nested conditions are stored as raw `TokenStream` values and combined with `all(...)`.
- Condition evaluation uses the `cfg` option lookup supplied to `CfgPredicate::evaluate()`.
- Best-effort helpers skip malformed nested entries.
