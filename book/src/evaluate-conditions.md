# Evaluate preserved conditions

Call `ExpandedAttr::parse_condition()` to turn a nested attribute's preserved
tokens into `CfgPredicate`. Direct attributes return `Ok(None)` because they
have no guard. Supply the configuration state for the code being inspected when
you evaluate the predicate.

## Parse and evaluate a guard

Evaluation delegates every leaf option to your callback:

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
evaluator applies boolean predicate semantics while the callback remains the
source of truth for every leaf value.

## Preserve a guard without evaluating it

Use `ExpandedAttr::condition()` when generated output or a later processing
stage only needs the raw combined `TokenStream`. Forwarding those tokens keeps
the original configuration decision with the consumer and avoids inventing
target or feature state.
