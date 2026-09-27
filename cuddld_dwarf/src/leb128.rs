use cuddld_macros::{Display, Error};
use cuddld_utils::raw_types::{RawReadError, RawType, RawWriteError};
use std::io::Read;
use std::io::prelude::Write;

pub fn decode_unsigned_leb128(reader: &mut dyn Read) -> Result<u128, Leb128Error> {
    let mut result: u128 = 0;
    let mut shift = 0;
    loop {
        let mut buf = [0];
        reader.read_exact(&mut buf)?;

        result += ((buf[0] & 0x7f) as u128).checked_shl(shift).ok_or(Leb128Error::Overflow)?;
        shift += 7;

        if buf[0] & 0x80 == 0 {
            break;
        }
    }
    Ok(result)
}

pub fn decode_signed_leb128(reader: &mut dyn Read) -> Result<i128, Leb128Error> {
    let mut result: i128 = 0;
    let mut shift = 0;
    let mut buf;
    loop {
        buf = [0];
        reader.read_exact(&mut buf)?;

        result += ((buf[0] & 0x7f) as i128).checked_shl(shift).ok_or(Leb128Error::Overflow)?;
        shift += 7;

        if buf[0] & 0x80 == 0 {
            break;
        }
    }

    // Sign-extend the result if the last bit is positive (as number is encoded using two's
    // complement).
    if buf[0] & 0x40 != 0 {
        result |= (!0i128) << shift;
    }

    Ok(result)
}

pub struct Leb128;

macro_rules! impl_for {
    ($fn:ident: $($ty:ident),*) => {
        $(
            impl RawType<Leb128> for $ty {
                fn read(_ctx: &Leb128, reader: &mut dyn Read) -> Result<Self, RawReadError> {
                    match $fn(reader).and_then(|n| n.try_into().map_err(|_| Leb128Error::Overflow)) {
                        Ok(result) => Ok(result),
                        Err(Leb128Error::Io(io)) => Err(RawReadError::io::<Self>(io)),
                        Err(err) => Err(RawReadError::custom::<Self>(err.to_string())),
                    }
                }

                fn write(&self, _ctx: &Leb128, _writer: &mut dyn Write) -> Result<(), RawWriteError> {
                    unimplemented!("encoding leb128 types is not supported yet");
                }
            }
        )*
    }
}
impl_for!(decode_unsigned_leb128: u8, u16, u32, u64, u128);
impl_for!(decode_signed_leb128: i8, i16, i32, i64, i128);

#[derive(Debug, Error, Display)]
pub enum Leb128Error {
    #[transparent]
    Io(std::io::Error),
    #[display("the number is too large to be decoded")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_decode_unsigned_leb128() {
        assert_eq!(0, decode_unsigned_leb128(&mut Cursor::new(&[0])).unwrap());
        assert_eq!(1, decode_unsigned_leb128(&mut Cursor::new(&[1])).unwrap());
        assert_eq!(42, decode_unsigned_leb128(&mut Cursor::new(&[0x2A])).unwrap());
        assert_eq!(128, decode_unsigned_leb128(&mut Cursor::new(&[0x80, 0x01])).unwrap());
        assert_eq!(1234, decode_unsigned_leb128(&mut Cursor::new(&[0xD2, 0x09])).unwrap());
        assert_eq!(123456, decode_unsigned_leb128(&mut Cursor::new(&[0xC0, 0xC4, 0x07])).unwrap());
    }

    #[test]
    fn test_decode_signed_leb128() {
        assert_eq!(0, decode_signed_leb128(&mut Cursor::new(&[0])).unwrap());
        assert_eq!(1, decode_signed_leb128(&mut Cursor::new(&[1])).unwrap());
        assert_eq!(-1, decode_signed_leb128(&mut Cursor::new(&[0x7F])).unwrap());
        assert_eq!(42, decode_signed_leb128(&mut Cursor::new(&[0x2A])).unwrap());
        assert_eq!(-42, decode_signed_leb128(&mut Cursor::new(&[0x56])).unwrap());
        assert_eq!(128, decode_signed_leb128(&mut Cursor::new(&[0x80, 0x01])).unwrap());
        assert_eq!(-128, decode_signed_leb128(&mut Cursor::new(&[0x80, 0x7F])).unwrap());
        assert_eq!(1234, decode_signed_leb128(&mut Cursor::new(&[0xD2, 0x09])).unwrap());
        assert_eq!(-1234, decode_signed_leb128(&mut Cursor::new(&[0xAE, 0x76])).unwrap());
        assert_eq!(123456, decode_signed_leb128(&mut Cursor::new(&[0xC0, 0xC4, 0x07])).unwrap());
        assert_eq!(-123456, decode_signed_leb128(&mut Cursor::new(&[0xC0, 0xBB, 0x78])).unwrap());
    }

    #[test]
    fn test_raw_type() {
        assert_eq!(42, u8::read(&Leb128, &mut Cursor::new(&[0x2A])).unwrap());
        assert_eq!(-42, i8::read(&Leb128, &mut Cursor::new(&[0x56])).unwrap());
    }
}
