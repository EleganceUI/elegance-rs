// #[preview] / #[preview("Name")] / #[preview(root)] / #[preview(root, app_hash = EXPR)]
// fn name() -> impl IntoView { ... }
//
// keeps the function and registers it with the views module (by `inventory`, as ELG_PREVIEW
// registers at static-init time in C++). The registration is `elegance::__preview!`, which only
// preview builds (the `preview` feature) expand, so apps don't carry their previews.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, ItemFn, LitStr, Token};

// The attribute: nothing, a name, or `root` with an optional `app_hash = EXPR`.
enum Attr {
    Named(Option<LitStr>),
    Root(Option<Expr>),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Attr::Named(None));
        }
        if input.peek(LitStr) {
            return Ok(Attr::Named(Some(input.parse()?)));
        }
        let marker: Ident = input.parse()?;
        if marker != "root" {
            return Err(syn::Error::new_spanned(marker, "expected a name (`#[preview(\"Name\")]`) or `root`"));
        }
        if input.is_empty() {
            return Ok(Attr::Root(None));
        }
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        if key != "app_hash" {
            return Err(syn::Error::new_spanned(key, "expected `app_hash = ...`"));
        }
        input.parse::<Token![=]>()?;
        let hash: Expr = input.parse()?;
        input.parse::<Option<Token![,]>>()?;
        Ok(Attr::Root(Some(hash)))
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_inner(attr, item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_inner(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let function: ItemFn = syn::parse2(item)?;
    if !function.sig.inputs.is_empty() || !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(&function.sig, "a preview takes no arguments: `fn name() -> impl IntoView`"));
    }
    let ident = &function.sig.ident;
    let (name, root, app_hash) = match syn::parse2::<Attr>(attr)? {
        Attr::Named(name) => (name.map_or_else(|| ident.to_string(), |n| n.value()), false, quote!(0)),
        Attr::Root(hash) => (ident.to_string(), true, hash.map_or_else(|| quote!(0), |h| quote!(#h))),
    };
    Ok(quote! {
        // Only previews use it; ordinary builds leave them out.
        #[allow(dead_code)]
        #function

        ::elegance::__preview!(#name, #root, #app_hash, #ident);
    })
}
