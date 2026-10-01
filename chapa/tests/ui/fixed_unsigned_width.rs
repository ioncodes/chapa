use chapa::bitfield;

#[bitfield(u8, order = lsb0)]
struct Invalid {
    #[bits(0..=2, fixed = 8)]
    value: u8,
}
fn main() {}
