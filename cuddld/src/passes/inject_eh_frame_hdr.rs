use crate::interner::intern;
use crate::repr::object::Object;
use crate::repr::sections::EhFrameHdrSection;
use crate::repr::segments::{Segment, SegmentContent, SegmentType};
use cuddld_elf::ElfPermissions;

pub(crate) fn run(object: &mut Object) {
    if !object.emit_eh_frame_hdr {
        return;
    }

    let eh_frames = object
        .sections
        .iter()
        .filter(|section| section.name == intern(".eh_frame"))
        .collect::<Vec<_>>();
    let ef = match eh_frames.as_slice() {
        [] => return,
        [eh_frame] => eh_frame,
        _ => unimplemented!("multiple .eh_frame sections should've been merged"),
    };

    let id = object.sections.builder(".eh_frame_hdr", EhFrameHdrSection::new(ef.id)).create();

    object.segments.add(Segment {
        align: 4,
        type_: SegmentType::GnuEhFrame,
        perms: ElfPermissions::R,
        content: vec![SegmentContent::Section(id)],
    });
}
