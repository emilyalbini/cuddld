// https://refspecs.linuxbase.org/LSB_4.0.0/LSB-Core-generic/LSB-Core-generic/ehframechpt.html
// https://www.corsix.org/content/elf-eh-frame
// https://www.airs.com/blog/archives/460

use crate::eh_encoding::DwarfEhEncoding;
use crate::leb128::Leb128;
use cuddld_macros::{Display, Error};
use cuddld_utils::ints::{Address, ExtractNumber as _, Length, Offset, OutOfBoundsError};
use cuddld_utils::raw_types::{ContextFrom as _, RawReadError, RawType, RawTypeContext};
use std::error::Error as _;
use std::ffi::CString;
use std::io::prelude::Read;

pub fn parse_exception_frames(
    ctx: &RawTypeContext,
    reader: &mut dyn Read,
    section_start: Address,
) -> Result<Vec<CommonInformationEntry>, ExceptionFramesError> {
    let mut reader = CountReads::new(reader);
    let mut result = Vec::new();

    loop {
        let offset = reader.count.as_offset()?;

        let len = parse_length(ctx, &mut reader)?;
        if len.extract() == 0 {
            break;
        }

        let mut entry_reader = CountReads::new(&mut reader);
        let cie_id = u32::read(ctx, &mut entry_reader)?;
        if cie_id == 0 {
            result.push(parse_cie(ctx, entry_reader, len, offset)?);
        } else {
            let Some(cie) = result.last_mut() else {
                return Err(ExceptionFramesError::FdeBeforeCie);
            };
            parse_fde(cie, ctx, entry_reader, len, offset, section_start)?;
        }
    }

    Ok(result)
}

fn parse_cie(
    ctx: &RawTypeContext,
    mut reader: CountReads<impl Read>,
    len: Length,
    offset: Offset,
) -> Result<CommonInformationEntry, ExceptionFramesError> {
    let version = u8::read(ctx, &mut reader)?;
    if version != 1 {
        return Err(ExceptionFramesError::InvalidCieVersion(version));
    }

    let raw_augmentation = CString::read(ctx, &mut reader)?;
    let raw_augmentation = raw_augmentation.as_bytes();
    let augmentations = if raw_augmentation.is_empty() {
        None
    } else if raw_augmentation[0] != b'z' {
        return Err(ExceptionFramesError::AugmentationNotStartWithZ);
    } else {
        let mut items = Vec::new();
        for chr in &raw_augmentation[1..] {
            match chr {
                b'L' => items.push(Augmentation::L),
                b'P' => items.push(Augmentation::P),
                b'R' => items.push(Augmentation::R),
                other => return Err(ExceptionFramesError::InvalidAugmentation(*other as _)),
            }
        }
        Some(items)
    };

    let code_alignment_factor = u64::read(&Leb128, &mut reader)?;
    let data_alignment_factor = i64::read(&Leb128, &mut reader)?;
    let return_address_column = u64::read(&Leb128, &mut reader)?;

    let mut address_pointer_encoding = None;
    let mut lsda_pointer_encoding = None;
    let mut personality_routine = None;
    if let Some(augmentations) = &augmentations {
        let augmentation_data_len = u32::read(&Leb128, &mut reader)?;
        let mut augmentation_reader = CountReads::new(&mut reader);

        for augmentation in augmentations {
            match augmentation {
                Augmentation::L => {
                    lsda_pointer_encoding =
                        Some(DwarfEhEncoding::read(ctx, &mut augmentation_reader)?)
                }
                Augmentation::P => {
                    let encoding = DwarfEhEncoding::read(ctx, &mut augmentation_reader)?;
                    personality_routine = Some(
                        u64::read(
                            &DwarfEhEncoding::context_from(ctx, &encoding),
                            &mut augmentation_reader,
                        )?
                        .into(),
                    );
                }
                Augmentation::R => {
                    address_pointer_encoding =
                        Some(DwarfEhEncoding::read(ctx, &mut augmentation_reader)?);
                }
            }
        }

        if Length::from(augmentation_data_len) != augmentation_reader.count {
            return Err(ExceptionFramesError::WrongAugmentationDataLen);
        }
    }

    let mut instructions = vec![0; len.add_offset(reader.count.as_offset()?.neg())?.extract() as _];
    reader.read_exact(&mut instructions)?;

    Ok(CommonInformationEntry {
        offset,
        code_alignment_factor,
        data_alignment_factor,
        return_address_column,
        augmentations,
        address_pointer_encoding,
        lsda_pointer_encoding,
        personality_routine,
        instructions,
        frames: Vec::new(),
    })
}

fn parse_fde(
    cie: &mut CommonInformationEntry,
    ctx: &RawTypeContext,
    mut reader: CountReads<impl Read>,
    len: Length,
    offset: Offset,
    section_start: Address,
) -> Result<(), ExceptionFramesError> {
    let Some(address_enc) = cie.address_pointer_encoding else {
        return Err(ExceptionFramesError::AddressPointerEncodingMissing);
    };
    let address_ctx = DwarfEhEncoding::context_from(ctx, &address_enc);

    let address_start = reader.count;
    let address_offset = i64::read(&address_ctx, &mut reader)?;
    let address = section_start
        .offset(offset)?
        .offset(address_start.as_offset()?)?
        .offset(address_offset.into())?
        .offset(4.into() /* length field */)?;
    let length = u64::try_from(i64::read(&address_ctx, &mut reader)?)
        .map_err(|_| ExceptionFramesError::NegativeLength)?;

    let mut lsda_address = None;
    if let Some(augmentations) = &cie.augmentations {
        let augmentation_data_len = u32::read(&Leb128, &mut reader)?;
        let augmentation_offset = reader.count;
        let mut augmentation_reader = CountReads::new(&mut reader);

        for augmentation in augmentations {
            match augmentation {
                Augmentation::L => {
                    let lsda_enc = cie.lsda_pointer_encoding.unwrap();
                    let lsda_ctx = DwarfEhEncoding::context_from(ctx, &lsda_enc);
                    let lsda_start = augmentation_reader.count.add(augmentation_offset)?;
                    let lsda_offset = i64::read(&lsda_ctx, &mut augmentation_reader)?;
                    lsda_address = Some(
                        section_start
                            .offset(offset)?
                            .offset(lsda_start.as_offset()?)?
                            .offset(lsda_offset.into())?
                            .offset(4.into() /* length field */)?,
                    );
                }
                Augmentation::P => {}
                Augmentation::R => {}
            }
        }

        if Length::from(augmentation_data_len) != augmentation_reader.count {
            return Err(ExceptionFramesError::WrongAugmentationDataLen);
        }
    }

    let mut instructions = vec![0; len.add_offset(reader.count.as_offset()?.neg())?.extract() as _];
    reader.read_exact(&mut instructions)?;

    cie.frames.push(FrameDescriptionEntry {
        offset,
        address,
        length: length.into(),
        lsda_address,
        instructions,
    });

    Ok(())
}

fn parse_length(
    ctx: &RawTypeContext,
    reader: &mut dyn Read,
) -> Result<Length, ExceptionFramesError> {
    let short = match u32::read(ctx, reader) {
        Ok(short) => short,
        Err(err) => {
            // If we reached the end of the file, return a length of 0.
            if let Some(source) = err.source() {
                if let Some(io) = source.downcast_ref::<std::io::Error>() {
                    if io.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Ok(0u64.into());
                    }
                }
            }
            return Err(err.into());
        }
    };
    if short == 0xFFFF {
        // The spec says that if the u32 is 0xFFFF we should parse the next u64, but that means the
        // length of the length field is dynamic, and we are currently hardcoding it to be 4 when
        // calculating addresses in this file. We thus reject entries larger than 2GB.
        return Err(ExceptionFramesError::EntryTooLarge);
    } else {
        Ok(short.into())
    }
}

#[derive(Debug)]
pub struct CommonInformationEntry {
    pub offset: Offset,
    pub code_alignment_factor: u64,
    pub data_alignment_factor: i64,
    pub return_address_column: u64,
    pub augmentations: Option<Vec<Augmentation>>,
    pub address_pointer_encoding: Option<DwarfEhEncoding>,
    pub lsda_pointer_encoding: Option<DwarfEhEncoding>,
    pub personality_routine: Option<Address>,
    pub instructions: Vec<u8>,
    pub frames: Vec<FrameDescriptionEntry>,
}

#[derive(Debug)]
pub struct FrameDescriptionEntry {
    pub offset: Offset,
    pub address: Address,
    pub length: Length,
    pub lsda_address: Option<Address>,
    pub instructions: Vec<u8>,
}

#[derive(Debug)]
pub enum Augmentation {
    L,
    P,
    R,
}

struct CountReads<R: Read> {
    reader: R,
    count: Length,
}

impl<R: Read> CountReads<R> {
    fn new(reader: R) -> Self {
        Self { reader, count: 0u64.into() }
    }
}

impl<R: Read> Read for CountReads<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.count = self.count.add(len.into()).map_err(std::io::Error::other)?;
        Ok(len)
    }
}

#[derive(Debug, Error, Display)]
pub enum ExceptionFramesError {
    #[transparent]
    Read(RawReadError),
    #[transparent]
    Io(std::io::Error),
    #[transparent]
    OutOfBounds(OutOfBoundsError),
    #[display("invalid CIE version: {f0}")]
    InvalidCieVersion(u8),
    #[display("the augmentation string doesn't start with z")]
    AugmentationNotStartWithZ,
    #[display("invalid entry in the augmentation string: {f0}")]
    InvalidAugmentation(char),
    #[display("there are {f0} trailing bytes in the augmentation data")]
    TrailingAugmentationData(usize),
    #[display("found an FDE before a CIE")]
    FdeBeforeCie,
    #[display("the address pointer encoding is missing from the CIE")]
    AddressPointerEncodingMissing,
    #[display("negative length in the FDE")]
    NegativeLength,
    #[display("the length of the augmentation data doesn't match what was read")]
    WrongAugmentationDataLen,
    #[display("the entry is too large")]
    EntryTooLarge,
}
