use proc_macro2::{TokenStream, TokenTree};

/// Splits a TokenStream by top-level commas, respecting recursive depths of
/// parentheses `()`, brackets `[]`, and braces `{}`.
///
/// This is used to split the arguments of `cfg_attr(condition, attr1, attr2)`
/// where `attr1` or `attr2` might themselves contain complex tokens.
pub struct CommaSplitter {
    input: std::vec::IntoIter<TokenTree>,
    current_buffer: Vec<TokenTree>,
    depth: isize,
}

impl CommaSplitter {
    pub fn new(tokens: TokenStream) -> Self {
        Self {
            input: tokens.into_iter().collect::<Vec<_>>().into_iter(),
            current_buffer: Vec::new(),
            depth: 0,
        }
    }
}

impl Iterator for CommaSplitter {
    type Item = TokenStream;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.input.next() {
                Some(tt) => {
                    match &tt {
                        TokenTree::Punct(p) if p.as_char() == ',' && self.depth == 0 => {
                            // Split point found at top-level comma.
                            let stream =
                                TokenStream::from_iter(std::mem::take(&mut self.current_buffer));
                            return Some(stream);
                        }
                        TokenTree::Group(_g) => {
                            // Groups (parens, brackets, braces) contain their own token streams.
                            // We treat the entire group as a single token unit at this level,
                            // so we don't need to manually track depth for them.
                            self.current_buffer.push(tt);
                        }
                        TokenTree::Punct(p) => {
                            // Track angle brackets for proper handling of generics like `Type<A, B>`.
                            if p.as_char() == '<' {
                                self.depth += 1;
                            } else if p.as_char() == '>' {
                                self.depth -= 1;
                            }
                            self.current_buffer.push(tt);
                        }
                        _ => {
                            self.current_buffer.push(tt);
                        }
                    }
                }
                None => {
                    if !self.current_buffer.is_empty() {
                        let stream =
                            TokenStream::from_iter(std::mem::take(&mut self.current_buffer));
                        return Some(stream);
                    } else {
                        return None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_split_simple() {
        let ts = quote! { a, b, c };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_with_group() {
        let ts = quote! { fn_call(a, b), c };
        // Group logic ensures `fn_call(a, b)` is treated as one item before the comma.
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts[0], "fn_call (a , b)");
        assert_eq!(parts[1], "c");
    }

    #[test]
    fn test_split_with_generics() {
        let ts = quote! { Type<A, B>, C };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].replace(" ", ""), "Type<A,B>");
        assert_eq!(parts[1], "C");
    }

    #[test]
    fn test_split_trailing_comma() {
        let ts = quote! { a, b, };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts, vec!["a", "b"]);
    }

    #[test]
    fn test_split_nested_groups() {
        let ts = quote! { (a, (b, c)), d };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts.len(), 2);
        // Spacing varies by quote verions, just check content
        assert!(parts[0].contains("(a"));
        assert_eq!(parts[1], "d");
    }

    #[test]
    fn test_split_mixed_delimiters() {
        let ts = quote! { [a, {b, c}], (d) };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].contains("["));
        assert!(parts[0].contains("{"));
        assert!(parts[1].contains("("));
    }

    #[test]
    fn test_split_empty() {
        let ts = quote! {};
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert!(parts.is_empty());
    }

    #[test]
    fn test_split_only_commas() {
        // Not a typical use case for valid Rust attributes, but robustness check
        let ts = quote! { , , };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        // Depending on implementation, might yield empty streams or nothing.
        // Current impl yields empty streams if buffer is empty?
        // Actually, current impl:
        // if comma matches:
        //   stream = take(buffer) -> return Some(stream)
        // So ", ," ->
        // 1. comma: buffer empty -> yields empty stream
        // 2. comma: buffer empty -> yields empty stream
        // 3. None: buffer empty -> returns None
        // So we expect 2 empty strings
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "");
        assert_eq!(parts[1], "");
    }

    #[test]
    fn test_complex_generics_recursion() {
        let ts = quote! { Type<A, Vec<B>>, C };
        let splitter = CommaSplitter::new(ts);
        let parts: Vec<String> = splitter.map(|s| s.to_string()).collect();
        assert_eq!(parts.len(), 2);
        let p0 = parts[0].replace(" ", "");
        assert_eq!(p0, "Type<A,Vec<B>>");
        assert_eq!(parts[1], "C");
    }
}
