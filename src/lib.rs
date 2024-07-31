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

    fn assert_compression8(input: &[u8], output: &[u8]) {
        let mut buf = Vec::new();
        compress8(input, &mut buf).unwrap();
        assert_eq!(output, &buf);
    }

    fn assert_decompression8(input: &[u8], output: &[u8]) {
        let mut buf = Vec::new();
        decompress8(input, &mut buf).unwrap();
        assert_eq!(output, &buf);
    }

    fn assert_roundtrip8(plaintext: &[u8], compressed: &[u8]) {
        assert_compression8(plaintext, compressed);
        assert_decompression8(compressed, plaintext);
    }

    #[test]
    fn test_empty() {
        assert_roundtrip8(&[], &[]);
    }

    #[test]
    fn test_single_byte() {
        for b in 0..=255 {
            assert_roundtrip8(&[b], &[b]);
        }
    }

    #[test]
    fn test_hello_world() {
        assert_roundtrip8(
            b"Hello, World!\n",
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x50",
        );
    }

    #[test]
    fn test_hello_world_alternatives() {
        assert_decompression8(
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x51",
            b"Hello, World!\n",
        );
        assert_decompression8(
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x52",
            b"Hello, World!\n",
        );
        assert_decompression8(
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x54",
            b"Hello, World!\n",
        );
        assert_decompression8(
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x55",
            b"Hello, World!\n",
        );
        assert_decompression8(
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x56",
            b"Hello, World!\n",
        );
        assert_decompression8(
            b"\x48\xa5\xa8\xf9\x81\x62\x19\x2f\x91\x16\x4a\x40\x57",
            b"Hello, World!\n",
        );
    }

    #[test]
    fn test_anti_hello_world() {
        assert_roundtrip8(b"HH+(($$###\"\"\x10\x0a#'(H*H(()(\x0b$", b"Hello, World!\n");
    }

    #[test]
    #[ignore = "slow (takes around 4 seconds)"] // Use 'cargo test -- --include-ignored' or similar.
    fn test_two_bytes() {
        for b1 in 0..=255 {
            for b2 in 0..=255 {
                let mut buf = Vec::new();
                compress8(&[b1, b2][..], &mut buf).unwrap();
                assert_decompression8(&buf, &[b1, b2]);
            }
        }
    }

    #[test]
    fn test_short() {
        // Look at this! General-purpose compression that manages to shorten (these) 7 bytes to just 6 bytes!
        assert_roundtrip8(b"short", b"\x73\x51\x3e\xf2\x00");
        assert_roundtrip8(b"shorter", b"\x73\x51\x3e\xf2\x02\xb4");
    }
}
