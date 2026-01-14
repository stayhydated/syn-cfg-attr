use syn::{Field, parse_quote};
use syn_cfg_attr::{AttributeHelpers, ExpandedAttr};

// Mock structures to simulate usage
struct KorumaAttr {
    name: String,
}

impl syn::parse::Parse for KorumaAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Simple parser for demonstration: expects identifier
        let ident: syn::Ident = input.parse()?;
        Ok(KorumaAttr {
            name: ident.to_string(),
        })
    }
}

fn parse_field(field: &Field) {
    println!("Processing field attributes...");

    // Automatically looks inside cfg_attr and handles nesting
    let koruma_attrs = field.attrs.find_attribute("koruma");

    if koruma_attrs.is_empty() {
        println!("No koruma attributes found.");
        return;
    }

    for expanded in koruma_attrs {
        match expanded.parse_args::<KorumaAttr>() {
            Ok(attr) => {
                println!("Found koruma attribute: {}", attr.name);
                if let ExpandedAttr::Nested { condition, .. } = &expanded {
                    println!("  - Condition: {}", condition);
                }
            },
            Err(e) => println!("  - Error parsing args: {}", e),
        }
    }
}

fn main() {
    // Example 1: Direct attribute
    let field1: Field = parse_quote! {
        #[koruma(skip)]
        field1: i32
    };
    parse_field(&field1);

    // Example 2: Nested inside cfg_attr
    let field2: Field = parse_quote! {
        #[cfg_attr(feature = "validation", koruma(required), other_attr)]
        field2: String
    };
    parse_field(&field2);

    // Example 3: Deeply nested
    let field3: Field = parse_quote! {
        #[cfg_attr(all(), cfg_attr(any(), koruma(validate)))]
        field3: u32
    };
    parse_field(&field3);
}
