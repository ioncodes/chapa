use chapa::{bitfield, BitEnum};

/// Primitive and bool fields with `default = ...`.
#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Config {
    #[bits(0)]
    enabled: bool,
    #[bits(1..=3, default = 5)]
    mode: u8,
    #[bits(4..=7)]
    channel: u8,
    #[bits(8, default = true)]
    ready: bool,
}

#[test]
fn zero_is_all_zero() {
    let c = Config::zeroed();
    assert_eq!(c.raw(), 0);
    assert_eq!(c.mode(), 0);
    assert_eq!(c.ready(), false);
}

#[test]
fn default_applies_defaults() {
    let c = Config::default();
    assert_eq!(c.enabled(), false); // no default -> zero
    assert_eq!(c.mode(), 5);
    assert_eq!(c.channel(), 0); // no default -> zero
    assert_eq!(c.ready(), true);
    // mode=5 (bits 1..=3 -> 0b101 << 1 = 0xA) | ready=true (bit 8 -> 0x100)
    assert_eq!(c.raw(), 0x10A);
}

#[test]
fn default_differs_from_zero() {
    assert_ne!(Config::default(), Config::zeroed());
}

#[test]
fn from_raw_ignores_defaults() {
    // from_raw is the verbatim escape hatch; it never injects defaults.
    let c = Config::from_raw(0);
    assert_eq!(c.mode(), 0);
    assert_eq!(c.ready(), false);
}

#[test]
fn zero_is_const() {
    const C: Config = Config::zeroed();
    assert_eq!(C.raw(), 0);
}

#[test]
fn defaults_can_be_overridden() {
    let c = Config::default().with_mode(2);
    assert_eq!(c.mode(), 2);
    assert_eq!(c.ready(), true); // untouched default remains
}

/// Readonly fields can carry a default even though they have no setter.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ReadonlyDefault {
    #[bits(0..=3)]
    value: u8,
    #[bits(4..=7, readonly, default = 0xF)]
    version: u8,
}

#[test]
fn readonly_field_default() {
    let r = ReadonlyDefault::default();
    assert_eq!(r.version(), 0xF);
    assert_eq!(r.value(), 0);
    assert_eq!(r.raw(), 0xF0);
    // zeroed() still ignores the default
    assert_eq!(ReadonlyDefault::zeroed().version(), 0);
}

/// A struct with no `default` anywhere: `default()` equals `zeroed()`.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct NoDefaults {
    #[bits(0..=7)]
    byte: u8,
}

#[test]
fn no_defaults_default_equals_zero() {
    assert_eq!(NoDefaults::zeroed().raw(), 0);
    assert_eq!(NoDefaults::default().raw(), 0);
    assert_eq!(NoDefaults::default(), NoDefaults::zeroed());
}

/// Out-of-range default values truncate to the field width, just like setters.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Truncating {
    #[bits(0..=2, default = 0xFF)]
    three_bits: u8,
}

#[test]
fn default_truncates_to_field_width() {
    assert_eq!(Truncating::default().three_bits(), 0x7);
    assert_eq!(Truncating::default().raw(), 0x7);
}

/// MSB0 ordering: default lands in the correct physical bits.
#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Msb0Default {
    #[bits(0..=3, default = 0xA)]
    opcode: u8,
    #[bits(4..=7)]
    dst: u8,
}

#[test]
fn msb0_default_placement() {
    assert_eq!(Msb0Default::default().raw(), 0xA000_0000);
    assert_eq!(Msb0Default::default().opcode(), 0xA);
}

#[derive(Debug, PartialEq, Clone, Copy, BitEnum)]
pub enum Mode {
    Off = 0,
    On = 1,
    #[fallback]
    Turbo = 2,
}

/// Enum fields can carry a default expressed as a variant.
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct WithEnum {
    #[bits(0)]
    active: bool,
    #[bits(1..=2, default = Mode::Turbo)]
    mode: Mode,
}

#[test]
fn enum_field_default() {
    let w = WithEnum::default();
    assert_eq!(w.mode(), Mode::Turbo); // discriminant 2 -> bits 1..=2 = 0b100
    assert_eq!(w.active(), false);
    assert_eq!(w.raw(), 0x4);
    assert_eq!(WithEnum::zeroed().mode(), Mode::Off); // zeroed() ignores defaults
}

/// Nested bitfield fields can carry a default expressed as a struct value.
#[bitfield(u8, order = lsb0, width = 4)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Nibble {
    #[bits(0..=1)]
    high: u8,
    #[bits(2..=3)]
    low: u8,
}

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct WithNested {
    #[bits(0..=3, default = Nibble::zeroed().with_high(3).with_low(1))]
    nibble: Nibble,
    #[bits(4..=7)]
    rest: u8,
}

#[test]
fn nested_field_default() {
    let w = WithNested::default();
    assert_eq!(w.nibble().high(), 3);
    assert_eq!(w.nibble().low(), 1);
    assert_eq!(w.rest(), 0);
    // high=3 (bits 0..=1 = 0b11), low=1 (bits 2..=3 = 0b01) -> nibble = 0b0111
    assert_eq!(w.raw(), 0x7);
    assert_eq!(WithNested::zeroed().raw(), 0);
}
