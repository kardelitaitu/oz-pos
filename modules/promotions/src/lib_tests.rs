//! Tests for the promotions module stub lifecycle.

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
}

#[test]
fn module_id_matches_manifest() {
    assert_eq!(PromotionsModule::new().id(), "promotions");
    assert_eq!(MODULE_ID, "promotions");
}

#[test]
fn declares_sales_dependency() {
    assert_eq!(PromotionsModule::new().dependencies(), &["sales"]);
}

#[test]
fn manifest_json_matches_module_declaration() {
    let manifest = include_str!("../manifest.json");
    let parsed: serde_json::Value =
        serde_json::from_str(manifest).expect("manifest.json must be valid JSON");
    assert_eq!(parsed["id"], "promotions");

    let declared: Vec<&str> = parsed["dependencies"]
        .as_array()
        .expect("dependencies must be an array")
        .iter()
        .map(|v| v.as_str().expect("dependency must be a string"))
        .collect();
    assert_eq!(declared, PromotionsModule::new().dependencies().to_vec());
}

#[test]
fn full_lifecycle_through_kernel() {
    let mut kernel = kernel_with_deps();
    kernel
        .register(Box::new(PromotionsModule::new()))
        .expect("register promotions");
    assert!(kernel.is_registered("promotions"));

    kernel.load_all().expect("load_all");
    kernel.start_all().expect("start_all");
    kernel.stop_all().expect("stop_all");

    assert!(kernel.is_registered("promotions"));
}

#[test]
fn missing_dependency_is_rejected() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(PromotionsModule::new()))
        .expect("register promotions");
    assert!(kernel.load_all().is_err());
}

#[test]
fn duplicate_registration_fails() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(PromotionsModule::new()))
        .expect("first registration");
    assert!(kernel.register(Box::new(PromotionsModule::new())).is_err());
}

#[test]
fn lifecycle_hooks_are_individually_ok() {
    let mut module = PromotionsModule::new();
    assert!(module.on_load().is_ok());
    assert!(module.on_start().is_ok());
    assert!(module.on_stop().is_ok());
}

#[test]
fn validation_error_carries_field_and_message() {
    let err = PromotionsError::validation("discount_bps", "must be between 0 and 10000");
    assert!(err.to_string().contains("discount_bps"));
    assert!(err.to_string().contains("10000"));
}
