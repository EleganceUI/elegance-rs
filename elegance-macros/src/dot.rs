use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Token, Type,
};

struct Entry {
    name: Ident,
    conversions: Vec<(Type, Expr)>,
}

struct Dots {
    entries: Vec<Entry>,
    namespaces: Vec<(Type, Vec<Type>)>,
}

impl Parse for Dots {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        let mut namespaces = Vec::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            let body;
            braced!(body in input);
            if name == "namespaces" {
                while !body.is_empty() {
                    let target = body.parse()?;
                    body.parse::<Token![=>]>()?;
                    let sources;
                    bracketed!(sources in body);
                    namespaces.push((
                        target,
                        Punctuated::<Type, Token![,]>::parse_terminated(&sources)?
                            .into_iter()
                            .collect(),
                    ));
                    if !body.is_empty() {
                        body.parse::<Token![,]>()?;
                    }
                }
            } else {
                let mut conversions = Vec::new();
                while !body.is_empty() {
                    let ty = body.parse()?;
                    body.parse::<Token![=>]>()?;
                    conversions.push((ty, body.parse()?));
                    if !body.is_empty() {
                        body.parse::<Token![,]>()?;
                    }
                }
                entries.push(Entry { name, conversions });
            }
        }
        Ok(Self {
            entries,
            namespaces,
        })
    }
}

pub fn expand(input: TokenStream) -> TokenStream {
    match syn::parse2::<Dots>(input).and_then(generate) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

/// The `Dot` type named `name` (`r#move` is `move`): `#dot::Dot<(#dot::chars::m, ...)>`, with
/// `dot` the path of `elegance::dot`. Spans point at `name`, for errors.
pub fn name_type(name: &Ident, dot: &TokenStream) -> TokenStream {
    let text = name.to_string();
    let text = text.trim_start_matches("r#");
    let chars = text.chars().map(|c| {
        let char_name = match c {
            '_' => "__".to_string(),
            '0'..='9' => format!("_{c}"),
            // Identifiers in elg! are ASCII; anything else gets a type that doesn't exist.
            c if c.is_ascii_alphabetic() => c.to_string(),
            _ => "unsupported_character".to_string(),
        };
        let ident = Ident::new(&char_name, name.span());
        quote!(#dot::chars::#ident)
    });
    quote!(#dot::Dot<(#(#chars,)*)>)
}

fn generate(dots: Dots) -> syn::Result<TokenStream> {
    let mut targets: BTreeMap<String, (Type, Vec<Ident>)> = BTreeMap::new();
    let markers = dots.entries.iter().map(|entry| {
        let name = &entry.name;
        let ty = name_type(name, &quote!(self));
        let conversions = entry.conversions.iter().map(|(ty, value)| {
            quote! {
                impl From<#name> for #ty {
                    fn from(_: #name) -> #ty { #value }
                }
            }
        });
        quote! {
            pub type #name = #ty;
            #(#conversions)*
        }
    });
    for entry in &dots.entries {
        for (ty, _) in &entry.conversions {
            targets
                .entry(quote!(#ty).to_string())
                .or_insert_with(|| (ty.clone(), Vec::new()))
                .1
                .push(entry.name.clone());
        }
    }
    // Namespace-only aliases describe existing blanket/IntoProp conversions, without adding
    // From impls that could overlap them. Sources always refer to the original marker table.
    let mut aliases = Vec::new();
    for (ty, sources) in dots.namespaces {
        let mut fields = Vec::new();
        for source in sources {
            let Some((_, names)) = targets.get(&quote!(#source).to_string()) else {
                return Err(syn::Error::new_spanned(
                    source,
                    "unknown dot namespace source",
                ));
            };
            for name in names {
                if !fields.contains(name) {
                    fields.push(name.clone());
                }
            }
        }
        aliases.push((ty, fields));
    }
    let namespaces = targets
        .values()
        .cloned()
        .map(|(ty, fields)| (ty, fields, true))
        .chain(aliases.into_iter().map(|(ty, fields)| (ty, fields, false)))
        .map(|(ty, fields, direct)| {
            let suffix: String = quote!(#ty)
                .to_string()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let name = format_ident!("{}Values", suffix);
            if direct {
                // A type with From impls holds its values, so a parameter of exactly that type
                // works.
                quote! {
                    #[derive(Clone)]
                    pub struct #name { #(pub #fields: #ty),* }
                    impl Default for #name {
                        fn default() -> Self {
                            Self { #(#fields: <#ty as From<#fields>>::from(Dot::NEW)),* }
                        }
                    }
                    impl DotValues for #ty { type Values = #name; }
                }
            } else {
                // The dots themselves, which convert through the parameter's Into/IntoProp.
                quote! {
                    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
                    pub struct #name { #(pub #fields: #fields),* }
                    impl DotValues for #ty { type Values = #name; }
                }
            }
        });
    let fields = dots.entries.iter().map(|e| &e.name);
    Ok(quote! {
        #(#markers)*
        /// All leading-dot markers, for receivers without typed completion helpers.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct Values { #(pub #fields: #fields),* }
        #(#namespaces)*
    })
}
