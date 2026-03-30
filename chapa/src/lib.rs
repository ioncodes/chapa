//! # chapa
//!
//! Bitfield structs, batteries included!
//!
//! `chapa` exposes a single attribute macro, [`bitfield`], that turns an ordinary
//! struct into a newtype backed by a single primitive. Every field maps to an exact
//! range of bits and gets a generated getter, setter, and `with_*` builder.
//!
//! ## Features
//!
//! - **MSB0 and LSB0 support**: Naturally write bit orders as per datasheet
//! - **Enum fields**: Use enums as bitfield fields with `#[derive(BitEnum)]`
//! - **Nested bitfields**: Embed one bitfield struct inside another
//! - **Readonly fields**: Suppress setter generation with `readonly` or a leading `_` prefix
//! - **Aliases**: Expose extra accessor names with `alias = "name"` or `alias = ["a", "b"]`
//! - **Overlays**: Allow multiple logically distinct field groups to share the same bit range
//! - **Bitwise operators**: `&`, `|`, `^`, `!`, `&=`, `|=`, `^=` with the backing storage type work directly on the struct
//! - **Bit extraction**: [`extract_bits!`] masks a value to keep only the specified bit ranges
//!
//! ## Quick start
//!
//! ```rust
//! use chapa::bitfield;
//!
//! // An 8-bit status register, bit 0 is the LSB
//! #[bitfield(u8, order = lsb0)]
//! #[derive(Copy, Clone, Debug, PartialEq)]
//! pub struct StatusReg {
//!     #[bits(0)] enabled: bool,
//!     #[bits(1..=3)] mode: u8,
//!     #[bits(4..=7)] _reserved: u8, // Can be omitted; "_" makes it readonly
//! }
//!
//! let r = StatusReg::new()
//!     .with_enabled(true)
//!     .with_mode(5);
//!
//! assert_eq!(r.enabled(), true);
//! assert_eq!(r.mode(), 5);
//! assert_eq!(r.reserved(), 0); // accessible as `reserved`, not `_reserved`
//! ```
//!
//! ## `#[bitfield(...)]` options
//!
//! | Option | Required | Description |
//! |---|---|---|
//! | `u8` / `u16` / `u32` / `u64` / `u128` | Yes | Backing storage type |
//! | `order = msb0` / `order = lsb0` | Yes | Bit numbering convention |
//! | `width = N` | No | Effective logical width, must be <= storage width |
//!
//! ## `#[bits(...)]` options
//!
//! | Option | Description |
//! |---|---|
//! | `N` | Single bit at index N |
//! | `N..=M` | Inclusive range from bit N to bit M |
//! | `N..M` | Half-open range (equivalent to `N..=(M-1)`) |
//! | `readonly` | Suppress `set_*` and `with_*` generation |
//! | `alias = "name"` | Generate additional accessor under `name` |
//! | `alias = ["a","b"]` | Multiple aliases |
//! | `overlay = "group"` | Allow overlap with fields in other overlay groups |
//!
//! ## MSB-0 example
//!
//! ```rust
//! use chapa::bitfield;
//!
//! // A 32-bit value where bit 0 is the most-significant bit
//! #[bitfield(u32, order = msb0)]
//! #[derive(Copy, Clone, Debug, PartialEq)]
//! pub struct ControlWord {
//!     #[bits(0..=3)] opcode: u8,
//!     #[bits(4..=7)] dst: u8,
//!     #[bits(8..=31, readonly)] payload: u32,
//! }
//!
//! let cw = ControlWord::new()
//!     .with_opcode(0xA)
//!     .with_dst(0x3);
//! assert_eq!(cw.raw(), 0xA300_0000);
//! ```
//!
//! ## Enum fields
//!
//! Use `#[derive(BitEnum)]` on an enum to automatically implement [`BitField`],
//! allowing it to be used as a bitfield field type. `Copy` and `Clone` are derived
//! automatically.
//!
//! Note: Invalid raw values map to the last variant.
//!
//! ```rust
//! use chapa::{bitfield, BitEnum};
//!
//! #[derive(Debug, PartialEq, BitEnum)]
//! pub enum VideoFormat {
//!     Ntsc = 0,
//!     Pal = 1,
//!     Mpal = 2,
//!     Debug = 3,
//! }
//!
//! #[bitfield(u16, order = lsb0)]
//! #[derive(Copy, Clone, Debug, PartialEq)]
//! pub struct DisplayConfig {
//!     #[bits(0)] enable: bool,
//!     #[bits(1..=2)] fmt: VideoFormat,
//! }
//!
//! let dc = DisplayConfig::new()
//!     .with_enable(true)
//!     .with_fmt(VideoFormat::Pal);
//! assert_eq!(dc.fmt(), VideoFormat::Pal);
//! ```
//!
//! ## Nested bitfields
//!
//! A field whose type implements [`BitField`] (i.e. any type annotated with
//! `#[bitfield]`) can be used as a nested field.
//!
//! ```rust
//! use chapa::bitfield;
//!
//! #[bitfield(u8, order = msb0, width = 4)]
//! #[derive(Copy, Clone, Debug, PartialEq)]
//! pub struct Nibble {
//!     #[bits(0..=1)] high: u8,
//!     #[bits(2..=3)] low: u8,
//! }
//!
//! #[bitfield(u32, order = msb0)]
//! #[derive(Copy, Clone, Debug, PartialEq)]
//! pub struct Word {
//!     #[bits(0..=3)] top: Nibble,
//!     #[bits(28..=31)] bot: u8,
//! }
//! ```
//!
//! ## Overlay groups
//!
//! Fields in **different** overlay groups may share bit ranges. This is useful for
//! instruction formats where the same bits are interpreted differently depending on
//! other bits, such as instruction decoding or MMIO registers that change meaning
//! based on encoded bits.
//!
//! ```rust
//! use chapa::bitfield;
//!
//! #[bitfield(u32, order = msb0)]
//! #[derive(Copy, Clone, Debug, PartialEq)]
//! pub struct Instr {
//!     #[bits(0..=5)] opcode: u8,
//!
//!     #[bits(6..=10,  overlay = "r_form")] rs: u8,
//!     #[bits(11..=15, overlay = "r_form")] ra: u8,
//!     #[bits(16..=20, overlay = "r_form")] rb: u8,
//!
//!     #[bits(6..=10,  overlay = "i_form")] dst: u8,
//!     #[bits(11..=31, overlay = "i_form")] imm: u32,
//! }
//! ```
//!
//! ## Bitwise operations
//!
//! Every bitfield struct implements [`BitAnd`](core::ops::BitAnd),
//! [`BitOr`](core::ops::BitOr), [`BitXor`](core::ops::BitXor),
//! [`Not`](core::ops::Not), [`BitAndAssign`](core::ops::BitAndAssign),
//! [`BitOrAssign`](core::ops::BitOrAssign), and
//! [`BitXorAssign`](core::ops::BitXorAssign) against its backing storage type.
//!
//! ```rust
//! use chapa::bitfield;
//!
//! #[bitfield(u32, order = msb0)]
//! #[derive(Copy, Clone, PartialEq, Debug)]
//! pub struct StatusReg {
//!     #[bits(0)] enabled: bool,
//!     #[bits(1..=7)] flags: u8,
//! }
//!
//! const MASK: u32 = 0x0000_00FF;
//! let a = StatusReg::new().with_enabled(true);
//! let b: u32 = 0x0000_00AA;
//!
//! let result = (a & !MASK) | (b & MASK); // result: StatusReg
//! ```
//!
//! ## Bit extraction with `extract_bits!`
//!
//! [`extract_bits!`] keeps only the specified bit positions from a value, zeroing all others.
//! Bits can be single indices or inclusive ranges; the ordering and storage type are either
//! supplied explicitly (for raw integers) or deduced from the struct's [`BitField`] impl.
//!
//! ```rust
//! use chapa::{bitfield, extract_bits};
//!
//! #[bitfield(u32, order = msb0)]
//! #[derive(Copy, Clone)]
//! pub struct Msr { /* ... */ }
//!
//! let msr = Msr::from_raw(0xFFFF_FFFF);
//!
//! // Struct form: ordering deduced; returns Msr with non-selected bits zeroed
//! let masked: Msr = extract_bits!(msr; 0..=0, 5..=9, 16..=31);
//!
//! // Explicit form for raw integers: const-evaluated mask
//! let raw: u32 = extract_bits!(msb0 u32; 0xFFFF_FFFFu32; 0, 5..=9, 16..=31);
//! assert_eq!(raw, masked.raw());
//! ```
//!
//! See the [`extract_bits!`] documentation for full syntax details.
//!
//! ## Generated API
//!
//! For a field `foo: u8` spanning bits `4..=7` the macro generates:
//!
//! | Item | Signature |
//! |---|---|
//! | Constant | `pub const FOO_SHIFT: u32` |
//! | Constant | `pub const FOO_MASK: StorageType` |
//! | Getter | `pub const fn foo(&self) -> u8` |
//! | Setter | `pub fn set_foo(&mut self, val: u8)` |
//! | Builder | `pub const fn with_foo(self, val: u8) -> Self` |
//!
//! Additionally, every struct implements the following traits:
//!
//! | Trait             | Signature                                            |
//! |-------------------|------------------------------------------------------|
//! | `BitAnd`          | `fn bitand(self, rhs: StorageType) -> Self`          |
//! | `BitOr`           | `fn bitor(self, rhs: StorageType) -> Self`           |
//! | `BitXor`          | `fn bitxor(self, rhs: StorageType) -> Self`          |
//! | `Not`             | `fn not(self) -> Self`                               |
//! | `BitAndAssign`    | `fn bitand_assign(&mut self, rhs: StorageType)`      |
//! | `BitOrAssign`     | `fn bitor_assign(&mut self, rhs: StorageType)`       |
//! | `BitXorAssign`    | `fn bitxor_assign(&mut self, rhs: StorageType)`      |

#![no_std]

pub mod mask;

pub use chapa_macros::bitfield;
pub use chapa_macros::BitEnum;
pub use mask::{lsb0_mask, msb0_mask};

/// Trait for types that can be used as the backing storage of a bitfield.
///
/// Implemented for `u8`, `u16`, `u32`, `u64`, and `u128`.
pub trait BitStorage: Copy + Sized {
    /// Total number of bits in this type.
    const BITS: u32;
    /// The zero value for this type.
    const ZERO: Self;
    /// Truncating cast from `u128` (used by [`extract_bits!`] internals).
    fn from_u128(v: u128) -> Self;
}

/// Trait implemented by every struct produced by the [`bitfield`] macro and
/// every enum annotated with `#[derive(BitEnum)]`.
///
/// Allows bitfield structs and enums to be used as nested field types inside
/// other bitfield structs.
pub trait BitField: Copy + Sized {
    /// The underlying storage type (e.g. `u8`, `u32`).
    type Storage: BitStorage;

    /// `true` if this bitfield uses MSB0 ordering (bit 0 = most-significant bit).
    ///
    /// Set automatically by the `#[bitfield]` macro. Always `false` for
    /// `#[derive(BitEnum)]` enums (they have no ordering; they are only
    /// used as field types and are never passed to [`extract_bits!`]).
    const IS_MSB0: bool;

    /// Wraps a raw storage value in the bitfield newtype.
    fn from_raw(raw: Self::Storage) -> Self;

    /// Returns the raw storage value.
    fn raw(&self) -> Self::Storage;
}

// Blanket impls of BitStorage for every supported primitive integer.
macro_rules! impl_bit_storage {
    ($($ty:ty),*) => {
        $(
            impl BitStorage for $ty {
                const BITS: u32 = <$ty>::BITS;
                const ZERO: Self = 0;
                #[inline(always)]
                fn from_u128(v: u128) -> Self { v as $ty }
            }
        )*
    };
}

impl_bit_storage!(u8, u16, u32, u64, u128);
