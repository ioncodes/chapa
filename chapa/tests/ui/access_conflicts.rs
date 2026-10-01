use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Explicit {
    #[bits(0, readonly, writeonly)]
    value: bool,
}
#[bitfield(u8, order = lsb0)]
struct Implicit {
    #[bits(0, writeonly)]
    _value: bool,
}
#[bitfield(u8, order = lsb0)]
struct Fixed {
    #[bits(0, fixed = true, writeonly)]
    value: bool,
}
fn main() {}
