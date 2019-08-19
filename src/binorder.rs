use std::array::TryFromSliceError;
use std::convert::TryInto;

pub fn to_be_u16(payload: &[u8]) -> Result<u16, TryFromSliceError> {
    Ok(u16::from_be_bytes(
        payload.try_into().map(|v: &[u8; 2]| v.clone())?,
    ))
}

pub fn to_be_u32(payload: &[u8]) -> Result<u32, TryFromSliceError> {
    Ok(u32::from_be_bytes(
        payload.try_into().map(|v: &[u8; 4]| v.clone())?,
    ))
}

pub fn to_le_u32(payload: &[u8]) -> Result<u32, TryFromSliceError> {
    Ok(u32::from_le_bytes(
        payload.try_into().map(|v: &[u8; 4]| v.clone())?,
    ))
}
