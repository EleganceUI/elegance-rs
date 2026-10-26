// #[component] fn Name(props...) -> impl IntoView { body }
//
// becomes a struct `Name` whose props are set by builder methods (named arguments in elg!),
// and which is a Component once every required prop is set. Required props are type
// parameters that start as `Missing`, so a missing one is a compile error.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parser, punctuated::Punctuated, FnArg, ItemFn, Pat, Token, Type};

struct Prop {
    name: syn::Ident,
    ty: Type,
    default: Option<TokenStream>,
}

/// `Prop<T>` -> `T`: such a prop is reactive, set from anything `IntoProp<T>` takes.
fn prop_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else { return None };
    match (last.ident == "Prop", args.args.len(), args.args.first()) {
        (true, 1, Some(syn::GenericArgument::Type(inner))) => Some(inner),
        _ => None,
    }
}

/// The setter's parameter type and the conversion to the prop's type.
fn setter_input(ty: &Type) -> (TokenStream, TokenStream) {
    match prop_inner(ty) {
        Some(inner) => (
            quote!(impl ::elegance::IntoProp<#inner>),
            quote!(::elegance::IntoProp::into_prop(value)),
        ),
        None => (quote!(impl ::std::convert::Into<#ty>), quote!(value.into())),
    }
}

fn option_inner(ty: &Type) -> bool {
    if let Type::Path(path) = ty
        && let Some(last) = path.path.segments.last() {
            return last.ident == "Option"
                && matches!(last.arguments, syn::PathArguments::AngleBracketed(_));
        }
    false
}

// `#[prop(default)]` or `#[prop(default = expr)]`, where `expr` may use the DSL's leading-dot
// names: `#[prop(default = .compact)]`, `#[prop(default = Font::system(12.0, .bold))]`.
fn prop_default(attrs: &mut Vec<syn::Attribute>, ty: &Type) -> syn::Result<Option<TokenStream>> {
    let mut default = None;
    let mut error = None;
    attrs.retain(|attr| {
        if !attr.path().is_ident("prop") {
            return true;
        }
        let result = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                if meta.input.peek(Token![=]) {
                    let tokens: TokenStream = meta.value()?.parse()?;
                    let items: Vec<_> = tokens.clone().into_iter().collect();
                    let dot_name = match items.as_slice() {
                        [proc_macro2::TokenTree::Punct(dot), proc_macro2::TokenTree::Ident(name)]
                            if dot.as_char() == '.' =>
                        {
                            Some(crate::rewrite::dot_value(name, None))
                        }
                        _ => None,
                    };
                    let is_dot = dot_name.is_some();
                    let value = match dot_name {
                        Some(value) => value,
                        None => {
                            let rewritten = crate::rewrite::rewrite(tokens);
                            syn::parse2::<syn::Expr>(rewritten.clone())?;
                            rewritten
                        }
                    };
                    // Converted as the setter converts, so `default = "Untitled"` works for a
                    // `Prop<String>`; a lone `.name` converts into the prop's type, known here.
                    default = Some(match (prop_inner(ty), is_dot) {
                        (Some(inner), _) => quote!(::elegance::IntoProp::<#inner>::into_prop(#value)),
                        (None, true) => quote!(<#ty as ::std::convert::From<_>>::from(#value)),
                        (None, false) => value,
                    });
                } else {
                    default = Some(quote!(::std::default::Default::default()));
                }
                Ok(())
            } else {
                Err(meta.error("expected `default` or `default = ...`"))
            }
        });
        if let Err(e) = result {
            error = Some(e);
        }
        false
    });
    match error {
        Some(e) => Err(e),
        None => Ok(default),
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_inner(attr, item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_inner(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        let _ = Punctuated::<syn::Meta, Token![,]>::parse_terminated.parse2(attr.clone())?;
        return Err(syn::Error::new_spanned(
            attr,
            "#[component] takes no arguments",
        ));
    }
    let mut function: ItemFn = syn::parse2(item)?;
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &function.sig.generics,
            "components can't be generic",
        ));
    }
    if matches!(function.sig.output, syn::ReturnType::Default) {
        return Err(syn::Error::new_spanned(
            &function.sig,
            "a component returns its view: `-> impl IntoView`",
        ));
    }

    let mut props = Vec::new();
    for input in function.sig.inputs.iter_mut() {
        let FnArg::Typed(typed) = input else {
            return Err(syn::Error::new_spanned(
                input,
                "components are functions, not methods",
            ));
        };
        let default = prop_default(&mut typed.attrs, &typed.ty)?;
        let Pat::Ident(pat) = typed.pat.as_ref() else {
            return Err(syn::Error::new_spanned(
                &typed.pat,
                "a prop is a plain name: `title: String`",
            ));
        };
        let ty = (*typed.ty).clone();
        let default =
            default.or_else(|| option_inner(&ty).then(|| quote!(::std::option::Option::None)));
        props.push(Prop {
            name: pat.ident.clone(),
            ty,
            default,
        });
    }

    let vis = function.vis.clone();
    let name = function.sig.ident.clone();
    let body_fn = format_ident!("__elg_{}_body", name);
    let docs: Vec<&syn::Attribute> = function
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .collect();
    let docs: Vec<syn::Attribute> = docs.into_iter().cloned().collect();
    function.attrs.retain(|a| !a.path().is_ident("doc"));
    function.sig.ident = body_fn.clone();
    function.vis = syn::Visibility::Inherited;

    let private = quote!(::elegance::__private);

    if props.is_empty() {
        return Ok(quote! {
            #[allow(non_snake_case)]
            #function

            #(#docs)*
            #[derive(Clone, Copy, Debug, Default)]
            #vis struct #name;

            impl #private::ElgNew<()> for #name {
                type Out = #name;
                fn elg_new(_: ()) -> #name {
                    #name
                }
            }

            impl #private::Component for #name {
                fn body(&self) -> impl #private::IntoView {
                    #body_fn()
                }
            }
        });
    }

    // One type parameter per required prop.
    let params: Vec<syn::Ident> = props
        .iter()
        .enumerate()
        .filter(|(_, p)| p.default.is_none())
        .map(|(i, _)| format_ident!("__P{}", i))
        .collect();
    let param_of = |i: usize| -> Option<syn::Ident> {
        props[i]
            .default
            .is_none()
            .then(|| format_ident!("__P{}", i))
    };

    let fields = props.iter().enumerate().map(|(i, p)| {
        let field = &p.name;
        match param_of(i) {
            Some(param) => quote!(#field: #param),
            None => {
                let ty = &p.ty;
                quote!(#field: #ty)
            }
        }
    });
    let defaults = params.iter().map(|p| quote!(#p = #private::Missing));
    let missing = params.iter().map(|_| quote!(#private::Missing));
    let missing2 = missing.clone();
    let initial = props.iter().map(|p| {
        let field = &p.name;
        match &p.default {
            Some(default) => quote!(#field: #default),
            None => quote!(#field: #private::Missing),
        }
    });

    let setters = props.iter().enumerate().map(|(i, p)| {
        let field = &p.name;
        let ty = &p.ty;
        let (input, convert) = setter_input(ty);
        let moved = props.iter().filter(|q| q.name != p.name).map(|q| {
            let f = &q.name;
            quote!(#f: self.#f)
        });
        match param_of(i) {
            Some(_) => {
                let result_params = props.iter().enumerate().filter_map(|(j, _)| {
                    param_of(j).map(|param| if j == i { quote!(#ty) } else { quote!(#param) })
                });
                quote! {
                    #vis fn #field(self, value: #input) -> #name<#(#result_params),*> {
                        #name { #field: #convert, #(#moved),* }
                    }
                }
            }
            None => quote! {
                #vis fn #field(mut self, value: #input) -> Self {
                    self.#field = #convert;
                    self
                }
            },
        }
    });

    let dot_helpers = props.iter().filter_map(|p| {
        crate::methods::target(&p.ty, &[])
            .map(|target| crate::methods::helper(&p.name, 0, &target, &vis, &[]))
    });

    let complete = props
        .iter()
        .enumerate()
        .filter_map(|(i, p)| param_of(i).map(|_| &p.ty));
    let args = props.iter().map(|p| {
        let field = &p.name;
        quote!(::std::clone::Clone::clone(&self.#field))
    });

    Ok(quote! {
        #[allow(non_snake_case)]
        #function

        #(#docs)*
        #vis struct #name<#(#defaults),*> {
            #(#fields),*
        }

        impl #name<#(#missing),*> {
            /// A builder for this component: set its props, then use it as a view.
            #vis fn new() -> Self {
                Self { #(#initial),* }
            }
        }

        impl #private::ElgNew<()> for #name<#(#missing2),*> {
            type Out = Self;
            fn elg_new(_: ()) -> Self {
                Self::new()
            }
        }

        impl<#(#params),*> #name<#(#params),*> {
            #(#setters)*
            #(#dot_helpers)*
        }

        impl #private::Component for #name<#(#complete),*> {
            fn body(&self) -> impl #private::IntoView {
                #body_fn(#(#args),*)
            }
        }
    })
}
