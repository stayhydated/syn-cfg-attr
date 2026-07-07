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
//! use syn_cfg_attr::{AttributeHelpers, CfgOption, ExpandedAttr};
//!
//! # fn main() -> syn::Result<()> {
//! let attrs: Vec<Attribute> = vec![
//!     parse_quote!(#[serde(default)]),
//!     parse_quote!(#[cfg_attr(feature = "json", serde(rename = "id"))]),
//! ];
//!
//! let serde_attrs = attrs.try_find_attribute("serde")?;
//! assert_eq!(serde_attrs.len(), 2);
//!
//! for attr in serde_attrs {
//!     if let ExpandedAttr::Nested { condition, .. } = &attr {
//!         assert_eq!(condition.to_string(), "feature = \"json\"");
//!     }
//!
//!     if let Some(condition) = attr.parse_condition()? {
//!         let enabled = condition.evaluate(|option| match option {
//!             CfgOption::NameValue { name, value } => {
//!                 name == "feature" && value.value() == "json"
//!             },
//!             CfgOption::Flag(_) => false,
//!         });
//!         assert!(enabled);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Behavior
//!
//! - Direct attributes are returned as [`ExpandedAttr::Direct`].
//! - Attributes inside `cfg_attr(condition, ...)` are returned as
//!   [`ExpandedAttr::Nested`] with their combined `condition`.
//! - Nested `cfg_attr` entries are expanded recursively, and nested conditions
//!   are combined with `all(...)`.
//! - [`CfgPredicate`] parses and evaluates preserved conditions with
//!   caller-provided `cfg` option state.
//! - Fallible helpers report nested entries that cannot be parsed as
//!   [`syn::Meta`].
//! - [`ExpandedAttr::parse_args`] mirrors `syn` list-style attribute parsing for
//!   both direct and nested attributes.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Ident, LitStr, Meta, Path, Result, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

mod splitter;
use splitter::CommaSplitter;

/// A parsed `cfg` predicate.
///
/// Use [`ExpandedAttr::parse_condition`] to parse the condition stored on a
/// nested attribute, then [`CfgPredicate::evaluate`] to evaluate it with the
/// caller's target, feature, or custom `cfg` state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CfgPredicate {
    /// A bare `cfg` option such as `unix` or `target_thread_local`.
    Flag(Ident),
    /// A key-value `cfg` option such as `feature = "json"` or `target_os = "linux"`.
    NameValue {
        /// The option name.
        name: Ident,
        /// The string literal value.
        value: LitStr,
    },
    /// An `all(...)` predicate.
    All(Vec<CfgPredicate>),
    /// An `any(...)` predicate.
    Any(Vec<CfgPredicate>),
    /// A `not(...)` predicate.
    Not(Box<CfgPredicate>),
}

/// A single `cfg` option passed to [`CfgPredicate::evaluate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CfgOption<'a> {
    /// A bare `cfg` option such as `unix`.
    Flag(&'a Ident),
    /// A key-value `cfg` option such as `feature = "json"`.
    NameValue {
        /// The option name.
        name: &'a Ident,
        /// The string literal value.
        value: &'a LitStr,
    },
}

impl CfgPredicate {
    /// Evaluates this predicate using `is_enabled` for bare and key-value options.
    ///
    /// The callback is the source of truth for target, feature, and custom
    /// `cfg` state.
    pub fn evaluate(&self, mut is_enabled: impl FnMut(CfgOption<'_>) -> bool) -> bool {
        self.evaluate_with(&mut is_enabled)
    }

    fn evaluate_with(&self, is_enabled: &mut impl FnMut(CfgOption<'_>) -> bool) -> bool {
        match self {
            CfgPredicate::Flag(path) => is_enabled(CfgOption::Flag(path)),
            CfgPredicate::NameValue { name, value } => {
                is_enabled(CfgOption::NameValue { name, value })
            },
            CfgPredicate::All(predicates) => predicates
                .iter()
                .all(|predicate| predicate.evaluate_with(is_enabled)),
            CfgPredicate::Any(predicates) => predicates
                .iter()
                .any(|predicate| predicate.evaluate_with(is_enabled)),
            CfgPredicate::Not(predicate) => !predicate.evaluate_with(is_enabled),
        }
    }
}

impl Parse for CfgPredicate {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident = input.parse::<Ident>()?;

        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let value = input.parse::<LitStr>()?;
            return Ok(CfgPredicate::NameValue { name: ident, value });
        }

        if ident == "all" && input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let predicates = parse_cfg_predicate_list(&content)?;
            return Ok(CfgPredicate::All(predicates));
        }

        if ident == "any" && input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let predicates = parse_cfg_predicate_list(&content)?;
            return Ok(CfgPredicate::Any(predicates));
        }

        if ident == "not" && input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let mut predicates = parse_cfg_predicate_list(&content)?.into_iter();
            let Some(predicate) = predicates.next() else {
                return Err(content.error("not(...) expects exactly one cfg predicate"));
            };
            if predicates.next().is_some() {
                return Err(content.error("not(...) expects exactly one cfg predicate"));
            }
            return Ok(CfgPredicate::Not(Box::new(predicate)));
        }

        Ok(CfgPredicate::Flag(ident))
    }
}

fn parse_cfg_predicate_list(input: ParseStream<'_>) -> Result<Vec<CfgPredicate>> {
    Ok(
        Punctuated::<CfgPredicate, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect(),
    )
}

/// Represents an attribute that might have been extracted from a `#[cfg_attr(...)]`.
#[derive(Clone)]
pub enum ExpandedAttr {
    /// A standard attribute written directly on the item, such as `#[foo(...)]`.
    Direct(Attribute),
    /// An attribute found inside `cfg_attr(condition, ...)`.
    Nested {
        /// The parsed meta of the inner attribute, which does not include `#[...]` tokens.
        attr: Meta,
        /// The combined `cfg_attr` condition guarding this attribute.
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
    ///
    /// # Errors
    ///
    /// Returns an error when the attribute is not list-style or when `T` cannot
    /// be parsed from the list arguments.
    pub fn parse_args<T: Parse>(&self) -> Result<T> {
        match self {
            ExpandedAttr::Direct(attr) => attr.parse_args(),
            ExpandedAttr::Nested { attr, .. } => parse_meta_args(attr),
        }
    }

    /// Returns the raw combined `cfg_attr` condition for nested attributes.
    pub fn condition(&self) -> Option<&TokenStream> {
        match self {
            ExpandedAttr::Direct(_) => None,
            ExpandedAttr::Nested { condition, .. } => Some(condition),
        }
    }

    /// Parses the raw combined `cfg_attr` condition for nested attributes.
    ///
    /// Direct attributes return `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns an error when a nested attribute's preserved condition is not a
    /// valid `cfg` predicate.
    pub fn parse_condition(&self) -> Result<Option<CfgPredicate>> {
        self.condition()
            .map(|condition| syn::parse2(condition.clone()))
            .transpose()
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
    /// Best-effort expansion skips nested entries that cannot be parsed as [`Meta`].
    /// Use [`try_flattened_attributes`](Self::try_flattened_attributes) to report them.
    fn flattened_attributes(&self) -> Vec<ExpandedAttr>;

    /// Flattens all attributes and reports nested entries that cannot be parsed.
    ///
    /// # Errors
    ///
    /// Returns an error when a `cfg_attr` nested entry cannot be parsed as [`Meta`].
    fn try_flattened_attributes(&self) -> Result<Vec<ExpandedAttr>>;

    /// Finds all attributes matching the given identifier, including those inside `cfg_attr`.
    ///
    /// This is a convenience method that calls [`flattened_attributes`](Self::flattened_attributes)
    /// and filters by the given `ident`.
    fn find_attribute(&self, ident: &str) -> Vec<ExpandedAttr>;

    /// Finds all matching attributes and reports nested entries that cannot be parsed.
    ///
    /// # Errors
    ///
    /// Returns an error when a `cfg_attr` nested entry cannot be parsed as [`Meta`].
    fn try_find_attribute(&self, ident: &str) -> Result<Vec<ExpandedAttr>>;
}

impl AttributeHelpers for Vec<Attribute> {
    fn flattened_attributes(&self) -> Vec<ExpandedAttr> {
        let mut results = Vec::new();
        for attr in self {
            let _ = flatten_attr_recursive(attr, &mut results, None, InvalidNestedEntry::Skip);
        }
        results
    }

    fn try_flattened_attributes(&self) -> Result<Vec<ExpandedAttr>> {
        let mut results = Vec::new();
        for attr in self {
            flatten_attr_recursive(attr, &mut results, None, InvalidNestedEntry::Report)?;
        }
        Ok(results)
    }

    fn find_attribute(&self, ident: &str) -> Vec<ExpandedAttr> {
        self.flattened_attributes()
            .into_iter()
            .filter(|attr| attr.is_ident(ident))
            .collect()
    }

    fn try_find_attribute(&self, ident: &str) -> Result<Vec<ExpandedAttr>> {
        Ok(self
            .try_flattened_attributes()?
            .into_iter()
            .filter(|attr| attr.is_ident(ident))
            .collect())
    }
}

#[derive(Clone, Copy)]
enum InvalidNestedEntry {
    Skip,
    Report,
}

fn flatten_attr_recursive(
    attr: &Attribute,
    results: &mut Vec<ExpandedAttr>,
    parent_condition: Option<&TokenStream>,
    invalid_nested_entry: InvalidNestedEntry,
) -> Result<()> {
    if attr.path().is_ident("cfg_attr") {
        let tokens = match &attr.meta {
            Meta::List(list) => &list.tokens,
            _ => {
                return match invalid_nested_entry {
                    InvalidNestedEntry::Skip => Ok(()),
                    InvalidNestedEntry::Report => Err(syn::Error::new_spanned(
                        &attr.meta,
                        "cfg_attr must be a list attribute",
                    )),
                };
            },
        };

        let mut splitter = CommaSplitter::new(tokens.clone());

        if let Some(condition_stream) = splitter.next() {
            let combined_condition = combine_conditions(parent_condition, condition_stream);
            for inner_tokens in splitter {
                match syn::parse2::<Meta>(inner_tokens) {
                    Ok(nested_meta) if nested_meta.path().is_ident("cfg_attr") => {
                        let synthetic_attr = Attribute {
                            pound_token: Default::default(),
                            style: syn::AttrStyle::Outer,
                            bracket_token: Default::default(),
                            meta: nested_meta,
                        };
                        flatten_attr_recursive(
                            &synthetic_attr,
                            results,
                            Some(&combined_condition),
                            invalid_nested_entry,
                        )?;
                    },
                    Ok(nested_meta) => {
                        results.push(ExpandedAttr::Nested {
                            attr: nested_meta,
                            condition: combined_condition.clone(),
                            original: Box::new(attr.clone()),
                        });
                    },
                    Err(error) => {
                        if let InvalidNestedEntry::Report = invalid_nested_entry {
                            return Err(error);
                        }
                    },
                }
            }
        }
    } else {
        results.push(ExpandedAttr::Direct(attr.clone()));
    }

    Ok(())
}

fn combine_conditions(parent: Option<&TokenStream>, current: TokenStream) -> TokenStream {
    match parent {
        Some(parent) => quote! { all(#parent, #current) },
        None => current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn compact_tokens(tokens: &TokenStream) -> String {
        tokens.to_string().replace(' ', "")
    }

    fn nested_condition(attr: &ExpandedAttr) -> &TokenStream {
        attr.condition().expect("nested attribute has a condition")
    }

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

        assert_eq!(nested_condition(&flattened[0]).to_string(), "all ()");
    }

    #[test]
    fn test_flatten_recursive_cfg_attr() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(a, cfg_attr(b, foo))])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 1);
        assert!(flattened[0].is_ident("foo"));

        assert_eq!(compact_tokens(nested_condition(&flattened[0])), "all(a,b)");
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

        for attr in flattened {
            assert_eq!(nested_condition(&attr).to_string(), "my_cond");
        }
    }

    #[test]
    fn test_complex_condition() {
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[cfg_attr(any(target_os="linux", feature="flag"), foo)])];
        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 1);
        let s = nested_condition(&flattened[0]).to_string();
        assert!(s.contains("any"));
        assert!(s.contains("target_os"));
        assert!(s.contains("linux"));
        assert!(s.contains("feature"));
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

        assert_eq!(nested_condition(z).to_string(), "cond_a");
        assert_eq!(compact_tokens(nested_condition(x)), "all(cond_a,cond_b)");
        assert_eq!(compact_tokens(nested_condition(y)), "all(cond_a,cond_b)");
    }

    #[test]
    fn test_parse_condition_and_evaluate() {
        let attrs: Vec<Attribute> = vec![parse_quote!(
            #[cfg_attr(all(unix, feature = "json", not(target_os = "windows")), foo)]
        )];
        let flattened = attrs.flattened_attributes();
        let condition = flattened[0]
            .parse_condition()
            .expect("condition parses")
            .expect("nested attribute has a condition");

        let enabled = condition.evaluate(|option| match option {
            CfgOption::Flag(name) => name == "unix",
            CfgOption::NameValue { name, value } => name == "feature" && value.value() == "json",
        });
        assert!(enabled);

        let disabled = condition.evaluate(|option| match option {
            CfgOption::Flag(name) => name == "unix",
            CfgOption::NameValue { name, value } => name == "feature" && value.value() == "toml",
        });
        assert!(!disabled);
    }

    #[test]
    fn test_any_condition_parses_and_evaluates() {
        let condition: CfgPredicate =
            syn::parse_str(r#"any(windows, feature = "json")"#).expect("condition parses");

        let enabled = condition.evaluate(|option| match option {
            CfgOption::Flag(name) => name == "unix",
            CfgOption::NameValue { name, value } => name == "feature" && value.value() == "json",
        });
        assert!(enabled);

        let disabled = condition.evaluate(|option| match option {
            CfgOption::Flag(_) | CfgOption::NameValue { .. } => false,
        });
        assert!(!disabled);
    }

    #[test]
    fn test_parse_condition_rejects_not_with_multiple_predicates() {
        let error = syn::parse_str::<CfgPredicate>("not(unix, windows)")
            .expect_err("not accepts exactly one predicate");

        assert!(error.to_string().contains("exactly one"));
    }

    #[test]
    fn test_cfg_predicate_reports_parse_errors() {
        assert!(syn::parse_str::<CfgPredicate>("").is_err());
        assert!(syn::parse_str::<CfgPredicate>("feature =").is_err());
        assert!(syn::parse_str::<CfgPredicate>("all(foo::bar)").is_err());
        assert!(syn::parse_str::<CfgPredicate>("any(foo::bar)").is_err());
        assert!(syn::parse_str::<CfgPredicate>("not(foo::bar)").is_err());
    }

    #[test]
    fn test_direct_attribute_has_no_condition() {
        let attr = ExpandedAttr::Direct(parse_quote!(#[foo]));

        assert!(attr.condition().is_none());
        assert!(
            attr.parse_condition()
                .expect("direct condition parses")
                .is_none()
        );
    }

    #[test]
    fn test_try_helpers_return_filtered_attrs() {
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[foo]), parse_quote!(#[cfg_attr(c, foo, bar)])];

        let flattened = attrs
            .try_flattened_attributes()
            .expect("fallible flatten succeeds");
        assert_eq!(flattened.len(), 3);

        let foos = attrs
            .try_find_attribute("foo")
            .expect("fallible find succeeds");
        assert_eq!(foos.len(), 2);
        assert!(foos.iter().all(|attr| attr.is_ident("foo")));

        let invalid_attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(c, foo + bar)])];
        assert!(invalid_attrs.try_find_attribute("foo").is_err());
    }

    #[test]
    fn test_try_flattened_attributes_recurses_through_nested_cfg_attr() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(a, cfg_attr(b, foo))])];

        let flattened = attrs
            .try_flattened_attributes()
            .expect("fallible recursive flatten succeeds");

        assert_eq!(flattened.len(), 1);
        assert_eq!(compact_tokens(nested_condition(&flattened[0])), "all(a,b)");
    }

    #[test]
    fn test_empty_cfg_attr_list_is_ignored() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr()])];

        assert!(attrs.flattened_attributes().is_empty());
        assert!(
            attrs
                .try_flattened_attributes()
                .expect("empty cfg_attr list flattens")
                .is_empty()
        );
    }

    #[test]
    fn test_try_flattened_attributes_reports_recursive_invalid_nested_entries() {
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[cfg_attr(a, cfg_attr(b, foo + bar), valid)])];

        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 1);
        assert!(flattened[0].is_ident("valid"));

        assert!(attrs.try_flattened_attributes().is_err());
    }

    #[test]
    fn test_malformed_cfg_attr_form_is_skipped_or_reported() {
        let attrs = vec![Attribute {
            pound_token: Default::default(),
            style: syn::AttrStyle::Outer,
            bracket_token: Default::default(),
            meta: Meta::Path(parse_quote!(cfg_attr)),
        }];

        assert!(attrs.flattened_attributes().is_empty());

        let error = attrs
            .try_flattened_attributes()
            .err()
            .expect("fallible flatten reports malformed cfg_attr");
        assert!(
            error
                .to_string()
                .contains("cfg_attr must be a list attribute")
        );
    }

    #[test]
    fn test_parse_condition_rejects_path_option_names() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(foo::bar, baz)])];
        let flattened = attrs.flattened_attributes();

        assert!(flattened[0].parse_condition().is_err());
    }

    #[test]
    fn test_try_flattened_attributes_reports_invalid_nested_entries() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[cfg_attr(c, foo + bar, valid)])];

        let flattened = attrs.flattened_attributes();
        assert_eq!(flattened.len(), 1);
        assert!(flattened[0].is_ident("valid"));

        assert!(attrs.try_flattened_attributes().is_err());
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
