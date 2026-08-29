//! Tests for the kitchen module stub lifecycle.

use super::*;
use platform_kernel::Kernel;

/// Minimal stand-in for a dependency the kernel must resolve.
#[derive(Debug)]
struct StubModule(&'static str);

impl Module for StubModule {
    fn id(&self) -> ModuleId {
        self.0
    }
}

fn kernel_with_deps() -> Kernel {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(StubModule("sales")))
        .expect("register sales stub");
    kernel
        .register(Box::new(StubModule("terminal")))
        .expect("register terminal stub");
    kernel
}

#[test]
fn module_id_matches_manifest() {
    assert_eq!(KitchenModule::new().id(), "kitchen");
    assert_eq!(MODULE_ID, "kitchen");
}

#[test]
fn declares_sales_and_terminal_dependencies() {
    assert_eq!(
        KitchenModule::new().dependencies(),
        &["sales", "terminal"][..]
    );
}

#[test]
fn manifest_json_matches_module_declaration() {
    let manifest = include_str!("../manifest.json");
    let parsed: serde_json::Value =
        serde_json::from_str(manifest).expect("manifest.json must be valid JSON");
    assert_eq!(parsed["id"], "kitchen");

    let declared: Vec<&str> = parsed["dependencies"]
        .as_array()
        .expect("dependencies must be an array")
        .iter()
        .map(|v| v.as_str().expect("dependency must be a string"))
        .collect();
    assert_eq!(declared, KitchenModule::new().dependencies().to_vec());
}

#[test]
fn full_lifecycle_through_kernel() {
    let mut kernel = kernel_with_deps();
    kernel
        .register(Box::new(KitchenModule::new()))
        .expect("register kitchen");
    assert!(kernel.is_registered("kitchen"));

    kernel.load_all().expect("load_all");
    kernel.start_all().expect("start_all");
    kernel.stop_all().expect("stop_all");

    assert!(kernel.is_registered("kitchen"));
}

#[test]
fn partial_dependencies_are_rejected() {
    // Only `sales` is registered; `terminal` is missing.
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(StubModule("sales")))
        .expect("register sales stub");
    kernel
        .register(Box::new(KitchenModule::new()))
        .expect("register kitchen");
    assert!(kernel.load_all().is_err());
}

#[test]
fn duplicate_registration_fails() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(KitchenModule::new()))
        .expect("first registration");
    assert!(kernel.register(Box::new(KitchenModule::new())).is_err());
}

#[test]
fn lifecycle_hooks_are_individually_ok() {
    let mut module = KitchenModule::new();
    assert!(module.on_load().is_ok());
    assert!(module.on_start().is_ok());
    assert!(module.on_stop().is_ok());
}

#[test]
fn validation_error_carries_field_and_message() {
    let err = KitchenError::validation("station_id", "unknown station");
    assert!(err.to_string().contains("station_id"));
    assert!(err.to_string().contains("unknown station"));
}
