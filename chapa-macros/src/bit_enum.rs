//! Attribute macro for implementing [`chapa::BitField`] on C-like enums.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, Expr, Fields, ItemEnum, Lit};

use crate::model::StorageKind;

/// Generates the annotated enum plus `Copy`, `Clone`, `BitField`, and `TryFrom`
/// impls for it.
///
/// The enum must be C-like (unit variants only) and mark exactly one variant
/// `#[fallback]`. That variant is returned by `from_raw` for any storage value
/// matching no discriminant; `try_from_raw`/`TryFrom` report such values as
/// [`chapa::InvalidBitPattern`] instead of coercing them. Copy and Clone are
/// implemented automatically (required by the `BitField: Copy` bound).
pub fn generate(item: ItemEnum) -> syn::Result<TokenStream> {
    let variants = &item.variants;

    if variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "bitenum requires at least one variant",
        ));
    }

    // Resolve discriminant values and locate the `#[fallback]` variant.
    let mut resolved: Vec<(&syn::Ident, u128)> = Vec::with_capacity(variants.len());
    let mut next_discrim: u128 = 0;
    let mut fallback: Option<&syn::Ident> = None;

    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "bitenum only supports unit variants (no fields)",
            ));
        }

        if variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("fallback"))
        {
            if fallback.is_some() {
                return Err(syn::Error::new_spanned(
                    variant,
                    "bitenum: only one variant may be marked #[fallback]",
                ));
            }
            fallback = Some(&variant.ident);
        }

        let value = match &variant.discriminant {
            Some((_, expr)) => parse_discriminant(expr)?,
            None => next_discrim,
        };

        resolved.push((&variant.ident, value));
        next_discrim = value + 1;
    }

    let fallback = fallback.ok_or_else(|| {
        syn::Error::new_spanned(
            &item.ident,
            "bitenum requires exactly one variant marked #[fallback]; it is \
             returned by `from_raw` for unrecognized values (use \
             `try_from_raw`/`TryFrom` to detect them instead of coercing)",
        )
    })?;

    let max_val = resolved.iter().map(|(_, v)| *v).max().unwrap();
    let bits_needed = if max_val == 0 {
        1
    } else {
        u128::BITS - max_val.leading_zeros()
    };
    let storage = StorageKind::smallest_fitting(bits_needed)
        .ok_or_else(|| syn::Error::new_spanned(&item.ident, "discriminant value exceeds u128"))?;

    // Remove derives that the macro implements itself and forward the rest.
    // The derive attribute is forwarded even when no traits remain (an empty
    // `#[derive()]` is valid) so the original `derive` path span survives into
    // the output and IDEs keep highlighting the attribute.
    let mut copy_span = None;
    let mut clone_span = None;
    let mut user_attrs: Vec<TokenStream> = Vec::new();
    for attr in &item.attrs {
        if attr.path().is_ident("derive") {
            let mut kept: Vec<syn::Path> = Vec::new();
            let _ = attr.parse_nested_meta(|meta| {
                let trait_name = meta.path.segments.last().map(|segment| &segment.ident);
                if trait_name.is_some_and(|ident| ident == "Copy") {
                    copy_span = Some(meta.path.span());
                } else if trait_name.is_some_and(|ident| ident == "Clone") {
                    clone_span = Some(meta.path.span());
                } else {
                    kept.push(meta.path.clone());
                }
                Ok(())
            });
            let path = attr.path();
            user_attrs.push(quote! { #[#path(#(#kept),*)] });
        } else {
            user_attrs.push(quote! { #attr });
        }
    }

    // Re-emit the enum with `#[fallback]` markers stripped; nothing consumes
    // them downstream and they are not registered inert attributes.
    let cleaned_variants = variants.iter().map(|variant| {
        let attrs = variant
            .attrs
            .iter()
            .filter(|attr| !attr.path().is_ident("fallback"));
        let ident = &variant.ident;
        let discriminant = variant
            .discriminant
            .as_ref()
            .map(|(eq, expr)| quote! { #eq #expr });
        quote! { #(#attrs)* #ident #discriminant }
    });

    let vis = &item.vis;
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let generics = &item.generics;
    let storage_ident = format_ident!("{}", storage.unsigned_ident());

    let enum_def = quote! {
        #(#user_attrs)*
        #vis enum #name #generics {
            #(#cleaned_variants),*
        }
    };

    // BitField requires Copy, so every bitenum implements Copy and Clone.
    let copy_impl = {
        let span = copy_span.unwrap_or_else(proc_macro2::Span::call_site);
        quote_spanned! { span =>
            impl #impl_generics ::core::marker::Copy for #name #ty_generics #where_clause {}
        }
    };
    let clone_impl = {
        let span = clone_span.unwrap_or_else(proc_macro2::Span::call_site);
        quote_spanned! { span =>
            impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
                #[inline(always)]
                fn clone(&self) -> Self {
                    *self
                }
            }
        }
    };

    let operand_impl = quote! {
        impl #impl_generics ::chapa::BitOperand<#storage_ident> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn into_storage(self) -> #storage_ident {
                self as #storage_ident
            }
        }
    };

    let from_raw_arms = resolved.iter().map(|(ident, value)| {
        let lit = syn::LitInt::new(&value.to_string(), ident.span());
        quote! { #lit => #name::#ident, }
    });

    let try_from_raw_arms = resolved.iter().map(|(ident, value)| {
        let lit = syn::LitInt::new(&value.to_string(), ident.span());
        quote! { #lit => ::core::result::Result::Ok(#name::#ident), }
    });

    // Reflection metadata, emitted only under the `reflection` feature. Both
    // branches always compile, so `resolved` is never unused.
    let reflection_impl = if cfg!(feature = "reflection") {
        let name_str = name.to_string();
        let variants = resolved.iter().map(|(ident, value)| {
            let vstr = ident.to_string();
            let lit = proc_macro2::Literal::u128_unsuffixed(*value);
            quote! { (#lit, #vstr) }
        });
        quote! {
            impl #impl_generics ::chapa::Reflect for #name #ty_generics #where_clause {
                const REFLECT: ::chapa::FieldKind = ::chapa::FieldKind::Enum(&::chapa::EnumInfo {
                    name: #name_str,
                    variants: &[ #(#variants),* ],
                });
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #enum_def

        #copy_impl
        #clone_impl
        #operand_impl

        impl #impl_generics ::chapa::BitField for #name #ty_generics #where_clause {
            type Storage = #storage_ident;
            // Enums have no bit ordering; IS_MSB0 is meaningless here but required by the trait.
            const IS_MSB0: bool = false;

            #[inline(always)]
            fn from_raw(raw: #storage_ident) -> Self {
                match raw {
                    #(#from_raw_arms)*
                    _ => #name::#fallback,
                }
            }

            #[inline(always)]
            fn try_from_raw(
                raw: #storage_ident,
            ) -> ::core::result::Result<Self, ::chapa::InvalidBitPattern<#storage_ident>> {
                match raw {
                    #(#try_from_raw_arms)*
                    other => ::core::result::Result::Err(::chapa::InvalidBitPattern::new(other)),
                }
            }

            #[inline(always)]
            fn raw(&self) -> #storage_ident {
                *self as #storage_ident
            }
        }

        impl #impl_generics ::core::convert::TryFrom<#storage_ident> for #name #ty_generics #where_clause {
            type Error = ::chapa::InvalidBitPattern<#storage_ident>;

            #[inline(always)]
            fn try_from(raw: #storage_ident) -> ::core::result::Result<Self, Self::Error> {
                <Self as ::chapa::BitField>::try_from_raw(raw)
            }
        }

        #reflection_impl
    })
}

/// Extracts a `u128` value from a discriminant expression.
/// Only supports integer literals.
fn parse_discriminant(expr: &Expr) -> syn::Result<u128> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(int) => int
                .base10_parse::<u128>()
                .map_err(|e| syn::Error::new(int.span(), format!("invalid discriminant: {e}"))),
            _ => Err(syn::Error::new(
                lit.span(),
                "bitenum discriminants must be integer literals",
            )),
        },
        _ => Err(syn::Error::new(
            expr.span(),
            "bitenum discriminants must be integer literals",
        )),
    }
}
