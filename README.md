# chapa

[![crates.io](https://img.shields.io/crates/v/chapa.svg)](https://crates.io/crates/chapa)
[![docs.rs](https://docs.rs/chapa/badge.svg)](https://docs.rs/chapa)

Bitfield structs, batteries included!

`chapa` exposes an attribute macro, `#[bitfield]`, that turns an ordinary
struct into a newtype backed by a single primitive. Every field maps to an exact
range of bits and gets a generated getter, setter, and `with_*` builder. A
companion attribute macro, `#[bitenum]`, makes a C-like enum usable as a field
type.

## Features

- **MSB0 and LSB0 support**: Naturally write bit orders as per datasheet
- **Signed fields**: `i8`...`i128` field types with automatic sign extension
- **Enum fields**: Use enums as bitfield fields with `#[bitenum]`
- **Nested bitfields**: Embed one bitfield struct inside another
- **Readonly fields**: Suppress setter generation with `readonly` or a leading `_` prefix
- **Default values**: Set a field's initial value with `default = ...`
- **Aliases**: Expose extra accessor names with `alias = "name"` or `alias = ["a", "b"]`
- **Overlays**: Allow multiple logically distinct field groups to share the same bit range
- **Bitwise operators**: `&`, `|`, `^`, `!`, `&=`, `|=`, `^=` with the backing storage type work directly on the struct
- **Raw arithmetic**: `wrapping_`, `saturating_`, `checked_`, and `overflowing_` variants of `add`/`sub` on the raw storage value
- **Bit extraction**: `extract_bits!` masks a value to keep only the specified bit ranges
- **Bit insertion**: `place_bits!` shifts a value into a range, `insert_bits!` merges already-positioned bits
- **Reflection**: Opt into the `reflection` feature for compile-time field metadata (`FIELDS`, bit positions, enum variants)

## MSRV

Requires Rust 1.83 or newer (the generated getters, setters, and `with_*`
builders are `const fn`).

## Quick start

```rust
use chapa::bitfield;

// An 8-bit status register, bit 0 is the LSB
#[bitfield(u8, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct StatusReg {
    #[bits(0)] enabled: bool,
    #[bits(1..=3)] mode: u8,
    #[bits(4..=7)] _reserved: u8, // Leading "_" makes the field readonly
}

let r = StatusReg::zeroed()
    .with_enabled(true)
    .with_mode(5);

assert!(r.enabled());
assert_eq!(r.mode(), 5);
assert_eq!(r.reserved(), 0);    // accessible as `reserved`, not `_reserved`
```

## `#[bitfield(...)]` options

| Option                                | Required | Description                                       |
| ------------------------------------- | -------- | ------------------------------------------------- |
| `u8` / `u16` / `u32` / `u64` / `u128` | Yes      | Backing storage type                              |
| `order = msb0` / `order = lsb0`       | Yes      | Bit numbering convention                          |
| `width = N`                           | No       | Effective logical width, must be <= storage width |

## `#[bits(...)]` options

| Option              | Description                                       |
| ------------------- | ------------------------------------------------- |
| `N`                 | Single bit at index N                             |
| `N..=M`             | Inclusive range from bit N to bit M               |
| `N..M`              | Half-open range (equivalent to `N..=(M-1)`)       |
| `readonly`          | Suppress `set_*` and `with_*` generation          |
| `default = <expr>`  | Starting value applied by `default()`             |
| `alias = "name"`    | Generate additional accessor under `name`         |
| `alias = ["a","b"]` | Multiple aliases                                  |
| `overlay = "group"` | Allow overlap with fields in other overlay groups |

A field's type may be `bool` (single bit), an unsigned integer (`u8`...`u128`),
a signed integer (`i8`...`i128`, two's-complement: sign-extended on read,
truncated to the field width on write), a `#[bitenum]` enum, or another
bitfield struct.

## MSB-0 example

```rust
use chapa::bitfield;

// A 32-bit value where bit 0 is the most-significant bit
#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ControlWord {
    #[bits(0..=3)] opcode: u8,
    #[bits(4..=7)] dst: u8,
    #[bits(8..=31, readonly)] payload: u32,
}

let cw = ControlWord::zeroed()
  .with_opcode(0xA)
  .with_dst(0x3);
assert_eq!(cw.raw(), 0xA300_0000);
```

## Enum fields

Use `#[bitenum]` on an enum to implement `BitField`, allowing it to be
used as a bitfield field type. The enum must mark exactly one variant
`#[fallback]`.

```rust
use chapa::{bitfield, bitenum, BitField};

#[bitenum]
#[derive(Debug, PartialEq)]
pub enum VideoFormat {
    Ntsc = 0,
    Pal = 1,
    Mpal = 2,
    #[fallback]
    Debug = 3,
}

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DisplayConfig {
    #[bits(0)] enable: bool,
    #[bits(1..=2)] fmt: VideoFormat,
}

let dc = DisplayConfig::zeroed()
    .with_enable(true)
    .with_fmt(VideoFormat::Pal);
assert_eq!(dc.fmt(), VideoFormat::Pal);

// Unrecognized raw values are handled two ways:
//   - from_raw (and the getter dc.fmt()) coerce them to the #[fallback] variant
//   - try_from_raw / TryFrom reject them, so corrupt input can be detected
assert_eq!(VideoFormat::from_raw(9), VideoFormat::Debug);   // coerced to #[fallback]
assert!(VideoFormat::try_from_raw(9).is_err());             // detected
assert_eq!(VideoFormat::try_from(9u8).unwrap_err().raw, 9); // TryFrom<u8>
```

## Nested bitfields

A field whose type implements `chapa::BitField` (i.e. any type annotated with
`#[bitfield]`) can be used as a nested field.

```rust
use chapa::bitfield;

#[bitfield(u8, order = msb0, width = 4)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Nibble {
    #[bits(0..=1)] high: u8,
    #[bits(2..=3)] low: u8,
}

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Word {
    #[bits(0..=3)] top: Nibble,
    #[bits(28..=31)] bottom: u8,
}

let nibble = Nibble::zeroed().with_high(2).with_low(1);
let word = Word::zeroed().with_top(nibble).with_bottom(0xA);

assert_eq!(nibble.raw(), 0b1001);
assert_eq!(word.raw(), 0x9000_000A);
assert_eq!(word.top(), nibble);
```

## Overlay groups

Fields in **different** overlay groups may share bit ranges. This is useful for
instruction formats where the same bits are interpreted differently depending on
other bits. This is useful for instruction decoding, but also to handle specific MMIO registers
that change their meaning depending on certain encoded bits.

```rust
use chapa::bitfield;

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Instr {
    #[bits(0..=5)] opcode: u8,

    #[bits(6..=10,  overlay = "r_form")] rs: u8,
    #[bits(11..=15, overlay = "r_form")] ra: u8,
    #[bits(16..=20, overlay = "r_form")] rb: u8,

    #[bits(6..=10,  overlay = "i_form")] dst: u8,
    #[bits(11..=31, overlay = "i_form")] imm: u32,
}

let r_form = Instr::zeroed().with_opcode(0x20).with_rs(3).with_ra(4).with_rb(5);
assert_eq!((r_form.rs(), r_form.ra(), r_form.rb()), (3, 4, 5));

let i_form = Instr::zeroed().with_opcode(0x08).with_dst(7).with_imm(0x1_2345);
assert_eq!((i_form.dst(), i_form.imm()), (7, 0x1_2345));
assert_eq!(i_form.rs(), i_form.dst()); // Both names cover bits 6..=10
```

## Constructors and default values

Every struct has a `const fn zeroed()` that returns an all-zero value. There is
no `new()`. Add `default = <expr>` to give a field a different initial value.
This automatically implements `Default`. The `zeroed()` and `from_raw()`
methods do not apply field defaults. If no fields have defaults, you can still
use `#[derive(Default)]` to make `default()` return `zeroed()`.

`default` works on any field type (`bool`, integer, `#[bitenum]` enum,
or nested bitfield, e.g. `default = Mode::On`), including `readonly` ones.
Values wider than the field truncate to its width, exactly like a setter.

```rust
use chapa::bitfield;

#[bitfield(u16, order = lsb0)]
#[derive(Debug, PartialEq)]
pub struct Config {
    #[bits(0)] enabled: bool,
    #[bits(1..=3, default = 5)] mode: u8,
    #[bits(8, default = true)] ready: bool,
}

let c = Config::default();
assert_eq!(c.mode(), 5);
assert_eq!(c.ready(), true);
assert_eq!(c.enabled(), false);   // no default -> zero

// zeroed() and from_raw never inject defaults
assert_eq!(Config::zeroed().mode(), 0);
assert_eq!(Config::from_raw(0).mode(), 0);
```

## Bitwise operations

Every bitfield struct implements `BitAnd`, `BitOr`, `BitXor`, `Not`,
`BitAndAssign`, `BitOrAssign`, and `BitXorAssign` against its backing storage type.

```rust
use chapa::bitfield;

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StatusReg {
    #[bits(0)] enabled: bool,
    #[bits(1..=7)] flags: u8,
}

const HIGH_BYTE: u32 = 0xFF00_0000;
let current = StatusReg::from_raw(0x1234_5678);
let incoming: u32 = 0xABCD_EF01;

// Keep the low 24 bits of `current`, replacing only its high byte.
let updated = (current & !HIGH_BYTE) | (incoming & HIGH_BYTE);
assert_eq!(updated.raw(), 0xAB34_5678);
assert!(updated.enabled());
assert_eq!(updated.flags(), 0x2B);
```

## Raw arithmetic

Every bitfield struct provides `wrapping_add`, `wrapping_sub`, `saturating_add`,
`saturating_sub`, `checked_add`, `checked_sub`, `overflowing_add`, and
`overflowing_sub`, mirroring the methods on the backing storage type. They
operate on the full raw storage value, exactly like `raw()`: carries and
borrows propagate across field boundaries, and bit ordering plays no role.

```rust
use chapa::bitfield;

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Counter {
    #[bits(0..=7)] low: u8,
    #[bits(8..=15)] high: u8,
}

let c = Counter::from_raw(0xFFFF).wrapping_add(1);
assert_eq!(c.raw(), 0x0000);

// Carries cross field boundaries, just like on the raw integer.
let c = Counter::from_raw(0x00FF).wrapping_add(1);
assert_eq!((c.low(), c.high()), (0x00, 0x01));

assert_eq!(Counter::from_raw(0xFFFF).checked_add(1), None);
assert_eq!(Counter::from_raw(0x0005).saturating_sub(0x10).raw(), 0x0000);

let (c, borrowed) = Counter::from_raw(0x0000).overflowing_sub(1);
assert_eq!(c.raw(), 0xFFFF);
assert!(borrowed);
```

If you need wrap-around at a single field's width instead, the setters already
truncate to the field width, so `c.set_low(c.low().wrapping_add(1))` wraps
correctly within `low` alone.

## Bit extraction

`extract_bits!` keeps only the specified bit positions from a value, zeroing all others.
Bits can be single indices, inclusive `start..=end` ranges, or half-open
`start..end` ranges. Indices and ranges may be runtime expressions.

For raw integers, specify the ordering and type explicitly:

```rust
use chapa::extract_bits;

let val: u32 = 0xFFFF_FFFF;
// MSB0: keep bits 0, 5–9, 16–31
let masked = extract_bits!(msb0 u32; val; 0, 5..=9, 16..=31);
assert_eq!(masked, 0x87C0_FFFF);

// LSB0: keep bits 0–3 and 12–15
let masked = extract_bits!(lsb0 u16; val as u16; 0..=3, 12..=15);
assert_eq!(masked, 0xF00F);

// Runtime ranges work too.
let offset = 8u8;
let masked = extract_bits!(lsb0 u32; val; offset..offset + 8);
assert_eq!(masked, 0x0000_FF00);
```

For chapa bitfield structs, omit the ordering. It is deduced from the struct's
`#[bitfield]` definition, and the result has the same struct type:

```rust
use chapa::{bitfield, extract_bits};

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone)]
pub struct Packet {
    #[bits(0)] priority: bool,
    #[bits(5..=9)] kind: u8,
    #[bits(16..=31)] payload: u16,
}

let packet = Packet::from_raw(0xFFFF_FFFF);
let masked: Packet = extract_bits!(packet; 0, 5..=9, 16..=31);
assert_eq!(masked.raw(), 0x87C0_FFFF);
```

The explicit form (`msb0 u32`) remains usable in const contexts when its value
and bit specs are literals. Runtime specs are evaluated when the macro is
called. The struct form calls an `#[inline]` helper and has no language-level
`const` guarantee.

## Bit insertion

These macros update bit ranges without using field setters:

- `place_bits!` shifts a right-aligned value into one bit or range.
- `insert_bits!` copies already-positioned bits into one or more ranges.

Both macros support explicit `msb0` and `lsb0` forms. With a bitfield value, the
ordering is inferred. They return the updated value instead of changing it in
place. Bit indices and ranges may be runtime expressions.

```rust
use chapa::{bitfield, place_bits, insert_bits};

#[bitfield(u32, order = lsb0)]
pub struct Reg {
    #[bits(0..=7)] b0: u8,
    #[bits(8..=15)] b1: u8,
    #[bits(16..=31)] hi: u16,
}

// Write 0xAB to bits 8..=15.
let mut reg = Reg::from_raw(0xDEAD_0000);
reg = place_bits!(reg; 8..=15; 0xABu8);
assert_eq!(reg.b1(), 0xAB);
assert_eq!(reg.hi(), 0xDEAD);

// Replace the low two bytes with already-positioned bits.
reg = insert_bits!(reg; 0..=15; 0x0000_1234u32);
assert_eq!(reg.raw(), 0xDEAD_1234);
```

The explicit forms remain const-evaluable with literal specs. For an `msb0`
bitfield with `width = N`, use the explicit form because the inferred form uses
the full storage width.

## Reflection

Enable the `reflection` feature to get compile-time field metadata for every
`#[bitfield]` struct and `#[bitenum]` enum:

```toml
[dependencies]
chapa = { version = "0.8", features = ["reflection"] }
```

Each bitfield struct gains an inherent `FIELDS: &'static [FieldInfo]` const
describing its fields: their accessor name, bit position, aliases and how the
raw bits should be interpreted. Offsets and widths are **physical** (in
storage-value "coordinates"), so a field's value is always
`(raw >> offset) & ((1 << width) - 1)` regardless of `msb0`/`lsb0` ordering.
Nested enum and struct fields carry their own variant table / fields.

```rust
use chapa::{bitfield, bitenum, FieldKind};

#[bitenum]
pub enum Mode { Off = 0, On = 1, #[fallback] Reserved = 3 }

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone)]
pub struct Reg {
    #[bits(0)] enabled: bool,
    #[bits(1..=2)] mode: Mode,
    #[bits(4..=7)] count: u8,
}

let mode = Reg::FIELDS.iter().find(|f| f.name == "mode").unwrap();
assert_eq!(mode.offset, 1);
assert_eq!(mode.width, 2);
if let FieldKind::Enum(info) = mode.kind {
    assert_eq!(info.name, "Mode");
    assert_eq!(info.variants, &[(0, "Off"), (1, "On"), (3, "Reserved")]);
} else {
    panic!("mode should be reflected as an enum");
}
```

`FieldKind` distinguishes `Bool`, `Uint`, `Sint`, `Enum(&EnumInfo)` and
`Struct(&[FieldInfo])`. The types (`FieldInfo`, `FieldKind`, `EnumInfo`) and the
`Reflect` trait are re-exported at the crate root when the feature is on.

## Generated API

For a field `foo: u8` spanning bits `4..=7` the macro generates:

| Item     | Signature                                      |
| -------- | ---------------------------------------------- |
| Constant | `pub const FOO_SHIFT: u32`                     |
| Constant | `pub const FOO_MASK: StorageType`              |
| Getter   | `pub const fn foo(&self) -> u8`                |
| Setter   | `pub const fn set_foo(&mut self, val: u8)`     |
| Builder  | `pub const fn with_foo(self, val: u8) -> Self` |

Every struct also provides these methods (`N` is the storage size in bytes):

| Item       | Signature                                            |
| ---------- | ---------------------------------------------------- |
| Zeroed     | `pub const fn zeroed() -> Self`                      |
| Raw access | `pub const fn from_raw(val: StorageType) -> Self`    |
| Raw access | `pub const fn raw(&self) -> StorageType`             |
| Bytes      | `pub const fn to_le_bytes(self) -> [u8; N]`          |
| Bytes      | `pub const fn to_be_bytes(self) -> [u8; N]`          |
| Bytes      | `pub const fn to_ne_bytes(self) -> [u8; N]`          |
| Bytes      | `pub const fn from_le_bytes(bytes: [u8; N]) -> Self` |
| Bytes      | `pub const fn from_be_bytes(bytes: [u8; N]) -> Self` |
| Bytes      | `pub const fn from_ne_bytes(bytes: [u8; N]) -> Self` |
| Arithmetic | `pub const fn wrapping_add(self, rhs: StorageType) -> Self` (same shape for `wrapping_sub`, `saturating_add`, `saturating_sub`) |
| Arithmetic | `pub const fn checked_add(self, rhs: StorageType) -> Option<Self>` (same shape for `checked_sub`) |
| Arithmetic | `pub const fn overflowing_add(self, rhs: StorageType) -> (Self, bool)` (same shape for `overflowing_sub`) |

The byte conversions and arithmetic methods operate on the full storage value,
matching `raw()` and `from_raw()`.

Additionally, every struct implements the following traits:

| Trait          | Signature                                       |
| -------------- | ----------------------------------------------- |
| `BitAnd`       | `fn bitand(self, rhs: StorageType) -> Self`     |
| `BitOr`        | `fn bitor(self, rhs: StorageType) -> Self`      |
| `BitXor`       | `fn bitxor(self, rhs: StorageType) -> Self`     |
| `Not`          | `fn not(self) -> Self`                          |
| `BitAndAssign` | `fn bitand_assign(&mut self, rhs: StorageType)` |
| `BitOrAssign`  | `fn bitor_assign(&mut self, rhs: StorageType)`  |
| `BitXorAssign` | `fn bitxor_assign(&mut self, rhs: StorageType)` |

## Contributors

* [Estus](https://github.com/Estus-Dev)
* [nett_hier](https://github.com/netthier)
