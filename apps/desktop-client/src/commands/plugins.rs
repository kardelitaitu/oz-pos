use tauri::{Manager, State, command};

use crate::error::AppError;
use crate::state::AppState;

#[command]
pub async fn reload_plugins(state: State<'_, AppState>) -> Result<(), AppError> {
    let plugins_dir = state
        .app
        .as_ref()
        .and_then(|app| app.path().app_data_dir().ok())
        .map(|d| d.join("plugins"));

    match plugins_dir {
        Some(dir) if dir.exists() => {
            let mut guard = state.plugins.lock().await;
            let new_pm = oz_plugin::PluginManager::new(&dir)
                .map_err(|e| AppError::Internal(format!("plugin reload failed: {e}")))?;
            *guard = Some(new_pm);
            tracing::info!("plugins manually reloaded");
            Ok(())
        }
        Some(_) => Err(AppError::Invalid("plugins directory not found".into())),
        None => Err(AppError::Invalid("app data dir not available".into())),
    }
}
