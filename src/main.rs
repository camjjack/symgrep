use clap::Parser;
use goblin::Hint;
use ignore::WalkBuilder;
use log::info;
use rayon::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

mod engine;
use engine::{SymbolKind, parse_symbols};

#[derive(Parser, Debug)]
#[command(name = "symgrep")]
#[command(about = "Grep for symbols in ELF, Mach-O and PE binaries", long_about = None)]
struct Args {
    /// Limit results to exported symbols only
    #[arg(short, long, default_value_t = false, conflicts_with = "imports_only")]
    exports_only: bool,

    /// Limit results to imported symbols only
    #[arg(short, long, default_value_t = false)]
    imports_only: bool,

    /// The regex pattern to search for in symbol names
    pattern: String,

    /// The root path to search in (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,
}

/// Cheap pre-filter: peek the first 16 bytes and keep only files whose magic
/// identifies a format we extract symbols from. This rejects the bulk of a tree
/// (source, data, etc.) without mmapping or fully parsing, and uses goblin's own
/// detection so look-alike files (e.g. Java `.class`, which shares Mach-O fat
/// magic) are not mistaken for binaries.
fn is_supported_binary(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path)
        && let Ok(hint) = goblin::peek(&mut file)
    {
        return matches!(
            hint,
            Hint::Elf(_) | Hint::Mach(_) | Hint::MachFat(_) | Hint::PE
        );
    }
    false
}

fn main() -> ExitCode {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let args = Args::parse();

    // Compile the regex once, up front, so an invalid pattern fails fast with a
    // single clear error instead of once per file inside the parallel scan.
    let re = match regex::Regex::new(&args.pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("Invalid regex '{}': {}", args.pattern, e);
            return ExitCode::from(2);
        }
    };

    let mut binaries = Vec::new();

    let walker = WalkBuilder::new(&args.path)
        .git_ignore(true)
        .hidden(false)
        .build();

    info!("Scanning for binaries...");
    for entry in walker.flatten() {
        if let Some(ft) = entry.file_type()
            && ft.is_file()
            && is_supported_binary(entry.path())
        {
            binaries.push(entry.path().to_path_buf());
        }
    }

    info!(
        "Found {} binaries. Parsing with {} threads...",
        binaries.len(),
        rayon::current_num_threads()
    );
    // Track results across threads so we can return a grep-style exit code:
    // 0 if any symbol matched, 1 if none matched, 2 if any file errored.
    let found = AtomicBool::new(false);
    let errored = AtomicBool::new(false);

    binaries.into_par_iter().for_each(|path| {
        match parse_symbols(&path, &re, !args.exports_only, !args.imports_only) {
            Ok(matches) => {
                if !matches.is_empty() {
                    found.store(true, Ordering::Relaxed);
                    // Build the whole block and emit it with a single write so
                    // output from concurrent threads cannot interleave.
                    let mut out = String::new();
                    out.push_str(&format!("{}\n", path.display()));
                    for m in matches {
                        let kind_str = match m.kind {
                            SymbolKind::Import => "IMPORT",
                            SymbolKind::Export => "EXPORT",
                        };
                        out.push_str(&format!("  {} [{}]\n", m.name, kind_str));
                    }
                    print!("{out}");
                }
            }
            Err(e) => {
                errored.store(true, Ordering::Relaxed);
                eprintln!("Error parsing {:?}: {}", path, e);
            }
        }
    });

    if errored.into_inner() {
        ExitCode::from(2)
    } else if found.into_inner() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
