//! Derive macro for implementing [`chapa::BitField`] on C-like enums.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Expr, Fields, Lit};

use crate::model::StorageKind;

/// Generates the `BitField` and `TryFrom` impls for a C-like enum.
///
/// The enum is expected to derive `Copy` + `Clone` itself (required by the
/// `BitField: Copy` bound) and to mark exactly one variant `#[fallback]`. That
/// variant is returned by `from_raw` for any storage value matching no
/// discriminant; `try_from_raw`/`TryFrom` report such values as
/// [`chapa::InvalidBitPattern`] instead of coercing them.
pub fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let variants = match &input.data {
        Data::Enum(e) => &e.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "BitEnum can only be derived for enums",
            ))
        }
    };

    if variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "BitEnum requires at least one variant",
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
                "BitEnum only supports unit variants (no fields)",
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
                    "BitEnum: only one variant may be marked #[fallback]",
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
            &input.ident,
            "BitEnum requires exactly one variant marked #[fallback]; it is \
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
        .ok_or_else(|| syn::Error::new_spanned(&input.ident, "discriminant value exceeds u128"))?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let storage_ident = format_ident!("{}", storage.ident());

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
                "BitEnum discriminants must be integer literals",
            )),
        },
        _ => Err(syn::Error::new(
            expr.span(),
            "BitEnum discriminants must be integer literals",
        )),
    }
}
