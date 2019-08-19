use std::array::TryFromSliceError;
use std::convert::TryInto;

pub fn to_be_u32(payload: &[u8]) -> Result<u32, TryFromSliceError> {
    Ok(u32::from_be_bytes(
        payload.try_into().map(|v: &[u8; 4]| v.clone())?,
    ))
}
