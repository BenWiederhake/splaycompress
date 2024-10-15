mod bits;
mod common;
mod splay;
mod symbol;

use bits::{BitReader, BitWriter};
use common::Direction;
use splay::{Arena16, Arena8, NodeArena};
use std::fmt::Debug;
use std::io::{ErrorKind, Read, Result, Write};
use symbol::{
    SymbolRead, SymbolRead16BE, SymbolRead16LE, SymbolRead8, SymbolWrite, SymbolWrite16BE,
    SymbolWrite16LE, SymbolWrite8,
};

/// Filemagic for "raw splaycompress data with 8-bit symbols, no metadata except this filemagic".
/// I generated this by taking 6 random bytes, the NUL byte, and the '\\r' byte, and re-shuffling
/// them until neither of the two "special" bytes are at either end. This should provide a good
/// balance between global uniqueness and built-in error detection.
///
/// Alternate representations: b"\xb3\xa9\x14\x00\xb9l\r\xd8" or s6kUALlsDdg= or "scallion passenger
/// baboon adroitness sentence handiwork ancient stupendous"
pub const MAGIC_FORMAT_SYMBOL8: &[u8] = b"\xb3\xa9\x14\x00\xb9\x6c\x0d\xd8";

/// Filemagic for "raw splaycompress data with 16-bit little-endian symbols, no metadata except this filemagic".
/// I generated this by taking 6 random bytes, the NUL byte, and the '\\r' byte, and re-shuffling
/// them until neither of the two "special" bytes are at either end. This should provide a good
/// balance between global uniqueness and built-in error detection.
///
/// Alternate representations: b"\xf2A\xc0O\r\x00Z\xf6" or 8kHATw0AWvY= or "uproot decadence
/// slowdown document ancient adroitness enlist vocalist"
pub const MAGIC_FORMAT_SYMBOL16LE: &[u8] = b"\xf2\x41\xc0\x4f\x0d\x00\x5a\xf6";

/// Filemagic for "raw splaycompress data with 16-bit big-endian symbols, no metadata except this filemagic".
/// This is the reverse of `MAGIC_FORMAT_SYMBOL16LE`. This should provide a good
/// balance between global uniqueness, built-in error detection, and recognizability.
///
/// Alternate representations: b"\xf6Z\x00\rO\xc0A\xf2" or 9loADU/AQfI= or "village existence
/// aardvark asteroid dropper recipe cranky vagabond"
pub const MAGIC_FORMAT_SYMBOL16BE: &[u8] = b"\xf6\x5a\x00\x0d\x4f\xc0\x41\xf2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Flavor {
    Symbol8,
    Symbol16BE,
    Symbol16LE,
}

pub fn compress<R: Read, W: Write>(flavor: Flavor, r: R, w: W) -> Result<()> {
    match flavor {
        Flavor::Symbol8 => compress8(r, w),
        Flavor::Symbol16BE => compress16be(r, w),
        Flavor::Symbol16LE => compress16le(r, w),
    }
}

pub fn compress8<R: Read, W: Write>(r: R, w: W) -> Result<()> {
    let mut arena = Arena8::new_uniform();
    compress_raw(&mut arena, &mut SymbolRead8(r), w)
}

pub fn compress16be<R: Read, W: Write>(r: R, w: W) -> Result<()> {
    let mut arena = Arena16::new_uniform();
    compress_raw(&mut arena, &mut SymbolRead16BE(r), w)
}

pub fn compress16le<R: Read, W: Write>(r: R, w: W) -> Result<()> {
    let mut arena = Arena16::new_uniform();
    compress_raw(&mut arena, &mut SymbolRead16LE(r), w)
}

pub fn decompress<R: Read, W: Write>(flavor: Flavor, r: R, w: W) -> Result<()> {
    match flavor {
        Flavor::Symbol8 => decompress8(r, w),
        Flavor::Symbol16BE => decompress16be(r, w),
        Flavor::Symbol16LE => decompress16le(r, w),
    }
}

pub fn decompress8<R: Read, W: Write>(r: R, w: W) -> Result<()> {
    let mut arena = Arena8::new_uniform();
    decompress_raw(&mut arena, r, &mut SymbolWrite8(w))
}

pub fn decompress16be<R: Read, W: Write>(r: R, w: W) -> Result<()> {
    let mut arena = Arena16::new_uniform();
    decompress_raw(&mut arena, r, &mut SymbolWrite16BE(w))
}

pub fn decompress16le<R: Read, W: Write>(r: R, w: W) -> Result<()> {
    let mut arena = Arena16::new_uniform();
    decompress_raw(&mut arena, r, &mut SymbolWrite16LE(w))
}

pub fn compress_raw<
    T: Clone + Copy + Debug + Eq + Ord + PartialEq + PartialOrd,
    A: NodeArena<T>,
    R: SymbolRead<T>,
    W: Write,
>(
    arena: &mut A,
    r: &mut R,
    w: W,
) -> Result<()> {
    let mut walker = arena.splayable_mut();
    let mut writer = BitWriter::new(w);
    loop {
        assert!(walker.is_root());
        if let Some(symbol) = r.read_one()? {
            while !walker.is_leaf() {
                let bit = symbol > walker.current_value();
                walker.go(Direction::from_bit(bit));
                writer.write_bit(bit)?;
            }
            walker.splay_parent_of_leaf();
            debug_assert!(walker.is_consistent());
        } else {
            break;
        }
    }
    assert!(walker.is_root());
    let need_pad_bits = writer.padding_needed();
    if need_pad_bits > 0 {
        let goal = walker.find_deep_internal(need_pad_bits);
        for _ in 0..need_pad_bits {
            let bit = goal > walker.current_value();
            walker.go(Direction::from_bit(bit));
            assert!(!walker.is_leaf());
            assert!(writer.padding_needed() > 0);
            writer.write_bit(bit)?;
        }
        assert_eq!(writer.padding_needed(), 0);
    }
    writer.flush()
}

pub fn decompress_raw<
    T: Clone + Copy + Debug + Eq + Ord + PartialEq + PartialOrd,
    A: NodeArena<T>,
    R: Read,
    W: SymbolWrite<T>,
>(
    arena: &mut A,
    r: R,
    w: &mut W,
) -> Result<()> {
    let mut walker = arena.splayable_mut();
    let mut reader = BitReader::new(r);
    loop {
        let bit = match reader.read_bit() {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                w.flush()?;
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        };
        walker.go(Direction::from_bit(bit));
        if walker.is_leaf() {
            w.write_one(walker.current_value())?;
            walker.splay_parent_of_leaf();
            debug_assert!(walker.is_consistent());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_compression(flavor: Flavor, input: &[u8], output: &[u8]) {
        let mut buf = Vec::new();
        compress(flavor, input, &mut buf).unwrap();
        assert_eq!(output, &buf);
    }

    fn assert_decompression(flavor: Flavor, input: &[u8], output: &[u8]) {
        let mut buf = Vec::new();
        decompress(flavor, input, &mut buf).unwrap();
        assert_eq!(output, &buf);
    }

    fn assert_roundtrip(flavor: Flavor, plaintext: &[u8], compressed: &[u8]) {
        assert_compression(flavor, plaintext, compressed);
        assert_decompression(flavor, compressed, plaintext);
    }

    #[test]
    fn test_empty() {
        assert_roundtrip(Flavor::Symbol8, &[], &[]);
        assert_roundtrip(Flavor::Symbol16BE, &[], &[]);
        assert_roundtrip(Flavor::Symbol16LE, &[], &[]);
    }

    #[test]
    fn test_single_symbol_8() {
        for b in 0..=255 {
            assert_roundtrip(Flavor::Symbol8, &[b], &[b]);
        }
    }

    #[test]
    #[ignore = "slow (takes around 30 seconds with --release)"]
    fn test_single_symbol_16() {
        for b1 in 0..=255 {
            for b2 in 0..=255 {
                assert_roundtrip(Flavor::Symbol16BE, &[b1, b2], &[b1, b2]);
                assert_roundtrip(Flavor::Symbol16LE, &[b1, b2], &[b2, b1]); // flipped!
            }
        }
    }

    #[test]
    fn test_hello_world() {
        assert_roundtrip(
            Flavor::Symbol8,
            b"Hello, World!\n",
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x50",
        );
        assert_roundtrip(
            Flavor::Symbol16BE,
            b"Hello, World!\n",
            b"\x48\x65\xac\x6c\x99\x60\x40\xaf\x8e\x4a\xf4\x43\x0a",
        );
        assert_roundtrip(
            Flavor::Symbol16LE,
            b"Hello, World!\n",
            b"\x65\x48\xa8\xd8\x16\x37\xcd\xc8\x34\x9b\xd5\x36\x02\x88\x40",
        );
    }

    #[test]
    fn test_16_odd() {
        assert_decompression(Flavor::Symbol16BE, b"\x48\x65", b"He");
        assert_decompression(Flavor::Symbol16BE, b"\x48\x65\x00", b"He");
        assert_decompression(Flavor::Symbol16BE, b"\x48\x65\xff", b"He");
    }

    #[test]
    fn test_hello_world_alternatives() {
        assert_decompression(
            Flavor::Symbol8,
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x51",
            b"Hello, World!\n",
        );
        assert_decompression(
            Flavor::Symbol8,
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x52",
            b"Hello, World!\n",
        );
        assert_decompression(
            Flavor::Symbol8,
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x54",
            b"Hello, World!\n",
        );
        assert_decompression(
            Flavor::Symbol8,
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x55",
            b"Hello, World!\n",
        );
        assert_decompression(
            Flavor::Symbol8,
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x56",
            b"Hello, World!\n",
        );
        assert_decompression(
            Flavor::Symbol8,
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x57",
            b"Hello, World!\n",
        );
    }

    #[test]
    fn test_anti_hello_world() {
        assert_roundtrip(
            Flavor::Symbol8,
            b"HH+(($$###\"\"\x10\x0a#'(H*H(()(\x0b$",
            b"Hello, World!\n",
        );
    }

    #[test]
    #[ignore = "slow (takes around 4 seconds with --release)"] // Use 'cargo test -- --include-ignored' or similar.
    fn test_two_bytes() {
        for b1 in 0..=255 {
            for b2 in 0..=255 {
                let mut buf = Vec::new();
                compress8(&[b1, b2][..], &mut buf).unwrap();
                assert_decompression(Flavor::Symbol8, &buf, &[b1, b2]);
            }
        }
    }

    #[test]
    fn test_short() {
        // Look at this! General-purpose compression that manages to shorten (these) 7 bytes to just 6 bytes!
        assert_roundtrip(Flavor::Symbol8, b"short", b"\x73\x51\x3e\xf2\x00");
        assert_roundtrip(Flavor::Symbol8, b"shorter", b"\x73\x51\x3e\xf2\x02\xb4");
    }
}
