// #[preview] / #[preview("Name")] / #[preview(root)] fn name() -> impl IntoView { ... }
//
// keeps the function and registers it with the views module (by `inventory`, as ELG_PREVIEW
// registers at static-init time in C++). The registration is `elegance::__preview!`, which only
// preview builds (the `preview` feature) expand, so apps don't carry their previews.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn, LitStr};

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
    let (name, root) = if attr.is_empty() {
        (ident.to_string(), false)
    } else if let Ok(marker) = syn::parse2::<Ident>(attr.clone()) {
        if marker != "root" {
            return Err(syn::Error::new_spanned(marker, "expected a name (`#[preview(\"Name\")]`) or `root`"));
        }
        (ident.to_string(), true)
    } else {
        (syn::parse2::<LitStr>(attr)?.value(), false)
    };
    Ok(quote! {
        // Only previews use it; ordinary builds leave them out.
        #[allow(dead_code)]
        #function

        ::elegance::__preview!(#name, #root, #ident);
    })
}
