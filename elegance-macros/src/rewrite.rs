// Token rewrites applied to every Rust expression in the DSL:
//   .ident   (where an expression starts) -> ::elegance::dot::ident
//   $ident                                -> ident.binding()
// and, for a whole argument, "text {name}" -> Interpolated(move || ::std::format!("text {name}")).

use proc_macro2::{Delimiter, Group, Ident, Spacing, TokenStream, TokenTree};
use quote::quote;

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "yield", "box", "do", "final", "macro", "override", "priv",
    "typeof", "unsized", "virtual", "abstract", "become", "try",
];

// Keywords after which an expression starts (so a following `.x` is shorthand).
const EXPRESSION_KEYWORDS: &[&str] = &[
    "return", "break", "in", "match", "if", "while", "else", "yield", "move",
];

fn dot_path(name: &Ident) -> TokenStream {
    let text = name.to_string();
    let ident = if KEYWORDS.contains(&text.as_str()) {
        Ident::new_raw(&text, name.span())
    } else {
        name.clone()
    };
    quote!(::elegance::dot::#ident)
}

/// Whether a `.` after `prev` starts an expression (the shorthand) rather than a method call.
fn starts_expression(prev: Option<&TokenTree>) -> bool {
    match prev {
        None => true,
        Some(TokenTree::Group(_)) | Some(TokenTree::Literal(_)) => false,
        Some(TokenTree::Ident(ident)) => EXPRESSION_KEYWORDS.contains(&ident.to_string().as_str()),
        // `?` and `.` end expressions (`x?.y`, `a..b` is handled by the caller).
        Some(TokenTree::Punct(p)) => !matches!(p.as_char(), '?' | '.'),
    }
}

pub fn rewrite(tokens: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let mut out = TokenStream::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            TokenTree::Group(group) => {
                let mut rewritten = Group::new(group.delimiter(), rewrite(group.stream()));
                rewritten.set_span(group.span());
                out.extend([TokenTree::Group(rewritten)]);
            }
            TokenTree::Punct(p) if p.as_char() == '.' && p.spacing() == Spacing::Alone => {
                let prev = if i > 0 { Some(&tokens[i - 1]) } else { None };
                let after_dot = matches!(prev, Some(TokenTree::Punct(q)) if q.as_char() == '.');
                match tokens.get(i + 1) {
                    Some(TokenTree::Ident(name)) if !after_dot && starts_expression(prev) => {
                        out.extend(dot_path(name));
                        i += 2;
                        continue;
                    }
                    _ => out.extend([token.clone()]),
                }
            }
            TokenTree::Punct(p) if p.as_char() == '$' => match tokens.get(i + 1) {
                Some(TokenTree::Ident(name)) => {
                    out.extend(quote!(#name.binding()));
                    i += 2;
                    continue;
                }
                _ => out.extend([token.clone()]),
            },
            _ => out.extend([token.clone()]),
        }
        i += 1;
    }
    out
}

/// A whole argument: rewritten, and a lone string literal with `{placeholders}` made reactive.
pub fn argument(tokens: TokenStream) -> TokenStream {
    let items: Vec<TokenTree> = tokens.clone().into_iter().collect();
    if let [TokenTree::Literal(literal)] = items.as_slice() {
        if let Ok(text) =
            syn::parse2::<syn::LitStr>(TokenStream::from(TokenTree::Literal(literal.clone())))
        {
            if has_placeholder(&text.value()) {
                return quote!(::elegance::__private::Interpolated(
                    move || ::std::format!(#text)
                ));
            }
        }
    }
    rewrite(tokens)
}

fn has_placeholder(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if bytes.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            return true;
        }
        i += 1;
    }
    false
}

/// Splits a parenthesized argument list at top-level commas (keeping turbofish generics whole).
pub fn split_commas(tokens: TokenStream) -> Vec<TokenStream> {
    let mut parts = Vec::new();
    let mut current = TokenStream::new();
    let mut angle_depth = 0usize;
    let mut prev_colons = false;
    for token in tokens {
        match &token {
            TokenTree::Punct(p) if p.as_char() == ',' && angle_depth == 0 => {
                parts.push(std::mem::take(&mut current));
                prev_colons = false;
                continue;
            }
            TokenTree::Punct(p) if p.as_char() == '<' && (prev_colons || angle_depth > 0) => {
                angle_depth += 1
            }
            TokenTree::Punct(p) if p.as_char() == '>' && angle_depth > 0 => angle_depth -= 1,
            _ => {}
        }
        prev_colons = matches!(&token, TokenTree::Punct(p) if p.as_char() == ':');
        current.extend([token]);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// `name: value` (not `a::b`): the name and the value.
pub fn named(tokens: &TokenStream) -> Option<(Ident, TokenStream)> {
    let items: Vec<TokenTree> = tokens.clone().into_iter().collect();
    match items.as_slice() {
        [TokenTree::Ident(name), TokenTree::Punct(colon), rest @ ..]
            if colon.as_char() == ':' && colon.spacing() == Spacing::Alone && !rest.is_empty() =>
        {
            let not_path = !matches!(rest.first(), Some(TokenTree::Punct(p)) if p.as_char() == ':');
            not_path.then(|| (name.clone(), rest.iter().cloned().collect()))
        }
        _ => None,
    }
}

pub fn is_group(token: Option<&TokenTree>, delimiter: Delimiter) -> bool {
    matches!(token, Some(TokenTree::Group(g)) if g.delimiter() == delimiter)
}
