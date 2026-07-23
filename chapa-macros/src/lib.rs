//! Procedural macro implementation for the `chapa` bitfield crate.
//!
//! This crate is not meant to be used directly. Use the `chapa` crate instead,
//! which re-exports the [`bitfield`] and [`bitenum`] attribute macros from
//! here.

mod bit_enum;
mod codegen;
mod model;
mod ordering;
mod parse;
mod validate;

use proc_macro::TokenStream;
use syn::spanned::Spanned;

use model::BitfieldArgs;

/// The `#[bitfield]` attribute macro.
///
/// Transforms an annotated struct into a zero-overhead bitfield backed by a
/// single primitive integer. See the `chapa` crate documentation for full
/// usage examples and available options.
#[proc_macro_attribute]
pub fn bitfield(attr: TokenStream, item: TokenStream) -> TokenStream {
    match bitfield_impl(attr, item) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

/// Parses the attribute + struct, validates semantics, and drives code generation.
fn bitfield_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = syn::parse::<BitfieldArgs>(attr)?;
    let item_struct = syn::parse::<syn::ItemStruct>(item)?;

    let def = parse::parse_struct(&args, &item_struct)?;
    validate::validate(&def)?;
    let output = codegen::generate(&def);

    Ok(output.into())
}

/// The `#[bitenum]` attribute macro.
///
/// Implements `chapa::BitField` for a C-like enum. The enum must mark exactly
/// one variant `#[fallback]`. That variant is returned by `from_raw` for any
/// raw value matching no discriminant. Use `try_from_raw`/`TryFrom` to detect
/// such values instead of coercing them.
#[proc_macro_attribute]
pub fn bitenum(attr: TokenStream, item: TokenStream) -> TokenStream {
    match bitenum_impl(attr, item) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn bitenum_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::TokenStream::from(attr).span(),
            "bitenum takes no arguments",
        ));
    }
    let item_enum = syn::parse::<syn::ItemEnum>(item)?;
    bit_enum::generate(item_enum).map(Into::into)
}
