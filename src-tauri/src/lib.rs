mod application;
mod domain;
mod infrastructure;
mod tauri_api;

use application::service::AppService;
use infrastructure::{sqlite_repo::SqliteRepository, watcher::WatcherManager};
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::Manager;
use tauri_api::commands::*;

pub struct AppState {
    pub service: Arc<AppService>,
    pub watcher: Mutex<WatcherManager>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let repo = SqliteRepository::new(app_data.join("organizer.sqlite3"));
            let service = Arc::new(AppService::new(repo).map_err(|e| std::io::Error::other(e.to_string()))?);
            let mut watcher = WatcherManager::new();
            if let Some(dir) = service.mods_dir() {
                if dir.is_dir() {
                    let _ = watcher.watch(&dir, Arc::clone(&service), app.handle().clone());
                }
            }
            app.manage(AppState { service, watcher: Mutex::new(watcher) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            rescan,
            set_mods_directory,
            create_category,
            delete_category,
            set_category_color,
            set_category_icon,
            move_category,
            move_mod,
            set_mod_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
