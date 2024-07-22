// zig (only on root)
// zigzig (A/B/C becomes A\B\C)
// zigzag ("<" becomes "nAn")

mod splay;

use std::mem::size_of;

fn main() {
    println!("Hello, world!");
    println!("sizeof bool = {}", size_of::<bool>());
    println!("sizeof u8 = {}", size_of::<u8>());
}
