use crate::cli::{CliInput, CliInputOptions};
use crate::diagnostics::builders::NoSymbolNameAtArchiveStartDiagnostic;
use crate::interner::{Interned, intern};
use crate::repr::symbols::{SymbolValue, Symbols};
use crate::utils::file_type::{FileType, FileTypeError};
use crate::utils::resolve_cli_input::{ResolveCliInputError, resolve_cli_input};
use cuddld_ar::{ArFile, ArMemberId, ArReadError, ArReader};
use cuddld_diagnostics::ObjectSpan;
use cuddld_elf::{ElfObject, ElfReader, ElfSectionContent, ElfSymbolBinding, LoadError};
use cuddld_macros::{Display, Error};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;

pub(super) struct ObjectsReader<'a> {
    search_paths: &'a [PathBuf],
    remaining_inputs: &'a [CliInput],
    current_archive: Option<PendingArchive<'a>>,
}

impl<'a> ObjectsReader<'a> {
    pub(super) fn new(search_paths: &'a [PathBuf], inputs: &'a [CliInput]) -> Self {
        Self { remaining_inputs: inputs, current_archive: None, search_paths }
    }

    pub(super) fn next_object(
        &mut self,
        symbols: &Symbols,
    ) -> Result<Option<NextObject>, ReadObjectsError> {
        loop {
            if let Some(result) = self.next_from_archive(symbols)? {
                return Ok(Some(result));
            }

            if self.remaining_inputs.is_empty() {
                return Ok(None);
            }
            let input = &self.remaining_inputs[0];
            self.remaining_inputs = &self.remaining_inputs[1..];

            let resolved = resolve_cli_input(self.search_paths, input)?;

            let mut r = BufReader::new(
                File::open(&resolved.path)
                    .map_err(|e| ReadObjectsError::OpenFailed(resolved.path.clone(), e))?,
            );
            match FileType::from_magic_number(&resolved.path, &mut r)? {
                FileType::Elf => {
                    return Ok(Some(NextObject {
                        source: ObjectSpan::new_file(&resolved.path),
                        library_name: LibraryName::Known(resolved.library_name),
                        requirements: vec![],
                        reader: ElfReader::new_owned(Box::new(r)).map_err(|e| {
                            ReadObjectsError::FileParseFailed(resolved.path.clone(), e)
                        })?,
                        options: input.options.clone(),
                    }));
                }
                FileType::Ar => {
                    if let Some(archive) =
                        PendingArchive::new(resolved.path.clone(), r, symbols, &input.options)?
                    {
                        self.current_archive = Some(archive);
                    }
                    continue;
                }
            }
        }
    }

    fn next_from_archive(
        &mut self,
        symbols: &Symbols,
    ) -> Result<Option<NextObject>, ReadObjectsError> {
        let Some(pending_archive) = &mut self.current_archive else { return Ok(None) };
        match pending_archive.next(symbols)? {
            Some((file, requirements)) => {
                match ElfReader::new_owned(Box::new(Cursor::new(file.content))) {
                    Ok(reader) => Ok(Some(NextObject {
                        source: ObjectSpan::new_archive_member(&pending_archive.path, &file.name),
                        library_name: LibraryName::InsideArchive,
                        requirements,
                        reader,
                        options: pending_archive.options.clone(),
                    })),
                    Err(err) => Err(ReadObjectsError::ArchiveFileParseFailed(
                        file.name,
                        pending_archive.path.clone(),
                        err,
                    )),
                }
            }
            None => {
                self.current_archive = None;
                Ok(None)
            }
        }
    }
}

struct PendingArchive<'a> {
    path: PathBuf,
    reader: ArReader<BufReader<File>>,
    pending_members: VecDeque<(ArMemberId, Vec<ObjectRequirement>)>,
    loaded_members: HashSet<ArMemberId>,
    options: &'a CliInputOptions,
}

impl<'a> PendingArchive<'a> {
    fn new(
        path: PathBuf,
        reader: BufReader<File>,
        symbols: &Symbols,
        options: &'a CliInputOptions,
    ) -> Result<Option<Self>, ReadObjectsError> {
        let mut this = PendingArchive {
            reader: ArReader::new(reader)
                .map_err(|e| ReadObjectsError::ExtractFailed(path.clone(), e))?,
            path,
            pending_members: VecDeque::new(),
            loaded_members: HashSet::new(),
            options,
        };

        // Only return a new instance of PendingArchive if there are actually objects to load.
        this.calculate_pending(symbols)?;
        Ok(Some(this).filter(|this| !this.pending_members.is_empty()))
    }

    fn calculate_pending(&mut self, symbols: &Symbols) -> Result<(), ReadObjectsError> {
        let Some(symbol_table) = self.reader.symbol_table() else {
            return Err(ReadObjectsError::NoSymbolTableAtArchiveStart {
                diagnostic: NoSymbolNameAtArchiveStartDiagnostic {
                    archive_path: self.path.clone(),
                },
                path: self.path.clone(),
            });
        };

        let mut ordered = Vec::new();
        let mut require_symbols = HashMap::new();
        for (symbol_name, member_id) in &symbol_table.symbols {
            if let Ok(symbol) = symbols.get_global(intern(symbol_name))
                && let SymbolValue::Undefined { .. } = symbol.value()
            {
                if !require_symbols.contains_key(member_id) {
                    ordered.push(*member_id);
                }
                require_symbols
                    .entry(*member_id)
                    .or_insert_with(HashSet::new)
                    .insert(symbol.name());
            }
        }

        for member_id in ordered {
            // We want to maintain the ordering of the ArMemberId to ensure determinism in the
            // linker output (aka we need to store it in a Vec). The HashSet is used as a quick
            // way to lookup, since it doesn't preserve ordering.
            //
            // This also prevents loading the same object file multiple times when scanning
            // the archive again for new required symbols.
            if self.loaded_members.insert(member_id) {
                self.pending_members.push_back((
                    member_id,
                    // The elf spec says that we shouldn't load an archive if it only contains weak
                    // symbols, so we require that at least one of the symbols in it is global.
                    // Unfortunately we have to defer this step to later using a requirement,
                    // because the information we have access to right now (the symbols cache in the
                    // archive) doesn't specify whether the symbol is weak or not.
                    vec![ObjectRequirement::HasGlobalSymbol(
                        require_symbols.remove(&member_id).unwrap_or_default(),
                    )],
                ));
            }
        }

        Ok(())
    }

    fn next(
        &mut self,
        symbols: &Symbols,
    ) -> Result<Option<(ArFile, Vec<ObjectRequirement>)>, ReadObjectsError> {
        loop {
            if let Some((member_id, require_symbols)) = self.pending_members.pop_front() {
                return Ok(Some((
                    self.reader
                        .read_member_by_id(&member_id)
                        .map_err(|e| ReadObjectsError::ExtractFailed(self.path.clone(), e))?,
                    require_symbols,
                )));
            }

            // After loading the archive, new undefined symbols that can be satisfied by the
            // archive can appear. This can happen for example if an object file later in the
            // archive depends on object files earlier in the archive. We thus continue trying to
            // load from the archive until there are no more pending files to load.
            self.calculate_pending(symbols)?;
            if self.pending_members.is_empty() {
                return Ok(None);
            }
        }
    }
}

pub(super) struct NextObject {
    pub(super) reader: ElfReader<'static>,
    pub(super) library_name: LibraryName,
    pub(super) requirements: Vec<ObjectRequirement>,
    pub(super) source: ObjectSpan,
    pub(super) options: CliInputOptions,
}

pub(super) enum LibraryName {
    Known(String),
    InsideArchive,
}

pub(super) enum ObjectRequirement {
    HasGlobalSymbol(HashSet<Interned<String>>),
}

impl ObjectRequirement {
    pub(super) fn matches(&self, elf: &ElfObject) -> bool {
        match self {
            ObjectRequirement::HasGlobalSymbol(names) => {
                for section in elf.sections.values() {
                    // We need to resolve the symbol names, and at this point we don't have access
                    // to the merged string table, so we have to find it in the elf file and keep
                    // track of it.
                    let mut strings = None;

                    let ElfSectionContent::SymbolTable(table) = &section.content else { continue };
                    for symbol in table.symbols.values() {
                        match symbol.binding {
                            ElfSymbolBinding::Local
                            | ElfSymbolBinding::Weak
                            | ElfSymbolBinding::Unknown(_) => continue,
                            ElfSymbolBinding::Global => {}
                        }

                        let strings = match &strings {
                            Some(s) => s,
                            None => {
                                let Some(section) = elf.sections.get(&symbol.name.section) else {
                                    continue;
                                };
                                let ElfSectionContent::StringTable(table) = &section.content else {
                                    continue;
                                };
                                strings = Some(table);
                                table
                            }
                        };
                        let Some(name) = strings.get(symbol.name.offset) else { continue };
                        let name: Interned<String> = intern(name);
                        if names.contains(&name) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

#[derive(Debug, Error, Display)]
pub(crate) enum ReadObjectsError {
    #[display("failed to open {f0:?}")]
    OpenFailed(PathBuf, #[source] std::io::Error),
    #[display("failed to extract the next file from the archive {f0:?}")]
    ExtractFailed(PathBuf, #[source] ArReadError),
    #[display("failed to parse archive member {f0} of {f1:?}")]
    ArchiveFileParseFailed(String, PathBuf, #[source] LoadError),
    #[display("failed to parse {f0:?}")]
    FileParseFailed(PathBuf, #[source] LoadError),
    #[display("the first member of the archive {path:?} is not a symbol table")]
    NoSymbolTableAtArchiveStart {
        path: PathBuf,
        #[diagnostic]
        diagnostic: NoSymbolNameAtArchiveStartDiagnostic,
    },
    #[transparent]
    FileType(FileTypeError),
    #[transparent]
    ResolveCliInput(ResolveCliInputError),
}
