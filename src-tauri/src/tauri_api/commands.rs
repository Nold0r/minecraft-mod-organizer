use crate::{application::service::{AppService, WorkspaceSnapshot}, infrastructure::watcher::WatcherManager, AppState};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Result<WorkspaceSnapshot, String> {
    state.service.snapshot().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rescan(state: State<'_, AppState>) -> Result<WorkspaceSnapshot, String> {
    state.service.rescan_snapshot().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mods_directory(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<WorkspaceSnapshot, String> {
    let path = PathBuf::from(path);
    let snapshot = state.service.set_mods_dir(path.clone()).map_err(|e| e.to_string())?;
    state.watcher.lock().watch(&path, Arc::clone(&state.service), app).map_err(|e| e.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
pub fn create_category(state: State<'_, AppState>, name: String, parent_id: Option<String>) -> Result<WorkspaceSnapshot, String> {
    state.service.create_category(name, parent_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: String) -> Result<WorkspaceSnapshot, String> {
    state.service.delete_category(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_category_color(state: State<'_, AppState>, id: String, color: Option<String>) -> Result<WorkspaceSnapshot, String> {
    state.service.set_category_color(&id, color).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_category_icon(state: State<'_, AppState>, id: String, icon_hex: Option<String>) -> Result<WorkspaceSnapshot, String> {
    state.service.set_category_icon(&id, icon_hex).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_category(state: State<'_, AppState>, id: String, parent_id: Option<String>, index: usize) -> Result<WorkspaceSnapshot, String> {
    state.service.move_category(&id, parent_id, index).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_mod(state: State<'_, AppState>, id: String, category_id: Option<String>, index: usize) -> Result<WorkspaceSnapshot, String> {
    state.service.move_mod(&id, category_id, index).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mod_enabled(state: State<'_, AppState>, id: String, enabled: bool) -> Result<WorkspaceSnapshot, String> {
    state.service.set_mod_enabled(&id, enabled).map_err(|e| e.to_string())
}
