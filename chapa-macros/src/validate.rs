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
                def.args.storage.ident(),
                def.args.storage.bit_width()
            ),
        ));
    }
    Ok(())
}

/// Checks that every field range is non-empty and within `effective_width`.
fn validate_ranges(def: &BitfieldDef) -> Vec<syn::Error> {
    let mut errs = Vec::new();
    for f in &def.fields {
        if f.range.start > f.range.end {
            errs.push(syn::Error::new(
                f.range.span,
                format!(
                    "bit range {}..={} is backwards (start > end)",
                    f.range.start, f.range.end
                ),
            ));
        }
        if f.range.end >= def.effective_width {
            errs.push(syn::Error::new(
                f.range.span,
                format!(
                    "bit {} is out of bounds for effective width {}",
                    f.range.end, def.effective_width
                ),
            ));
        }
    }
    errs
}

/// Checks that field types are wide enough to hold their declared bit range
/// (e.g. a `bool` must be exactly 1 bit; a `u8` field can span at most 8 bits).
fn validate_field_types(def: &BitfieldDef) -> Vec<syn::Error> {
    let mut errs = Vec::new();
    for f in &def.fields {
        let width = f.range.width();
        match &f.ty {
            FieldType::Bool => {
                if width != 1 {
                    errs.push(syn::Error::new(
                        f.range.span,
                        format!(
                            "bool field `{}` must be exactly 1 bit wide, but range spans {} bits",
                            f.accessor_name, width
                        ),
                    ));
                }
            }
            FieldType::Primitive(sk) => {
                if width > sk.bit_width() {
                    errs.push(syn::Error::new(
                        f.range.span,
                        format!(
                            "field `{}` range spans {} bits but type `{}` can only hold {} bits",
                            f.accessor_name,
                            width,
                            sk.ident(),
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

            if !a.range.overlaps(&b.range) {
                continue;
            }

            let a_overlay = a.overlay.as_deref();
            let b_overlay = b.overlay.as_deref();

            match (a_overlay, b_overlay) {
                // Both base fields -> error
                (None, None) => {
                    errs.push(syn::Error::new(
                        b.range.span,
                        format!(
                            "field `{}` (bits {}..={}) overlaps with `{}` (bits {}..={})",
                            b.accessor_name, b.range.start, b.range.end,
                            a.accessor_name, a.range.start, a.range.end,
                        ),
                    ));
                }
                // One base, one overlay -> error
                (None, Some(_)) | (Some(_), None) => {
                    let (base, over) = if a_overlay.is_none() { (a, b) } else { (b, a) };
                    errs.push(syn::Error::new(
                        over.range.span,
                        format!(
                            "overlay field `{}` (bits {}..={}) overlaps with base field `{}` (bits {}..={})",
                            over.accessor_name, over.range.start, over.range.end,
                            base.accessor_name, base.range.start, base.range.end,
                        ),
                    ));
                }
                // Same overlay group -> error
                (Some(oa), Some(ob)) if oa == ob => {
                    errs.push(syn::Error::new(
                        b.range.span,
                        format!(
                            "fields `{}` and `{}` in overlay group \"{}\" overlap at bits {}..={}",
                            a.accessor_name,
                            b.accessor_name,
                            oa,
                            a.range.start.max(b.range.start),
                            a.range.end.min(b.range.end),
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
