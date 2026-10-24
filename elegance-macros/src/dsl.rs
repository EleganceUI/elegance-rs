// Parses and expands `elg!`.
//
// node     := element | `{` expr `}` mods | `(` expr `)` block mods | if | match
// element  := path args? block? mods           (a bare path is an expression)
// args     := `(` (expr | ident `:` expr),* `)`
// block    := `{` node* `}` | `{` pat `in` node `}`   (opening on the element's line; a `{`
//             on a later line starts the next node, an embedded expression)
// mods     := (`.` ident args?)*                 (no args: `expr.name`, plain Rust, so the IDE
//                                                   can complete a modifier while it's typed)
//
// Parsing recovers from errors: they're reported where they are, and the rest still expands, so
// type checking and completion keep working in the rest of the view while one part is being
// edited.
// if       := `if` cond `{` node `}` (`else` (if | `{` node `}`))?
// match    := `match` expr `{` (pat (`if` guard)? `=>` (node | `{` node `}`) `,`?)* `}`

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};

use crate::rewrite::{argument, is_group, named, rewrite, split_commas};

struct Args {
    positional: Vec<TokenStream>,
    named: Vec<(proc_macro2::Ident, TokenStream)>,
}

enum Block {
    Views(Vec<Node>),
    Content { pattern: TokenStream, body: Box<Node> },
}

struct Modifier {
    name: proc_macro2::Ident,
    // `::<T>` turbofish, if any.
    generics: TokenStream,
    // None: no parentheses (yet).
    args: Option<Args>,
}

enum Node {
    Element { path: TokenStream, args: Option<Args>, block: Option<Block>, modifiers: Vec<Modifier> },
    Expr { expr: TokenStream, modifiers: Vec<Modifier> },
    LayoutCall { layout: TokenStream, block: Block, modifiers: Vec<Modifier> },
    If { condition: TokenStream, then: Box<Node>, otherwise: Option<Box<Node>> },
    // Each arm: its pattern (with any guard) and its view.
    Match { value: TokenStream, arms: Vec<(TokenStream, Node)> },
}

type Result<T> = std::result::Result<T, syn::Error>;

fn error<T>(span: Span, message: &str) -> Result<T> {
    Err(syn::Error::new(span, message))
}

struct Parser {
    tokens: Vec<TokenTree>,
    pos: usize,
    end_span: Span,
    errors: Vec<syn::Error>,
}

impl Parser {
    fn new(stream: TokenStream, end_span: Span) -> Self {
        Self { tokens: stream.into_iter().collect(), pos: 0, end_span, errors: Vec::new() }
    }

    fn adopt_errors(&mut self, inner: &mut Parser) {
        self.errors.append(&mut inner.errors);
    }

    fn peek(&self) -> Option<&TokenTree> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&TokenTree> {
        self.tokens.get(self.pos + offset)
    }

    fn next(&mut self) -> Option<TokenTree> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn span(&self) -> Span {
        self.peek().map_or(self.end_span, TokenTree::span)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn is_ident(&self, text: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(i)) if i == text)
    }

    fn is_punct(&self, offset: usize, c: char) -> bool {
        matches!(self.peek_at(offset), Some(TokenTree::Punct(p)) if p.as_char() == c)
    }

    // At `=>`.
    fn at_arrow(&self) -> bool {
        self.is_punct(0, '=') && self.is_punct(1, '>')
    }

    fn skip_separators(&mut self) {
        while self.is_punct(0, ',') || self.is_punct(0, ';') {
            self.pos += 1;
        }
    }

    fn nodes(&mut self) -> Vec<Node> {
        let mut nodes = Vec::new();
        self.skip_separators();
        while !self.at_end() {
            let start = self.pos;
            match self.node() {
                Ok(node) => nodes.push(node),
                Err(error) => {
                    self.errors.push(error);
                    // Skip past the problem and carry on with the next node.
                    if self.pos == start {
                        self.pos += 1;
                    }
                }
            }
            self.skip_separators();
        }
        nodes
    }

    fn node(&mut self) -> Result<Node> {
        if self.is_ident("if") {
            return self.if_node();
        }
        if self.is_ident("match") {
            return self.match_node();
        }
        match self.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let group = g.clone();
                self.pos += 1;
                let modifiers = self.modifiers()?;
                let inner = rewrite(group.stream());
                Ok(Node::Expr { expr: quote!({ #inner }), modifiers })
            }
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                let group = g.clone();
                self.pos += 1;
                if !is_group(self.peek(), Delimiter::Brace) {
                    return error(group.span(), "a layout `(expr)` takes its children in a `{ ... }` block");
                }
                let block = self.block()?;
                let modifiers = self.modifiers()?;
                Ok(Node::LayoutCall { layout: rewrite(group.stream()), block, modifiers })
            }
            Some(TokenTree::Ident(_)) | Some(TokenTree::Punct(_)) if self.starts_path() => self.element(),
            Some(TokenTree::Literal(lit)) => {
                error(lit.span(), "a string isn't a view here: write Text(\"...\")")
            }
            _ => error(self.span(), "expected a view: an element like Text(\"...\"), `{ expr }`, or `if`"),
        }
    }

    fn starts_path(&self) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(_))) || (self.is_punct(0, ':') && self.is_punct(1, ':'))
    }

    // A path: `a::b::C`, maybe with turbofish generics `::<T>`.
    fn path(&mut self) -> TokenStream {
        let mut path = TokenStream::new();
        loop {
            if self.is_punct(0, ':') && self.is_punct(1, ':') {
                path.extend([self.next().unwrap(), self.next().unwrap()]);
                if self.is_punct(0, '<') {
                    let mut depth = 0;
                    while let Some(token) = self.next() {
                        let c = if let TokenTree::Punct(p) = &token { Some(p.as_char()) } else { None };
                        path.extend([token]);
                        match c {
                            Some('<') => depth += 1,
                            Some('>') => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
            }
            match self.peek() {
                Some(TokenTree::Ident(_)) => path.extend([self.next().unwrap()]),
                _ => break,
            }
            if !(self.is_punct(0, ':') && self.is_punct(1, ':')) {
                break;
            }
        }
        path
    }

    fn element(&mut self) -> Result<Node> {
        let path = self.path();
        let mut head_end = self.tokens[self.pos - 1].span();
        let args = match self.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                let stream = g.stream();
                head_end = g.span_close();
                self.pos += 1;
                Some(parse_args(stream))
            }
            _ => None,
        };
        let block_follows = matches!(self.peek(), Some(TokenTree::Group(g))
            if g.delimiter() == Delimiter::Brace && same_line(head_end, g.span_open()));
        let block = if block_follows { Some(self.block()?) } else { None };
        let modifiers = self.modifiers()?;
        if args.is_none() && block.is_none() {
            // A variable or constant holding a view.
            return Ok(Node::Expr { expr: path, modifiers });
        }
        Ok(Node::Element { path, args, block, modifiers })
    }

    fn block(&mut self) -> Result<Block> {
        let Some(TokenTree::Group(group)) = self.next() else { unreachable!() };
        let mut inner = Parser::new(group.stream(), group.span_close());
        let block = if let Some(pattern) = inner.content_pattern() {
            let nodes = inner.nodes();
            let body = inner.single(nodes, group.span(), "an `item in` block holds one view; wrap several in a stack");
            Block::Content { pattern, body: Box::new(body) }
        } else {
            Block::Views(inner.nodes())
        };
        self.adopt_errors(&mut inner);
        Ok(block)
    }

    // `name in`, `(a, b) in`, `_ in`, `&x in` at the start of a block.
    fn content_pattern(&mut self) -> Option<TokenStream> {
        let mut end = None;
        for (i, token) in self.tokens.iter().enumerate() {
            match token {
                TokenTree::Ident(ident) if ident == "in" => {
                    end = Some(i);
                    break;
                }
                TokenTree::Ident(_) => {}
                TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => {}
                TokenTree::Punct(p) if matches!(p.as_char(), '_' | '&' | ',' | ':') => {}
                _ => return None,
            }
        }
        let end = end.filter(|&e| e > 0)?;
        // Paths like `Foo::Bar(x) in` aren't patterns we accept; an element would be a call.
        if end >= 2 && matches!(self.tokens[end - 1], TokenTree::Group(_)) && matches!(self.tokens[end - 2], TokenTree::Ident(_)) {
            return None;
        }
        let pattern: TokenStream = self.tokens[..end].iter().cloned().collect();
        self.pos = end + 1;
        Some(pattern)
    }

    fn modifiers(&mut self) -> Result<Vec<Modifier>> {
        let mut modifiers = Vec::new();
        while self.is_punct(0, '.') && !self.is_punct(1, '.') {
            let dot_span = self.span();
            self.pos += 1;
            let Some(TokenTree::Ident(name)) = self.peek().cloned() else {
                // A `.` still waiting for its name: report it, keep what came before.
                self.errors.push(syn::Error::new(dot_span, "expected a modifier name after `.`"));
                break;
            };
            self.pos += 1;
            let mut generics = TokenStream::new();
            if self.is_punct(0, ':') && self.is_punct(1, ':') && self.is_punct(2, '<') {
                let mut depth = 0;
                while let Some(token) = self.next() {
                    let c = if let TokenTree::Punct(p) = &token { Some(p.as_char()) } else { None };
                    generics.extend([token]);
                    match c {
                        Some('<') => depth += 1,
                        Some('>') => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let args = match self.peek() {
                Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                    let stream = g.stream();
                    self.pos += 1;
                    Some(parse_args(stream))
                }
                // Expanded as `expr.name`: rustc says to add parentheses, and an IDE completes it.
                _ => None,
            };
            modifiers.push(Modifier { name, generics: rewrite(generics), args });
        }
        Ok(modifiers)
    }

    fn if_node(&mut self) -> Result<Node> {
        let if_span = self.span();
        self.pos += 1;
        let mut condition = TokenStream::new();
        while !self.at_end() && !is_group(self.peek(), Delimiter::Brace) {
            condition.extend([self.next().unwrap()]);
        }
        if condition.is_empty() {
            return error(if_span, "`if` needs a condition");
        }
        let then = self.branch(if_span)?;
        let otherwise = if self.is_ident("else") {
            self.pos += 1;
            if self.is_ident("if") {
                Some(Box::new(self.if_node()?))
            } else {
                Some(Box::new(self.branch(if_span)?))
            }
        } else {
            None
        };
        if self.is_punct(0, '.') && !self.is_punct(1, '.') {
            return error(self.span(), "modifiers can't follow an `if`; wrap it: `{ elg!{ if ... } }.padding(8)` or modify each branch");
        }
        Ok(Node::If { condition: rewrite(condition), then: Box::new(then), otherwise })
    }

    fn match_node(&mut self) -> Result<Node> {
        let match_span = self.span();
        self.pos += 1;
        let mut value = TokenStream::new();
        while !self.at_end() && !is_group(self.peek(), Delimiter::Brace) {
            value.extend([self.next().unwrap()]);
        }
        if value.is_empty() {
            return error(match_span, "`match` needs a value");
        }
        let Some(TokenTree::Group(body)) = self.next() else {
            return error(match_span, "expected the arms of the `match` in `{ ... }`");
        };
        let mut inner = Parser::new(body.stream(), body.span_close());
        let mut arms = Vec::new();
        inner.skip_separators();
        while !inner.at_end() {
            // The pattern (and guard) runs up to `=>`.
            let arm_span = inner.span();
            let mut pattern = TokenStream::new();
            while !inner.at_end() && !inner.at_arrow() {
                pattern.extend([inner.next().unwrap()]);
            }
            if inner.at_end() {
                inner.errors.push(syn::Error::new(arm_span, "expected `=>` and a view after the pattern"));
                break;
            }
            inner.pos += 2;
            // `=> { view }` is a branch, like an `if`'s; otherwise one element.
            let view = if is_group(inner.peek(), Delimiter::Brace) {
                inner.branch(arm_span)
            } else {
                inner.node()
            };
            match view {
                Ok(view) => arms.push((rewrite(pattern), view)),
                Err(e) => inner.errors.push(e),
            }
            inner.skip_separators();
        }
        self.adopt_errors(&mut inner);
        if self.is_punct(0, '.') && !self.is_punct(1, '.') {
            return error(self.span(), "modifiers can't follow a `match`; modify each arm");
        }
        Ok(Node::Match { value: rewrite(value), arms })
    }

    fn branch(&mut self, span: Span) -> Result<Node> {
        let Some(TokenTree::Group(group)) = self.next() else {
            return error(span, "expected a `{ ... }` branch");
        };
        let mut inner = Parser::new(group.stream(), group.span_close());
        let nodes = inner.nodes();
        self.adopt_errors(&mut inner);
        Ok(self.single(nodes, group.span(), "an `if` branch holds one view; wrap several in a stack"))
    }

    // The one node of a block; with several, reports `message` and keeps the first.
    fn single(&mut self, mut nodes: Vec<Node>, span: Span, message: &str) -> Node {
        if nodes.len() > 1 {
            self.errors.push(syn::Error::new(span, message));
        }
        if nodes.is_empty() {
            Node::Expr { expr: quote!(()), modifiers: Vec::new() }
        } else {
            nodes.swap_remove(0)
        }
    }
}

// Whether `b` starts on the line where `a` ends.
fn same_line(a: Span, b: Span) -> bool {
    a.unwrap().end().line() == b.unwrap().start().line()
}

fn parse_args(stream: TokenStream) -> Args {
    let mut args = Args { positional: Vec::new(), named: Vec::new() };
    for part in split_commas(stream) {
        match named(&part) {
            Some((name, value)) => args.named.push((name, argument(value))),
            None => args.positional.push(argument(part)),
        }
    }
    args
}

fn expand_node(node: &Node) -> TokenStream {
    match node {
        Node::Element { path, args, block, modifiers } => {
            let positional = args.as_ref().map(|a| a.positional.as_slice()).unwrap_or(&[]);
            let mut expr = quote!(#path::elg_new((#(#positional,)*)));
            if let Some(args) = args {
                for (name, value) in &args.named {
                    expr = quote!(#expr.#name(#value));
                }
            }
            if let Some(block) = block {
                expr = expand_block(expr, block);
            }
            expand_modifiers(expr, modifiers)
        }
        Node::Expr { expr, modifiers } => expand_modifiers(quote!((#expr)), modifiers),
        Node::LayoutCall { layout, block, modifiers } => {
            let expr = expand_block(quote!((#layout)), block);
            expand_modifiers(expr, modifiers)
        }
        Node::If { condition, then, otherwise } => {
            let then = expand_node(then);
            let otherwise = otherwise.as_ref().map(|o| expand_node(o)).unwrap_or_else(|| quote!(()));
            quote!(::elegance::__private::if_view(move || -> bool { #condition }, #then, #otherwise))
        }
        Node::Match { value, arms } => {
            let patterns = arms.iter().map(|(pattern, _)| pattern);
            let views = arms.iter().map(|(_, view)| expand_node(view));
            quote!(::elegance::__private::match_view(
                move || #value,
                move |__elg_value| match ::std::clone::Clone::clone(__elg_value) {
                    #(#patterns => ::elegance::__private::IntoView::into_view(#views),)*
                }
            ))
        }
    }
}

fn expand_block(expr: TokenStream, block: &Block) -> TokenStream {
    match block {
        Block::Views(children) => {
            let children = children.iter().map(expand_node);
            quote!(::elegance::__private::ElgChildren::elg_children(
                #expr,
                ::std::vec![#(::elegance::__private::IntoView::into_view(#children)),*]
            ))
        }
        Block::Content { pattern, body } => {
            let body = expand_node(body);
            quote!(#expr.elg_content(move |#pattern| #body))
        }
    }
}

fn expand_modifiers(mut expr: TokenStream, modifiers: &[Modifier]) -> TokenStream {
    for Modifier { name, generics, args } in modifiers {
        let Some(args) = args else {
            expr = quote!(#expr.#name #generics);
            continue;
        };
        let positional = &args.positional;
        if args.named.is_empty() {
            expr = quote_spanned!(name.span()=> #expr.#name #generics(#(#positional),*));
        } else {
            let with = format_ident!("{}_with", name, span = name.span());
            let options = format_ident!("__elg_options");
            let setters = args.named.iter().map(|(n, v)| quote!(.#n(#v)));
            expr = quote_spanned!(name.span()=> #expr.#with #generics(#(#positional,)* |#options| #options #(#setters)*));
        }
    }
    expr
}

pub fn expand(input: TokenStream) -> TokenStream {
    let mut parser = Parser::new(input, Span::call_site());
    let nodes = parser.nodes();
    if nodes.is_empty() && parser.errors.is_empty() {
        parser.errors.push(syn::Error::new(Span::call_site(), "elg! needs a view"));
    }
    let root = parser.single(nodes, Span::call_site(), "elg! takes one root view; wrap several in a stack");
    let body = expand_node(&root);
    // Errors go next to the expansion rather than replacing it.
    let errors = parser.errors.iter().map(syn::Error::to_compile_error);
    quote!({
        #(#errors;)*
        #[allow(unused_imports)]
        use ::elegance::__private::{ElgChildren as _, ElgNew as _, ViewExt as _};
        #body
    })
}
