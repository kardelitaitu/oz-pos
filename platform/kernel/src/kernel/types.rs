//! Kernel lifecycle status types.

/// Represents the runtime status of a registered module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    /// Module is registered but not yet loaded.
    Registered,
    /// Module's `on_load` has been called.
    Loaded,
    /// Module's `on_start` has been called.
    Started,
    /// Module has been stopped (after `on_stop`). Can be restarted.
    Stopped,
}
