//! Parsing logic for the `#[bitfield]` attribute and `#[bits(...)]` field annotations.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Ident, LitInt, Token};

use crate::model::*;

/// Parsed content of `#[bitfield(u32, order = msb0, width = 16)]`.
impl Parse for BitfieldArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse storage type
        let storage_ident: Ident = input.parse()?;
        let storage_span = storage_ident.span();
        let storage = StorageKind::from_str(&storage_ident.to_string()).ok_or_else(|| {
            syn::Error::new(
                storage_span,
                format!(
                    "unsupported storage type `{}`, expected one of: u8, u16, u32, u64, u128",
                    storage_ident
                ),
            )
        })?;

        let mut order: Option<(BitOrder, Span)> = None;
        let mut width: Option<(u32, Span)> = None;

        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }

            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "order" => {
                    let val: Ident = input.parse()?;
                    let bit_order = match val.to_string().as_str() {
                        "msb0" => BitOrder::Msb0,
                        "lsb0" => BitOrder::Lsb0,
                        _ => {
                            return Err(syn::Error::new(
                                val.span(),
                                "expected `msb0` or `lsb0`",
                            ))
                        }
                    };
                    order = Some((bit_order, key.span()));
                }
                "width" => {
                    let val: LitInt = input.parse()?;
                    let w: u32 = val.base10_parse()?;
                    width = Some((w, val.span()));
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown attribute `{}`", key),
                    ))
                }
            }
        }

        let (order_val, order_span) = order.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "missing required `order = msb0|lsb0`")
        })?;

        Ok(BitfieldArgs {
            storage,
            storage_span,
            order: order_val,
            order_span,
            width: width.map(|(w, _)| w),
            width_span: width.map(|(_, s)| s),
        })
    }
}

/// Content parsed from `#[bits(...)]` on a field.
struct BitsAttr {
    range: BitRange,
    readonly: bool,
    aliases: Vec<String>,
    overlay: Option<String>,
}

fn parse_bits_attr(input: ParseStream) -> syn::Result<BitsAttr> {
    let mut readonly = false;
    let mut aliases = Vec::new();
    let mut overlay = None;

    // Parse the range part. Could be:
    // - single literal: `5`
    // - range expression: `0..4` or `0..=3`
    let range = if input.peek(LitInt) {
        // Could be single bit or start of a range
        let start_lit: LitInt = input.parse()?;
        let start: u32 = start_lit.base10_parse()?;
        let span = start_lit.span();

        if input.peek(Token![..]) {
            // Range
            let dots_span = input.parse::<Token![..]>()?.span();
            let _ = dots_span;
            let inclusive = input.peek(Token![=]);
            if inclusive {
                input.parse::<Token![=]>()?;
            }
            let end_lit: LitInt = input.parse()?;
            let end_val: u32 = end_lit.base10_parse()?;
            let end = if inclusive { end_val } else { end_val - 1 };
            BitRange {
                start,
                end,
                span,
            }
        } else {
            // Single bit
            BitRange {
                start,
                end: start,
                span,
            }
        }
    } else {
        return Err(input.error("expected bit index or range"));
    };

    // Parse optional keywords
    while !input.is_empty() {
        input.parse::<Token![,]>()?;
        if input.is_empty() {
            break;
        }

        let key: Ident = input.parse()?;
        match key.to_string().as_str() {
            "readonly" => {
                readonly = true;
            }
            "alias" => {
                input.parse::<Token![=]>()?;
                if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        let lit: syn::LitStr = content.parse()?;
                        aliases.push(lit.value());
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                } else {
                    let lit: syn::LitStr = input.parse()?;
                    aliases.push(lit.value());
                }
            }
            "overlay" => {
                input.parse::<Token![=]>()?;
                let lit: syn::LitStr = input.parse()?;
                overlay = Some(lit.value());
            }
            _ => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown bits attribute `{}`", key),
                ))
            }
        }
    }

    Ok(BitsAttr {
        range,
        readonly,
        aliases,
        overlay,
    })
}

fn determine_field_type(ty: &syn::Type) -> FieldType {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            if name == "bool" {
                return FieldType::Bool;
            }
            if let Some(sk) = StorageKind::from_str(&name) {
                return FieldType::Primitive(sk);
            }
        }
    }
    FieldType::Nested(ty.clone())
}

/// Parses the struct definition into a [`BitfieldDef`], resolving effective
/// width and processing every field's `#[bits(...)]` annotation.
pub fn parse_struct(
    args: &BitfieldArgs,
    item: &syn::ItemStruct,
) -> syn::Result<BitfieldDef> {
    let effective_width = args.width.unwrap_or_else(|| args.storage.bit_width());

    let fields_named = match &item.fields {
        syn::Fields::Named(f) => f,
        _ => {
            return Err(syn::Error::new_spanned(
                item,
                "bitfield requires a struct with named fields",
            ))
        }
    };

    let mut field_defs = Vec::new();

    for field in &fields_named.named {
        let field_name = field.ident.as_ref().unwrap();

        // Find #[bits(...)] attribute
        let bits_attr = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("bits"))
            .ok_or_else(|| {
                syn::Error::new_spanned(field_name, "missing `#[bits(...)]` attribute")
            })?;

        let parsed: BitsAttr = bits_attr.parse_args_with(parse_bits_attr)?;

        let name_str = field_name.to_string();
        let (accessor_name, implicit_readonly) = if name_str.starts_with('_') {
            (name_str[1..].to_string(), true)
        } else {
            (name_str.clone(), false)
        };

        let readonly = parsed.readonly || implicit_readonly;
        let field_type = determine_field_type(&field.ty);

        field_defs.push(FieldDef {
            name: field_name.clone(),
            accessor_name,
            ty: field_type,
            raw_ty: field.ty.clone(),
            range: parsed.range,
            readonly,
            aliases: parsed.aliases,
            overlay: parsed.overlay,
            span: field_name.span(),
        });
    }

    // Collect user attrs (exclude bitfield)
    let user_attrs: Vec<syn::Attribute> = item
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("bitfield"))
        .cloned()
        .collect();

    Ok(BitfieldDef {
        args: args.clone(),
        effective_width,
        fields: field_defs,
        vis: item.vis.clone(),
        name: item.ident.clone(),
        user_attrs,
    })
}
