// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Desktop shell binary entry point.
//!
//! Delegates immediately to `oz_pos_app_lib::run`, which owns Tauri setup,
//! `AppState` construction, and command registration. Keeping this file at
//! one call means the desktop and tablet binaries share the same shape.

fn main() {
    oz_pos_app_lib::run();
}
