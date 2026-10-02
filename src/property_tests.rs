use super::*;
use proptest::prelude::*;
use syn::parse::Parser as _;

pub(crate) fn property_config() -> ProptestConfig {
    let mut config = ProptestConfig::default();
    if std::env::var_os("PROPTEST_CASES").is_none() {
        config.cases = 64;
    }
    config
}

#[derive(Clone, Debug)]
enum Predicate {
    Option(u8),
    Boolean(bool),
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
}

impl Predicate {
    fn source(&self) -> String {
        match self {
            Self::Option(id) if *id < 2 => format!("flag{id}"),
            Self::Option(id) => format!("feature = \"value{id}\""),
            Self::Boolean(value) => value.to_string(),
            Self::All(children) | Self::Any(children) => {
                let operator = if matches!(self, Self::All(_)) {
                    "all"
                } else {
                    "any"
                };
                let children = children.iter().map(Self::source).collect::<Vec<_>>();
                format!("{operator}({})", children.join(","))
            },
            Self::Not(child) => format!("not({})", child.source()),
        }
    }

    // Each bit represents one of the 16 assignments to four options. Bitwise
    // truth tables provide an oracle independent of the production evaluator's
    // recursive, short-circuit callback traversal.
    fn truth_table(&self) -> u16 {
        match self {
            Self::Option(id) => (0..16)
                .filter(|assignment| assignment & (1 << id) != 0)
                .fold(0, |table, assignment| table | (1 << assignment)),
            Self::Boolean(true) => u16::MAX,
            Self::Boolean(false) => 0,
            Self::All(children) => children
                .iter()
                .fold(u16::MAX, |table, child| table & child.truth_table()),
            Self::Any(children) => children
                .iter()
                .fold(0, |table, child| table | child.truth_table()),
            Self::Not(child) => !child.truth_table(),
        }
    }
}

fn predicates() -> BoxedStrategy<Predicate> {
    prop_oneof![
        (0u8..4).prop_map(Predicate::Option),
        any::<bool>().prop_map(Predicate::Boolean)
    ]
    .prop_recursive(4, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Predicate::All),
            prop::collection::vec(inner.clone(), 0..4).prop_map(Predicate::Any),
            inner.prop_map(|child| Predicate::Not(Box::new(child))),
        ]
    })
    .boxed()
}

fn evaluate(predicate: &CfgPredicate, assignment: u8) -> bool {
    predicate.evaluate(|option| {
        let id: u8 = match option {
            CfgOption::Flag(name) => name
                .to_string()
                .strip_prefix("flag")
                .unwrap()
                .parse()
                .unwrap(),
            CfgOption::NameValue { name, value } => {
                assert_eq!(name, "feature");
                value
                    .value()
                    .strip_prefix("value")
                    .unwrap()
                    .parse()
                    .unwrap()
            },
        };
        assignment & (1 << id) != 0
    })
}

#[derive(Clone, Debug)]
enum AttributeTree {
    Leaf(u8),
    Guard(Predicate, Vec<Self>),
}

struct ExpectedLeaf {
    meta: Meta,
    guard: Option<u16>,
}

impl AttributeTree {
    fn source(&self, parent: Option<u16>, leaves: &mut Vec<ExpectedLeaf>) -> String {
        match self {
            Self::Leaf(kind) => {
                let index = leaves.len();
                let source = match kind {
                    0 => format!("leaf{index}"),
                    1 => format!("foo({index})"),
                    2 => format!("foo = \"leaf{index}\""),
                    3 => format!("bar([{index}, {}], \"a, b\")", index + 1),
                    _ => format!("ns::foo({index})"),
                };
                leaves.push(ExpectedLeaf {
                    meta: syn::parse_str(&source).unwrap(),
                    guard: parent,
                });
                source
            },
            Self::Guard(condition, children) => {
                let table = condition.truth_table();
                let combined = parent.map_or(table, |parent| parent & table);
                let children = children
                    .iter()
                    .map(|child| child.source(Some(combined), leaves))
                    .collect::<Vec<_>>();
                format!("cfg_attr({}, {})", condition.source(), children.join(","))
            },
        }
    }
}

fn forests() -> impl Strategy<Value = Vec<AttributeTree>> {
    let tree = (0u8..5)
        .prop_map(AttributeTree::Leaf)
        .prop_recursive(3, 24, 4, |inner| {
            (predicates(), prop::collection::vec(inner, 0..4))
                .prop_map(|(guard, children)| AttributeTree::Guard(guard, children))
        });
    prop::collection::vec(tree, 0..5)
}

fn parse_attributes(source: &str) -> Vec<Attribute> {
    Attribute::parse_outer.parse_str(source).unwrap()
}

fn check_leaves(
    actual: &[ExpandedAttr],
    expected: &[ExpectedLeaf],
) -> std::result::Result<(), TestCaseError> {
    prop_assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        let meta = match actual {
            ExpandedAttr::Direct(attr) => &attr.meta,
            ExpandedAttr::Nested { attr, .. } => attr,
        };
        prop_assert_eq!(meta, &expected.meta);
        let condition = actual.parse_condition().unwrap();
        prop_assert_eq!(condition.is_some(), expected.guard.is_some());
        if let (Some(condition), Some(table)) = (condition, expected.guard) {
            for assignment in 0..16 {
                prop_assert_eq!(
                    evaluate(&condition, assignment),
                    table & (1 << assignment) != 0
                );
            }
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(property_config())]

    #[test]
    fn predicates_match_all_truth_assignments(model in predicates()) {
        let parsed: CfgPredicate = syn::parse_str(&model.source()).unwrap();
        for assignment in 0..16 {
            prop_assert_eq!(evaluate(&parsed, assignment), model.truth_table() & (1 << assignment) != 0);
        }
        let all: CfgPredicate = syn::parse_str(&format!("all(false, {})", model.source())).unwrap();
        let any: CfgPredicate = syn::parse_str(&format!("any(true, {})", model.source())).unwrap();
        prop_assert!(!all.evaluate(|_| panic!("false must short-circuit")));
        prop_assert!(any.evaluate(|_| panic!("true must short-circuit")));
    }

    #[test]
    fn forests_preserve_order_metadata_and_ancestor_guards(forest in forests()) {
        let mut expected = Vec::new();
        let source = forest.iter().map(|tree| format!("#[{}]", tree.source(None, &mut expected))).collect::<String>();
        let attrs = parse_attributes(&source);
        let strict = attrs.try_flattened_attributes().unwrap();
        check_leaves(&strict, &expected)?;
        check_leaves(&attrs.flattened_attributes(), &expected)?;
        let filtered = expected.iter()
            .filter(|leaf| leaf.meta.path().is_ident("foo"))
            .map(|leaf| ExpectedLeaf { meta: leaf.meta.clone(), guard: leaf.guard })
            .collect::<Vec<_>>();
        check_leaves(&attrs.try_find_attribute("foo").unwrap(), &filtered)?;
        check_leaves(&attrs.find_attribute("foo"), &filtered)?;
    }

    #[test]
    fn malformed_nested_entry_is_reported_or_skipped(guard in predicates(), index in 0usize..4, recursive in any::<bool>()) {
        let mut entries = vec!["left(1)", "middle(2)", "right(3)"];
        entries.insert(index, "broken + tokens");
        let inner = format!("cfg_attr({}, {})", guard.source(), entries.join(","));
        let source = if recursive { format!("#[cfg_attr(true, {inner})]") } else { format!("#[{inner}]") };
        let attrs = parse_attributes(&source);
        prop_assert!(attrs.try_flattened_attributes().is_err());
        prop_assert!(attrs.try_find_attribute("left").is_err());
        let table = guard.truth_table();
        let expected = ["left(1)", "middle(2)", "right(3)"].into_iter()
            .map(|source| ExpectedLeaf { meta: syn::parse_str(source).unwrap(), guard: Some(table) })
            .collect::<Vec<_>>();
        check_leaves(&attrs.flattened_attributes(), &expected)?;
    }

    #[test]
    fn invalid_predicate_classes_remain_errors(id in 0u8..4, class in 0u8..4) {
        let source = match class {
            0 => format!("not(flag{id}, true)"),
            1 => format!("all(flag{id}::other)"),
            2 => format!("feature = {id}"),
            _ => format!("feature{id} ="),
        };
        prop_assert!(syn::parse_str::<CfgPredicate>(&source).is_err());
    }
}

#[test]
fn false_guards_keep_leaves_and_invalid_guards_remain_raw() {
    let attrs = parse_attributes("#[cfg_attr(false, foo)] #[cfg_attr(foo::bar, baz)]");
    let expanded = attrs.try_flattened_attributes().unwrap();
    assert_eq!(expanded.len(), 2);
    assert!(
        !expanded[0]
            .parse_condition()
            .unwrap()
            .unwrap()
            .evaluate(|_| panic!("false needs no options"))
    );
    assert!(expanded[1].condition().is_some());
    assert!(expanded[1].parse_condition().is_err());
}
