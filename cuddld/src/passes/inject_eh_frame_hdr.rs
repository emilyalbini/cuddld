use crate::interner::intern;
use crate::repr::object::Object;
use crate::repr::sections::{EhFrameHdrSection, SectionContent, SectionId};
use crate::repr::segments::{Segment, SegmentContent, SegmentType};
use cuddld_dwarf::exception_frames::{ExceptionFramesError, parse_exception_frames};
use cuddld_elf::ElfPermissions;
use cuddld_macros::{Display, Error};
use cuddld_utils::ints::Address;
use std::io::Cursor;

pub(crate) fn run(object: &mut Object) -> Result<(), InjectEhFrameHdrError> {
    if !object.emit_eh_frame_hdr {
        return Ok(());
    }

    let eh_frames = object
        .sections
        .iter()
        .filter(|section| section.name == intern(".eh_frame"))
        .collect::<Vec<_>>();
    let eh_frame = match eh_frames.as_slice() {
        [] => return Ok(()),
        [eh_frame] => eh_frame,
        _ => panic!("multiple .eh_frame sections should've been merged"),
    };

    let SectionContent::Data(eh_frame_data) = &eh_frame.content else {
        return Err(InjectEhFrameHdrError::EhFrameNotProgram(eh_frame.id));
    };

    let parsed = parse_exception_frames(
        &object.raw_type_context(),
        &mut Cursor::new(&eh_frame_data.bytes),
        Address::from(0u8),
    )?;

    let count: usize = parsed.iter().map(|cie| cie.frames.len()).sum();

    let id = object
        .sections
        .builder(".eh_frame_hdr", EhFrameHdrSection::placeholder(eh_frame.id, count))
        .create();

    object.segments.add(Segment {
        align: 4,
        type_: SegmentType::GnuEhFrame,
        perms: ElfPermissions::R,
        content: vec![SegmentContent::Section(id)],
    });

    Ok(())
}

#[derive(Debug, Display, Error)]
pub(crate) enum InjectEhFrameHdrError {
    #[transparent]
    Parse(ExceptionFramesError),
    #[display("the .eh_frame section ({f0:?}) is not a program section")]
    EhFrameNotProgram(SectionId),
}
