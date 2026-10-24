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

fn generate(dots: Dots) -> syn::Result<TokenStream> {
    let mut targets: BTreeMap<String, (Type, Vec<Ident>)> = BTreeMap::new();
    let markers = dots.entries.iter().map(|entry| {
        let name = &entry.name;
        let conversions = entry.conversions.iter().map(|(ty, value)| {
            quote! {
                impl From<#name> for #ty {
                    fn from(_: #name) -> #ty { #value }
                }
            }
        });
        quote! {
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
            pub struct #name;
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
        .chain(aliases)
        .map(|(ty, fields)| {
            let suffix: String = quote!(#ty)
                .to_string()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let name = format_ident!("{}Values", suffix);
            quote! {
                #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
                pub struct #name { #(pub #fields: #fields),* }
                impl DotValues for #ty { type Values = #name; }
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
