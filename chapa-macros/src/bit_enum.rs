//! Derive macro for implementing [`chapa::BitField`] on C-like enums.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Expr, Fields, Lit};

use crate::model::StorageKind;

/// Generates `Copy`, `Clone`, and `BitField` impls for a C-like enum.
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

    // Resolve discriminant values.
    let mut resolved: Vec<(&syn::Ident, u128)> = Vec::with_capacity(variants.len());
    let mut next_discrim: u128 = 0;

    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "BitEnum only supports unit variants (no fields)",
            ));
        }

        let value = match &variant.discriminant {
            Some((_, expr)) => parse_discriminant(expr)?,
            None => next_discrim,
        };

        resolved.push((&variant.ident, value));
        next_discrim = value + 1;
    }

    let max_val = resolved.iter().map(|(_, v)| *v).max().unwrap();
    let bits_needed = if max_val == 0 { 1 } else { u128::BITS - max_val.leading_zeros() };
    let storage = StorageKind::smallest_fitting(bits_needed).ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "discriminant value exceeds u128")
    })?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let storage_ident = format_ident!("{}", storage.ident());

    let from_raw_arms = resolved.iter().map(|(ident, value)| {
        let lit = syn::LitInt::new(&value.to_string(), ident.span());
        quote! { #lit => #name::#ident, }
    });

    let last_variant = &resolved.last().unwrap().0;

    Ok(quote! {
        impl #impl_generics ::core::marker::Copy for #name #ty_generics #where_clause {}
        impl #impl_generics ::core::clone::Clone for #name #ty_generics #where_clause {
            #[inline(always)]
            fn clone(&self) -> Self { *self }
        }

        impl #impl_generics ::chapa::BitField for #name #ty_generics #where_clause {
            type Storage = #storage_ident;

            #[inline(always)]
            fn from_raw(raw: #storage_ident) -> Self {
                match raw {
                    #(#from_raw_arms)*
                    _ => #name::#last_variant,
                }
            }

            #[inline(always)]
            fn raw(&self) -> #storage_ident {
                *self as #storage_ident
            }
        }
    })
}

/// Extracts a `u128` value from a discriminant expression.
/// Only supports integer literals.
fn parse_discriminant(expr: &Expr) -> syn::Result<u128> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(int) => int.base10_parse::<u128>().map_err(|e| {
                syn::Error::new(int.span(), format!("invalid discriminant: {e}"))
            }),
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
