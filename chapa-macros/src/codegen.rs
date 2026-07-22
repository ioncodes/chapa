//! Code generation for the `#[bitfield]` macro output.
//!
//! [`generate`] turns a fully validated [`BitfieldDef`] into a [`TokenStream`]
//! containing the newtype struct, associated constants, accessor methods, and
//! trait impls.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};

use crate::model::*;
use crate::ordering;

/// Generates the complete output `TokenStream` for a bitfield struct.
///
/// Emits:
/// - A `#[repr(transparent)]` newtype wrapping the storage type.
/// - `{FIELD}_SHIFT` and `{FIELD}_MASK` associated constants for every field.
/// - `zeroed()`, `from_raw()`, and `raw()` inherent methods.
/// - `to_{le,be,ne}_bytes()` / `from_{le,be,ne}_bytes()` inherent methods.
/// - `{wrapping,saturating,checked,overflowing}_{add,sub}()` inherent methods
///   operating on the raw storage value.
/// - `field()` getter, `set_field()` setter, and `with_field()` builder for each
///   non-readonly field; only the getter for readonly fields.
/// - Alias methods for every `alias = ...` annotation.
/// - `BitField`, `From<Storage>`, and `From<Self>` trait impls.
pub fn generate(def: &BitfieldDef) -> TokenStream {
    let vis = &def.vis;
    let name = &def.name;
    let storage_ident = format_ident!("{}", def.args.storage.unsigned_ident());
    let byte_count = (def.args.storage.bit_width() / 8) as usize;

    // These derives are removed from the struct because their implementations
    // are generated below. Copy and Clone are always implemented. Debug remains
    // opt-in, while a field default also enables Default.
    let debug_span = &def.debug_span;
    let default_span = &def.default_span;
    let filtered_attrs = &def.user_attrs;

    // Generate struct
    let struct_def = quote! {
        #(#filtered_attrs)*
        #[repr(transparent)]
        #vis struct #name(#storage_ident);
    };

    // rust-analyzer fix (?)
    let shadow_fields = def.fields.iter().map(|f| {
        let field_name = &f.name;
        let field_ty = &f.raw_ty;
        quote! { #field_name: #field_ty }
    });
    let span_anchor = quote! {
        const _: () = {
            #[allow(dead_code)]
            struct ChapaFieldSpans {
                #(#shadow_fields),*
            }
        };
    };

    // Generate associated consts and methods
    let mut consts = Vec::new();
    let mut methods = Vec::new();
    // Per-field `default = ...` contributions, OR'd together inside the generated
    // `Default::default()`.
    let mut default_contribs = Vec::new();
    // Per-field `FieldInfo` literals for the `reflection` feature.
    let mut field_infos = Vec::new();

    for field in &def.fields {
        let phys = ordering::compute(def.args.order, &field.range, def.effective_width);

        let accessor = &field.accessor_name;
        let shift_name = format_ident!("{}_SHIFT", accessor.to_uppercase(), span = field.span);
        let mask_name = format_ident!("{}_MASK", accessor.to_uppercase(), span = field.span);

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

        // `const fn` is possible only when packing/unpacking this field is const.
        // Nested and enum fields go through `BitField::raw`/`from_raw`, which are
        // not const, so their accessors are emitted without `const`.
        let maybe_const = if matches!(field.ty, FieldType::Nested(_)) {
            quote! {}
        } else {
            quote! { const }
        };

        // The contribution of `value` to the storage: shifted and masked into this
        // field's bits, truncating anything too wide. Shared by the setter, the
        // `with_*` builder, and the `default = ...` initializer so the packing
        // logic lives in exactly one place.
        let insert = |value: &TokenStream| match &field.ty {
            FieldType::Bool => quote! { (if #value { Self::#mask_name } else { 0 }) },
            FieldType::PrimitiveUnsigned(_) => {
                quote! { (((#value as #storage_ident) << Self::#shift_name) & Self::#mask_name) }
            }
            FieldType::PrimitiveSigned(sk) => {
                let field_ty = format_ident!("{}", sk.signed_ident());
                quote! { ((((#value as #field_ty) as #storage_ident) << Self::#shift_name) & Self::#mask_name) }
            }
            FieldType::Nested(ty) => quote! {
                (((<#ty as ::chapa::BitField>::raw(&(#value)) as #storage_ident) << Self::#shift_name) & Self::#mask_name)
            },
        };

        // Fold a `default = ...` value into the bits produced by `Default::default()`.
        if let Some(default_expr) = &field.default {
            default_contribs.push(insert(&quote! { (#default_expr) }));
        }

        let getter_name = format_ident!("{}", accessor, span = field.span);
        let getter_doc = format!(
            "Returns the `{}` field (bits {}..={}).",
            accessor, field.range.start, field.range.end
        );
        let field_width = phys.field_width;

        // Reflection metadata for this field, built only when the feature is on
        // (mirrors the gating in `bit_enum.rs`). `offset`/`width` are physical
        // (storage-value coordinates), so consumers extract with
        // `(raw >> offset) & ((1 << width) - 1)` regardless of ordering. Enum vs.
        // nested-struct is resolved through the field type's own `Reflect` impl,
        // since both are `FieldType::Nested` at the field site.
        if cfg!(feature = "reflection") {
            let field_kind = match &field.ty {
                FieldType::Bool => quote! { ::chapa::FieldKind::Bool },
                FieldType::PrimitiveUnsigned(_) => quote! { ::chapa::FieldKind::Uint },
                FieldType::PrimitiveSigned(_) => quote! { ::chapa::FieldKind::Sint },
                FieldType::Nested(ty) => quote! { <#ty as ::chapa::Reflect>::REFLECT },
            };
            let readonly = field.readonly;
            let aliases = &field.aliases;
            field_infos.push(quote! {
                ::chapa::FieldInfo {
                    name: #accessor,
                    offset: #shift_val,
                    width: #field_width,
                    aliases: &[ #(#aliases),* ],
                    readonly: #readonly,
                    kind: #field_kind,
                }
            });
        }

        // Generate getter
        let getter_body = match &field.ty {
            FieldType::Bool => {
                quote! { (self.0 & Self::#mask_name) != 0 }
            }
            FieldType::PrimitiveUnsigned(sk) => {
                let field_ty = format_ident!("{}", sk.unsigned_ident());
                quote! { ((self.0 >> Self::#shift_name) & ((1 << #field_width) - 1)) as #field_ty }
            }
            FieldType::PrimitiveSigned(sk) => {
                let field_ty = format_ident!("{}", sk.signed_ident());
                let unsigned_ty = format_ident!("{}", sk.unsigned_ident());
                let sign_shift = sk.bit_width() - field_width;
                quote! {
                    ((((self.0 >> Self::#shift_name) as #unsigned_ty) << #sign_shift) as #field_ty) >> #sign_shift
                }
            }
            FieldType::Nested(ty) => {
                let nested_storage =
                    StorageKind::smallest_fitting(field_width).unwrap_or(StorageKind::W128);
                let nested_storage_ident = format_ident!("{}", nested_storage.unsigned_ident());
                quote! {
                    let bits = ((self.0 >> Self::#shift_name) & ((1 << #field_width) - 1)) as #nested_storage_ident;
                    <#ty as ::chapa::BitField>::from_raw(bits)
                }
            }
        };

        let return_ty = match &field.ty {
            FieldType::Bool => quote! { bool },
            FieldType::PrimitiveUnsigned(sk) => {
                let ty = format_ident!("{}", sk.unsigned_ident());
                quote! { #ty }
            }
            FieldType::PrimitiveSigned(sk) => {
                let ty = format_ident!("{}", sk.signed_ident());
                quote! { #ty }
            }
            FieldType::Nested(ty) => quote! { #ty },
        };

        methods.push(quote! {
            #[doc = #getter_doc]
            #[inline(always)]
            #vis #maybe_const fn #getter_name(&self) -> #return_ty {
                #getter_body
            }
        });

        // Generate setter and with_* (unless readonly)
        if !field.readonly {
            let setter_name = format_ident!("set_{}", accessor, span = field.span);
            let with_name = format_ident!("with_{}", accessor, span = field.span);
            let setter_doc = format!(
                "Sets the `{}` field (bits {}..={}).",
                accessor, field.range.start, field.range.end
            );
            let with_doc = format!(
                "Returns a copy with the `{}` field set (bits {}..={}).",
                accessor, field.range.start, field.range.end
            );

            let param_ty = match &field.ty {
                FieldType::Bool => quote! { bool },
                FieldType::PrimitiveUnsigned(sk) => {
                    let ty = format_ident!("{}", sk.unsigned_ident());
                    quote! { #ty }
                }
                FieldType::PrimitiveSigned(sk) => {
                    let ty = format_ident!("{}", sk.signed_ident());
                    quote! { #ty }
                }
                FieldType::Nested(ty) => quote! { #ty },
            };

            // Both the setter and the `with_*` builder clear the field's bits and
            // OR in the new value; `const` is gated on the field type (see above).
            let value = insert(&quote! { val });
            let mutate_body = quote! {
                self.0 = (self.0 & !Self::#mask_name) | #value;
            };

            methods.push(quote! {
                #[doc = #setter_doc]
                #[inline(always)]
                #vis #maybe_const fn #setter_name(&mut self, val: #param_ty) {
                    #mutate_body
                }
            });

            methods.push(quote! {
                #[doc = #with_doc]
                #[inline(always)]
                #[must_use]
                #vis #maybe_const fn #with_name(mut self, val: #param_ty) -> Self {
                    #mutate_body
                    self
                }
            });

            // Generate aliases
            for alias in &field.aliases {
                let alias_getter = format_ident!("{}", alias);
                let alias_setter = format_ident!("set_{}", alias);
                let alias_with = format_ident!("with_{}", alias);
                let doc_alias = format!("Alias for [`{}`](Self::{}).", accessor, accessor);
                let doc_alias_set =
                    format!("Alias for [`set_{}`](Self::set_{}).", accessor, accessor);
                let doc_alias_with =
                    format!("Alias for [`with_{}`](Self::with_{}).", accessor, accessor);

                methods.push(quote! {
                    #[doc = #doc_alias]
                    #[doc(alias = #accessor)]
                    #[inline(always)]
                    #vis #maybe_const fn #alias_getter(&self) -> #return_ty {
                        self.#getter_name()
                    }
                });

                methods.push(quote! {
                    #[doc = #doc_alias_set]
                    #[doc(alias = #accessor)]
                    #[inline(always)]
                    #vis #maybe_const fn #alias_setter(&mut self, val: #param_ty) {
                        self.#setter_name(val)
                    }
                });

                methods.push(quote! {
                    #[doc = #doc_alias_with]
                    #[doc(alias = #accessor)]
                    #[inline(always)]
                    #[must_use]
                    #vis #maybe_const fn #alias_with(self, val: #param_ty) -> Self {
                        self.#with_name(val)
                    }
                });
            }
        } else {
            // Readonly aliases: only getter
            for alias in &field.aliases {
                let alias_getter = format_ident!("{}", alias);
                let doc_alias = format!("Alias for [`{}`](Self::{}).", accessor, accessor);

                methods.push(quote! {
                    #[doc = #doc_alias]
                    #[doc(alias = #accessor)]
                    #[inline(always)]
                    #vis #maybe_const fn #alias_getter(&self) -> #return_ty {
                        self.#getter_name()
                    }
                });
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
        impl ::core::ops::BitAndAssign<#storage_ident> for #name {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: #storage_ident) { self.0 &= rhs; }
        }
        impl ::core::ops::BitOrAssign<#storage_ident> for #name {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: #storage_ident) { self.0 |= rhs; }
        }
        impl ::core::ops::BitXorAssign<#storage_ident> for #name {
            #[inline(always)]
            fn bitxor_assign(&mut self, rhs: #storage_ident) { self.0 ^= rhs; }
        }
    };

    // BitField requires Copy, so every generated struct implements Copy and Clone.
    let copy_impl = {
        let span = def.copy_span.unwrap_or_else(proc_macro2::Span::call_site);
        quote_spanned! { span =>
            impl ::core::marker::Copy for #name {}
        }
    };
    let clone_impl = {
        let span = def.clone_span.unwrap_or_else(proc_macro2::Span::call_site);
        quote_spanned! { span =>
            impl ::core::clone::Clone for #name {
                #[inline(always)]
                fn clone(&self) -> Self {
                    *self
                }
            }
        }
    };

    // A field default automatically enables Default. An explicit derive also
    // works and returns zeroed() when no field defaults are present.
    let default_impl = if default_span.is_some() || def.fields.iter().any(|f| f.default.is_some()) {
        let span = default_span.unwrap_or_else(proc_macro2::Span::call_site);
        quote_spanned! { span =>
            impl ::core::default::Default for #name {
                #[inline(always)]
                fn default() -> Self {
                    Self(0 #( | #default_contribs )*)
                }
            }
        }
    } else {
        quote! {}
    };

    // Only emit a Debug impl when the user opted in with `#[derive(Debug)]`.
    let debug_impl = if let Some(debug_span) = debug_span {
        let name_str = name.to_string();
        let debug_fields: Vec<TokenStream> = def
            .fields
            .iter()
            .map(|field| {
                let getter = format_ident!("{}", field.accessor_name);
                let field_str = &field.accessor_name;
                quote! { .field(#field_str, &self.#getter()) }
            })
            .collect();
        quote_spanned! { *debug_span =>
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

    // Arithmetic on the raw storage value: wrapping, saturating, checked, and
    // overflowing add/sub. These operate on the full storage value like `raw()`,
    // so carries and borrows propagate across field boundaries.
    let arith_methods: Vec<TokenStream> = [("add", "addition"), ("sub", "subtraction")]
        .iter()
        .map(|(op, op_doc)| {
            let wrapping_fn = format_ident!("wrapping_{}", op);
            let saturating_fn = format_ident!("saturating_{}", op);
            let checked_fn = format_ident!("checked_{}", op);
            let overflowing_fn = format_ident!("overflowing_{}", op);
            let wrapping_doc =
                format!("Wrapping (modular) {op_doc} on the raw storage value.");
            let saturating_doc = format!(
                "Saturating {op_doc} on the raw storage value, clamping at the storage bounds."
            );
            let checked_doc = format!(
                "Checked {op_doc} on the raw storage value, returning `None` on overflow."
            );
            let overflowing_doc = format!(
                "Overflowing {op_doc} on the raw storage value, returning the wrapped result and whether overflow occurred."
            );
            quote! {
                #[doc = #wrapping_doc]
                ///
                /// Operates on the full storage value, so carries and borrows
                /// propagate across field boundaries.
                #[inline(always)]
                #[must_use]
                #vis const fn #wrapping_fn(self, rhs: #storage_ident) -> Self {
                    Self(self.0.#wrapping_fn(rhs))
                }

                #[doc = #saturating_doc]
                ///
                /// Operates on the full storage value, so carries and borrows
                /// propagate across field boundaries.
                #[inline(always)]
                #[must_use]
                #vis const fn #saturating_fn(self, rhs: #storage_ident) -> Self {
                    Self(self.0.#saturating_fn(rhs))
                }

                #[doc = #checked_doc]
                ///
                /// Operates on the full storage value, so carries and borrows
                /// propagate across field boundaries.
                #[inline(always)]
                #[must_use]
                #vis const fn #checked_fn(self, rhs: #storage_ident) -> Option<Self> {
                    match self.0.#checked_fn(rhs) {
                        Some(val) => Some(Self(val)),
                        None => None,
                    }
                }

                #[doc = #overflowing_doc]
                ///
                /// Operates on the full storage value, so carries and borrows
                /// propagate across field boundaries.
                #[inline(always)]
                #[must_use]
                #vis const fn #overflowing_fn(self, rhs: #storage_ident) -> (Self, bool) {
                    let (val, overflowed) = self.0.#overflowing_fn(rhs);
                    (Self(val), overflowed)
                }
            }
        })
        .collect();

    // Add byte conversions for each byte order.
    let byte_methods: Vec<TokenStream> = [("le", "little"), ("be", "big"), ("ne", "native")]
        .iter()
        .map(|(endian, order)| {
            let to_fn = format_ident!("to_{}_bytes", endian);
            let from_fn = format_ident!("from_{}_bytes", endian);
            let to_doc = format!("Returns the raw storage value as bytes in {order}-endian order.");
            let from_doc = format!("Creates an instance from bytes in {order}-endian order.");
            quote! {
                #[doc = #to_doc]
                #[inline(always)]
                #vis const fn #to_fn(self) -> [u8; #byte_count] {
                    self.0.#to_fn()
                }

                #[doc = #from_doc]
                ///
                /// Preserves the full storage value, including bits outside the
                /// bitfield width.
                #[inline(always)]
                #vis const fn #from_fn(bytes: [u8; #byte_count]) -> Self {
                    Self(#storage_ident::#from_fn(bytes))
                }
            }
        })
        .collect();

    // Reflection metadata, emitted only under the `reflection` feature. Both
    // branches always compile, so `field_infos` is never unused.
    let reflection_impl = if cfg!(feature = "reflection") {
        quote! {
            impl #name {
                /// Compile-time field metadata (enabled by the `reflection` feature).
                #vis const FIELDS: &'static [::chapa::FieldInfo] = &[ #(#field_infos),* ];
            }

            impl ::chapa::Reflect for #name {
                const REFLECT: ::chapa::FieldKind = ::chapa::FieldKind::Struct(#name::FIELDS);
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #struct_def
        #span_anchor

        impl #name {
            #(#consts)*

            /// Creates an instance with all bits set to zero.
            ///
            /// Field `default = ...` values are applied by `Default::default`,
            /// not here.
            #[inline(always)]
            #vis const fn zeroed() -> Self {
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

            #(#byte_methods)*

            #(#arith_methods)*

            #(#methods)*
        }

        #copy_impl
        #clone_impl
        #trait_impl
        #from_impls
        #ops_impls
        #debug_impl
        #default_impl
        #reflection_impl
    }
}

/// Converts a `u128` mask value to a correctly-typed token literal for the
/// given storage kind, so rustc doesn't need an extra cast.
fn storage_mask_literal(storage: StorageKind, mask: u128) -> TokenStream {
    match storage {
        StorageKind::W8 => {
            let v = mask as u8;
            quote! { #v }
        }
        StorageKind::W16 => {
            let v = mask as u16;
            quote! { #v }
        }
        StorageKind::W32 => {
            let v = mask as u32;
            quote! { #v }
        }
        StorageKind::W64 => {
            let v = mask as u64;
            quote! { #v }
        }
        StorageKind::W128 => {
            let v = mask;
            quote! { #v }
        }
    }
}
