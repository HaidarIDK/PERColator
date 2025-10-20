//! Instruction data serialization utilities
//!
//! Provides zero-copy serialization for instruction parameters to minimize
//! compute units consumed during deserialization.

use crate::error::PercolatorError;

/// Read a u8 from a byte slice at the given offset
#[inline]
pub fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, PercolatorError> {
    if *offset >= data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let value = data[*offset];
    *offset += 1;
    Ok(value)
}

/// Read a u16 from a byte slice (little-endian) at the given offset
#[inline]
pub fn read_u16(data: &[u8], offset: &mut usize) -> Result<u16, PercolatorError> {
    if *offset + 2 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = [data[*offset], data[*offset + 1]];
    *offset += 2;
    Ok(u16::from_le_bytes(bytes))
}

/// Read a u32 from a byte slice (little-endian) at the given offset
#[inline]
pub fn read_u32(data: &[u8], offset: &mut usize) -> Result<u32, PercolatorError> {
    if *offset + 4 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[*offset..*offset + 4]);
    *offset += 4;
    Ok(u32::from_le_bytes(bytes))
}

/// Read a u64 from a byte slice (little-endian) at the given offset
#[inline]
pub fn read_u64(data: &[u8], offset: &mut usize) -> Result<u64, PercolatorError> {
    if *offset + 8 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[*offset..*offset + 8]);
    *offset += 8;
    Ok(u64::from_le_bytes(bytes))
}

/// Read a u128 from a byte slice (little-endian) at the given offset
#[inline]
pub fn read_u128(data: &[u8], offset: &mut usize) -> Result<u128, PercolatorError> {
    if *offset + 16 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[*offset..*offset + 16]);
    *offset += 16;
    Ok(u128::from_le_bytes(bytes))
}

/// Read an i64 from a byte slice (little-endian) at the given offset
#[inline]
pub fn read_i64(data: &[u8], offset: &mut usize) -> Result<i64, PercolatorError> {
    if *offset + 8 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[*offset..*offset + 8]);
    *offset += 8;
    Ok(i64::from_le_bytes(bytes))
}

/// Read a fixed-size array from a byte slice at the given offset
#[inline]
pub fn read_bytes<const N: usize>(data: &[u8], offset: &mut usize) -> Result<[u8; N], PercolatorError> {
    if *offset + N > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(&data[*offset..*offset + N]);
    *offset += N;
    Ok(bytes)
}

/// Write a u8 to a byte slice at the given offset
#[inline]
pub fn write_u8(data: &mut [u8], offset: &mut usize, value: u8) -> Result<(), PercolatorError> {
    if *offset >= data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    data[*offset] = value;
    *offset += 1;
    Ok(())
}

/// Write a u16 to a byte slice (little-endian) at the given offset
#[inline]
pub fn write_u16(data: &mut [u8], offset: &mut usize, value: u16) -> Result<(), PercolatorError> {
    if *offset + 2 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = value.to_le_bytes();
    data[*offset..*offset + 2].copy_from_slice(&bytes);
    *offset += 2;
    Ok(())
}

/// Write a u32 to a byte slice (little-endian) at the given offset
#[inline]
pub fn write_u32(data: &mut [u8], offset: &mut usize, value: u32) -> Result<(), PercolatorError> {
    if *offset + 4 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = value.to_le_bytes();
    data[*offset..*offset + 4].copy_from_slice(&bytes);
    *offset += 4;
    Ok(())
}

/// Write a u64 to a byte slice (little-endian) at the given offset
#[inline]
pub fn write_u64(data: &mut [u8], offset: &mut usize, value: u64) -> Result<(), PercolatorError> {
    if *offset + 8 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = value.to_le_bytes();
    data[*offset..*offset + 8].copy_from_slice(&bytes);
    *offset += 8;
    Ok(())
}

/// Write a u128 to a byte slice (little-endian) at the given offset
#[inline]
pub fn write_u128(data: &mut [u8], offset: &mut usize, value: u128) -> Result<(), PercolatorError> {
    if *offset + 16 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = value.to_le_bytes();
    data[*offset..*offset + 16].copy_from_slice(&bytes);
    *offset += 16;
    Ok(())
}

/// Write an i64 to a byte slice (little-endian) at the given offset
#[inline]
pub fn write_i64(data: &mut [u8], offset: &mut usize, value: i64) -> Result<(), PercolatorError> {
    if *offset + 8 > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    let bytes = value.to_le_bytes();
    data[*offset..*offset + 8].copy_from_slice(&bytes);
    *offset += 8;
    Ok(())
}

/// Write a fixed-size array to a byte slice at the given offset
#[inline]
pub fn write_bytes<const N: usize>(data: &mut [u8], offset: &mut usize, value: &[u8; N]) -> Result<(), PercolatorError> {
    if *offset + N > data.len() {
        return Err(PercolatorError::InvalidInstruction);
    }
    data[*offset..*offset + N].copy_from_slice(value);
    *offset += N;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write_u8() {
        let mut data = [0u8; 10];
        let mut offset = 0;
        
        write_u8(&mut data, &mut offset, 42).unwrap();
        assert_eq!(offset, 1);
        
        offset = 0;
        assert_eq!(read_u8(&data, &mut offset).unwrap(), 42);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_read_write_u64() {
        let mut data = [0u8; 20];
        let mut offset = 0;
        
        write_u64(&mut data, &mut offset, 0x123456789ABCDEF0).unwrap();
        assert_eq!(offset, 8);
        
        offset = 0;
        assert_eq!(read_u64(&data, &mut offset).unwrap(), 0x123456789ABCDEF0);
        assert_eq!(offset, 8);
    }

    #[test]
    fn test_read_write_bytes() {
        let mut data = [0u8; 40];
        let mut offset = 0;
        
        let hash = [0xAB; 32];
        write_bytes(&mut data, &mut offset, &hash).unwrap();
        assert_eq!(offset, 32);
        
        offset = 0;
        assert_eq!(read_bytes::<32>(&data, &mut offset).unwrap(), hash);
        assert_eq!(offset, 32);
    }

    #[test]
    fn test_insufficient_data() {
        let data = [0u8; 2];
        let mut offset = 0;
        
        assert!(read_u64(&data, &mut offset).is_err());
    }
}

