# Evaluate preserved conditions

Call `ExpandedAttr::parse_condition()` to turn a nested attribute's preserved
tokens into `CfgPredicate`. Direct attributes return `Ok(None)` because they
have no guard. Supply the configuration state for the code being inspected when
you evaluate the predicate.

## Parse and evaluate a guard

Evaluation asks your callback whether each visited flag or name-value option is
enabled:

```rust
use syn::{Attribute, parse_quote};
use syn_cfg_attr::{AttributeHelpers, CfgOption};

let attrs: Vec<Attribute> = vec![parse_quote!(
    #[cfg_attr(all(unix, feature = "serde"), serde(default))]
)];

let attr = attrs
    .try_find_attribute("serde")?
    .into_iter()
    .next()
    .expect("the serde attribute is present after expansion");

let predicate = attr
    .parse_condition()?
    .expect("the expanded attribute is conditional");

let enabled = predicate.evaluate(|option| match option {
    CfgOption::Flag(name) => name == "unix",
    CfgOption::NameValue { name, value } => {
        name == "feature" && value.value() == "serde"
    },
});

assert!(enabled);
# Ok::<(), syn::Error>(())
```

`CfgPredicate` supports:

- bare identifier flags such as `unix`;
- identifier and string-literal pairs such as `feature = "serde"`;
- `all(...)` and `any(...)` predicate lists;
- `not(...)` with exactly one predicate.

Other condition syntax returns `syn::Error` from `parse_condition()`. The
evaluator short-circuits `all(...)` at the first false predicate and `any(...)`
at the first true predicate. `all()` evaluates to true and `any()` to false.
Use the callback to answer configuration queries; it may visit only part of the
predicate.

## Preserve a guard without evaluating it

Use `ExpandedAttr::condition()` to attach the combined guard to generated
output:

```rust
use proc_macro2::TokenStream;
use quote::quote;
use syn_cfg_attr::ExpandedAttr;

fn guarded_item(attr: &ExpandedAttr, item: TokenStream) -> TokenStream {
    match attr.condition() {
        Some(condition) => quote!(#[cfg(#condition)] #item),
        None => item,
    }
}
```

Keep the guard as tokens so the generated item's compilation determines whether
it is enabled. Direct attributes leave the generated item unconditional.
