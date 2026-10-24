// #[preview] / #[preview("Name")] fn name() -> impl IntoView { ... }
//
// keeps the function and registers it with the module's previews (by `inventory`, as
// ELG_PREVIEW registers at static-init time in C++).

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemFn, LitStr};

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
    let name = if attr.is_empty() { ident.to_string() } else { syn::parse2::<LitStr>(attr)?.value() };
    let body = format_ident!("__elg_preview_{}", ident);
    Ok(quote! {
        #function

        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #body() -> ::elegence::View {
            ::elegence::IntoView::into_view(#ident())
        }

        ::elegence::__private::inventory::submit! {
            ::elegence::__private::PreviewItem { name: #name, body: #body, file: ::std::file!(), line: ::std::line!() }
        }
    })
}
