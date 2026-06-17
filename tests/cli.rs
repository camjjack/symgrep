use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::Path;

#[test]
fn test_cli_runs() {
    cargo_bin_cmd!("symgrep").arg("--help").assert().success();
}

#[test]
fn test_hello_world_scan() {
    let elf_path = Path::new("tests/fixtures/main");

    cargo_bin_cmd!("symgrep")
        .args(["calculate_sum", elf_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("calculate_sum"))
        .stdout(predicate::str::contains("[IMPORT]"));
}

#[test]
fn test_exports_only_filter() {
    // calculate_sum is exported by libtest.so and imported by main; with
    // --exports-only only the export should appear.
    cargo_bin_cmd!("symgrep")
        .args(["--exports-only", "calculate_sum", "tests/fixtures"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[EXPORT]"))
        .stdout(predicate::str::contains("[IMPORT]").not());
}

#[test]
fn test_imports_only_filter() {
    cargo_bin_cmd!("symgrep")
        .args(["--imports-only", "calculate_sum", "tests/fixtures"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[IMPORT]"))
        .stdout(predicate::str::contains("[EXPORT]").not());
}

#[test]
fn test_conflicting_filters_rejected() {
    cargo_bin_cmd!("symgrep")
        .args(["-e", "-i", "calculate_sum", "tests/fixtures"])
        .assert()
        .failure();
}

#[test]
fn test_directory_recursion() {
    // Scanning the fixtures directory should find both ELF files.
    cargo_bin_cmd!("symgrep")
        .args(["calculate_sum", "tests/fixtures"])
        .assert()
        .success()
        .stdout(predicate::str::contains("libtest.so"))
        .stdout(predicate::str::contains("main"));
}
