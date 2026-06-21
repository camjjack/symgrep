# symgrep

`grep` for symbols. A fast, cross-platform tool to search imported and exported symbols in ELF, Mach-O and PE binaries.

[![CI](https://github.com/camjjack/symgrep/actions/workflows/ci.yml/badge.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)]()
[![Crates.io](https://img.shields.io/crates/v/symgrep.svg)]()

## Features

*   **Blazing Fast:** Parallelized search utilizing all CPU cores via Rayon.
*   **Multi-Format:** Searches **ELF**, **Mach-O** and **PE** binaries; the format is auto-detected per file.
*   **Smart Filtering:** Differentiates between **Imported** and **Exported** symbols. Reports dynamic/external symbols (ELF `.dynsym`, the Mach-O symbol table, PE import/export tables) rather than every static symbol.
*   **Regex Support:** Powerful pattern matching for symbol names.
*   **Cross-Platform:** Pre-built binaries for Linux (x86_64/ARM64), macOS (Apple Silicon), and Windows.

## Installation

Installation via cargo or download pre-built release binaries.
### Via Cargo
```bash
cargo install symgrep
```

### Release binaries

Available at https://github.com/camjjack/symgrep/releases/latest

## Exit codes

Like `grep`, symgrep reports its result through the exit code:

| Code | Meaning                          |
| ---- | -------------------------------- |
| 0    | At least one symbol matched      |
| 1    | No symbols matched               |
| 2    | An error occurred (e.g. invalid regex, unreadable file) |
