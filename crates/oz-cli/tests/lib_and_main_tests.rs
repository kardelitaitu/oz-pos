//! Integration tests for oz-cli library exports — compile-time checks that
//! key types, modules, and re-exports resolve without errors.

#[test]
fn test_core_types_accessible() {
    // oz_cli re-exports Cli and CliError at the crate root.
    let _cli: oz_cli::Cli;
    let _err: oz_cli::CliError;
}

#[test]
fn test_modules_compile() {
    // Verify the module tree is reachable.
    let _cli: oz_cli::cli::Cli;
    let _run: fn() -> anyhow::Result<()> = oz_cli::commands::run;
    let _err: oz_cli::error::CliError;
}
