use std::array::TryFromSliceError;
use std::convert::TryInto;

pub fn to_be_u32(payload: &Vec<u8>) -> Result<u32, TryFromSliceError> {
    Ok(u32::from_be_bytes(
        payload.as_slice().try_into().map(|v: &[u8; 4]| v.clone())?,
    ))
}
