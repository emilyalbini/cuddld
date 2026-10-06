use crate::ids::ElfSectionId;
use crate::writer::LayoutError;
use crate::writer::layout::Part;
use crate::{
    ElfABI, ElfClass, ElfEndian, ElfObject, ElfSection, ElfSectionContent, ElfSegmentType,
};
use cuddld_dwarf::eh_encoding::DwarfEhEncoding;
use cuddld_utils::raw_types::RawTypeContext;

pub trait LayoutDetailsProvider<S: Copy> {
    fn class(&self) -> ElfClass;
    fn endian(&self) -> ElfEndian;
    fn abi(&self) -> ElfABI;

    fn sections_count(&self) -> usize;
    fn segments_count(&self) -> usize;

    fn program_section_len(&self, id: S) -> usize;
    fn uninitialized_section_len(&self, id: S) -> usize;
    fn string_table_len(&self, id: S) -> usize;
    fn symbols_in_table_count(&self, id: S) -> usize;
    fn sections_in_group_count(&self, id: S) -> usize;
    fn dynamic_directives_count(&self, id: S) -> usize;
    fn relocations_in_table_count(&self, id: S) -> usize;
    fn hash_details(&self, id: S) -> LayoutDetailsHash;
    fn gnu_hash_details(&self, id: S) -> LayoutDetailsGnuHash;
    fn note_details(&self, id: S) -> Vec<LayoutDetailsNote>;
    fn eh_frame_hdr_details(&self, id: S) -> LayoutDetailsEhFrameHdr;

    fn parts_for_sections(&self) -> Result<Vec<Part<S>>, LayoutError>;
    fn parts_groups(&self) -> Result<Vec<LayoutPartsGroup<S>>, LayoutError>;

    fn raw_type_context(&self) -> RawTypeContext {
        RawTypeContext::new(self.class(), self.endian(), self.abi())
    }
}

pub struct LayoutDetailsHash {
    pub buckets: usize,
    pub chain: usize,
}

pub struct LayoutDetailsGnuHash {
    pub bloom: usize,
    pub buckets: usize,
    pub chain: usize,
}

pub struct LayoutDetailsNote {
    pub name_len: usize,
    pub value_len: usize,
}

pub struct LayoutDetailsEhFrameHdr {
    pub frame_pointer_encoding: DwarfEhEncoding,
    pub entry_count_encoding: DwarfEhEncoding,
    pub entry_encoding: DwarfEhEncoding,
    pub entries_count: usize,
}

pub struct LayoutPartsGroup<S> {
    pub align: u64,
    pub parts: Vec<Part<S>>,
}

macro_rules! cast_section {
    ($self:expr, $id:expr, $variant:ident) => {
        match $self.sections.get(&$id).map(|s| &s.content) {
            Some(ElfSectionContent::$variant(inner)) => inner,
            Some(_) => panic!("section {:?} is of the wrong type", $id),
            None => panic!("missing section {:?}", $id),
        }
    };
}

impl LayoutDetailsProvider<ElfSectionId> for ElfObject {
    fn class(&self) -> ElfClass {
        self.env.class
    }

    fn endian(&self) -> ElfEndian {
        self.env.endian
    }

    fn abi(&self) -> ElfABI {
        self.env.abi
    }

    fn sections_count(&self) -> usize {
        self.sections.len()
    }

    fn segments_count(&self) -> usize {
        self.segments.len()
    }

    fn program_section_len(&self, id: ElfSectionId) -> usize {
        cast_section!(self, id, Program).raw.len()
    }

    fn uninitialized_section_len(&self, id: ElfSectionId) -> usize {
        cast_section!(self, id, Uninitialized).len as _
    }

    fn string_table_len(&self, id: ElfSectionId) -> usize {
        cast_section!(self, id, StringTable).len()
    }

    fn symbols_in_table_count(&self, id: ElfSectionId) -> usize {
        cast_section!(self, id, SymbolTable).symbols.len()
    }

    fn sections_in_group_count(&self, id: ElfSectionId) -> usize {
        cast_section!(self, id, Group).sections.len()
    }

    fn dynamic_directives_count(&self, id: ElfSectionId) -> usize {
        cast_section!(self, id, Dynamic).directives.len()
    }

    fn relocations_in_table_count(&self, id: ElfSectionId) -> usize {
        match self.sections.get(&id).map(|s| &s.content) {
            Some(ElfSectionContent::Rel(rel)) => rel.relocations.len(),
            Some(ElfSectionContent::Rela(rela)) => rela.relocations.len(),
            Some(_) => panic!("section {id:?} is of the wrong type"),
            None => panic!("missing section {id:?}"),
        }
    }

    fn hash_details(&self, id: ElfSectionId) -> LayoutDetailsHash {
        let hash = cast_section!(self, id, Hash);
        LayoutDetailsHash { buckets: hash.buckets.len(), chain: hash.chain.len() }
    }

    fn gnu_hash_details(&self, id: ElfSectionId) -> LayoutDetailsGnuHash {
        let gnu_hash = cast_section!(self, id, GnuHash);
        LayoutDetailsGnuHash {
            bloom: gnu_hash.bloom.len(),
            buckets: gnu_hash.buckets.len(),
            chain: gnu_hash.chain.len(),
        }
    }

    fn note_details(&self, id: ElfSectionId) -> Vec<LayoutDetailsNote> {
        cast_section!(self, id, Note)
            .notes
            .iter()
            .map(|note| LayoutDetailsNote {
                name_len: note.name().len(),
                value_len: note.value_len(&self.raw_type_context()),
            })
            .collect()
    }

    fn eh_frame_hdr_details(&self, id: ElfSectionId) -> LayoutDetailsEhFrameHdr {
        let eh_frame_hdr = cast_section!(self, id, EhFrameHdr);
        LayoutDetailsEhFrameHdr {
            frame_pointer_encoding: eh_frame_hdr.frame_pointer_encoding,
            entry_count_encoding: eh_frame_hdr.entry_count_encoding,
            entry_encoding: eh_frame_hdr.entry_encoding,
            entries_count: eh_frame_hdr.entries.len(),
        }
    }

    fn parts_for_sections(&self) -> Result<Vec<Part<ElfSectionId>>, LayoutError> {
        let mut result = Vec::new();

        result.push(Part::Header);
        result.push(Part::ProgramHeaders);
        result.push(Part::SectionHeaders);

        for (id, section) in &self.sections {
            let Some(part) = part_for_section(*id, section)? else { continue };
            result.push(part);
        }
        Ok(result)
    }

    fn parts_groups(&self) -> Result<Vec<LayoutPartsGroup<ElfSectionId>>, LayoutError> {
        let mut groups = Vec::new();
        for segment in &self.segments {
            match &segment.type_ {
                ElfSegmentType::ProgramHeaderTable => continue,
                ElfSegmentType::Interpreter => {}
                ElfSegmentType::Load => {}
                ElfSegmentType::Dynamic => continue,
                ElfSegmentType::Note => continue,
                ElfSegmentType::GnuEhFrame => continue,
                ElfSegmentType::GnuStack => continue,
                ElfSegmentType::GnuRelro => continue,
                ElfSegmentType::GnuProperty => continue,
                ElfSegmentType::Null => continue,
                ElfSegmentType::Unknown(_) => continue,
            };

            let mut group = LayoutPartsGroup { align: segment.align, parts: Vec::new() };
            let range = segment.virtual_address..=(segment.virtual_address + segment.memory_size);
            for (id, section) in &self.sections {
                if section.memory_address == 0 || !range.contains(&section.memory_address) {
                    continue;
                }
                let Some(part) = part_for_section(*id, section)? else { continue };
                group.parts.push(part);
            }
            if !group.parts.is_empty() {
                groups.push(group);
            }
        }
        Ok(groups)
    }
}

fn part_for_section(
    id: ElfSectionId,
    section: &ElfSection,
) -> Result<Option<Part<ElfSectionId>>, LayoutError> {
    Ok(Some(match &section.content {
        ElfSectionContent::Null => return Ok(None),
        ElfSectionContent::Program(_) => Part::ProgramSection(id),
        ElfSectionContent::Uninitialized(_) => Part::UninitializedSection(id),
        ElfSectionContent::SymbolTable(_) => Part::SymbolTable(id),
        ElfSectionContent::StringTable(_) => Part::StringTable(id),
        ElfSectionContent::Rel(_) => Part::Rel(id),
        ElfSectionContent::Rela(_) => Part::Rela(id),
        ElfSectionContent::Group(_) => Part::Group(id),
        ElfSectionContent::Hash(_) => Part::Hash(id),
        ElfSectionContent::Dynamic(_) => Part::Dynamic(id),
        ElfSectionContent::GnuHash(_) => Part::GnuHash(id),
        ElfSectionContent::EhFrameHdr(_) => Part::EhFrameHdr(id),

        ElfSectionContent::Note(_) => Part::Note(id),
        ElfSectionContent::Unknown(_) => {
            return Err(LayoutError::UnknownSection);
        }
    }))
}
