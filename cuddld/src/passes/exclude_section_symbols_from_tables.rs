use crate::repr::object::Object;
use crate::repr::symbols::SymbolValue;
use crate::repr::symbols::views::AllSymbols;

pub(crate) fn remove(object: &mut Object) {
    for symbol in object.symbols.iter_mut(&AllSymbols) {
        if let SymbolValue::Section { .. } = &symbol.value() {
            symbol.mark_exclude_from_tables()
        }
    }
}
