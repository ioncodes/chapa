use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Invalid {
    #[bits(0..=2, fixed = -5)]
    value: i8,
}
fn main() {}
