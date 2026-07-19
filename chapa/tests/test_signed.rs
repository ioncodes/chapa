use chapa::bitfield;

// A 3-bit signed immediate: raw 0b111 decodes as -1i8 (0b1111_1111), not 7.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Imm3 {
    #[bits(0..=2)]
    imm: i8,
    #[bits(3..=7)]
    rest: u8,
}

// ARM branch encoding: cond in the top nibble, signed imm24 in the low bits.
#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BranchInstr {
    #[bits(0..=3)]
    cond: u8,
    #[bits(4..=7)]
    opcode: u8,
    #[bits(8..=31)]
    offset: i32,
}

// Field type wider than the storage.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WideField {
    #[bits(0..=2)]
    v: i32,
}

// Field spanning the entire storage (sign shift of zero).
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FullWidth {
    #[bits(0..=7)]
    v: i8,
}

#[test]
fn sign_extends_on_read() {
    assert_eq!(Imm3::from_raw(0b111).imm(), -1);
    assert_eq!(Imm3::from_raw(0b100).imm(), -4);
    assert_eq!(Imm3::from_raw(0b011).imm(), 3);
    assert_eq!(Imm3::from_raw(0b000).imm(), 0);
}

#[test]
fn truncates_on_write() {
    assert_eq!(Imm3::zeroed().with_imm(-1).raw(), 0b111);
    assert_eq!(Imm3::zeroed().with_imm(-4).raw(), 0b100);
    assert_eq!(Imm3::zeroed().with_imm(3).raw(), 0b011);
}

#[test]
fn roundtrip_all_values() {
    for v in -4i8..=3 {
        assert_eq!(Imm3::zeroed().with_imm(v).imm(), v);
    }
}

#[test]
fn neighbors_untouched() {
    // Setting a negative value must not spill sign bits into `rest`.
    let r = Imm3::from_raw(0xF8).with_imm(-1);
    assert_eq!(r.raw(), 0xFF);
    assert_eq!(r.rest(), 0x1F);

    let r = r.with_imm(0);
    assert_eq!(r.raw(), 0xF8);
    assert_eq!(r.rest(), 0x1F);
}

#[test]
fn getter_ignores_neighbor_bits() {
    // All neighbor bits set, field bits positive: no stray sign extension.
    assert_eq!(Imm3::from_raw(0xFB).imm(), 3);
}

#[test]
fn msb0_sign_extension() {
    // `b .-0` -> EA FF FF FE: cond=AL, imm24 = 0xFFFFFE = -2
    let i = BranchInstr::from_raw(0xEAFF_FFFE);
    assert_eq!(i.cond(), 0xE);
    assert_eq!(i.opcode(), 0xA);
    assert_eq!(i.offset(), -2);
}

#[test]
fn msb0_write() {
    let i = BranchInstr::from_raw(0xEA00_0000).with_offset(-2);
    assert_eq!(i.raw(), 0xEAFF_FFFE);
    assert_eq!(
        BranchInstr::zeroed().with_offset(0x7F_FFFF).offset(),
        0x7F_FFFF
    );
}

#[test]
fn field_type_wider_than_storage() {
    assert_eq!(WideField::from_raw(0b101).v(), -3);
    assert_eq!(WideField::zeroed().with_v(-3).raw(), 0b101);
    assert_eq!(WideField::zeroed().with_v(2).v(), 2);
}

#[test]
fn full_width_field() {
    assert_eq!(FullWidth::from_raw(0xFF).v(), -1);
    assert_eq!(FullWidth::zeroed().with_v(-128).raw(), 0x80);
    assert_eq!(FullWidth::from_raw(0x7F).v(), 127);
}

#[test]
fn signed_default() {
    #[bitfield(u16, order = lsb0)]
    #[derive(Copy, Clone, Default)]
    pub struct Cfg {
        #[bits(0..=4, default = -6)]
        trim: i8,
        #[bits(5..=7)]
        pad: u8,
    }

    assert_eq!(Cfg::default().trim(), -6);
    assert_eq!(Cfg::default().raw(), 0b11010);
    assert_eq!(Cfg::zeroed().trim(), 0);
}

#[test]
fn readonly_signed() {
    #[bitfield(u8, order = lsb0)]
    #[derive(Copy, Clone)]
    pub struct Ro {
        #[bits(0..=3, readonly)]
        v: i8,
        #[bits(4..=7)]
        _pad: u8,
    }

    assert_eq!(Ro::from_raw(0x0F).v(), -1);
}

#[test]
fn const_eval() {
    const IMM: i8 = Imm3::from_raw(0b100).imm();
    const REG: Imm3 = Imm3::zeroed().with_imm(-1);
    assert_eq!(IMM, -4);
    assert_eq!(REG.raw(), 0b111);
}

#[test]
fn consts() {
    assert_eq!(Imm3::IMM_SHIFT, 0);
    assert_eq!(Imm3::IMM_MASK, 0b111);
    assert_eq!(BranchInstr::OFFSET_SHIFT, 0);
    assert_eq!(BranchInstr::OFFSET_MASK, 0x00FF_FFFF);
}
