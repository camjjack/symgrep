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
