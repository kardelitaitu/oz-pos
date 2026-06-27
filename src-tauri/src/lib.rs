//! Tauri v2 application entry point.
//!
//! Wires the [`AppState`] (DB connection, driver registry, config) into the
//! Tauri builder, registers all `#[tauri::command]` handlers, and starts the
//! runtime. Mobile builds use the same code via `#[cfg_attr(mobile,
//! tauri::mobile_entry_point)]`.
//!
//! Adding a new command:
//! 1. Define `pub async fn` with `#[tauri::command]` in `commands/<feature>.rs`.
//! 2. Add it to the `invoke_handler!` macro below in the same order as the
//!    `commands` module re-exports.
//! 3. Document the command in the `tauri-ipc` skill.

pub mod commands;
pub mod error;
pub mod state;

use crate::error::AppError;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise structured logging early so the very first line of Tauri
    // output is captured.
    oz_logging::init();

    let result: Result<(), AppError> = tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new(&app.handle())
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::ping,
            commands::health::version,
            commands::sales::start_sale,
            commands::sales::add_line,
            commands::sales::complete_sale,
            commands::hardware::open_cash_drawer,
            commands::hardware::print_receipt,
        ])
        .run(tauri::generate_context!())
        .map_err(AppError::from);

    if let Err(e) = result {
        tracing::error!(error = %e, "OZ-POS exited with error");
        std::process::exit(1);
    }
}
