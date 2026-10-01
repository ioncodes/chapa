use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct DefaultFixed {
    #[bits(0, fixed = true, default = true)]
    value: bool,
}
#[bitfield(u8, order = lsb0)]
struct Duplicate {
    #[bits(0, fixed = true, fixed = true)]
    value: bool,
}
#[bitfield(u8, order = lsb0)]
struct Unsupported {
    #[bits(0, fixed = 0)]
    value: Custom,
}
fn main() {}
