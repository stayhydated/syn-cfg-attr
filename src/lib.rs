//! Expand `cfg_attr` entries in `syn` attribute lists.
//!
//! `syn-cfg-attr` is for procedural macros and code-generation tools that need
//! to parse attributes the same way whether they were written directly or inside
//! `cfg_attr`. It flattens a `Vec<syn::Attribute>` into [`ExpandedAttr`] values
//! while preserving the `cfg` predicate that guarded each nested attribute.
//!
//! # Example
//!
//! ```
//! use syn::{Attribute, parse_quote};
//! use syn_cfg_attr::{AttributeHelpers, ExpandedAttr};
//!
//! let attrs: Vec<Attribute> = vec![
//!     parse_quote!(#[serde(default)]),
//!     parse_quote!(#[cfg_attr(feature = "json", serde(rename = "id"))]),
//! ];
//!
//! let serde_attrs = attrs.find_attribute("serde");
//! assert_eq!(serde_attrs.len(), 2);
//!
//! for attr in serde_attrs {
//!     if let ExpandedAttr::Nested { condition, .. } = &attr {
//!         assert_eq!(condition.to_string(), "feature = \"json\"");
//!     }
//! }
//! ```
//!
//! # Behavior
//!
//! - Direct attributes are returned as [`ExpandedAttr::Direct`].
//! - Attributes inside `cfg_attr(condition, ...)` are returned as
//!   [`ExpandedAttr::Nested`] with their immediate `condition`.
//! - Nested `cfg_attr` entries are expanded recursively, but nested conditions
//!   are not combined or evaluated.
//! - Nested entries that cannot be parsed as [`syn::Meta`] are skipped.
//! - [`ExpandedAttr::parse_args`] mirrors `syn` list-style attribute parsing for
//!   both direct and nested attributes.

use proc_macro2::TokenStream;
use syn::{Attribute, Meta, Path, Result, parse::Parse};

mod splitter;
use splitter::CommaSplitter;

/// Represents an attribute that might have been extracted from a `#[cfg_attr(...)]`.
#[derive(Clone)]
pub enum ExpandedAttr {
    /// A standard attribute written directly on the item, such as `#[foo(...)]`.
    Direct(Attribute),
    /// An attribute found inside `cfg_attr(condition, ...)`.
    Nested {
        /// The parsed meta of the inner attribute, which does not include `#[...]` tokens.
        attr: Meta,
        /// The immediate `cfg_attr` condition guarding this attribute.
        condition: TokenStream,
        /// The `cfg_attr` being expanded, kept for span reporting or inspection.
        ///
        /// During recursive expansion this can be a synthetic attribute built from a
        /// nested `cfg_attr` meta item rather than the original outer source attribute.
        original: Box<Attribute>,
    },
}

impl ExpandedAttr {
    /// Parses the arguments of a list-style attribute.
    ///
    /// Direct attributes use [`Attribute::parse_args`]. Nested `cfg_attr`
    /// entries parse the tokens inside their `Meta::List`. Path-only and
    /// name-value attributes return an error.
    pub fn parse_args<T: Parse>(&self) -> Result<T> {
        match self {
            ExpandedAttr::Direct(attr) => attr.parse_args(),
            ExpandedAttr::Nested { attr, .. } => parse_meta_args(attr),
        }
    }

    /// Returns the path that identifies the attribute.
    pub fn path(&self) -> &Path {
        match self {
            ExpandedAttr::Direct(attr) => attr.path(),
            ExpandedAttr::Nested { attr, .. } => attr.path(),
        }
    }

    /// Returns whether the attribute path is a single identifier matching `ident`.
    pub fn is_ident(&self, ident: &str) -> bool {
        self.path().is_ident(ident)
    }
}

fn parse_meta_args<T: Parse>(meta: &Meta) -> Result<T> {
    meta.require_list()?.parse_args()
}

/// Extension trait for working with `Vec<Attribute>` that handles `cfg_attr` expansion.
pub trait AttributeHelpers {
    /// Flattens all attributes, expanding any `cfg_attr(...)` into their inner attributes.
    ///
    /// This recursively processes nested `cfg_attr` and returns both direct attributes
    /// and attributes found inside `cfg_attr`, wrapped in [`ExpandedAttr`].
    /// Nested entries that cannot be parsed as [`Meta`] are skipped.
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
            flatten_attr_recursive(attr, &mut results);
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

fn flatten_attr_recursive(attr: &Attribute, results: &mut Vec<ExpandedAttr>) {
    if attr.path().is_ident("cfg_attr") {
        let tokens = match &attr.meta {
            Meta::List(list) => &list.tokens,
            _ => return,
        };

        let mut splitter = CommaSplitter::new(tokens.clone());

        if let Some(condition_stream) = splitter.next() {
            for inner_tokens in splitter {
                if let Ok(nested_meta) = syn::parse2::<Meta>(inner_tokens.clone()) {
                    if nested_meta.path().is_ident("cfg_attr") {
                        let synthetic_attr = Attribute {
                            pound_token: Default::default(),
                            style: syn::AttrStyle::Outer,
                            bracket_token: Default::default(),
                            meta: nested_meta,
                        };
                        flatten_attr_recursive(&synthetic_attr, results);
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
        let attr3: Attribute = parse_quote!(#[cfg_attr(c, foo = "bar")]);
        let flattened3 = vec![attr3].flattened_attributes();
        let exp3 = &flattened3[0];
        assert!(exp3.parse_args::<syn::LitStr>().is_err());

        // 4. Direct NameValue: #[foo = "bar"]
        let attr4: Attribute = parse_quote!(#[foo = "bar"]);
        let exp4 = ExpandedAttr::Direct(attr4);
        assert!(exp4.parse_args::<syn::LitStr>().is_err());
    }
}
