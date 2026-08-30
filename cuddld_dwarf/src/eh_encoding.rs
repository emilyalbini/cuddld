use cuddld_utils::raw_types::{
    ContextFrom, PointerSize, RawReadError, RawType, RawTypeContext, RawWriteError, SizedRawType,
};
use std::io::{Read, Write};

#[derive(Clone, Copy)]
pub enum DwarfEhEncoding {
    Present { value_format: DwarfEhValueFormat, application: DwarfEhApplication },
    Missing,
}

impl DwarfEhEncoding {
    pub fn value_size(&self, ctx: &RawTypeContext) -> usize {
        match self {
            DwarfEhEncoding::Present { value_format, .. } => match value_format {
                DwarfEhValueFormat::Pointer => u64::size(&PointerSize(ctx)),
                DwarfEhValueFormat::ULeb128 => unimplemented!(),
                DwarfEhValueFormat::U16 => u16::size(ctx),
                DwarfEhValueFormat::U32 => u32::size(ctx),
                DwarfEhValueFormat::U64 => u64::size(ctx),
                DwarfEhValueFormat::ILeb128 => unimplemented!(),
                DwarfEhValueFormat::I16 => i16::size(ctx),
                DwarfEhValueFormat::I32 => i32::size(ctx),
                DwarfEhValueFormat::I64 => i64::size(ctx),
            },
            DwarfEhEncoding::Missing => 0,
        }
    }
}

impl RawType for DwarfEhEncoding {
    fn read(ctx: &RawTypeContext, reader: &mut dyn Read) -> Result<Self, RawReadError> {
        let raw = u8::read(ctx, reader)?;
        if raw == 0xFF {
            return Ok(DwarfEhEncoding::Missing);
        }

        let value_format = match raw & 0xF {
            0x0 => DwarfEhValueFormat::Pointer,
            0x1 => DwarfEhValueFormat::ULeb128,
            0x2 => DwarfEhValueFormat::U16,
            0x3 => DwarfEhValueFormat::U32,
            0x4 => DwarfEhValueFormat::U64,
            0x9 => DwarfEhValueFormat::ILeb128,
            0xA => DwarfEhValueFormat::I16,
            0xB => DwarfEhValueFormat::I32,
            0xC => DwarfEhValueFormat::I64,
            unknown => {
                return Err(RawReadError::custom::<Self>(format!(
                    "unknown value format {unknown:#x}"
                )));
            }
        };
        let application = match raw >> 4 {
            0x0 => DwarfEhApplication::Absolute,
            0x1 => DwarfEhApplication::PcRel,
            0x2 => DwarfEhApplication::TextRel,
            0x3 => DwarfEhApplication::DataRel,
            0x4 => DwarfEhApplication::FuncRel,
            0x5 => DwarfEhApplication::Aligned,
            unknown => {
                return Err(RawReadError::custom::<Self>(format!(
                    "unknown application {unknown:#x}"
                )));
            }
        };
        Ok(DwarfEhEncoding::Present { value_format, application })
    }

    fn write(&self, ctx: &RawTypeContext, writer: &mut dyn Write) -> Result<(), RawWriteError> {
        match self {
            DwarfEhEncoding::Present { value_format, application } => {
                let value_format: u8 = match value_format {
                    DwarfEhValueFormat::Pointer => 0x0,
                    DwarfEhValueFormat::ULeb128 => 0x1,
                    DwarfEhValueFormat::U16 => 0x2,
                    DwarfEhValueFormat::U32 => 0x3,
                    DwarfEhValueFormat::U64 => 0x4,
                    DwarfEhValueFormat::ILeb128 => 0x9,
                    DwarfEhValueFormat::I16 => 0xA,
                    DwarfEhValueFormat::I32 => 0xB,
                    DwarfEhValueFormat::I64 => 0xC,
                };
                let application: u8 = match application {
                    DwarfEhApplication::Absolute => 0x00,
                    DwarfEhApplication::PcRel => 0x10,
                    DwarfEhApplication::TextRel => 0x20,
                    DwarfEhApplication::DataRel => 0x30,
                    DwarfEhApplication::FuncRel => 0x40,
                    DwarfEhApplication::Aligned => 0x50,
                };
                u8::write(&(value_format | application), ctx, writer)
            }
            DwarfEhEncoding::Missing => u8::write(&0xFF, ctx, writer),
        }
    }
}

impl SizedRawType for DwarfEhEncoding {
    fn size(ctx: &RawTypeContext) -> usize {
        u8::size(ctx)
    }
}

impl std::fmt::Debug for DwarfEhEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present { value_format, application } => write!(f, "{value_format:?} | {application:?}"),
            Self::Missing => write!(f, "Missing"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DwarfEhValueFormat {
    /// DW_EH_PE_absptr: The Value is a literal pointer whose size is determined by the
    /// architecture.
    Pointer,
    /// DW_EH_PE_uleb128: Unsigned value is encoded using the Little Endian Base 128 (LEB128).
    ULeb128,
    /// DW_EH_PE_udata2: A 2 bytes unsigned value.
    U16,
    /// DW_EH_PE_udata4: A 4 bytes unsigned value.
    U32,
    /// DW_EH_PE_udata8: An 8 bytes unsigned value.
    U64,
    /// DW_EH_PE_uleb128: Signed value is encoded using the Little Endian Base 128 (LEB128).
    ILeb128,
    /// DW_EH_PE_udata2: A 2 bytes signed value.
    I16,
    /// DW_EH_PE_udata4: A 4 bytes signed value.
    I32,
    /// DW_EH_PE_udata8: An 8 bytes signed value.
    I64,
}

#[derive(Debug, Clone, Copy)]
pub enum DwarfEhApplication {
    Absolute,
    /// DW_EH_PE_pcrel: Value is relative to the current program counter.
    PcRel,
    /// DW_EH_PE_textrel: Value is relative to the beginning of the .text section.
    TextRel,
    /// DW_EH_PE_datarel: Value is relative to the beginning of the .got or .eh_frame_hdr section.
    DataRel,
    /// DW_EH_PE_funcrel: Value is relative to the beginning of the function.
    FuncRel,
    /// DW_EH_PE_aligned: Value is aligned to an address unit sized boundary.
    Aligned,
}

pub struct DwarfEhContext<'a> {
    pub ctx: &'a RawTypeContext,
    pub encoding: &'a DwarfEhEncoding,
}

impl ContextFrom for DwarfEhEncoding {
    type Context<'a> = DwarfEhContext<'a>;

    fn context_from<'a>(ctx: &'a RawTypeContext, encoding: &'a Self) -> Self::Context<'a> {
        DwarfEhContext { ctx, encoding }
    }
}

impl RawType<DwarfEhContext<'_>> for u64 {
    fn read(ctx: &DwarfEhContext<'_>, reader: &mut dyn Read) -> Result<Self, RawReadError> {
        Ok(match ctx.encoding {
            DwarfEhEncoding::Present { value_format, .. } => match value_format {
                DwarfEhValueFormat::Pointer => u64::read(&PointerSize(ctx.ctx), reader)?,
                DwarfEhValueFormat::U16 => u16::read(ctx.ctx, reader)?.into(),
                DwarfEhValueFormat::U32 => u32::read(ctx.ctx, reader)?.into(),
                DwarfEhValueFormat::U64 => u64::read(ctx.ctx, reader)?,
                vf => {
                    return Err(RawReadError::custom::<Self>(format!(
                        "failed to read {vf:?} into u64"
                    )));
                }
            },
            DwarfEhEncoding::Missing => {
                return Err(RawReadError::custom::<Self>("failed to read missing value".into()));
            }
        })
    }

    fn write(&self, ctx: &DwarfEhContext<'_>, writer: &mut dyn Write) -> Result<(), RawWriteError> {
        match ctx.encoding {
            DwarfEhEncoding::Present { value_format, .. } => match value_format {
                DwarfEhValueFormat::Pointer => u64::write(self, &PointerSize(ctx.ctx), writer)?,
                DwarfEhValueFormat::U16 => u16::write(&(*self as _), ctx.ctx, writer)?,
                DwarfEhValueFormat::U32 => u32::write(&(*self as _), ctx.ctx, writer)?,
                DwarfEhValueFormat::U64 => u64::write(self, ctx.ctx, writer)?,
                vf => {
                    return Err(RawWriteError::custom::<Self>(format!(
                        "failed to write {vf:?} from u64"
                    )));
                }
            },
            DwarfEhEncoding::Missing => {}
        }
        Ok(())
    }
}

impl RawType<DwarfEhContext<'_>> for i64 {
    fn read(ctx: &DwarfEhContext<'_>, reader: &mut dyn Read) -> Result<Self, RawReadError> {
        Ok(match ctx.encoding {
            DwarfEhEncoding::Present { value_format, .. } => match value_format {
                DwarfEhValueFormat::I16 => i16::read(ctx.ctx, reader)?.into(),
                DwarfEhValueFormat::I32 => i32::read(ctx.ctx, reader)?.into(),
                DwarfEhValueFormat::I64 => i64::read(ctx.ctx, reader)?,
                vf => {
                    return Err(RawReadError::custom::<Self>(format!(
                        "failed to read {vf:?} into i64"
                    )));
                }
            },
            DwarfEhEncoding::Missing => {
                return Err(RawReadError::custom::<Self>("failed to read missing value".into()));
            }
        })
    }

    fn write(&self, ctx: &DwarfEhContext<'_>, writer: &mut dyn Write) -> Result<(), RawWriteError> {
        match ctx.encoding {
            DwarfEhEncoding::Present { value_format, .. } => match value_format {
                DwarfEhValueFormat::I16 => i16::write(&(*self as _), ctx.ctx, writer)?,
                DwarfEhValueFormat::I32 => i32::write(&(*self as _), ctx.ctx, writer)?,
                DwarfEhValueFormat::I64 => i64::write(self, ctx.ctx, writer)?,
                vf => {
                    return Err(RawWriteError::custom::<Self>(format!(
                        "failed to write {vf:?} from i64"
                    )));
                }
            },
            DwarfEhEncoding::Missing => {}
        }
        Ok(())
    }
}
