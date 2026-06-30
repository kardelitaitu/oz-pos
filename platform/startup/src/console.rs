//! tokio-console integration.
//!
//! Enable with `RUSTFLAGS="--cfg tokio_unstable"` and the `console`
//! feature flag to visualise async task graphs, resource utilisation,
//! and polling durations in the `tokio-console` dashboard.

/// Initialise the tokio-console subscriber.
///
/// Should be called at the very start of `main()`, before any other
/// tracing setup, so that console instrumentation captures all tasks.
///
/// # Panics
///
/// Panics if called more than once (the underlying subscriber
/// registration is a `OnceCell`).
#[cfg(feature = "console")]
pub fn init_console_subscriber() {
    console_subscriber::init();
}

/// No-op when console feature is disabled.
#[cfg(not(feature = "console"))]
pub fn init_console_subscriber() {
    tracing::debug!(
        "tokio-console disabled (compile with `console` feature + RUSTFLAGS=\"--cfg tokio_unstable\")"
    );
}
