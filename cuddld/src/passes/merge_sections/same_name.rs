use crate::interner::{Interned, intern};
use crate::repr::object::Object;
use crate::repr::sections::{DataSection, SectionContent, SectionId};
use cuddld_diagnostics::ObjectSpan;
use cuddld_elf::{ElfDeduplication, ElfPermissions, ElfProgramSectionKind};
use cuddld_macros::{Display, Error};
use cuddld_utils::ints::{Length, Offset, OutOfBoundsError};
use std::collections::BTreeMap;

pub(super) fn run(
    object: &mut Object,
) -> Result<BTreeMap<SectionId, SameNameMerge>, MergeSameNameError> {
    let mut grouped_sections: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for section in object.sections.iter() {
        let SectionContent::Data(data) = &section.content else { continue };
        let (name, sort_order) = match data.kind {
            ElfProgramSectionKind::Program => (section.name, 0),
            // For .init_array and .fini_array, we need to consider all .{init,fini}_array.*
            // sections (like .init_array.98) to be the same, and merge them in order according to
            // the number after the dot.
            ElfProgramSectionKind::InitArray => (
                intern(".init_array"),
                init_fini_sort_order("init array", ".init_array", &*section.name.resolve())?,
            ),
            ElfProgramSectionKind::FiniArray => (
                intern(".fini_array"),
                init_fini_sort_order("fini array", ".fini_array", &*section.name.resolve())?,
            ),
        };
        grouped_sections
            .entry(GroupKey {
                kind: data.kind,
                name,
                perms: data.perms,
                deduplication: data.deduplication,
                inside_relro: data.inside_relro,
            })
            .or_default()
            .push(GroupValue { id: section.id, source: section.source.clone(), sort_order });
    }
    grouped_sections.retain(|_name, sections| sections.len() > 1);
    for sections in grouped_sections.values_mut() {
        sections.sort_by_key(|s| s.sort_order);
    }

    let mut merged = BTreeMap::new();
    for (key, sections) in grouped_sections {
        let id = object.sections.reserve_placeholder();
        let mut data = DataSection {
            kind: key.kind,
            perms: key.perms,
            deduplication: key.deduplication,
            bytes: Vec::new(),
            relocations: Vec::new(),
            inside_relro: key.inside_relro,
        };

        let mut sections_iter = sections.iter();
        let mut source = sections_iter.next().unwrap().source.clone();
        for old in sections_iter {
            source = source.merge(&old.source);
        }
        let interned_source = intern(source.clone());

        let mut offset = Offset::from(0);
        let mut is_retain = false;
        for old in sections {
            let old_section = object.sections.remove(old.id, None);
            let SectionContent::Data(old_data) = old_section.content else {
                panic!("only data sections should reach here");
            };

            data.bytes.extend_from_slice(&old_data.bytes);
            for mut relocation in old_data.relocations {
                relocation.offset = relocation.offset.add(offset)?;
                data.relocations.push(relocation);
            }

            if old_section.retain {
                is_retain = true;
            }

            merged.insert(old.id, SameNameMerge { target: id, offset, span: interned_source });
            offset = Length::from(data.bytes.len()).as_offset()?;
        }

        object
            .sections
            .builder(key.name.resolve().as_str(), data)
            .source(source)
            .retain(is_retain)
            .create_in_placeholder(id);
    }

    Ok(merged)
}

fn init_fini_sort_order(
    kind_for_error: &'static str,
    prefix: &str,
    name: &str,
) -> Result<u64, MergeSameNameError> {
    if name == prefix {
        return Ok(u64::MAX);
    } else if let Some(num) = name.strip_prefix(prefix).and_then(|s| s.strip_prefix('.')) {
        if let Ok(num) = num.parse() {
            return Ok(num);
        }
    }
    Err(MergeSameNameError::UnexpectedInitFiniName(kind_for_error, name.into()))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    kind: ElfProgramSectionKind,
    name: Interned<String>,
    perms: ElfPermissions,
    deduplication: ElfDeduplication,
    inside_relro: bool,
}

struct GroupValue {
    id: SectionId,
    source: ObjectSpan,
    sort_order: u64,
}

#[derive(Debug)]
pub(super) struct SameNameMerge {
    pub(super) target: SectionId,
    pub(super) offset: Offset,
    pub(super) span: Interned<ObjectSpan>,
}

#[derive(Debug, Error, Display)]
pub(crate) enum MergeSameNameError {
    #[transparent]
    OutOfBounds(OutOfBoundsError),
    #[display("unexpected name for {f0} section: {f1}")]
    UnexpectedInitFiniName(&'static str, String),
}
