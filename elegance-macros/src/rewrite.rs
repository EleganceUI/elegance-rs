// Token rewrites applied to every Rust expression in the DSL:
//   .ident   (where an expression starts) -> the Dot named `ident` (see elegance::dot), looked
//                                            up in the parameter type's namespace if known
//   $ident                                -> an error: pass the state itself
// and, for a whole argument, "text {name}" -> Interpolated(move || ::std::format!("text {name}"))
// (with `{size.0}` and other expressions pulled out as arguments: see `interpolate`).

use proc_macro2::{Delimiter, Group, Ident, Spacing, TokenStream, TokenTree};
use quote::{quote, quote_spanned};

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

/// `name` as a field or variable: raw if it's a keyword (`.move`), keeping its span.
pub fn field_ident(name: &Ident) -> Ident {
    let text = name.to_string();
    let text = text.trim_start_matches("r#");
    if KEYWORDS.contains(&text) {
        Ident::new_raw(text, name.span())
    } else {
        name.clone()
    }
}

/// The value `.name` stands for. The name stays an identifier in the expansion, with its span,
/// so rust-analyzer can highlight it and complete it as it's typed: a field of the namespace
/// when the parameter's type is known (see `expand_call`), otherwise a variable holding the
/// `Dot`, which converts like any value.
pub fn dot_value(name: &Ident, namespace: Option<&Ident>) -> TokenStream {
    let field = field_ident(name);
    match namespace {
        Some(namespace) => quote_spanned!(name.span()=>
            ::elegance::__private::dot_value(&#namespace.#field)
        ),
        None => {
            let ty = crate::dot::name_type(name, &quote!(::elegance::dot));
            quote_spanned!(name.span()=> { #[allow(unused_variables)] let #field = <#ty>::NEW; #field })
        }
    }
}

/// The `.names` at the top level of an argument (where its namespace applies), once each.
pub fn shorthand_names(tokens: &TokenStream) -> Vec<Ident> {
    let tokens: Vec<TokenTree> = tokens.clone().into_iter().collect();
    let mut names: Vec<Ident> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let TokenTree::Punct(p) = token else { continue };
        if p.as_char() != '.' {
            continue;
        }
        let prev = i.checked_sub(1).map(|i| &tokens[i]);
        let after_dot = matches!(prev, Some(TokenTree::Punct(q)) if q.as_char() == '.');
        if let Some(TokenTree::Ident(name)) = tokens.get(i + 1) {
            if !after_dot && starts_expression(prev) && !names.iter().any(|n| n == name) {
                names.push(field_ident(name));
            }
        }
    }
    names
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

pub fn has_shorthand(tokens: TokenStream) -> bool {
    let tokens: Vec<_> = tokens.into_iter().collect();
    tokens.iter().enumerate().any(|(i, token)| match token {
        // Nested calls and callbacks have their own argument types; do not use
        // the enclosing builder parameter's namespace for their shorthand.
        TokenTree::Group(_) => false,
        // A bare dot immediately followed by an argument comma has Joint
        // spacing even after split_commas removes the comma.
        TokenTree::Punct(p) if p.as_char() == '.' => {
            let prev = i.checked_sub(1).map(|i| &tokens[i]);
            !matches!(prev, Some(TokenTree::Punct(p)) if p.as_char() == '.')
                && starts_expression(prev)
                && matches!(tokens.get(i + 1), Some(TokenTree::Ident(_)) | None)
        }
        _ => false,
    })
}

pub fn rewrite(tokens: TokenStream) -> TokenStream {
    rewrite_in(tokens, None)
}

fn rewrite_in(tokens: TokenStream, namespace: Option<&Ident>) -> TokenStream {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let mut out = TokenStream::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token {
            TokenTree::Group(group) => {
                let mut rewritten = Group::new(group.delimiter(), rewrite_in(group.stream(), None));
                rewritten.set_span(group.span());
                out.extend([TokenTree::Group(rewritten)]);
            }
            TokenTree::Punct(p) if p.as_char() == '.' => {
                let prev = if i > 0 { Some(&tokens[i - 1]) } else { None };
                let after_dot = matches!(prev, Some(TokenTree::Punct(q)) if q.as_char() == '.');
                match tokens.get(i + 1) {
                    Some(TokenTree::Ident(name)) if !after_dot && starts_expression(prev) => {
                        out.extend(dot_value(name, namespace));
                        i += 2;
                        continue;
                    }
                    // A bare `.` (completion in progress) completes the namespace's fields. It's
                    // wrapped like a finished name (see `dot_value`): rust-analyzer finds the
                    // cursor by text offset, comparing this expansion with one where a name
                    // follows the dot, so the text before the dot must be the same.
                    None if !after_dot && starts_expression(prev) && namespace.is_some() => {
                        let namespace = namespace.unwrap();
                        out.extend(quote!(::elegance::__private::dot_value(&#namespace #p)));
                    }
                    _ => out.extend([token.clone()]),
                }
            }
            // `$name` was the old binding syntax; State converts to a Binding by itself now.
            TokenTree::Punct(p) if p.as_char() == '$' => match tokens.get(i + 1) {
                Some(TokenTree::Ident(name)) => {
                    let message = format!("`${name}` is gone: pass the state itself, `{name}`");
                    out.extend(syn::Error::new(p.span(), message).to_compile_error());
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
    argument_in(tokens, None)
}

pub fn argument_in(tokens: TokenStream, namespace: Option<&Ident>) -> TokenStream {
    let items: Vec<TokenTree> = tokens.clone().into_iter().collect();
    if let [TokenTree::Literal(literal)] = items.as_slice() {
        if let Ok(text) =
            syn::parse2::<syn::LitStr>(TokenStream::from(TokenTree::Literal(literal.clone())))
        {
            if has_placeholder(&text.value()) {
                let format = match interpolate(&text) {
                    Ok(format) => format,
                    Err(error) => return error.to_compile_error(),
                };
                return quote!(::elegance::__private::Interpolated(move || #format));
            }
        }
    }
    rewrite_in(tokens, namespace)
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

/// The `format!` for `"text {placeholders}"`. A name, index or empty placeholder stays as it is;
/// any other expression becomes a named argument (`{a + b}` -> `{__elg_0}`, `__elg_0 = (a + b)`),
/// and a field chain (`{size.0}`, `{user.name:>8}`) is formatted by reference inside a read of its
/// root, so it works on `State`, `Computed` and `Binding` as well as plain values.
fn interpolate(text: &syn::LitStr) -> syn::Result<TokenStream> {
    let source = text.value();
    let mut format = String::new();
    let mut args = Vec::new();
    let mut rest = source.as_str();
    while let Some(i) = rest.find(['{', '}']) {
        format.push_str(&rest[..i]);
        rest = &rest[i..];
        if rest.starts_with("{{") || rest.starts_with("}}") || rest.starts_with('}') {
            let len = if rest.len() > 1 && rest.as_bytes()[1] == rest.as_bytes()[0] { 2 } else { 1 };
            format.push_str(&rest[..len]);
            rest = &rest[len..];
            continue;
        }
        let Some(end) = rest.find('}') else {
            return Err(syn::Error::new(text.span(), "unclosed `{` in text; write `{{` for a literal brace"));
        };
        let placeholder = &rest[1..end];
        rest = &rest[end + 1..];
        let (arg, spec) = split_spec(placeholder);
        if is_plain_arg(arg.trim()) {
            format.push('{');
            format.push_str(placeholder);
            format.push('}');
            continue;
        }
        let not_expression = || syn::Error::new(text.span(), format!("`{arg}` in text isn't an expression"));
        let tokens = arg.parse().map_err(|_| not_expression())?;
        let expr: syn::Expr = syn::parse2(respan(tokens, text.span())).map_err(|_| not_expression())?;
        let name = Ident::new(&format!("__elg_{}", args.len()), text.span());
        let value = match field_chain(&expr) {
            Some((root, members)) => {
                let inner = syn::LitStr::new(&format!("{{:{}}}", spec.unwrap_or("")), text.span());
                quote!((&::elegance::__private::Reader(&#root)).elg_with(
                    |__elg_value| ::std::format!(#inner, __elg_value #(.#members)*)
                ))
            }
            None => {
                if let Some(spec) = spec {
                    format.push_str(&format!("{{{name}:{spec}}}"));
                    args.push(quote!(#name = (#expr)));
                    continue;
                }
                quote!((#expr))
            }
        };
        format.push_str(&format!("{{{name}}}"));
        args.push(quote!(#name = #value));
    }
    format.push_str(rest);
    let format = syn::LitStr::new(&format, text.span());
    Ok(quote!({
        #[allow(unused_imports)]
        use ::elegance::__private::{ReadPlain as _, ReadReactive as _};
        ::std::format!(#format #(, #args)*)
    }))
}

/// `arg:spec` at the first `:` outside brackets that isn't part of a `::` path.
fn split_spec(placeholder: &str) -> (&str, Option<&str>) {
    let bytes = placeholder.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b':' if bytes.get(i + 1) == Some(&b':') => i += 1,
            b':' if depth == 0 => return (&placeholder[..i], Some(&placeholder[i + 1..])),
            _ => {}
        }
        i += 1;
    }
    (placeholder, None)
}

/// What `format!` takes itself: `{}`, `{0}`, `{name}`.
fn is_plain_arg(arg: &str) -> bool {
    arg.is_empty()
        || arg.bytes().all(|b| b.is_ascii_digit())
        || syn::parse_str::<Ident>(arg).is_ok()
}

/// `root.a.0.b` -> (`root`, [`a`, `0`, `b`]).
fn field_chain(expr: &syn::Expr) -> Option<(&Ident, Vec<&syn::Member>)> {
    match expr {
        syn::Expr::Field(field) => {
            let (root, mut members) = field_chain(&field.base)?;
            members.push(&field.member);
            Some((root, members))
        }
        syn::Expr::Path(path) if path.qself.is_none() && path.attrs.is_empty() => {
            Some((path.path.get_ident()?, Vec::new()))
        }
        _ => None,
    }
}

/// Points tokens parsed from inside a string literal at that literal, for errors.
fn respan(tokens: TokenStream, span: proc_macro2::Span) -> TokenStream {
    tokens
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(group) = &token {
                let mut inner = Group::new(group.delimiter(), respan(group.stream(), span));
                inner.set_span(span);
                return TokenTree::Group(inner);
            }
            token.set_span(span);
            token
        })
        .collect()
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
            if colon.as_char() == ':' && colon.spacing() == Spacing::Alone =>
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_dot_before_comma_keeps_its_completion_receiver() {
        let parts = split_commas("alignment: ., title: \"Hello\"".parse().unwrap());
        let (_, value) = named(&parts[0]).unwrap();
        let Some(TokenTree::Punct(dot)) = value.clone().into_iter().next() else {
            panic!("expected dot");
        };
        assert_eq!(dot.spacing(), Spacing::Joint);
        assert!(has_shorthand(value.clone()));
        let namespace = Ident::new("dots", proc_macro2::Span::call_site());
        // Wrapped like a finished `.name`, so the text before the dot matches (see rewrite_in).
        assert_eq!(
            argument_in(value, Some(&namespace)).to_string(),
            ":: elegance :: __private :: dot_value (& dots .)"
        );
    }

    #[test]
    fn ranges_and_member_access_are_not_dot_shorthand() {
        for source in [
            "a..b",
            "..end",
            "start..",
            "0..=5",
            "value.leading",
            "value?.leading",
            "1.0",
        ] {
            let tokens: TokenStream = source.parse().unwrap();
            assert!(!has_shorthand(tokens.clone()), "{source}");
            assert_eq!(rewrite(tokens.clone()).to_string(), tokens.to_string());
        }
    }
}
