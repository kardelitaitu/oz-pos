//! Tests for the gift-cards module stub lifecycle.

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
    assert_eq!(GiftCardsModule::new().id(), "giftcards");
    assert_eq!(MODULE_ID, "giftcards");
}

#[test]
fn declares_sales_dependency() {
    assert_eq!(GiftCardsModule::new().dependencies(), &["sales"]);
}

#[test]
fn manifest_json_matches_module_declaration() {
    let manifest = include_str!("../manifest.json");
    let parsed: serde_json::Value =
        serde_json::from_str(manifest).expect("manifest.json must be valid JSON");
    assert_eq!(parsed["id"], "giftcards");

    let declared: Vec<&str> = parsed["dependencies"]
        .as_array()
        .expect("dependencies must be an array")
        .iter()
        .map(|v| v.as_str().expect("dependency must be a string"))
        .collect();
    assert_eq!(declared, GiftCardsModule::new().dependencies().to_vec());
}

#[test]
fn full_lifecycle_through_kernel() {
    let mut kernel = kernel_with_deps();
    kernel
        .register(Box::new(GiftCardsModule::new()))
        .expect("register giftcards");
    assert!(kernel.is_registered("giftcards"));

    kernel.load_all().expect("load_all");
    kernel.start_all().expect("start_all");
    kernel.stop_all().expect("stop_all");

    assert!(kernel.is_registered("giftcards"));
}

#[test]
fn missing_dependency_is_rejected() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(GiftCardsModule::new()))
        .expect("register giftcards");
    assert!(kernel.load_all().is_err());
}

#[test]
fn duplicate_registration_fails() {
    let mut kernel = Kernel::new();
    kernel
        .register(Box::new(GiftCardsModule::new()))
        .expect("first registration");
    assert!(kernel.register(Box::new(GiftCardsModule::new())).is_err());
}

#[test]
fn lifecycle_hooks_are_individually_ok() {
    let mut module = GiftCardsModule::new();
    assert!(module.on_load().is_ok());
    assert!(module.on_start().is_ok());
    assert!(module.on_stop().is_ok());
}

#[test]
fn validation_error_carries_field_and_message() {
    let err = GiftCardsError::validation("balance_minor", "must not be negative");
    assert!(err.to_string().contains("balance_minor"));
    assert!(err.to_string().contains("negative"));
}
