use chapa::bitfield;

/// Copy and Clone are implemented automatically.
#[bitfield(u8, order = lsb0)]
pub struct Bare {
    #[bits(0..=3)]
    low: u8,
    #[bits(4..=7)]
    high: u8,
}

/// Explicit Copy and Clone derives also work.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Explicit {
    #[bits(0..=7)]
    value: u8,
}

/// A field default implements Default for the struct.
#[bitfield(u16, order = lsb0)]
pub struct ImpliedDefault {
    #[bits(0..=3, default = 7)]
    mode: u8,
    #[bits(4, default = true)]
    ready: bool,
    #[bits(5..=15)]
    rest: u16,
}

/// A derive listing only traits the macro implements itself still expands
/// (the forwarded derive attribute is empty in this case).
#[bitfield(u8, order = lsb0)]
#[derive(Debug)]
pub struct OnlyOverridden {
    #[bits(0..=7)]
    value: u8,
}

fn assert_copy<T: Copy>() {}
fn assert_clone<T: Clone>() {}

#[test]
fn bare_struct_is_copy_clone() {
    assert_copy::<Bare>();
    assert_clone::<Bare>();

    // Using the builder does not consume the original value.
    let a = Bare::zeroed().with_low(0x3);
    let b = a.with_high(0xC);
    assert_eq!(a.raw(), 0x03);
    assert_eq!(b.raw(), 0xC3);
}

#[test]
fn explicit_derives_still_compile() {
    assert_copy::<Explicit>();
    assert_clone::<Explicit>();
    assert_eq!(Explicit::zeroed(), Explicit::from_raw(0));
}

#[test]
fn derive_with_only_overridden_traits_compiles() {
    assert_copy::<OnlyOverridden>();
    assert_clone::<OnlyOverridden>();
    let v = OnlyOverridden::zeroed().with_value(0xAB);
    assert_eq!(format!("{:?}", v), "OnlyOverridden { value: 171 }");
}

#[test]
fn field_default_implies_default_impl() {
    let d = ImpliedDefault::default();
    assert_eq!(d.mode(), 7);
    assert_eq!(d.ready(), true);
    assert_eq!(d.rest(), 0);
    // mode=7 (bits 0..=3) | ready (bit 4 -> 0x10)
    assert_eq!(d.raw(), 0x17);
}
