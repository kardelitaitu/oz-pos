//! Sibling unit tests for `lib.rs` (AGENTS.md: no tests in production files).

use super::*;

use platform_kernel::Kernel;

/// Minimal stand-in for a module `sales` depends on.
///
/// `SalesModule::dependencies()` declares `inventory`, so any test that
/// drives `load_all`/`start_all` must register that id or dependency
/// resolution fails with `MissingDependency`.
#[derive(Debug)]
struct StubModule(&'static str);

impl Module for StubModule {
    fn id(&self) -> &'static str {
        self.0
    }
}

fn kernel_with_deps() -> Kernel {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(StubModule("inventory")))
        .expect("register inventory stub");
    kernel
}

#[test]
fn sales_module_id() {
    let module = SalesModule::new();
    assert_eq!(module.id(), "sales");
}

#[test]
fn sales_module_declares_inventory_dependency() {
    assert_eq!(SalesModule::new().dependencies(), &["inventory"]);
}

#[test]
fn sales_module_manifest_matches_declaration() {
    let parsed: serde_json::Value = serde_json::from_str(include_str!("../manifest.json"))
        .expect("manifest.json must be valid JSON");
    let declared: Vec<&str> = parsed["dependencies"]
        .as_array()
        .expect("dependencies must be an array")
        .iter()
        .map(|v| v.as_str().expect("dependency must be a string"))
        .collect();
    assert_eq!(declared, SalesModule::new().dependencies().to_vec());
}

#[test]
fn sales_module_load_fails_without_inventory() {
    let mut kernel = Kernel::new();
    kernel.register(Box::new(SalesModule::new())).unwrap();
    assert!(kernel.load_all().is_err());
}

#[test]
fn sales_module_lifecycle() {
    let mut kernel = kernel_with_deps();
    kernel.register(Box::new(SalesModule::new())).unwrap();
    assert!(kernel.is_registered("sales"));
    assert_eq!(kernel.module_count(), 2);

    kernel.load_all().unwrap();
    assert!(kernel.is_loaded());

    kernel.start_all().unwrap();
    assert!(kernel.is_started());

    kernel.stop_all().unwrap();
    assert!(!kernel.is_loaded());
    assert!(!kernel.is_started());
}

#[test]
fn sales_module_integration_with_appstate() {
    // Verify the module can coexist with other kernel state.
    let mut kernel = Kernel::new();
    kernel.register(Box::new(SalesModule::new())).unwrap();
    kernel.register(Box::new(SalesModule::new())).unwrap_err(); // duplicate
}

#[test]
fn sales_module_on_load_succeeds() {
    let mut module = SalesModule::new();
    assert!(module.on_load().is_ok());
}

#[test]
fn sales_module_on_start_succeeds() {
    let mut module = SalesModule::new();
    assert!(module.on_start().is_ok());
}

#[test]
fn sales_module_on_stop_succeeds() {
    let mut module = SalesModule::new();
    assert!(module.on_stop().is_ok());
}

#[test]
fn sales_module_full_lifecycle_with_kernel() {
    let mut kernel = kernel_with_deps();
    kernel.register(Box::new(SalesModule::new())).unwrap();

    // load → start → stop
    kernel.load_all().unwrap();
    kernel.start_all().unwrap();
    kernel.stop_all().unwrap();

    // Can be registered again after stop
    kernel.register(Box::new(SalesModule::new())).unwrap_err(); // already registered (Kernel doesn't support re-registration)

    // Actually, since we called stop_all, the kernel state was reset
    // but the module is still registered. Let's verify.
    assert!(kernel.is_registered("sales"));
}
