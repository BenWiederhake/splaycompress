use std::io::{Error, ErrorKind, Read, Result, Write};

pub trait SymbolRead<T> {
    /// This is supposed to return exactly one symbol.
    /// TODO: Revisit this interface when dealing with higher throughput.
    /// Regular EOF should be indicated as `Ok(None)`, whereas ErrorKind::UnexpectedEof should
    /// indicate an actual error, like trying to read a u16 when only 2 bytes are left.
    fn read_one(&mut self) -> Result<Option<T>>;
}

pub struct SymbolRead8<R: Read>(pub R);

impl<R: Read> SymbolRead<u8> for SymbolRead8<R> {
    fn read_one(&mut self) -> Result<Option<u8>> {
        let mut buf = [0];
        match self.0.read_exact(buf.as_mut_slice()) {
            Ok(()) => Ok(Some(buf[0])),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Reads two bytes. The difference to read_exact([u8; 2]) is that *zero* bytes being available is
/// not an error, but *one* byte is an error.
fn read_two_bytes<R: Read>(r: &mut R) -> Result<Option<[u8; 2]>> {
    // Calling Read::read() by hand is a bad idea, because we might need to retry many times due to ErrKind::Interrupted.
    // Calling Read::read_exact() would lose the information whether we read zero or one byte.
    // Read::read_to_end() is nice, but would consume everything.

    // This is terribly inefficient: Avoid allocating just for these two bytes?!
    let mut buf = Vec::with_capacity(2);
    let bytes_read = r.take(2).read_to_end(&mut buf)?;
    assert_eq!(bytes_read, buf.len());
    match bytes_read {
        2 => Ok(Some([buf[0], buf[1]])),
        1 => Err(Error::new(
            ErrorKind::UnexpectedEof,
            "Cannot interpret last byte as u16",
        )),
        0 => Ok(None),
        _ => {
            panic!("Impossible number of bytes read into two-byte-buffer: {bytes_read}");
        }
    }
}

pub struct SymbolRead16LE<R: Read>(pub R);

impl<R: Read> SymbolRead<u16> for SymbolRead16LE<R> {
    fn read_one(&mut self) -> Result<Option<u16>> {
        let maybe_bytes = read_two_bytes(&mut self.0)?;
        Ok(maybe_bytes.map(u16::from_le_bytes))
    }
}

pub struct SymbolRead16BE<R: Read>(pub R);

impl<R: Read> SymbolRead<u16> for SymbolRead16BE<R> {
    fn read_one(&mut self) -> Result<Option<u16>> {
        let maybe_bytes = read_two_bytes(&mut self.0)?;
        Ok(maybe_bytes.map(u16::from_be_bytes))
    }
}

pub trait SymbolWrite<T> {
    /// This is supposed to write exactly one symbol.
    /// TODO: Revisit this interface when dealing with higher throughput.
    fn write_one(&mut self, symbol: T) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

pub struct SymbolWrite8<W: Write>(pub W);

impl<W: Write> SymbolWrite<u8> for SymbolWrite8<W> {
    fn write_one(&mut self, symbol: u8) -> Result<()> {
        let buf = [symbol];
        self.0.write_all(buf.as_slice())
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}

pub struct SymbolWrite16LE<W: Write>(pub W);

impl<W: Write> SymbolWrite<u16> for SymbolWrite16LE<W> {
    fn write_one(&mut self, symbol: u16) -> Result<()> {
        let buf = symbol.to_le_bytes();
        self.0.write_all(buf.as_slice())
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}

pub struct SymbolWrite16BE<W: Write>(pub W);

impl<W: Write> SymbolWrite<u16> for SymbolWrite16BE<W> {
    fn write_one(&mut self, symbol: u16) -> Result<()> {
        let buf = symbol.to_be_bytes();
        self.0.write_all(buf.as_slice())
    }

    fn flush(&mut self) -> Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read8_noop() {
        let buf = [42, 13, 37, 0, 255];
        SymbolRead8(buf.as_slice());
    }

    #[test]
    fn test_read8() {
        let buf = [42, 13, 37, 0, 255];
        let mut r = SymbolRead8(buf.as_slice());
        assert_eq!(r.read_one().unwrap(), Some(42));
        assert_eq!(r.read_one().unwrap(), Some(13));
        assert_eq!(r.read_one().unwrap(), Some(37));
        assert_eq!(r.read_one().unwrap(), Some(0));
        assert_eq!(r.read_one().unwrap(), Some(255));
        assert_eq!(r.read_one().unwrap(), None);
    }

    #[test]
    fn test_read16() {
        let buf = [0x12, 0x34, 0xAB, 0xCD, 0x00, 0x00, 0xFF, 0xFF];
        let mut r = SymbolRead16BE(buf.as_slice());
        assert_eq!(r.read_one().unwrap(), Some(0x1234));
        assert_eq!(r.read_one().unwrap(), Some(0xABCD));
        assert_eq!(r.read_one().unwrap(), Some(0x0000));
        assert_eq!(r.read_one().unwrap(), Some(0xFFFF));
        assert_eq!(r.read_one().unwrap(), None);
        let mut r = SymbolRead16LE(buf.as_slice());
        assert_eq!(r.read_one().unwrap(), Some(0x3412));
        assert_eq!(r.read_one().unwrap(), Some(0xCDAB));
        assert_eq!(r.read_one().unwrap(), Some(0x0000));
        assert_eq!(r.read_one().unwrap(), Some(0xFFFF));
        assert_eq!(r.read_one().unwrap(), None);
    }

    #[test]
    fn test_read16_odd() {
        let buf = [0x12, 0x34, 0x56];
        let mut r = SymbolRead16BE(buf.as_slice());
        assert_eq!(r.read_one().unwrap(), Some(0x1234));
        assert_eq!(r.read_one().unwrap_err().kind(), ErrorKind::UnexpectedEof);
        let mut r = SymbolRead16LE(buf.as_slice());
        assert_eq!(r.read_one().unwrap(), Some(0x3412));
        assert_eq!(r.read_one().unwrap_err().kind(), ErrorKind::UnexpectedEof);
    }

    #[test]
    fn write8_noop() {
        let mut buf = [1, 1, 1, 1, 1, 1, 1];
        SymbolWrite8(buf.as_mut_slice());
        assert_eq!(buf, [1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn write8() {
        let mut buf = [1, 1, 1, 1, 1, 1, 1];
        let mut w = SymbolWrite8(buf.as_mut_slice());
        w.write_one(42).unwrap();
        w.write_one(13).unwrap();
        w.write_one(37).unwrap();
        w.write_one(0).unwrap();
        w.write_one(255).unwrap();
        assert_eq!(buf, [42, 13, 37, 0, 255, 1, 1]);
    }

    #[test]
    fn write16_be() {
        let mut buf = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let mut w = SymbolWrite16BE(buf.as_mut_slice());
        w.write_one(0x1234).unwrap();
        w.write_one(0xABCD).unwrap();
        w.write_one(0x0000).unwrap();
        w.write_one(0xFFFF).unwrap();
        assert_eq!(
            buf,
            [0x12, 0x34, 0xAB, 0xCD, 0x00, 0x00, 0xFF, 0xFF, 1, 1, 1]
        );
    }

    #[test]
    fn write16_le() {
        let mut buf = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let mut w = SymbolWrite16LE(buf.as_mut_slice());
        w.write_one(0x1234).unwrap();
        w.write_one(0xABCD).unwrap();
        w.write_one(0x0000).unwrap();
        w.write_one(0xFFFF).unwrap();
        assert_eq!(
            buf,
            [0x34, 0x12, 0xCD, 0xAB, 0x00, 0x00, 0xFF, 0xFF, 1, 1, 1]
        );
    }
}
