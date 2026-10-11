use crate::{ElfProgramSection, ElfProgramSectionKind};
use crate::errors::LoadError;
use crate::reader::sections::reader::{SectionMetadata, SectionReader};

pub(super) fn read(
    reader: &mut SectionReader<'_, '_>,
    meta: &dyn SectionMetadata,
    kind: ElfProgramSectionKind,
) -> Result<ElfProgramSection, LoadError> {
    Ok(ElfProgramSection {
        kind,
        perms: meta.permissions(),
        deduplication: meta.deduplication_flag()?,
        raw: reader.content()?,
    })
}
