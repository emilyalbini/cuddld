use crate::raw::RawEhFrameHdrHeader;
use crate::reader::sections::SectionReader;
use crate::{ElfEhFrameHdr, ElfEhFrameHdrEntry, LoadError};

pub(super) fn read(reader: &mut SectionReader<'_, '_>) -> Result<ElfEhFrameHdr, LoadError> {
    let mut cursor = reader.content_cursor()?;

    let header: RawEhFrameHdrHeader = cursor.read_raw()?;
    if header.version != 1 {
        return Err(LoadError::BadEhFrameHdrVersion(header.version));
    }

    let frame_pointer = cursor.read_raw_ctx(&header.eh_frame_ptr_enc)?;
    let entries_count: u64 = cursor.read_raw_ctx(&header.fde_count_enc)?;

    let mut entries = Vec::new();
    for _ in 0..entries_count {
        entries.push(ElfEhFrameHdrEntry {
            pointer_offset: cursor.read_raw_ctx(&header.table_enc)?,
            fde_offset: cursor.read_raw_ctx(&header.table_enc)?,
        });
    }

    Ok(ElfEhFrameHdr {
        frame_pointer_encoding: header.eh_frame_ptr_enc,
        entry_count_encoding: header.fde_count_enc,
        entry_encoding: header.table_enc,
        frame_pointer,
        entries,
    })
}
