use std::io::*;

pub struct BitWriter<W: Write> {
    backing: W,
    nbits: usize, // invariant: `nbits <= 7`
    buf: u8,      // invariant: `buf & 0x80 == 0`
}

impl<W: Write> BitWriter<W> {
    pub fn new(backing: W) -> Self {
        Self {
            backing,
            nbits: 0,
            buf: 0,
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        assert_eq!(self.nbits, 0);
        self.backing.flush()
    }

    pub fn write_bit(&mut self, set: bool) -> Result<()> {
        self.buf <<= 1;
        if set {
            self.buf |= 1;
        }
        self.nbits += 1;
        if self.nbits == 8 {
            self.nbits = 0;
            let towrite = self.buf;
            self.buf = 0;
            // Might raise ErrorKind::WriteZero
            self.backing.write_all(&[towrite])
        } else {
            Ok(())
        }
    }

    pub fn padding_needed(&self) -> usize {
        if self.nbits > 0 {
            8 - self.nbits
        } else {
            0
        }
    }
}

pub struct BitReader<R: Read> {
    backing: R,
    nbits: usize, // invariant: `nbits <= 7`
    buf: u8,      // invariant: `buf & 0x01 == 0`
}

impl<R: Read> BitReader<R> {
    pub fn new(backing: R) -> Self {
        Self {
            backing,
            nbits: 0,
            buf: 0,
        }
    }

    pub fn read_bit(&mut self) -> Result<bool> {
        if self.nbits == 0 {
            let mut buf = [0];
            // Might raise ErrorKind::UnexpectedEof:
            self.backing.read_exact(&mut buf)?;
            self.buf = buf[0];
            self.nbits = 8;
        }
        let bit = self.buf & 0x80 != 0;
        self.buf <<= 1;
        self.nbits -= 1;
        Ok(bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write() {
        let mut buffer: [u8; 3] = [42, 42, 42];
        {
            let mut writer = BitWriter::new(buffer.as_mut_slice());
            assert_eq!(writer.padding_needed(), 0);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 7);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 6);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 5);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 4);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 3);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 2);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 1);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 0);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 7);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 6);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 5);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 4);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 3);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 2);
            writer.write_bit(true).unwrap();
            assert_eq!(writer.padding_needed(), 1);
            writer.write_bit(false).unwrap();
            assert_eq!(writer.padding_needed(), 0);
            writer.flush().unwrap();
        }
        assert_eq!(&buffer, &[0b1001_1100, 0b0011_1110, 42]);
    }

    #[test]
    fn test_read() {
        let buffer: [u8; 3] = [0b1001_1100, 0b0011_1110, 42];
        let mut reader = BitReader::new(buffer.as_slice());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
    }
}
