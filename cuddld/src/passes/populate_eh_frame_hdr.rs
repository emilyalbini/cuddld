use crate::interner::intern;
use crate::repr::object::Object;
use crate::repr::sections::{SectionContent, SectionId};
use cuddld_dwarf::exception_frames::{ExceptionFramesError, parse_exception_frames};
use cuddld_elf::ElfEhFrameHdrEntry;
use cuddld_elf::writer::layout::Layout;
use cuddld_macros::{Display, Error};
use cuddld_utils::ints::{ExtractNumber, OutOfBoundsError};
use std::io::Cursor;

pub(crate) fn run(
    object: &mut Object,
    layout: &Layout<SectionId>,
) -> Result<(), PopulateEhFrameHdrError> {
    let ctx = object.raw_type_context();

    let eh_frame_name = intern(".eh_frame");
    let mut eh_frame = None;
    let mut eh_frame_hdr = None;
    for section in object.sections.iter_mut() {
        if let SectionContent::EhFrameHdr(..) = &section.content {
            if eh_frame_hdr.is_some() {
                panic!("multiple .eh_frame_hdr are present");
            }
            eh_frame_hdr = Some(section);
        } else if section.name == eh_frame_name {
            if eh_frame.is_some() {
                panic!("multiple .eh_frame are present");
            }
            eh_frame = Some(section);
        }
    }

    // The section is only present if inject_eh_frame_hdr decided we needed to have it.
    let Some(eh_frame_hdr) = eh_frame_hdr else {
        return Ok(());
    };
    let SectionContent::EhFrameHdr(eh_frame_hdr_data) = &mut eh_frame_hdr.content else {
        panic!("the generated .eh_frame_hdr has the wrong section type");
    };
    let eh_frame_hdr_addr = layout
        .metadata_of_section(&eh_frame_hdr.id)
        .memory
        .as_ref()
        .expect(".eh_frame_hdr is not allocated in memory")
        .address;

    let eh_frame = eh_frame.expect("missing .eh_frame while .eh_frame_hdr is present");
    let SectionContent::Data(eh_frame_data) = &eh_frame.content else {
        return Err(PopulateEhFrameHdrError::EhFrameNotProgram(eh_frame.id));
    };
    let eh_frame_addr = layout
        .metadata_of_section(&eh_frame.id)
        .memory
        .as_ref()
        .expect(".eh_frame is not allocated in memory")
        .address;

    let parsed =
        parse_exception_frames(&ctx, &mut Cursor::new(&eh_frame_data.bytes), eh_frame_addr)?;

    let mut entries = Vec::new();
    for cie in &parsed {
        for fde in &cie.frames {
            entries.push(ElfEhFrameHdrEntry {
                pointer_offset: fde.address.offset(eh_frame_hdr_addr.as_offset()?.neg())?.extract()
                    as _,
                fde_offset: fde
                    .offset
                    .add(eh_frame_addr.as_offset()?)?
                    .add(eh_frame_hdr_addr.as_offset()?.neg())?
                    .extract() as _,
            })
        }
    }
    entries.sort_by_key(|entry| entry.pointer_offset);
    eh_frame_hdr_data.set_content(entries);

    Ok(())
}

#[derive(Debug, Display, Error)]
pub(crate) enum PopulateEhFrameHdrError {
    #[transparent]
    Parse(ExceptionFramesError),
    #[transparent]
    OutOfBounds(OutOfBoundsError),
    #[display("the .eh_frame section ({f0:?}) is not a program section")]
    EhFrameNotProgram(SectionId),
}
