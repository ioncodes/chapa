# chapa

Bitfield structs, batteries included!

`chapa` exposes a single attribute macro, `#[bitfield]`, that turns an ordinary
struct into newtype backed by a single primitive. Every field maps to an exact
range of bits and gets a generated getter, setter, and `with_*` builder.

## Features

- **MSB0 and LSB0 support**: Naturally write bit orders as per datasheet
- **Enum fields**: Use enums as bitfield fields with `#[derive(BitEnum)]`
- **Nested bitfields**: Embed one bitfield struct inside another
- **Readonly fields**: Suppress setter generation with `readonly` or a leading `_` prefix
- **Aliases**: Expose extra accessor names with `alias = "name"` or `alias = ["a", "b"]`
- **Overlays**: Allow multiple logically distinct field groups to share the same bit range
- **Bitwise operators**: `&`, `|`, `^`, `!` with the backing storage type work directly on the struct
- **Bit extraction**: `extract_bits!` masks a value to keep only the specified bit ranges

## Installation

```toml
[dependencies]
chapa = { git = "http://github.com/ioncodes/chapa" }
```

## Quick start

```rust
use chapa::bitfield;

// An 8-bit status register, bit 0 is the LSB
#[bitfield(u8, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StatusReg {
    #[bits(0)] enabled: bool,
    #[bits(1..=3)] mode: u8,
    #[bits(4..=7)] _reserved: u8 // Can be ommited; "_" makes it readonly
}

let r = StatusReg::new()
    .with_enabled(true)
    .with_mode(5);

assert_eq!(r.enabled(), true);
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
| `alias = "name"`    | Generate additional accessor under `name`         |
| `alias = ["a","b"]` | Multiple aliases                                  |
| `overlay = "group"` | Allow overlap with fields in other overlay groups |

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

let cw = ControlWord::new()
  .with_opcode(0xA)
  .with_dst(0x3);
assert_eq!(cw.raw(), 0xA300_0000);
```

## Enum fields

Use `#[derive(BitEnum)]` on an enum to automatically implement `BitField`,
allowing it to be used as a bitfield field type. `Copy` and `Clone` are derived
automatically.

```rust
use chapa::{bitfield, BitEnum};

#[derive(Debug, PartialEq, BitEnum)]
pub enum VideoFormat {
    Ntsc = 0,
    Pal = 1,
    Mpal = 2,
    Debug = 3,
}

#[bitfield(u16, order = lsb0)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DisplayConfig {
    #[bits(0)] enable: bool,
    #[bits(1..=2)] fmt: VideoFormat,
}

let dc = DisplayConfig::new()
    .with_enable(true)
    .with_fmt(VideoFormat::Pal);
assert_eq!(dc.fmt(), VideoFormat::Pal);
```

Note: Invalid raw values map to the last variant!

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
    #[bits(28..=31)] bot: u8,
}
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
```

## Bitwise operations

Every bitfield struct implements `BitAnd`, `BitOr`, `BitXor`, and `Not` against
its backing storage type.

```rust
use chapa::bitfield;

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone)]
pub struct Msr {
    #[bits(16, alias = "rnd1")] random1: bool,
    #[bits(17, alias = "rnd2")] random2: bool,
}

const RESTORE_MASK: u32 = 0x0000_FF73;

let srr1: u32 = 0x0000_8000;
let msr = Msr::new();

// No .raw() / from_raw() needed:
let updated = (msr & !RESTORE_MASK) | (srr1 & RESTORE_MASK);
```

## Bit extraction

`extract_bits!` keeps only the specified bit positions from a value, zeroing all others.
Bits can be single indices or inclusive `start..=end` ranges.

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
```

For chapa bitfield structs, omit the ordering, it is deduced from the struct's `#[bitfield]` definition and the result is returned as the same struct type:

```rust
use chapa::{bitfield, extract_bits};

#[bitfield(u32, order = msb0)]
#[derive(Copy, Clone)]
pub struct Msr { /* ... */ }

let msr = Msr::from_raw(0xFFFF_FFFF);
let masked: Msr = extract_bits!(msr; 0..=0, 5..=9, 16..=31);
let srr1: u32 = masked.raw();
```

The explicit form (`msb0 u32`) emits `const MASK: T = ...`, so the mask is guaranteed to be computed at compile time. The struct form calls an `#[inline]` helper; LLVM should constant-fold the mask in practice, but there is no language-level guarantee.

## Generated API

For a field `foo: u8` spanning bits `4..=7` the macro generates:

| Item     | Signature                                      |
| -------- | ---------------------------------------------- |
| Constant | `pub const FOO_SHIFT: u32`                     |
| Constant | `pub const FOO_MASK: StorageType`              |
| Getter   | `pub const fn foo(&self) -> u8`                |
| Setter   | `pub fn set_foo(&mut self, val: u8)`           |
| Builder  | `pub const fn with_foo(self, val: u8) -> Self` |

Additionally, every struct implements the following traits:

| Trait    | Signature                                   |
| -------- | ------------------------------------------- |
| `BitAnd` | `fn bitand(self, rhs: StorageType) -> Self` |
| `BitOr`  | `fn bitor(self, rhs: StorageType) -> Self`  |
| `BitXor` | `fn bitxor(self, rhs: StorageType) -> Self` |
| `Not`    | `fn not(self) -> Self`                      |