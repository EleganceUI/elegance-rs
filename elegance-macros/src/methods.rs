use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, Item, PathArguments, Signature, Type, TypeParamBound, Visibility,
};

// Extract the conversion target, not whether that target supports shorthand.
// Rust resolves DotValues at the call site, including for aliases and user types.
// Method-local generics cannot be inferred by an argument-free metadata helper;
// impl-level generics, by contrast, are already determined by the receiver.
pub(crate) fn target(ty: &Type, method_generics: &[String]) -> Option<Type> {
    fn mentions_generic(tokens: TokenStream, names: &[String]) -> bool {
        tokens.into_iter().any(|token| match token {
            proc_macro2::TokenTree::Ident(ident) => names.iter().any(|name| ident == name),
            proc_macro2::TokenTree::Group(group) => mentions_generic(group.stream(), names),
            _ => false,
        })
    }
    if mentions_generic(quote!(#ty), method_generics) {
        return None;
    }
    match ty {
        Type::Group(group) => target(&group.elem, method_generics),
        Type::Paren(paren) => target(&paren.elem, method_generics),
        Type::ImplTrait(imp) => imp.bounds.iter().find_map(|bound| {
            let TypeParamBound::Trait(bound) = bound else {
                return None;
            };
            let last = bound.path.segments.last()?;
            // Number's conversion contract is to_f32(); this is an argument
            // protocol, not a list of types eligible for completion.
            if last.ident == "Number" && matches!(last.arguments, PathArguments::None) {
                return Some(syn::parse_quote!(f32));
            }
            if last.ident != "Into" && last.ident != "IntoProp" {
                return None;
            }
            let PathArguments::AngleBracketed(args) = &last.arguments else {
                return None;
            };
            if args.args.len() != 1 {
                return None;
            }
            let Some(GenericArgument::Type(inner)) = args.args.first() else {
                return None;
            };
            if last.ident == "Into" {
                target(inner, method_generics)
            } else {
                Some(syn::parse_quote!(::elegance::prop::Prop<#inner>))
            }
        }),
        Type::Infer(_) | Type::Never(_) => None,
        _ => Some(ty.clone()),
    }
}

pub(crate) fn helper(
    name: &syn::Ident,
    index: usize,
    target: &Type,
    vis: &Visibility,
    attrs: &[syn::Attribute],
) -> TokenStream {
    let name = format_ident!(
        "__elg_dot_{}_{}",
        name.to_string().trim_start_matches("r#"),
        index
    );
    witness_helper(&name, target, vis, attrs)
}

fn witness_helper(
    name: &syn::Ident,
    target: &Type,
    vis: &Visibility,
    attrs: &[syn::Attribute],
) -> TokenStream {
    let cfg = attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"));
    quote! {
        #(#cfg)*
        #[doc(hidden)]
        #[allow(non_snake_case)]
        #vis fn #name(&self) -> ::elegance::dot::DotTarget<#target> {
            ::elegance::dot::DotTarget::new()
        }
    }
}

fn helpers(sig: &Signature, vis: &Visibility, attrs: &[syn::Attribute]) -> Vec<TokenStream> {
    let generics: Vec<_> = sig
        .generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Type(param) => param.ident.to_string(),
            syn::GenericParam::Lifetime(param) => param.lifetime.ident.to_string(),
            syn::GenericParam::Const(param) => param.ident.to_string(),
        })
        .collect();
    let mut helpers: Vec<_> = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(arg) => Some(arg),
            _ => None,
        })
        .enumerate()
        .filter_map(|(index, arg)| {
            target(&arg.ty, &generics).map(|ty| helper(&sig.ident, index, &ty, vis, attrs))
        })
        .collect();
    if let Some(base) = sig
        .ident
        .to_string()
        .trim_start_matches("r#")
        .strip_suffix("_with")
        && let Some(FnArg::Typed(arg)) = sig.inputs.last()
            && let Some(options) = options_target(&arg.ty, &generics) {
                let name = format_ident!("__elg_options_{base}");
                helpers.push(witness_helper(&name, &options, vis, attrs));
            }
    helpers
}

// Named-option metadata follows the closure protocol, not a list of modifiers
// or option types. Other method generics do not affect a concrete options type.
fn options_target(ty: &Type, method_generics: &[String]) -> Option<Type> {
    match ty {
        Type::Group(group) => options_target(&group.elem, method_generics),
        Type::Paren(paren) => options_target(&paren.elem, method_generics),
        Type::ImplTrait(ty) => ty.bounds.iter().find_map(|bound| {
            let TypeParamBound::Trait(bound) = bound else {
                return None;
            };
            let segment = bound.path.segments.last()?;
            if segment.ident != "FnOnce" {
                return None;
            }
            let PathArguments::Parenthesized(args) = &segment.arguments else {
                return None;
            };
            if args.inputs.len() != 1 {
                return None;
            }
            let input = args.inputs.first()?;
            let syn::ReturnType::Type(_, output) = &args.output else {
                return None;
            };
            if quote!(#input).to_string() != quote!(#output).to_string() {
                return None;
            }
            target(input, method_generics)
        }),
        _ => None,
    }
}

fn constructor_impl(imp: &syn::ItemImpl) -> syn::Result<TokenStream> {
    let method = imp
        .items
        .iter()
        .find_map(|item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == "new" => Some(method),
            _ => None,
        })
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &imp.self_ty,
                "#[elg_builder(constructor)] requires fn new() -> Self in this impl",
            )
        })?;
    let sig = &method.sig;
    let error = |message| syn::Error::new_spanned(sig, message);
    if !sig.inputs.is_empty() || sig.variadic.is_some() {
        return Err(error(
            "#[elg_builder(constructor)] requires a zero-argument constructor: fn new() -> Self",
        ));
    }
    if sig.asyncness.is_some() {
        return Err(error("#[elg_builder(constructor)] requires a synchronous constructor; async fn new is not supported"));
    }
    if sig.unsafety.is_some() {
        return Err(error("#[elg_builder(constructor)] requires a safe constructor; unsafe fn new is not supported"));
    }
    if !sig.generics.params.is_empty() {
        return Err(error(
            "#[elg_builder(constructor)] requires fn new without method generic parameters",
        ));
    }
    if !matches!(&sig.output, syn::ReturnType::Type(_, ty)
        if matches!(ty.as_ref(), Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self")))
    {
        return Err(error(
            "#[elg_builder(constructor)] requires fn new() -> Self (return type must be Self)",
        ));
    }
    let cfg = imp
        .attrs
        .iter()
        .chain(&method.attrs)
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"));
    let ty = &imp.self_ty;
    let (generics, _, where_clause) = imp.generics.split_for_impl();
    Ok(quote! {
        #(#cfg)*
        impl #generics ::elegance::dsl::ElgNew<()> for #ty #where_clause {
            type Out = Self;
            fn elg_new(_: ()) -> Self::Out {
                Self::new()
            }
        }
    })
}

pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_inner(attr, item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_inner(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let constructor = !attr.is_empty();
    if constructor && syn::parse2::<syn::Ident>(attr.clone())? != "constructor" {
        return Err(syn::Error::new_spanned(
            attr,
            "expected #[elg_builder] or #[elg_builder(constructor)]",
        ));
    }
    let item = syn::parse2::<Item>(item)?;
    if constructor && !matches!(&item, Item::Impl(imp) if imp.trait_.is_none()) {
        return Err(syn::Error::new_spanned(
            item,
            "#[elg_builder(constructor)] expects an inherent impl",
        ));
    }
    match item {
        Item::Impl(mut imp) if imp.trait_.is_none() => {
            let registration = if constructor {
                constructor_impl(&imp)?
            } else {
                TokenStream::new()
            };
            let mut additions = Vec::new();
            for item in &imp.items {
                if let syn::ImplItem::Fn(method) = item
                    && matches!(method.vis, Visibility::Public(_)) {
                        for tokens in helpers(&method.sig, &method.vis, &method.attrs) {
                            additions.push(syn::parse2::<syn::ImplItem>(tokens)?);
                        }
                    }
            }
            imp.items.extend(additions);
            Ok(quote!(#imp #registration))
        }
        Item::Trait(mut tr) => {
            let mut additions = Vec::new();
            for item in &tr.items {
                if let syn::TraitItem::Fn(method) = item {
                    for tokens in helpers(&method.sig, &Visibility::Inherited, &method.attrs) {
                        additions.push(syn::parse2::<syn::TraitItem>(tokens)?);
                    }
                }
            }
            tr.items.extend(additions);
            Ok(quote!(#tr))
        }
        item => Err(syn::Error::new_spanned(
            item,
            "#[elg_builder] expects an inherent impl or trait",
        )),
    }
}
