use chapa::{bitfield, BitEnum};

/// A simple LSB-0 register with a bool and integer field.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StatusReg {
    #[bits(0)] enabled: bool,
    #[bits(1..=3)] mode: u8,
    #[bits(4..=7, readonly)] _reserved: u8,
}

#[test]
fn debug_lsb0_struct() {
    let r = StatusReg::new().with_enabled(true).with_mode(5);
    let s = format!("{:?}", r);
    assert_eq!(s, "StatusReg { enabled: true, mode: 5, reserved: 0 }");
}

#[derive(Debug, PartialEq, BitEnum)]
pub enum Mode {
    Off = 0,
    On = 1,
    Turbo = 2,
    Reserved = 3,
}

/// Bitfield with an enum field.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CtrlReg {
    #[bits(0..=1)] mode: Mode,
    #[bits(2)] active: bool,
}

#[test]
fn debug_enum_field() {
    let r = CtrlReg::new().with_mode(Mode::On).with_active(true);
    let s = format!("{:?}", r);
    // Mode::On Debug output is "On"
    assert!(s.contains("On"));
    assert!(s.contains("active: true"));
}

/// Bitfield where Debug is in the derive list, macro should replace it with
/// the bitfield-aware impl.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlagsReg {
    #[bits(0)] flag_a: bool,
    #[bits(1)] flag_b: bool,
}

#[test]
fn debug_with_derive_debug_in_attrs() {
    let r = FlagsReg::new().with_flag_a(true);
    let s = format!("{:?}", r);
    assert_eq!(s, "FlagsReg { flag_a: true, flag_b: false }");
}
