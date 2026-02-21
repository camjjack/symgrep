use clap::Parser;
use ignore::WalkBuilder;
use log::info;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

mod engine;
use engine::{SymbolKind, parse_symbols};

#[derive(Parser, Debug)]
#[command(name = "symgrep")]
#[command(about = "Grep for symbols in ELF binaries", long_about = None)]
struct Args {
    /// Limit results to exported symbols only
    #[arg(short, long, default_value_t = false)]
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

fn is_elf_file(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        let mut header = [0u8; 4];
        // read_exact is efficient as it stops after the first 4 bytes
        if file.read_exact(&mut header).is_ok() {
            return header == [0x7f, b'E', b'L', b'F'];
        }
    }
    false
}

fn main() -> std::io::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let args = Args::parse();

    let mut elf_files = Vec::new();

    let walker = WalkBuilder::new(&args.path)
        .git_ignore(true)
        .hidden(false)
        .build();

    info!("Scanning for ELF files...");
    for entry in walker.flatten() {
        if let Some(ft) = entry.file_type()
            && ft.is_file()
            && is_elf_file(entry.path())
        {
            elf_files.push(entry.path().to_path_buf());
        }
    }

    info!(
        "Found {} ELF files. Parsing with {} threads...",
        elf_files.len(),
        rayon::current_num_threads()
    );
    elf_files.into_par_iter().for_each(|path| {
        match parse_symbols(
            &path,
            &args.pattern,
            !&args.exports_only,
            !&args.imports_only,
        ) {
            Ok(matches) => {
                if !matches.is_empty() {
                    println!("{}", path.display());
                    for m in matches {
                        let kind_str = match m.kind {
                            SymbolKind::Import => "IMPORT",
                            SymbolKind::Export => "EXPORT",
                        };
                        println!("  {} [{}]", m.name, kind_str);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error parsing {:?}: {}", path, e);
            }
        }
    });

    Ok(())
}
