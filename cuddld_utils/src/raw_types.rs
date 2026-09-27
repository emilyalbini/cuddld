use crate::bitfields::BitfieldReadError;
use crate::{Bits, Endian, OsAbi};
use std::ffi::CString;
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy)]
pub struct RawTypeContext {
    pub bits: Bits,
    pub endian: Endian,
    pub os_abi: OsAbi,
}

impl RawTypeContext {
    pub fn new(bits: impl Into<Bits>, endian: impl Into<Endian>, os_abi: impl Into<OsAbi>) -> Self {
        Self { bits: bits.into(), endian: endian.into(), os_abi: os_abi.into() }
    }
}

pub trait RawType<C = RawTypeContext>: Sized {
    fn read(ctx: &C, reader: &mut dyn Read) -> Result<Self, RawReadError>;
    fn write(&self, ctx: &C, writer: &mut dyn Write) -> Result<(), RawWriteError>;
}

pub trait SizedRawType<C = RawTypeContext> {
    fn size(ctx: &C) -> usize;
}

impl<const N: usize, T: RawType + Copy> RawType for [T; N] {
    fn read(ctx: &RawTypeContext, reader: &mut dyn Read) -> Result<Self, RawReadError> {
        let mut items = Vec::new();
        for _ in 0..N {
            items.push(RawReadError::wrap_type::<Self, _>(T::read(ctx, reader))?);
        }
        match items.try_into() {
            Ok(items) => Ok(items),
            Err(_) => unreachable!(),
        }
    }

    fn write(&self, ctx: &RawTypeContext, writer: &mut dyn Write) -> Result<(), RawWriteError> {
        for item in self {
            RawWriteError::wrap_type::<Self, _>(T::write(item, ctx, writer))?;
        }
        Ok(())
    }
}

impl<const N: usize, T: SizedRawType + Copy> SizedRawType for [T; N] {
    fn size(ctx: &RawTypeContext) -> usize {
        T::size(ctx) * N
    }
}

macro_rules! impl_rawtype_for_int {
    ($($int:ty),*) => {
        $(
            impl RawType for $int {
                fn read(ctx: &RawTypeContext, reader: &mut dyn std::io::Read) -> Result<Self, RawReadError> {
                    let mut buf = [0; std::mem::size_of::<$int>()];
                    reader.read_exact(&mut buf).map_err(RawReadError::io::<$int>)?;
                    Ok(match ctx.endian {
                        Endian::Big => <$int>::from_be_bytes(buf),
                        Endian::Little => <$int>::from_le_bytes(buf),
                    })
                }

                fn write(&self, ctx: &RawTypeContext, writer: &mut dyn std::io::Write) -> Result<(), RawWriteError> {
                    writer.write_all(&match ctx.endian {
                        Endian::Big => self.to_be_bytes(),
                        Endian::Little => self.to_le_bytes(),
                    }).map_err(RawWriteError::io::<$int>)
                }
            }

            impl SizedRawType for $int {
                fn size(_ctx: &RawTypeContext) -> usize {
                    std::mem::size_of::<$int>()
                }
            }
        )*
    }
}

impl_rawtype_for_int!(u8, u16, u32, u64, i8, i16, i32, i64);

pub struct RawPadding<const N: usize>;

impl<const N: usize> RawType for RawPadding<N> {
    fn read(_ctx: &RawTypeContext, reader: &mut dyn Read) -> Result<Self, RawReadError> {
        let mut buf = [0; N];
        reader.read_exact(&mut buf).map_err(RawReadError::io::<Self>)?;
        Ok(Self)
    }

    fn write(&self, _ctx: &RawTypeContext, writer: &mut dyn Write) -> Result<(), RawWriteError> {
        writer.write_all(&[0; N]).map_err(RawWriteError::io::<Self>)
    }
}

impl<const N: usize> SizedRawType for RawPadding<N> {
    fn size(_ctx: &RawTypeContext) -> usize {
        N
    }
}

pub struct PointerSize<'a>(pub &'a RawTypeContext);

macro_rules! impl_pointersized_for_int {
    ($($int:ident or $smallint:ident),*) => {
        $(
            impl RawType<PointerSize<'_>> for $int {

                fn read(ctx: &PointerSize<'_>, reader: &mut dyn Read) -> Result<Self, RawReadError> {
                    match ctx.0.bits {
                        Bits::Bits32 => <$smallint as RawType>::read(ctx.0, reader).map(|v| v as _),
                        Bits::Bits64 => <$int as RawType>::read(ctx.0, reader),
                    }
                }

                fn write(&self, ctx: &PointerSize<'_>, writer: &mut dyn Write) -> Result<(), RawWriteError> {
                    match ctx.0.bits {
                        Bits::Bits32 => <$smallint as RawType>::write(&(*self as _), ctx.0, writer),
                        Bits::Bits64 => <$int as RawType>::write(self, ctx.0, writer),
                    }
                }
            }

            impl SizedRawType<PointerSize<'_>> for $int {
                fn size(ctx: &PointerSize<'_>) -> usize {
                    match ctx.0.bits {
                        Bits::Bits32 => <$smallint as SizedRawType>::size(ctx.0),
                        Bits::Bits64 => <$int as SizedRawType>::size(ctx.0),
                    }
                }
            }
        )*
    }
}

impl_pointersized_for_int!(i64 or i32, u64 or u32);

impl RawType for CString {
    fn read(_ctx: &RawTypeContext, reader: &mut dyn Read) -> Result<Self, RawReadError> {
        let mut contents = Vec::new();
        let mut buf = [0];
        loop {
            reader.read_exact(&mut buf).map_err(RawReadError::io::<Self>)?;
            if buf[0] == 0 {
                break;
            } else {
                contents.push(buf[0]);
            }
        }
        Ok(CString::new(contents).unwrap())
    }

    fn write(&self, _ctx: &RawTypeContext, writer: &mut dyn Write) -> Result<(), RawWriteError> {
        writer.write_all(self.as_bytes_with_nul()).map_err(RawWriteError::io::<Self>)
    }
}

#[derive(Debug)]
pub struct RawReadError {
    source: ErrorSource,
    inner: RawReadErrorInner,
}

impl RawReadError {
    pub fn io<T>(err: std::io::Error) -> Self {
        Self {
            source: ErrorSource::Type(std::any::type_name::<T>()),
            inner: RawReadErrorInner::IO(err),
        }
    }

    pub fn bitfield<T>(err: BitfieldReadError) -> Self {
        Self {
            source: ErrorSource::Type(std::any::type_name::<T>()),
            inner: RawReadErrorInner::Bitfield(err),
        }
    }

    pub fn custom<T>(err: String) -> Self {
        Self {
            source: ErrorSource::Type(std::any::type_name::<T>()),
            inner: RawReadErrorInner::Custom(CustomError(err)),
        }
    }

    pub fn wrap_type<T, R>(result: Result<R, RawReadError>) -> Result<R, RawReadError> {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(RawReadError {
                source: ErrorSource::Type(std::any::type_name::<T>()),
                inner: RawReadErrorInner::Itself(Box::new(err)),
            }),
        }
    }

    pub fn wrap_field<T, R>(
        field: &'static str,
        result: Result<R, RawReadError>,
    ) -> Result<R, RawReadError> {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(RawReadError {
                source: ErrorSource::StructField { field, struct_: std::any::type_name::<T>() },
                inner: RawReadErrorInner::Itself(Box::new(err)),
            }),
        }
    }
}

#[derive(Debug)]
enum RawReadErrorInner {
    Itself(Box<RawReadError>),
    Bitfield(BitfieldReadError),
    Custom(CustomError),
    IO(std::io::Error),
}

impl std::fmt::Display for RawReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to read {}", self.source)
    }
}

impl std::error::Error for RawReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            RawReadErrorInner::IO(io) => Some(io),
            RawReadErrorInner::Bitfield(bitfield) => Some(bitfield),
            RawReadErrorInner::Itself(itself) => Some(itself),
            RawReadErrorInner::Custom(custom) => Some(custom),
        }
    }
}

#[derive(Debug)]
struct CustomError(String);

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CustomError {}

#[derive(Debug)]
pub struct RawWriteError {
    source: ErrorSource,
    inner: RawWriteErrorInner,
}

impl RawWriteError {
    pub fn io<T>(err: std::io::Error) -> Self {
        Self {
            source: ErrorSource::Type(std::any::type_name::<T>()),
            inner: RawWriteErrorInner::IO(err),
        }
    }

    pub fn custom<T>(err: String) -> Self {
        Self {
            source: ErrorSource::Type(std::any::type_name::<T>()),
            inner: RawWriteErrorInner::Custom(CustomError(err)),
        }
    }

    pub fn wrap_type<T, R>(result: Result<R, RawWriteError>) -> Result<R, RawWriteError> {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(RawWriteError {
                source: ErrorSource::Type(std::any::type_name::<T>()),
                inner: RawWriteErrorInner::Itself(Box::new(err)),
            }),
        }
    }

    pub fn wrap_field<T, R>(
        field: &'static str,
        result: Result<R, RawWriteError>,
    ) -> Result<R, RawWriteError> {
        match result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(RawWriteError {
                source: ErrorSource::StructField { field, struct_: std::any::type_name::<T>() },
                inner: RawWriteErrorInner::Itself(Box::new(err)),
            }),
        }
    }
}

impl std::error::Error for RawWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            RawWriteErrorInner::Itself(itself) => Some(itself),
            RawWriteErrorInner::IO(io) => Some(io),
            RawWriteErrorInner::Custom(custom) => Some(custom),
        }
    }
}

impl std::fmt::Display for RawWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to write {}", self.source)
    }
}

#[derive(Debug)]
enum RawWriteErrorInner {
    Itself(Box<RawWriteError>),
    IO(std::io::Error),
    Custom(CustomError),
}

#[derive(Debug)]
enum ErrorSource {
    Type(&'static str),
    StructField { field: &'static str, struct_: &'static str },
}

impl std::fmt::Display for ErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSource::Type(ty) => write!(f, "{ty}"),
            ErrorSource::StructField { field, struct_ } => {
                write!(f, "field \"{field}\" of struct {struct_}")
            }
        }
    }
}

pub trait ContextFrom: Sized {
    type Context<'a>
    where
        Self: 'a;

    fn context_from<'a>(ctx: &'a RawTypeContext, value: &'a Self) -> Self::Context<'a>;
}
