use chapa::{bitenum, bitfield};

#[bitfield(u16, order = lsb0)]
#[derive(Debug, PartialEq)]
struct LeftRegister {
    #[bits(0..=15)]
    value: u16,
}

#[bitfield(u16, order = msb0)]
#[derive(Debug, PartialEq)]
struct RightRegister {
    #[bits(0..=15)]
    value: u16,
}

#[bitenum]
enum U16Enum {
    Zero = 0,
    Flag = 0x0100,
    #[fallback]
    Unknown = 0xFFFF,
}

#[test]
fn raw_rhs_remains_supported() {
    let value = LeftRegister::from_raw(0xAAAA);

    // The unsuffixed literal still infers the exact storage type.
    assert_eq!((value & 0x0F0F).raw(), 0x0A0A);
    assert_eq!((value | 0x000Fu16).raw(), 0xAAAF);
    assert_eq!((value ^ 0xFFFFu16).raw(), 0x5555);
}

#[test]
fn same_type_rhs() {
    let lhs = LeftRegister::from_raw(0xFF00);
    let rhs = LeftRegister::from_raw(0x0FF0);

    assert_eq!((lhs & rhs).raw(), 0x0F00);
    assert_eq!((lhs | rhs).raw(), 0xFFF0);
    assert_eq!((lhs ^ rhs).raw(), 0xF0F0);
}

#[test]
fn same_storage_cross_type_rhs() {
    let lhs = LeftRegister::from_raw(0xFF00);
    let rhs = RightRegister::from_raw(0x0FF0);

    let result: LeftRegister = lhs & rhs;
    assert_eq!(result.raw(), 0x0F00);

    let reverse: RightRegister = rhs | lhs;
    assert_eq!(reverse.raw(), 0xFFF0);
}

#[test]
fn assignment_accepts_raw_same_and_cross_type_rhs() {
    let mut value = LeftRegister::from_raw(0xFF00);
    value &= RightRegister::from_raw(0x0FF0);
    assert_eq!(value.raw(), 0x0F00);

    value |= LeftRegister::from_raw(0x00F0);
    assert_eq!(value.raw(), 0x0FF0);

    value ^= 0x00FFu16;
    assert_eq!(value.raw(), 0x0F0F);
}

#[test]
fn bitenum_with_same_storage_is_an_operand() {
    let value = LeftRegister::from_raw(0xFFFF);
    assert_eq!((value & U16Enum::Flag).raw(), 0x0100);
}
