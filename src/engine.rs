use goblin::elf::Elf;
use memmap2::Mmap;
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
/// * `pattern` - The regex pattern string to match symbol names against.
pub fn parse_symbols(
    path: &Path,
    pattern: &str,
    include_imports: bool,
    include_exports: bool,
) -> Result<Vec<SymbolMatch>, String> {
    let file =
        File::open(path).map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;

    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|e| format!("Failed to mmap file {}: {}", path.display(), e))?;

    let elf =
        Elf::parse(&mmap).map_err(|e| format!("Failed to parse ELF {}: {}", path.display(), e))?;

    let re =
        regex::Regex::new(pattern).map_err(|e| format!("Invalid regex '{}': {}", pattern, e))?;

    let mut results = Vec::new();

    // Iterate over Dynamic Symbols (.dynsym)
    for sym in elf.dynsyms.iter() {
        if let Some(name) = elf.dynstrtab.get_at(sym.st_name)
            && re.is_match(name)
        {
            let kind = if sym.is_import() {
                SymbolKind::Import
            } else {
                SymbolKind::Export
            };
            match kind {
                SymbolKind::Import => {
                    if !include_imports {
                        continue;
                    }
                }
                SymbolKind::Export => {
                    if !include_exports {
                        continue;
                    }
                }
            }

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

    #[test]
    fn test_parse_known_elf() {
        let path = PathBuf::from("tests/fixtures/libtest.so");
        let results = parse_symbols(&path, "calculate_sum", true, true).unwrap();

        assert!(!results.is_empty());

        let main_sym = results.iter().find(|r| r.name == "calculate_sum").unwrap();
        assert!(matches!(main_sym.kind, SymbolKind::Export));
    }

    #[test]
    fn test_match_export() {
        let path = PathBuf::from("tests/fixtures/libtest.so");
        let results = parse_symbols(&path, "calculate_sum", false, true).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_dont_match_export() {
        let path = PathBuf::from("tests/fixtures/libtest.so");
        let results = parse_symbols(&path, "calculate_sum", false, false).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_match_import() {
        let path = PathBuf::from("tests/fixtures/main");
        let results = parse_symbols(&path, "calculate_sum", true, false).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_dont_match_import() {
        let path = PathBuf::from("tests/fixtures/main");
        let results = parse_symbols(&path, "calculate_sum", false, false).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_invalid_regex() {
        let path = PathBuf::from("/dev/null");
        let result = parse_symbols(&path, "[Invalid(", true, true);
        assert!(result.is_err());
    }
}
