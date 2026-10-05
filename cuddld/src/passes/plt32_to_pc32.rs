use crate::repr::object::Object;
use crate::repr::relocations::RelocationType;
use crate::repr::sections::SectionContent;
use crate::repr::symbols::SymbolVisibility;

// If a symbol is local or global but hidden, the PLT32 and PC32 relocation types are equivalent, as
// in practice they both replace the placeholder with the relative address to jump to. When that
// happens, we can replace PLT32 with PC32 and avoid generating unnecessary PLT entries.
//
// While this is *also* an optimization, it's crucial when a PLT32 relocation points to a section
// symbol (which is always local), because the current implementation of cuddld cannot generate PLT
// entries for section symbols. So if we don't replace the relocation type things will explode.
pub(crate) fn run(object: &mut Object) {
    for section in object.sections.iter_mut() {
        let SectionContent::Data(data) = &mut section.content else { continue };
        for relocation in &mut data.relocations {
            if relocation.type_ != RelocationType::PLT32 {
                continue;
            }

            let symbol = object.symbols.get(relocation.symbol);
            let should_convert = match symbol.visibility() {
                SymbolVisibility::Local => true,
                SymbolVisibility::Global { weak: _, hidden } => hidden,
            };

            if should_convert {
                relocation.type_ = RelocationType::Relative32;
            }
        }
    }
}
