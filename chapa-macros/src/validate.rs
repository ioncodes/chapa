use std::collections::HashMap;

use proc_macro2::Span;

use crate::model::*;

/// Runs all semantic checks on a parsed [`BitfieldDef`] and returns a combined
/// [`syn::Error`] if any violations are found.
pub fn validate(def: &BitfieldDef) -> syn::Result<()> {
    let mut errors: Option<syn::Error> = None;

    let mut push_err = |e: syn::Error| match &mut errors {
        Some(existing) => existing.combine(e),
        None => errors = Some(e),
    };

    if let Err(e) = validate_width(def) {
        push_err(e);
    }
    for e in validate_ranges(def) {
        push_err(e);
    }
    for e in validate_field_types(def) {
        push_err(e);
    }
    for e in validate_overlaps(def) {
        push_err(e);
    }
    for e in validate_aliases(def) {
        push_err(e);
    }

    match errors {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Checks that `effective_width` fits within the declared storage type.
fn validate_width(def: &BitfieldDef) -> syn::Result<()> {
    if def.effective_width > def.args.storage.bit_width() {
        return Err(syn::Error::new(
            def.args.width_span.unwrap_or(def.args.storage_span),
            format!(
                "width {} exceeds storage type `{}` capacity of {} bits",
                def.effective_width,
                def.args.storage.unsigned_ident(),
                def.args.storage.bit_width()
            ),
        ));
    }
    Ok(())
}

/// Checks that every field range is non-empty and within `effective_width`,
/// and that a field does not select the same storage bit more than once.
fn validate_ranges(def: &BitfieldDef) -> Vec<syn::Error> {
    let mut errs = Vec::new();
    for f in &def.fields {
        for range in &f.ranges {
            if range.start > range.end {
                errs.push(syn::Error::new(
                    range.span,
                    format!(
                        "bit range {}..={} is backwards (start > end)",
                        range.start, range.end
                    ),
                ));
            }
            if range.end >= def.effective_width {
                errs.push(syn::Error::new(
                    range.span,
                    format!(
                        "bit {} is out of bounds for effective width {}",
                        range.end, def.effective_width
                    ),
                ));
            }
        }

        for (index, range) in f.ranges.iter().enumerate() {
            for other in f.ranges.iter().skip(index + 1) {
                if range.overlaps(other) {
                    errs.push(syn::Error::new(
                        other.span,
                        format!(
                            "field `{}` selects overlapping ranges {}..={} and {}..={}",
                            f.accessor_name, range.start, range.end, other.start, other.end,
                        ),
                    ));
                }
            }
        }
    }
    errs
}

/// Checks that field types are wide enough to hold their declared bit range
/// (e.g. a `bool` must be exactly 1 bit; a `u8` field can span at most 8 bits).
fn validate_field_types(def: &BitfieldDef) -> Vec<syn::Error> {
    let mut errs = Vec::new();
    for f in &def.fields {
        let width = f.width();
        let span = f.ranges[0].span;
        match &f.ty {
            FieldType::Bool => {
                if width != 1 {
                    errs.push(syn::Error::new(
                        span,
                        format!(
                            "bool field `{}` must be exactly 1 bit wide, but range spans {} bits",
                            f.accessor_name, width
                        ),
                    ));
                }
            }
            FieldType::PrimitiveUnsigned(sk) => {
                if width > sk.bit_width() {
                    errs.push(syn::Error::new(
                        span,
                        format!(
                            "field `{}` range spans {} bits but type `{}` can only hold {} bits",
                            f.accessor_name,
                            width,
                            sk.unsigned_ident(),
                            sk.bit_width()
                        ),
                    ));
                }
            }
            FieldType::PrimitiveSigned(sk) => {
                if width > sk.bit_width() {
                    errs.push(syn::Error::new(
                        span,
                        format!(
                            "field `{}` range spans {} bits but type `{}` can only hold {} bits",
                            f.accessor_name,
                            width,
                            sk.signed_ident(),
                            sk.bit_width()
                        ),
                    ));
                }
            }
            FieldType::Nested(_) => {
                // Nested type width is checked by rustc at use site
            }
        }
    }
    errs
}

/// Checks for illegal bit-range overlaps between fields.
///
/// Overlaps are allowed only between fields that belong to **different** overlay
/// groups (`overlay = "..."`). Overlaps between two base fields, between a base
/// field and an overlay field, or between two fields in the **same** overlay
/// group are all errors.
fn validate_overlaps(def: &BitfieldDef) -> Vec<syn::Error> {
    let mut errs = Vec::new();
    let fields = &def.fields;

    for i in 0..fields.len() {
        for j in (i + 1)..fields.len() {
            let a = &fields[i];
            let b = &fields[j];

            let overlap = a.ranges.iter().find_map(|a_range| {
                b.ranges
                    .iter()
                    .find(|b_range| a_range.overlaps(b_range))
                    .map(|b_range| (a_range, b_range))
            });
            let Some((a_range, b_range)) = overlap else {
                continue;
            };

            let a_overlay = a.overlay.as_deref();
            let b_overlay = b.overlay.as_deref();

            match (a_overlay, b_overlay) {
                // Both base fields -> error
                (None, None) => {
                    errs.push(syn::Error::new(
                        b_range.span,
                        format!(
                            "field `{}` (bits {}..={}) overlaps with `{}` (bits {}..={})",
                            b.accessor_name,
                            b_range.start,
                            b_range.end,
                            a.accessor_name,
                            a_range.start,
                            a_range.end,
                        ),
                    ));
                }
                // One base, one overlay -> error
                (None, Some(_)) | (Some(_), None) => {
                    let (base, base_range, over, over_range) = if a_overlay.is_none() {
                        (a, a_range, b, b_range)
                    } else {
                        (b, b_range, a, a_range)
                    };
                    errs.push(syn::Error::new(
                        over_range.span,
                        format!(
                            "overlay field `{}` (bits {}..={}) overlaps with base field `{}` (bits {}..={})",
                            over.accessor_name, over_range.start, over_range.end,
                            base.accessor_name, base_range.start, base_range.end,
                        ),
                    ));
                }
                // Same overlay group -> error
                (Some(oa), Some(ob)) if oa == ob => {
                    errs.push(syn::Error::new(
                        b_range.span,
                        format!(
                            "fields `{}` and `{}` in overlay group \"{}\" overlap at bits {}..={}",
                            a.accessor_name,
                            b.accessor_name,
                            oa,
                            a_range.start.max(b_range.start),
                            a_range.end.min(b_range.end),
                        ),
                    ));
                }
                // Different overlay groups -> allowed
                (Some(_), Some(_)) => {}
            }
        }
    }
    errs
}

/// Checks that no two fields (including their aliases) expose the same accessor name.
fn validate_aliases(def: &BitfieldDef) -> Vec<syn::Error> {
    let mut errs = Vec::new();
    let mut seen: HashMap<String, Span> = HashMap::new();

    for f in &def.fields {
        // Register the accessor name
        let names = std::iter::once(f.accessor_name.clone()).chain(f.aliases.iter().cloned());
        for name in names {
            if seen.contains_key(&name) {
                errs.push(syn::Error::new(
                    f.span,
                    format!("accessor name `{}` is already used", name),
                ));
            } else {
                seen.insert(name, f.span);
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn definition(source: &str) -> BitfieldDef {
        let args: BitfieldArgs = syn::parse_str("u16, order = lsb0").expect("valid arguments");
        let item: syn::ItemStruct = syn::parse_str(source).expect("valid struct syntax");
        parse::parse_struct(&args, &item).expect("valid bitfield syntax")
    }

    #[test]
    fn rejects_overlapping_ranges_within_one_field() {
        let def = definition("struct Bad { #[bits(0..=3, 3..=5)] value: u8 }");

        let error = validate(&def).expect_err("overlapping segments must be rejected");
        assert!(error
            .to_string()
            .contains("selects overlapping ranges 0..=3 and 3..=5"));
    }

    #[test]
    fn detects_overlap_with_any_segment() {
        let def = definition(
            "struct Bad {
                #[bits(0, 8)] split: u8,
                #[bits(8)] other: bool,
            }",
        );

        let error = validate(&def).expect_err("field overlap must be rejected");
        assert!(error.to_string().contains("other"));
    }

    #[test]
    fn different_overlay_groups_may_overlap_split_segments() {
        let def = definition(
            "struct Good {
                #[bits(0, 8, overlay = \"first\")] split: u8,
                #[bits(8, overlay = \"second\")] other: bool,
            }",
        );

        validate(&def).expect("different overlay groups may overlap");
    }
}
