//! Module system kernel — lifecycle management and dependency resolution.
//!
//! The [`Kernel`] is the sole owner of the module lifecycle. It maintains
//! a registry of modules, resolves dependencies via topological sort,
//! and drives the lifecycle: **register → load → start → stop**.

pub mod dependency;
pub mod lifecycle;
#[cfg(test)]
mod split_tests;
#[cfg(test)]
mod tests;
pub mod types;

pub use dependency::HasDependencies;
pub use lifecycle::Kernel;
pub use types::ModuleStatus;
