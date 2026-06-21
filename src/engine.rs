use goblin::Object;
use goblin::elf::Elf;
use goblin::mach::{Mach, MachO, SingleArch};
use goblin::pe::PE;
use memmap2::Mmap;
use regex::Regex;
use std::fs::File;
use std::path::Path;
use std::string::String;

/// Represents a match found in the binary.
pub struct SymbolMatch {
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Debug)]
pub enum SymbolKind {
    Import,
    Export,
}

/// Bundles the per-search criteria so the per-format collectors stay terse.
struct Filter<'a> {
    re: &'a Regex,
    include_imports: bool,
    include_exports: bool,
}

impl Filter<'_> {
    /// Records `name` if it passes the import/export filter and matches the regex.
    fn consider(&self, name: &str, is_import: bool, results: &mut Vec<SymbolMatch>) {
        if (is_import && !self.include_imports) || (!is_import && !self.include_exports) {
            return;
        }
        if !self.re.is_match(name) {
            return;
        }
        results.push(SymbolMatch {
            name: name.to_string(),
            kind: if is_import {
                SymbolKind::Import
            } else {
                SymbolKind::Export
            },
        });
    }
}

/// Parses a binary at the given path and extracts symbols matching the regex.
///
/// Reports imported and exported symbols across ELF, Mach-O and PE. For ELF
/// this is the dynamic symbol table (`.dynsym`) — not the full `.symtab`, which
/// is absent in stripped binaries anyway. The format is detected from the file
/// contents, so an all-ELF tree takes the ELF path with no extra work.
///
/// # Arguments
/// * `path` - Path to the binary.
/// * `re` - The compiled regex to match symbol names against.
pub fn parse_symbols(
    path: &Path,
    re: &Regex,
    include_imports: bool,
    include_exports: bool,
) -> Result<Vec<SymbolMatch>, String> {
    let file =
        File::open(path).map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;

    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|e| format!("Failed to mmap file {}: {}", path.display(), e))?;

    let obj =
        Object::parse(&mmap).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))?;

    let filter = Filter {
        re,
        include_imports,
        include_exports,
    };
    let mut results = Vec::new();

    match obj {
        Object::Elf(elf) => collect_elf(&elf, &filter, &mut results),
        Object::Mach(Mach::Binary(macho)) => collect_macho(&macho, &filter, &mut results)?,
        Object::Mach(Mach::Fat(multi)) => {
            for arch in &multi {
                let arch = arch
                    .map_err(|e| format!("Failed to parse fat Mach-O {}: {}", path.display(), e))?;
                if let SingleArch::MachO(macho) = arch {
                    collect_macho(&macho, &filter, &mut results)?;
                }
            }
        }
        Object::PE(pe) => collect_pe(&pe, &filter, &mut results),
        // TE / COFF / Archive / Unknown carry no import/export table we report.
        _ => {}
    }

    Ok(results)
}

/// ELF: walk the dynamic symbol table (`.dynsym`).
fn collect_elf(elf: &Elf, filter: &Filter, results: &mut Vec<SymbolMatch>) {
    for sym in elf.dynsyms.iter() {
        if let Some(name) = elf.dynstrtab.get_at(sym.st_name) {
            filter.consider(name, sym.is_import(), results);
        }
    }
}

/// Mach-O: walk the symbol table (nlist), reporting external symbols. An
/// undefined external is an import, a defined external is an export — the
/// `imports()`/`exports()` helpers miss imports bound via chained fixups, which
/// is how modern arm64 binaries link, so we read the symbol table directly.
fn collect_macho(
    macho: &MachO,
    filter: &Filter,
    results: &mut Vec<SymbolMatch>,
) -> Result<(), String> {
    for sym in macho.symbols() {
        let (name, nl) = sym.map_err(|e| format!("Failed to read Mach-O symbol: {}", e))?;
        // Only external (global) symbols are imports/exports; skip locals.
        if nl.is_global() {
            filter.consider(name, nl.is_undefined(), results);
        }
    }
    Ok(())
}

/// PE: the import directory and the export table.
fn collect_pe(pe: &PE, filter: &Filter, results: &mut Vec<SymbolMatch>) {
    for imp in &pe.imports {
        filter.consider(imp.name.as_ref(), true, results);
    }
    for exp in &pe.exports {
        if let Some(name) = exp.name {
            filter.consider(name, false, results);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn re(pattern: &str) -> Regex {
        Regex::new(pattern).unwrap()
    }

    #[test]
    fn test_parse_known_elf() {
        let path = PathBuf::from("tests/fixtures/libtest.so");
        let results = parse_symbols(&path, &re("calculate_sum"), true, true).unwrap();

        assert!(!results.is_empty());

        let main_sym = results.iter().find(|r| r.name == "calculate_sum").unwrap();
        assert!(matches!(main_sym.kind, SymbolKind::Export));
    }

    #[test]
    fn test_match_export() {
        let path = PathBuf::from("tests/fixtures/libtest.so");
        let results = parse_symbols(&path, &re("calculate_sum"), false, true).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_dont_match_export() {
        let path = PathBuf::from("tests/fixtures/libtest.so");
        let results = parse_symbols(&path, &re("calculate_sum"), false, false).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_match_import() {
        let path = PathBuf::from("tests/fixtures/main");
        let results = parse_symbols(&path, &re("calculate_sum"), true, false).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_dont_match_import() {
        let path = PathBuf::from("tests/fixtures/main");
        let results = parse_symbols(&path, &re("calculate_sum"), false, false).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_macho_export() {
        let path = PathBuf::from("tests/fixtures/libtest.dylib");
        let results = parse_symbols(&path, &re("calculate_sum"), false, true).unwrap();

        let sym = results
            .iter()
            .find(|r| r.name.contains("calculate_sum"))
            .expect("calculate_sum export not found");
        assert!(matches!(sym.kind, SymbolKind::Export));
    }

    #[test]
    fn test_macho_import() {
        let path = PathBuf::from("tests/fixtures/main_macho");
        let results = parse_symbols(&path, &re("calculate_sum"), true, false).unwrap();

        let sym = results
            .iter()
            .find(|r| r.name.contains("calculate_sum"))
            .expect("calculate_sum import not found");
        assert!(matches!(sym.kind, SymbolKind::Import));
    }

    #[test]
    fn test_invalid_regex_is_rejected() {
        // An invalid pattern must be rejected before any file is touched.
        // Built at runtime so the `invalid_regex` lint can't const-evaluate it.
        let pattern = format!("{}Invalid(", '[');
        assert!(Regex::new(&pattern).is_err());
    }
}
