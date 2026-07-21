// Register tokio_unstable as a known cfg condition.
// This cfg is set at compile time via RUSTFLAGS="--cfg tokio_unstable"
// and is required by console-subscriber (tokio-console).
// Without this registration, clippy flags #[cfg(tokio_unstable)] as
// an unexpected cfg condition.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(tokio_unstable)");
}
