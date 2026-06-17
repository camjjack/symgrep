use goblin::elf::Elf;
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

/// Parses an ELF file at the given path and extracts symbols matching the regex.
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

    let elf =
        Elf::parse(&mmap).map_err(|e| format!("Failed to parse ELF {}: {}", path.display(), e))?;

    let mut results = Vec::new();

    // Iterate over Dynamic Symbols (.dynsym)
    for sym in elf.dynsyms.iter() {
        if let Some(name) = elf.dynstrtab.get_at(sym.st_name)
            && re.is_match(name)
        {
            let is_import = sym.is_import();
            if (is_import && !include_imports) || (!is_import && !include_exports) {
                continue;
            }

            let kind = if is_import {
                SymbolKind::Import
            } else {
                SymbolKind::Export
            };

            results.push(SymbolMatch {
                name: name.to_string(),
                kind,
            });
        }
    }

    Ok(results)
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
    fn test_invalid_regex_is_rejected() {
        // An invalid pattern must be rejected before any file is touched.
        // Built at runtime so the `invalid_regex` lint can't const-evaluate it.
        let pattern = format!("{}Invalid(", '[');
        assert!(Regex::new(&pattern).is_err());
    }
}
