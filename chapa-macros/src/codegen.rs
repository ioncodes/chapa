//! Code generation for the `#[bitfield]` macro output.
//!
//! [`generate`] turns a fully validated [`BitfieldDef`] into a [`TokenStream`]
//! containing the newtype struct, associated constants, accessor methods, and
//! trait impls.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::model::*;
use crate::ordering;

/// Generates the complete output `TokenStream` for a bitfield struct.
///
/// Emits:
/// - A `#[repr(transparent)]` newtype wrapping the storage type.
/// - `{FIELD}_SHIFT` and `{FIELD}_MASK` associated constants for every field.
/// - `new()`, `from_raw()`, and `raw()` inherent methods.
/// - `field()` getter, `set_field()` setter, and `with_field()` builder for each
///   non-readonly field; only the getter for readonly fields.
/// - Alias methods for every `alias = ...` annotation.
/// - `BitField`, `From<Storage>`, and `From<Self>` trait impls.
pub fn generate(def: &BitfieldDef) -> TokenStream {
    let vis = &def.vis;
    let name = &def.name;
    let storage_ident = format_ident!("{}", def.args.storage.ident());

    // If the user wrote `#[derive(Debug)]`, strip it and generate our own impl.
    // If they didn't, no Debug impl is emitted at all.
    let (user_derived_debug, filtered_attrs) = strip_debug_derive(&def.user_attrs);

    // Generate struct
    let struct_def = quote! {
        #(#filtered_attrs)*
        #[repr(transparent)]
        #vis struct #name(#storage_ident);
    };

    // Generate Copy + Clone impls are typically from derive, but user provides via attrs.
    // Generate associated consts and methods
    let mut consts = Vec::new();
    let mut methods = Vec::new();

    for field in &def.fields {
        let phys = ordering::compute(def.args.order, &field.range, def.effective_width);

        let accessor = &field.accessor_name;
        let shift_name = format_ident!("{}_SHIFT", accessor.to_uppercase());
        let mask_name = format_ident!("{}_MASK", accessor.to_uppercase());

        let shift_val = phys.shift;
        let mask_val = phys.mask;

        // Const for mask needs to be storage-typed
        let mask_literal = storage_mask_literal(def.args.storage, mask_val);

        let is_underscore_prefixed = field.name.to_string().starts_with('_');
        let maybe_allow_dead_code = if is_underscore_prefixed {
            quote! { #[allow(dead_code)] }
        } else {
            quote! {}
        };

        consts.push(quote! {
            #maybe_allow_dead_code
            #vis const #shift_name: u32 = #shift_val;
            #maybe_allow_dead_code
            #vis const #mask_name: #storage_ident = #mask_literal;
        });

        let getter_name = format_ident!("{}", accessor);
        let getter_doc = format!("Returns the `{}` field (bits {}..={}).", accessor, field.range.start, field.range.end);
        let field_width = phys.field_width;

        // Generate getter
        let getter_body = match &field.ty {
            FieldType::Bool => {
                quote! { (self.0 & Self::#mask_name) != 0 }
            }
            FieldType::Primitive(sk) => {
                let field_ty = format_ident!("{}", sk.ident());
                quote! { ((self.0 >> Self::#shift_name) & ((1 << #field_width) - 1)) as #field_ty }
            }
            FieldType::Nested(ty) => {
                let nested_storage = StorageKind::smallest_fitting(field_width)
                    .unwrap_or(StorageKind::U128);
                let nested_storage_ident = format_ident!("{}", nested_storage.ident());
                quote! {
                    let bits = ((self.0 >> Self::#shift_name) & ((1 << #field_width) - 1)) as #nested_storage_ident;
                    <#ty as ::chapa::BitField>::from_raw(bits)
                }
            }
        };

        let return_ty = match &field.ty {
            FieldType::Bool => quote! { bool },
            FieldType::Primitive(sk) => {
                let ty = format_ident!("{}", sk.ident());
                quote! { #ty }
            }
            FieldType::Nested(ty) => quote! { #ty },
        };

        // Nested-type getters call BitField::from_raw which is not const.
        let getter_tokens = if matches!(field.ty, FieldType::Nested(_)) {
            quote! {
                #[doc = #getter_doc]
                #[inline(always)]
                #vis fn #getter_name(&self) -> #return_ty {
                    #getter_body
                }
            }
        } else {
            quote! {
                #[doc = #getter_doc]
                #[inline(always)]
                #vis const fn #getter_name(&self) -> #return_ty {
                    #getter_body
                }
            }
        };
        methods.push(getter_tokens);

        // Generate setter and with_* (unless readonly)
        if !field.readonly {
            let setter_name = format_ident!("set_{}", accessor);
            let with_name = format_ident!("with_{}", accessor);
            let setter_doc = format!("Sets the `{}` field (bits {}..={}).", accessor, field.range.start, field.range.end);
            let with_doc = format!("Returns a copy with the `{}` field set (bits {}..={}).", accessor, field.range.start, field.range.end);

            let param_ty = match &field.ty {
                FieldType::Bool => quote! { bool },
                FieldType::Primitive(sk) => {
                    let ty = format_ident!("{}", sk.ident());
                    quote! { #ty }
                }
                FieldType::Nested(ty) => quote! { #ty },
            };

            let setter_body = match &field.ty {
                FieldType::Bool => {
                    quote! {
                        if val {
                            self.0 |= Self::#mask_name;
                        } else {
                            self.0 &= !Self::#mask_name;
                        }
                    }
                }
                FieldType::Primitive(_) => {
                    quote! {
                        self.0 = (self.0 & !Self::#mask_name) | (((val as #storage_ident) << Self::#shift_name) & Self::#mask_name);
                    }
                }
                FieldType::Nested(ty) => {
                    quote! {
                        let raw = <#ty as ::chapa::BitField>::raw(&val) as #storage_ident;
                        self.0 = (self.0 & !Self::#mask_name) | ((raw << Self::#shift_name) & Self::#mask_name);
                    }
                }
            };

            let with_body = match &field.ty {
                FieldType::Bool => {
                    quote! {
                        if val {
                            self.0 |= Self::#mask_name;
                        } else {
                            self.0 &= !Self::#mask_name;
                        }
                    }
                }
                FieldType::Primitive(_) => {
                    quote! {
                        self.0 = (self.0 & !Self::#mask_name) | (((val as #storage_ident) << Self::#shift_name) & Self::#mask_name);
                    }
                }
                FieldType::Nested(ty) => {
                    quote! {
                        let raw = <#ty as ::chapa::BitField>::raw(&val) as #storage_ident;
                        self.0 = (self.0 & !Self::#mask_name) | ((raw << Self::#shift_name) & Self::#mask_name);
                    }
                }
            };

            methods.push(quote! {
                #[doc = #setter_doc]
                #[inline(always)]
                #vis fn #setter_name(&mut self, val: #param_ty) {
                    #setter_body
                }
            });

            // Nested with_* calls BitField::raw which is not const.
            let with_tokens = if matches!(field.ty, FieldType::Nested(_)) {
                quote! {
                    #[doc = #with_doc]
                    #[inline(always)]
                    #[must_use]
                    #vis fn #with_name(mut self, val: #param_ty) -> Self {
                        #with_body
                        self
                    }
                }
            } else {
                quote! {
                    #[doc = #with_doc]
                    #[inline(always)]
                    #[must_use]
                    #vis const fn #with_name(mut self, val: #param_ty) -> Self {
                        #with_body
                        self
                    }
                }
            };
            methods.push(with_tokens);

            // Generate aliases
            for alias in &field.aliases {
                let alias_getter = format_ident!("{}", alias);
                let alias_setter = format_ident!("set_{}", alias);
                let alias_with = format_ident!("with_{}", alias);
                let doc_alias = format!("Alias for [`{}`](Self::{}).", accessor, accessor);
                let doc_alias_set = format!("Alias for [`set_{}`](Self::set_{}).", accessor, accessor);
                let doc_alias_with = format!("Alias for [`with_{}`](Self::with_{}).", accessor, accessor);

                let alias_getter_tokens = if matches!(field.ty, FieldType::Nested(_)) {
                    quote! {
                        #[doc = #doc_alias]
                        #[doc(alias = #accessor)]
                        #[inline(always)]
                        #vis fn #alias_getter(&self) -> #return_ty {
                            self.#getter_name()
                        }
                    }
                } else {
                    quote! {
                        #[doc = #doc_alias]
                        #[doc(alias = #accessor)]
                        #[inline(always)]
                        #vis const fn #alias_getter(&self) -> #return_ty {
                            self.#getter_name()
                        }
                    }
                };
                methods.push(alias_getter_tokens);

                methods.push(quote! {
                    #[doc = #doc_alias_set]
                    #[doc(alias = #accessor)]
                    #[inline(always)]
                    #vis fn #alias_setter(&mut self, val: #param_ty) {
                        self.#setter_name(val)
                    }
                });

                let alias_with_tokens = if matches!(field.ty, FieldType::Nested(_)) {
                    quote! {
                        #[doc = #doc_alias_with]
                        #[doc(alias = #accessor)]
                        #[inline(always)]
                        #[must_use]
                        #vis fn #alias_with(self, val: #param_ty) -> Self {
                            self.#with_name(val)
                        }
                    }
                } else {
                    quote! {
                        #[doc = #doc_alias_with]
                        #[doc(alias = #accessor)]
                        #[inline(always)]
                        #[must_use]
                        #vis const fn #alias_with(self, val: #param_ty) -> Self {
                            self.#with_name(val)
                        }
                    }
                };
                methods.push(alias_with_tokens);
            }
        } else {
            // Readonly aliases: only getter
            for alias in &field.aliases {
                let alias_getter = format_ident!("{}", alias);
                let doc_alias = format!("Alias for [`{}`](Self::{}).", accessor, accessor);

                let alias_getter_tokens = if matches!(field.ty, FieldType::Nested(_)) {
                    quote! {
                        #[doc = #doc_alias]
                        #[doc(alias = #accessor)]
                        #[inline(always)]
                        #vis fn #alias_getter(&self) -> #return_ty {
                            self.#getter_name()
                        }
                    }
                } else {
                    quote! {
                        #[doc = #doc_alias]
                        #[doc(alias = #accessor)]
                        #[inline(always)]
                        #vis const fn #alias_getter(&self) -> #return_ty {
                            self.#getter_name()
                        }
                    }
                };
                methods.push(alias_getter_tokens);
            }
        }
    }

    // BitField trait impl. IS_MSB0 exposes ordering so extract_bits! can deduce it
    let is_msb0 = def.args.order == BitOrder::Msb0;
    let trait_impl = quote! {
        impl ::chapa::BitField for #name {
            type Storage = #storage_ident;
            const IS_MSB0: bool = #is_msb0;

            #[inline(always)]
            fn from_raw(raw: #storage_ident) -> Self {
                Self(raw)
            }

            #[inline(always)]
            fn raw(&self) -> #storage_ident {
                self.0
            }
        }
    };

    // From impls
    let from_impls = quote! {
        impl From<#storage_ident> for #name {
            #[inline(always)]
            fn from(val: #storage_ident) -> Self {
                Self(val)
            }
        }

        impl From<#name> for #storage_ident {
            #[inline(always)]
            fn from(val: #name) -> Self {
                val.0
            }
        }
    };

    // Bitwise ops: &, |, ^, ! with the raw storage type
    let ops_impls = quote! {
        impl ::core::ops::BitAnd<#storage_ident> for #name {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, rhs: #storage_ident) -> Self { Self(self.0 & rhs) }
        }
        impl ::core::ops::BitOr<#storage_ident> for #name {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, rhs: #storage_ident) -> Self { Self(self.0 | rhs) }
        }
        impl ::core::ops::BitXor<#storage_ident> for #name {
            type Output = Self;
            #[inline(always)]
            fn bitxor(self, rhs: #storage_ident) -> Self { Self(self.0 ^ rhs) }
        }
        impl ::core::ops::Not for #name {
            type Output = Self;
            #[inline(always)]
            fn not(self) -> Self { Self(!self.0) }
        }
    };

    // Only emit a Debug impl when the user opted in with `#[derive(Debug)]`.
    let debug_impl = if user_derived_debug {
        let name_str = name.to_string();
        let debug_fields: Vec<TokenStream> = def.fields.iter().map(|field| {
            let getter = format_ident!("{}", field.accessor_name);
            let field_str = &field.accessor_name;
            quote! { .field(#field_str, &self.#getter()) }
        }).collect();
        quote! {
            impl ::core::fmt::Debug for #name {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_struct(#name_str)
                        #(#debug_fields)*
                        .finish()
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #struct_def

        impl #name {
            #(#consts)*

            /// Creates a new instance with all bits set to zero.
            #[inline(always)]
            #vis const fn new() -> Self {
                Self(0)
            }

            /// Creates an instance from a raw storage value.
            #[inline(always)]
            #vis const fn from_raw(val: #storage_ident) -> Self {
                Self(val)
            }

            /// Returns the raw storage value.
            #[inline(always)]
            #vis const fn raw(&self) -> #storage_ident {
                self.0
            }

            #(#methods)*
        }

        #trait_impl
        #from_impls
        #ops_impls
        #debug_impl
    }
}

/// Strips `Debug` from `#[derive(...)]` attribute lists so the macro can
/// provide its own `core::fmt::Debug` implementation instead.
///
/// Returns `(found, filtered_attrs)` where `found` is `true` when `Debug` was
/// present in at least one derive list. Derive attributes that become empty
/// after removing `Debug` are dropped entirely.
fn strip_debug_derive(attrs: &[syn::Attribute]) -> (bool, Vec<proc_macro2::TokenStream>) {
    let mut result = Vec::new();
    let mut found = false;
    for attr in attrs {
        if attr.path().is_ident("derive") {
            let mut paths: Vec<syn::Path> = Vec::new();
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("Debug") {
                    found = true;
                } else {
                    paths.push(meta.path.clone());
                }
                Ok(())
            });
            if !paths.is_empty() {
                result.push(quote! { #[derive(#(#paths),*)] });
            }
        } else {
            result.push(quote! { #attr });
        }
    }
    (found, result)
}

/// Converts a `u128` mask value to a correctly-typed token literal for the
/// given storage kind, so rustc doesn't need an extra cast.
fn storage_mask_literal(storage: StorageKind, mask: u128) -> TokenStream {
    match storage {
        StorageKind::U8 => {
            let v = mask as u8;
            quote! { #v }
        }
        StorageKind::U16 => {
            let v = mask as u16;
            quote! { #v }
        }
        StorageKind::U32 => {
            let v = mask as u32;
            quote! { #v }
        }
        StorageKind::U64 => {
            let v = mask as u64;
            quote! { #v }
        }
        StorageKind::U128 => {
            let v = mask;
            quote! { #v }
        }
    }
}
