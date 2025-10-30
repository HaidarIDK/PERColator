//! Instruction data deserialization helpers
//!
//! Provides utilities for safely parsing instruction data from byte slices.
//! All functions perform bounds checking and return errors on invalid input.

use crate::error::PercolatorError;

/// Read a u8 from instruction data
#[inline]
pub fn read_u8(data: &[u8], offset: usize) -> Result<u8, PercolatorError> {
    if offset >= data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    Ok(data[offset])
}

/// Read a u16 (little-endian) from instruction data
#[inline]
pub fn read_u16(data: &[u8], offset: usize) -> Result<u16, PercolatorError> {
    if offset + 2 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = [data[offset], data[offset + 1]];
    Ok(u16::from_le_bytes(bytes))
}

/// Read a u32 (little-endian) from instruction data
#[inline]
pub fn read_u32(data: &[u8], offset: usize) -> Result<u32, PercolatorError> {
    if offset + 4 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    Ok(u32::from_le_bytes(bytes))
}

/// Read a u64 (little-endian) from instruction data
#[inline]
pub fn read_u64(data: &[u8], offset: usize) -> Result<u64, PercolatorError> {
    if offset + 8 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    Ok(u64::from_le_bytes(bytes))
}

/// Read an i64 (little-endian) from instruction data
#[inline]
pub fn read_i64(data: &[u8], offset: usize) -> Result<i64, PercolatorError> {
    if offset + 8 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    Ok(i64::from_le_bytes(bytes))
}

/// Read a u128 (little-endian) from instruction data
#[inline]
pub fn read_u128(data: &[u8], offset: usize) -> Result<u128, PercolatorError> {
    if offset + 16 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[offset..offset + 16]);
    Ok(u128::from_le_bytes(bytes))
}

/// Read a fixed-size byte array from instruction data
#[inline]
pub fn read_bytes<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N], PercolatorError> {
    if offset + N > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(&data[offset..offset + N]);
    Ok(bytes)
}

/// Read a Side enum from instruction data
#[inline]
pub fn read_side(data: &[u8], offset: usize) -> Result<crate::Side, PercolatorError> {
    let val = read_u8(data, offset)?;
    match val {
        0 => Ok(crate::Side::Buy),
        1 => Ok(crate::Side::Sell),
        _ => Err(PercolatorError::InvalidSide),
    }
}

/// Instruction data reader with tracked offset
///
/// Provides a convenient way to sequentially read fields from instruction data
/// while automatically tracking the current offset.
pub struct InstructionReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> InstructionReader<'a> {
    /// Create a new instruction reader
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Get the current offset
    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Get remaining bytes
    #[inline]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    /// Read a u8 and advance offset
    #[inline]
    pub fn read_u8(&mut self) -> Result<u8, PercolatorError> {
        let val = read_u8(self.data, self.offset)?;
        self.offset += 1;
        Ok(val)
    }

    /// Read a u16 and advance offset
    #[inline]
    pub fn read_u16(&mut self) -> Result<u16, PercolatorError> {
        let val = read_u16(self.data, self.offset)?;
        self.offset += 2;
        Ok(val)
    }

    /// Read a u32 and advance offset
    #[inline]
    pub fn read_u32(&mut self) -> Result<u32, PercolatorError> {
        let val = read_u32(self.data, self.offset)?;
        self.offset += 4;
        Ok(val)
    }

    /// Read a u64 and advance offset
    #[inline]
    pub fn read_u64(&mut self) -> Result<u64, PercolatorError> {
        let val = read_u64(self.data, self.offset)?;
        self.offset += 8;
        Ok(val)
    }

    /// Read an i64 and advance offset
    #[inline]
    pub fn read_i64(&mut self) -> Result<i64, PercolatorError> {
        let val = read_i64(self.data, self.offset)?;
        self.offset += 8;
        Ok(val)
    }

    /// Read a u128 and advance offset
    #[inline]
    pub fn read_u128(&mut self) -> Result<u128, PercolatorError> {
        let val = read_u128(self.data, self.offset)?;
        self.offset += 16;
        Ok(val)
    }

    /// Read a fixed-size byte array and advance offset
    #[inline]
    pub fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], PercolatorError> {
        let val = read_bytes(self.data, self.offset)?;
        self.offset += N;
        Ok(val)
    }

    /// Read a Side enum and advance offset
    #[inline]
    pub fn read_side(&mut self) -> Result<crate::Side, PercolatorError> {
        let val = read_side(self.data, self.offset)?;
        self.offset += 1;
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8() {
        let data = [42u8, 0, 0, 0];
        assert_eq!(read_u8(&data, 0).unwrap(), 42);
        assert!(read_u8(&data, 4).is_err());
    }

    #[test]
    fn test_read_u16() {
        let data = [0x34, 0x12, 0, 0]; // 0x1234 in little-endian
        assert_eq!(read_u16(&data, 0).unwrap(), 0x1234);
        assert!(read_u16(&data, 3).is_err());
    }

    #[test]
    fn test_read_u32() {
        let data = [0x78, 0x56, 0x34, 0x12]; // 0x12345678 in little-endian
        assert_eq!(read_u32(&data, 0).unwrap(), 0x12345678);
        assert!(read_u32(&data, 1).is_err());
    }

    #[test]
    fn test_read_u64() {
        let data = [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01];
        assert_eq!(read_u64(&data, 0).unwrap(), 0x0102030405060708);
        assert!(read_u64(&data, 1).is_err());
    }

    #[test]
    fn test_read_bytes() {
        let data = [1, 2, 3, 4, 5];
        let result: [u8; 3] = read_bytes(&data, 1).unwrap();
        assert_eq!(result, [2, 3, 4]);
        assert!(read_bytes::<4>(&data, 2).is_err());
    }

    #[test]
    fn test_read_side() {
        let data = [0u8, 1u8, 2u8];
        assert_eq!(read_side(&data, 0).unwrap(), crate::Side::Buy);
        assert_eq!(read_side(&data, 1).unwrap(), crate::Side::Sell);
        assert!(read_side(&data, 2).is_err());
    }

    #[test]
    fn test_instruction_reader() {
        let data = [
            42u8,           // u8
            0x34, 0x12,     // u16
            0x78, 0x56, 0x34, 0x12, // u32
        ];

        let mut reader = InstructionReader::new(&data);
        assert_eq!(reader.remaining(), 7);

        assert_eq!(reader.read_u8().unwrap(), 42);
        assert_eq!(reader.offset(), 1);
        assert_eq!(reader.remaining(), 6);

        assert_eq!(reader.read_u16().unwrap(), 0x1234);
        assert_eq!(reader.offset(), 3);

        assert_eq!(reader.read_u32().unwrap(), 0x12345678);
        assert_eq!(reader.offset(), 7);
        assert_eq!(reader.remaining(), 0);

        assert!(reader.read_u8().is_err());
    }
}
