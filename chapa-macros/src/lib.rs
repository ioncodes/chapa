//! Procedural macro implementation for the `chapa` bitfield crate.
//!
//! This crate is not meant to be used directly. Use the `chapa` crate instead,
//! which re-exports the [`bitfield`] attribute macro and [`BitEnum`] derive
//! macro from here.

mod bit_enum;
mod codegen;
mod model;
mod ordering;
mod parse;
mod validate;

use proc_macro::TokenStream;

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

/// Derive macro that implements `chapa::BitField` for C-like enums.
///
/// Also auto-derives `Copy` and `Clone` (zero-cost for unit-variant enums).
#[proc_macro_derive(BitEnum)]
pub fn bit_enum(input: TokenStream) -> TokenStream {
    match bit_enum_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn bit_enum_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse::<syn::DeriveInput>(input)?;
    bit_enum::generate(input).map(Into::into)
}
