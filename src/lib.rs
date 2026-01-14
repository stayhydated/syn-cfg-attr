use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Attribute, Meta, Path, Result, parse::Parse};

mod splitter;
use splitter::CommaSplitter;

/// Represents an attribute that might have been extracted from a `#[cfg_attr(...)]`
#[derive(Clone)]
pub enum ExpandedAttr {
    /// A standard attribute like `#[foo(...)]`
    Direct(Attribute),
    /// An attribute found inside `cfg_attr(condition, ...)`
    /// `attr` is the parsed Meta of the inner attribute (since it lacks `#[...]` braces in the stream)
    /// `condition` is the `cfg` condition that guarded it.
    Nested {
        attr: Meta,
        condition: TokenStream,
        /// The original top-level `cfg_attr` attribute, kept for span reporting or inspection
        original: Box<Attribute>,
    },
}

impl ExpandedAttr {
    pub fn parse_args<T: Parse>(&self) -> Result<T> {
        match self {
            ExpandedAttr::Direct(attr) => attr.parse_args(),
            ExpandedAttr::Nested { attr, .. } => {
                match attr {
                    Meta::List(list) => list.parse_args(),
                    Meta::NameValue(nv) => {
                        // For name-value attributes (e.g., `#[key = "value"]`),
                        // we parse the value tokens directly to match standard behavior.
                        syn::parse2(nv.value.to_token_stream())
                    },
                    Meta::Path(_) => Err(syn::Error::new_spanned(
                        attr,
                        "Attribute path has no arguments",
                    )),
                }
            },
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            ExpandedAttr::Direct(attr) => attr.path(),
            ExpandedAttr::Nested { attr, .. } => attr.path(),
        }
    }

    pub fn is_ident(&self, ident: &str) -> bool {
        self.path().is_ident(ident)
    }
}

/// Extension trait for working with `Vec<Attribute>` that handles `cfg_attr` expansion.
pub trait AttributeHelpers {
    /// Flattens all attributes, expanding any `cfg_attr(...)` into their inner attributes.
    ///
    /// This recursively processes nested `cfg_attr` and returns both direct attributes
    /// and attributes found inside `cfg_attr`, wrapped in [`ExpandedAttr`].
    fn flattened_attributes(&self) -> Vec<ExpandedAttr>;

    /// Finds all attributes matching the given identifier, including those inside `cfg_attr`.
    ///
    /// This is a convenience method that calls [`flattened_attributes`](Self::flattened_attributes)
    /// and filters by the given `ident`.
    fn find_attribute(&self, ident: &str) -> Vec<ExpandedAttr>;
}

impl AttributeHelpers for Vec<Attribute> {
    fn flattened_attributes(&self) -> Vec<ExpandedAttr> {
        let mut results = Vec::new();
        for attr in self {
            flatten_attr_recursive(attr, &mut results, None);
        }
        results
    }

    fn find_attribute(&self, ident: &str) -> Vec<ExpandedAttr> {
        self.flattened_attributes()
            .into_iter()
            .filter(|attr| attr.is_ident(ident))
            .collect()
    }
}

fn flatten_attr_recursive(
    attr: &Attribute,
    results: &mut Vec<ExpandedAttr>,
    _inherited_condition: Option<&TokenStream>,
) {
    if attr.path().is_ident("cfg_attr") {
        let tokens = match &attr.meta {
            Meta::List(list) => &list.tokens,
            _ => return,
        };

        let mut splitter = CommaSplitter::new(tokens.clone());

        if let Some(condition_stream) = splitter.next() {
            // Note: We currently track the immediate condition.
            // Future improvements could combine `_inherited_condition` with `condition_stream`
            // to support AND-ing nested conditions (e.g. `cfg_attr(a, cfg_attr(b, ...))`).

            for inner_tokens in splitter {
                if let Ok(nested_meta) = syn::parse2::<Meta>(inner_tokens.clone()) {
                    if nested_meta.path().is_ident("cfg_attr") {
                        let synthetic_attr = Attribute {
                            pound_token: Default::default(),
                            style: syn::AttrStyle::Outer,
                            bracket_token: Default::default(),
                            meta: nested_meta,
                        };
                        flatten_attr_recursive(&synthetic_attr, results, Some(&condition_stream));
                    } else {
                        results.push(ExpandedAttr::Nested {
                            attr: nested_meta,
                            condition: condition_stream.clone(),
                            original: Box::new(attr.clone()),
                        });
                    }
                }
            }
        }
    } else {
        results.push(ExpandedAttr::Direct(attr.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_flatten_basic() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[foo]), parse_quote!(#[bar(x)])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 2);
        assert!(flattened[0].is_ident("foo"));
        assert!(flattened[1].is_ident("bar"));
    }

    #[test]
    fn test_flatten_cfg_attr() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(all(), foo, bar(y))])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 2);
        assert!(flattened[0].is_ident("foo"));
        assert!(flattened[1].is_ident("bar"));

        match &flattened[0] {
            ExpandedAttr::Nested { condition, .. } => {
                assert_eq!(condition.to_string(), "all ()");
            },
            _ => panic!("Expected Nested"),
        }
    }

    #[test]
    fn test_flatten_recursive_cfg_attr() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(a, cfg_attr(b, foo))])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 1);
        assert!(flattened[0].is_ident("foo"));

        match &flattened[0] {
            ExpandedAttr::Nested { condition, .. } => {
                assert_eq!(condition.to_string(), "b");
            },
            _ => panic!("Expected Nested"),
        }
    }

    #[test]
    fn test_find_attribute() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[foo]),
            parse_quote!(#[cfg_attr(all(), foo)]),
            parse_quote!(#[bar]),
            parse_quote!(#[cfg_attr(any(), bar)]),
        ];
        let foos = attrs.find_attribute("foo");
        assert_eq!(foos.len(), 2);

        let bars = attrs.find_attribute("bar");
        assert_eq!(bars.len(), 2);
    }

    #[test]
    fn test_cfg_attr_multiple_attrs() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(my_cond, a, b(val), c)])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 3);
        assert!(flattened[0].is_ident("a"));
        assert!(flattened[1].is_ident("b"));
        assert!(flattened[2].is_ident("c"));

        // condition should be preserved for all
        for attr in flattened {
            if let ExpandedAttr::Nested { condition, .. } = attr {
                assert_eq!(condition.to_string(), "my_cond");
            } else {
                panic!("Expected Nested layout");
            }
        }
    }

    #[test]
    fn test_complex_condition() {
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[cfg_attr(any(target_os="linux", feature="flag"), foo)])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 1);
        if let ExpandedAttr::Nested { condition, .. } = &flattened[0] {
            // Spacing might vary, check structure loosely or exact if confident
            let s = condition.to_string();
            assert!(s.contains("any"));
            assert!(s.contains("target_os"));
            assert!(s.contains("linux"));
            assert!(s.contains("feature"));
        } else {
            panic!("Expected Nested");
        }
    }

    #[test]
    fn test_deep_mixed_nesting() {
        // cfg_attr(a, cfg_attr(b, x, y), z)
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[cfg_attr(cond_a, cfg_attr(cond_b, x, y), z)])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 3);

        let z = flattened.iter().find(|a| a.is_ident("z")).unwrap();
        let x = flattened.iter().find(|a| a.is_ident("x")).unwrap();
        let y = flattened.iter().find(|a| a.is_ident("y")).unwrap();

        if let ExpandedAttr::Nested { condition, .. } = z {
            assert_eq!(condition.to_string(), "cond_a");
        }

        if let ExpandedAttr::Nested { condition, .. } = x {
            assert_eq!(condition.to_string(), "cond_b");
        }

        if let ExpandedAttr::Nested { condition, .. } = y {
            assert_eq!(condition.to_string(), "cond_b");
        }
    }

    #[test]
    fn test_parse_args_variants() {
        use syn::LitInt;

        // 1. Direct List: #[foo(1)]
        let attr1: Attribute = parse_quote!(#[foo(1)]);
        let exp1 = ExpandedAttr::Direct(attr1);
        assert!(exp1.parse_args::<LitInt>().is_ok());

        // 2. Nested List: cfg_attr(..., foo(1))
        let attr2: Attribute = parse_quote!(#[cfg_attr(c, foo(1))]);
        let flattened = vec![attr2].flattened_attributes();
        let exp2 = &flattened[0];
        assert!(exp2.parse_args::<LitInt>().is_ok());

        // 3. Nested NameValue: cfg_attr(..., foo = "bar")
        // Note: parse_args on NameValue usually errors in standard syn unless we handle it custom.
        // Our lib implementation handles it by parsing the value.
        let attr3: Attribute = parse_quote!(#[cfg_attr(c, foo = "bar")]);
        let flattened3 = vec![attr3].flattened_attributes();
        let exp3 = &flattened3[0];
        // Expect parsing a string literal
        assert!(exp3.parse_args::<syn::LitStr>().is_ok());
    }
}
