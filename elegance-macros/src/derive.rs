// #[derive(DotValues)] on an enum: each unit variant becomes a leading-dot name, in snake_case
// (`TopLeading` is `.top_leading`). It implements DotValues (a namespace whose fields are the
// variants, for typed lookup and completion), From and IntoProp from each name's Dot, so the
// names work wherever the enum is expected. The enum must be Clone: `.name` clones the field.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident};

pub fn expand(input: TokenStream) -> TokenStream {
    match expand_inner(input) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

/// `TopLeading` -> `top_leading`, `HTTPServer` -> `http_server`, `Digit0` -> `digit0`.
fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if prev.is_ascii_lowercase() || prev.is_ascii_digit() || (prev.is_ascii_uppercase() && next_lower) {
                out.push('_');
            }
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn expand_inner(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(&input.ident, "#[derive(DotValues)] works on enums"));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "#[derive(DotValues)] doesn't support generic enums",
        ));
    }
    let name = &input.ident;
    let vis = &input.vis;
    let values = format_ident!("{}DotValues", name);
    let dot = quote!(::elegance::dot);

    let mut fields = Vec::new();
    let mut variants = Vec::new();
    for variant in &data.variants {
        // Only unit variants have a single value to name.
        if !matches!(variant.fields, Fields::Unit) {
            continue;
        }
        let text = snake_case(&variant.ident.to_string());
        if matches!(text.as_str(), "self" | "super" | "crate") {
            return Err(syn::Error::new_spanned(&variant.ident, format!("`.{text}` can't be a name")));
        }
        // Raw, so keywords (`Move` -> `.move`) work as fields too.
        fields.push(Ident::new_raw(&text, variant.ident.span()));
        variants.push(&variant.ident);
    }

    let conversions = fields.iter().zip(&variants).map(|(field, variant)| {
        let ty = crate::dot::name_type(field, &dot);
        quote! {
            impl ::core::convert::From<#ty> for #name {
                fn from(_: #ty) -> #name { #name::#variant }
            }
            impl ::elegance::IntoProp<#name> for #ty {
                fn into_prop(self) -> ::elegance::Prop<#name> { ::elegance::Prop::Value(#name::#variant) }
            }
        }
    });

    Ok(quote! {
        /// The values `.name` can stand for where this enum is expected.
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis struct #values { #(pub #fields: #name),* }

        impl ::core::default::Default for #values {
            fn default() -> Self {
                Self { #(#fields: #name::#variants),* }
            }
        }

        impl #dot::DotValues for #name {
            type Values = #values;
        }

        impl #dot::DotEnum for #name {}

        #(#conversions)*
    })
}

#[cfg(test)]
mod tests {
    use super::snake_case;

    #[test]
    fn variant_names_become_snake_case() {
        assert_eq!(snake_case("Compact"), "compact");
        assert_eq!(snake_case("TopLeading"), "top_leading");
        assert_eq!(snake_case("HTTPServer"), "http_server");
        assert_eq!(snake_case("Digit0"), "digit0");
        assert_eq!(snake_case("F12"), "f12");
        assert_eq!(snake_case("Title2Bold"), "title2_bold");
    }
}
