#![warn(missing_docs)]

//! Platform Kernel — module system lifecycle, dependency resolution, event bus.
//!
//! This crate provides the [`Kernel`] struct that manages the module
//! lifecycle (register → load → start → stop) and resolves inter-module
//! dependencies via topological sort, along with the [`EventBus`] for
//! decoupled module-to-module communication.
//!
//! ## Contents
//!
//! - [`kernel`] — [`Kernel`] struct with registration, lifecycle, dependency resolution, and event bus
//! - [`event_bus`] — [`EventBus`] struct for topic-based event dispatch
//! - [`manifest`] — [`ModuleManifest`] struct with JSON parsing and validation
//! - [`error`] — [`KernelError`] enum
//!
//! ## Example
//! ```no_run
//! use platform_kernel::{Kernel, EventBus};
//! use foundation::contracts::Module;
//!
//! let mut kernel = Kernel::new();
//! kernel.register(Box::new(MyModule))?;
//! kernel.load_all()?;
//! kernel.start_all()?;
//! // ... application runs ...
//! kernel.stop_all()?;
//! ```

pub mod error;
pub mod event_bus;
pub mod kernel;
pub mod manifest;

pub use error::KernelError;
pub use event_bus::EventBus;
pub use kernel::{Kernel, ModuleStatus};
pub use manifest::ModuleManifest;
