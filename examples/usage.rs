use quote::ToTokens;
use syn::{Attribute, Field, parse_quote};
use syn_cfg_attr::{AttributeHelpers, CfgOption, ExpandedAttr};

struct KorumaAttr {
    name: String,
}

impl syn::parse::Parse for KorumaAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        Ok(KorumaAttr {
            name: ident.to_string(),
        })
    }
}

fn main() -> syn::Result<()> {
    let field: Field = parse_quote! {
        #[koruma(skip)]
        #[cfg_attr(feature = "validation", koruma(required), other_attr)]
        #[cfg_attr(
            all(unix, any(feature = "validation", feature = "serde"), not(target_os = "windows")),
            cfg_attr(target_os = "linux", koruma(validate))
        )]
        field: String
    };
    let attrs = field.attrs;

    println!("Fallible recursive expansion");
    let flattened = attrs.try_flattened_attributes()?;
    for expanded in &flattened {
        print_expanded_attr(expanded)?;
    }

    println!("\nFallible filtering and parse_args");
    for expanded in attrs.try_find_attribute("koruma")? {
        let koruma = expanded.parse_args::<KorumaAttr>()?;
        println!("  koruma({})", koruma.name);
    }

    println!("\nBest-effort helpers with malformed nested input");
    let malformed_attrs: Vec<Attribute> = vec![parse_quote!(
        #[cfg_attr(feature = "broken", koruma + invalid, koruma(fallback))]
    )];

    if let Err(error) = malformed_attrs.try_flattened_attributes() {
        println!("  try_flattened_attributes reported: {error}");
    }

    if let Err(error) = malformed_attrs.try_find_attribute("koruma") {
        println!("  try_find_attribute reported: {error}");
    }

    let best_effort = malformed_attrs.flattened_attributes();
    println!(
        "  flattened_attributes kept {} parseable attribute(s)",
        best_effort.len()
    );

    for expanded in malformed_attrs.find_attribute("koruma") {
        let koruma = expanded.parse_args::<KorumaAttr>()?;
        println!("  find_attribute kept koruma({})", koruma.name);
    }

    Ok(())
}

fn print_expanded_attr(expanded: &ExpandedAttr) -> syn::Result<()> {
    println!("  {}", expanded.path().to_token_stream());

    match expanded {
        ExpandedAttr::Direct(_) => {
            println!("    direct attribute");
        },
        ExpandedAttr::Nested {
            condition,
            original,
            ..
        } => {
            println!("    combined condition: {condition}");
            println!(
                "    containing attribute: {}",
                original.meta.to_token_stream()
            );

            let predicate = expanded
                .parse_condition()?
                .expect("nested attributes have a condition");
            println!(
                "    enabled in example cfg set: {}",
                predicate.evaluate(example_cfg_enabled)
            );
        },
    }

    Ok(())
}

fn example_cfg_enabled(option: CfgOption<'_>) -> bool {
    match option {
        CfgOption::Flag(name) => name == "unix",
        CfgOption::NameValue { name, value } => {
            (name == "feature" && value.value() == "validation")
                || (name == "target_os" && value.value() == "linux")
        },
    }
}
