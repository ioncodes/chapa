#![cfg(feature = "reflection")]

use chapa::{bitenum, bitfield, EnumInfo, FieldInfo, FieldKind};

#[bitenum]
#[derive(Debug, PartialEq)]
pub enum Mode {
    Off = 0,
    On = 1,
    #[fallback]
    Reserved = 3,
}

#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone)]
pub struct Inner {
    #[bits(0..=1)]
    lo: u8,
    #[bits(2..=3)]
    hi: u8,
}

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone)]
pub struct Reg {
    #[bits(0)]
    enabled: bool,
    #[bits(1..=2)]
    mode: Mode,
    #[bits(4..=7)]
    count: u8,
    #[bits(8..=11)]
    inner: Inner,
    #[bits(12..=14)]
    delta: i8,
}

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone)]
pub struct SplitReg {
    #[bits(12..=15, 0..=3)]
    split: u8,
}

fn field<'a>(fields: &'a [FieldInfo], name: &str) -> &'a FieldInfo {
    fields
        .iter()
        .find(|f| f.name == name)
        .expect("field exists")
}

#[test]
fn bool_field() {
    let f = field(Reg::FIELDS, "enabled");
    assert_eq!(f.offset, 0);
    assert_eq!(f.width, 1);
    assert!(matches!(f.kind, FieldKind::Bool));
}

#[test]
fn uint_field() {
    let f = field(Reg::FIELDS, "count");
    assert_eq!(f.offset, 4);
    assert_eq!(f.width, 4);
    assert!(matches!(f.kind, FieldKind::Uint));
}

#[test]
fn sint_field() {
    let f = field(Reg::FIELDS, "delta");
    assert_eq!(f.offset, 12);
    assert_eq!(f.width, 3);
    assert!(matches!(f.kind, FieldKind::Sint));
}

#[test]
fn enum_field_resolves_variants() {
    let f = field(Reg::FIELDS, "mode");
    assert_eq!(f.offset, 1);
    assert_eq!(f.width, 2);
    let info: &EnumInfo = match f.kind {
        FieldKind::Enum(info) => info,
        _ => panic!("expected enum kind"),
    };
    assert_eq!(info.name, "Mode");
    assert_eq!(
        info.variants,
        &[(0u128, "Off"), (1u128, "On"), (3u128, "Reserved")]
    );
}

#[test]
fn nested_struct_recurses() {
    let f = field(Reg::FIELDS, "inner");
    assert_eq!(f.offset, 8);
    assert_eq!(f.width, 4);
    let inner: &[FieldInfo] = match f.kind {
        FieldKind::Struct(fields) => fields,
        _ => panic!("expected struct kind"),
    };
    assert_eq!(inner.len(), 2);
    assert_eq!(field(inner, "lo").offset, 0);
    assert_eq!(field(inner, "hi").offset, 2);
}

#[test]
fn split_field_exposes_each_storage_segment() {
    let f = field(SplitReg::FIELDS, "split");
    assert_eq!(f.offset, 0);
    assert_eq!(f.width, 8);
    assert_eq!(f.segments, SplitReg::SPLIT_SEGMENTS);
    assert_eq!(f.segments.len(), 2);
    assert_eq!(f.segments[0].offset, 12);
    assert_eq!(f.segments[0].value_offset, 4);
    assert_eq!(f.segments[0].width, 4);
    assert_eq!(f.segments[1].offset, 0);
    assert_eq!(f.segments[1].value_offset, 0);
    assert_eq!(f.segments[1].width, 4);
}
