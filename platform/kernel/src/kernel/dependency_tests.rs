//! Tests for dependency collection from the `Module` trait.

use super::*;
use foundation::contracts::{ModuleId, ModuleResult};

#[derive(Debug)]
struct Leaf;

impl Module for Leaf {
    fn id(&self) -> ModuleId {
        "leaf"
    }
}

#[derive(Debug)]
struct Dependent;

impl Module for Dependent {
    fn id(&self) -> ModuleId {
        "dependent"
    }

    fn dependencies(&self) -> &'static [ModuleId] {
        &["leaf", "other"]
    }

    fn on_load(&mut self) -> ModuleResult {
        Ok(())
    }
}

#[test]
fn module_without_declaration_has_no_dependencies() {
    assert!(collect_dependencies(&Leaf).is_empty());
}

#[test]
fn declared_dependencies_are_collected_in_order() {
    assert_eq!(collect_dependencies(&Dependent), vec!["leaf", "other"]);
}

#[test]
fn collect_dependencies_works_through_trait_object() {
    let boxed: Box<dyn Module> = Box::new(Dependent);
    assert_eq!(collect_dependencies(boxed.as_ref()), vec!["leaf", "other"]);
}

#[test]
fn has_dependencies_trait_still_usable_for_non_module_types() {
    struct Standalone;
    impl HasDependencies for Standalone {
        fn dependencies(&self) -> Vec<&'static str> {
            vec!["inventory"]
        }
    }
    assert_eq!(Standalone.dependencies(), vec!["inventory"]);
}
