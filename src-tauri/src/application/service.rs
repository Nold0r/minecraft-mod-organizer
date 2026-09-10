use crate::domain::{category::Category, mod_artifact::{ModArtifact, ModEntryDto, PackagedModDto}};
use crate::infrastructure::{jar_scanner::JarScanner, sqlite_repo::SqliteRepository};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Папка не существует: {0}")]
    MissingDirectory(String),
    #[error("Категория не найдена: {0}")]
    MissingCategory(String),
    #[error("Нельзя вложить категорию саму в себя или в её потомка")]
    CategoryCycle,
    #[error("Мод не найден: {0}")]
    MissingMod(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Scan(#[from] crate::infrastructure::jar_scanner::ScanError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub mods_dir: Option<String>,
    pub categories: Vec<Category>,
    pub mods: Vec<ModEntryDto>,
}

#[derive(Clone)]
pub struct AppService {
    repo: Arc<SqliteRepository>,
    mods_dir: Arc<RwLock<Option<PathBuf>>>,
    artifacts: Arc<RwLock<Vec<ModArtifact>>>,
}

impl AppService {
    pub fn new(repo: SqliteRepository) -> Result<Self, AppError> {
        repo.init()?;
        let mods_dir = repo.get_setting("mods_dir")?.map(PathBuf::from);
        let artifacts = mods_dir
            .as_deref()
            .filter(|dir| dir.is_dir())
            .map(JarScanner::scan_dir)
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            repo: Arc::new(repo),
            mods_dir: Arc::new(RwLock::new(mods_dir)),
            artifacts: Arc::new(RwLock::new(artifacts)),
        })
    }

    pub fn mods_dir(&self) -> Option<PathBuf> {
        self.mods_dir.read().clone()
    }

    pub fn set_mods_dir(&self, path: PathBuf) -> Result<WorkspaceSnapshot, AppError> {
        if !path.is_dir() {
            return Err(AppError::MissingDirectory(path.display().to_string()));
        }
        self.repo.set_setting("mods_dir", &path.display().to_string())?;
        *self.mods_dir.write() = Some(path);
        self.rescan_snapshot()
    }

    pub fn create_category(&self, name: String, parent_id: Option<String>) -> Result<WorkspaceSnapshot, AppError> {
        if let Some(parent) = &parent_id {
            if self.repo.get_category(parent)?.is_none() {
                return Err(AppError::MissingCategory(parent.clone()));
            }
        }
        let index = self.repo.count_category_siblings(parent_id.as_deref())?;
        self.repo.insert_category(&Category {
            id: Uuid::new_v4().to_string(),
            name,
            parent_id,
            sort_index: index,
            color: None,
            icon_hex: None,
        })?;
        self.snapshot()
    }

    pub fn delete_category(&self, id: &str) -> Result<WorkspaceSnapshot, AppError> {
        if self.repo.get_category(id)?.is_none() {
            return Err(AppError::MissingCategory(id.to_string()));
        }
        self.repo.delete_category(id)?;
        self.snapshot()
    }

    pub fn set_category_color(&self, id: &str, color: Option<String>) -> Result<WorkspaceSnapshot, AppError> {
        if self.repo.get_category(id)?.is_none() {
            return Err(AppError::MissingCategory(id.to_string()));
        }
        self.repo.set_category_color(id, color.as_deref())?;
        self.snapshot()
    }

    pub fn set_category_icon(&self, id: &str, icon_hex: Option<String>) -> Result<WorkspaceSnapshot, AppError> {
        if self.repo.get_category(id)?.is_none() {
            return Err(AppError::MissingCategory(id.to_string()));
        }
        self.repo.set_category_icon(id, icon_hex.as_deref())?;
        self.snapshot()
    }

    pub fn move_category(&self, id: &str, parent_id: Option<String>, index: usize) -> Result<WorkspaceSnapshot, AppError> {
        let categories = self.repo.list_categories()?;
        if !categories.iter().any(|c| c.id == id) {
            return Err(AppError::MissingCategory(id.to_string()));
        }
        if let Some(parent) = &parent_id {
            if parent == id || is_descendant(parent, id, &categories) {
                return Err(AppError::CategoryCycle);
            }
        }
        let moving = categories.iter().find(|c| c.id == id).expect("checked above");
        let adjusted_index = if moving.parent_id == parent_id {
            let mut siblings: Vec<&Category> = categories.iter()
                .filter(|c| c.parent_id == moving.parent_id)
                .collect();
            siblings.sort_by_key(|c| c.sort_index);
            let old_index = siblings.iter().position(|c| c.id == id).unwrap_or(0);
            if old_index < index { index.saturating_sub(1) } else { index }
        } else {
            index
        };
        self.repo.move_category(id, parent_id.as_deref(), adjusted_index)?;
        self.snapshot()
    }

    pub fn move_mod(&self, id: &str, category_id: Option<String>, index: usize) -> Result<WorkspaceSnapshot, AppError> {
        if let Some(category) = &category_id {
            if self.repo.get_category(category)?.is_none() {
                return Err(AppError::MissingCategory(category.clone()));
            }
        }

        // Use the cached snapshot: drag-and-drop must never reopen and parse JAR files.
        let snapshot = self.snapshot()?;
        let moving = snapshot.mods.iter().find(|m| m.id == id).cloned()
            .ok_or_else(|| AppError::MissingMod(id.to_string()))?;
        let old_category = moving.category_id.clone();

        let old_siblings: Vec<&ModEntryDto> = snapshot.mods.iter()
            .filter(|m| m.category_id == old_category)
            .collect();
        let old_index = old_siblings.iter().position(|m| m.id == id).unwrap_or(0);
        let adjusted_index = if old_category == category_id && old_index < index {
            index.saturating_sub(1)
        } else {
            index
        };

        let mut updates: Vec<(String, Option<String>, i64)> = Vec::new();

        if old_category == category_id {
            let mut keys: Vec<String> = old_siblings.into_iter()
                .filter(|m| m.id != id)
                .map(|m| m.layout_key.clone())
                .collect();
            let insert_at = adjusted_index.min(keys.len());
            keys.insert(insert_at, moving.layout_key.clone());
            updates.extend(keys.into_iter().enumerate().map(|(i, key)| {
                (key, category_id.clone(), i as i64)
            }));
        } else {
            let old_keys: Vec<String> = old_siblings.into_iter()
                .filter(|m| m.id != id)
                .map(|m| m.layout_key.clone())
                .collect();
            updates.extend(old_keys.into_iter().enumerate().map(|(i, key)| {
                (key, old_category.clone(), i as i64)
            }));

            let mut target_keys: Vec<String> = snapshot.mods.iter()
                .filter(|m| m.category_id == category_id && m.id != id)
                .map(|m| m.layout_key.clone())
                .collect();
            let insert_at = adjusted_index.min(target_keys.len());
            target_keys.insert(insert_at, moving.layout_key.clone());
            updates.extend(target_keys.into_iter().enumerate().map(|(i, key)| {
                (key, category_id.clone(), i as i64)
            }));
        }

        self.repo.set_mod_layouts(&updates)?;
        self.snapshot()
    }

    pub fn set_mod_enabled(&self, id: &str, enabled: bool) -> Result<WorkspaceSnapshot, AppError> {
        let snapshot = self.snapshot()?;
        let entry = snapshot.mods.iter().find(|m| m.id == id)
            .ok_or_else(|| AppError::MissingMod(id.to_string()))?;
        let current = PathBuf::from(&entry.path);
        let target = toggled_path(&current, enabled);
        if current != target {
            std::fs::rename(&current, &target)?;
            if let Some(artifact) = self.artifacts.write().iter_mut().find(|artifact| artifact.id == id) {
                artifact.path = target.clone();
                artifact.file_name = target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&artifact.file_name)
                    .to_string();
                artifact.enabled = enabled;
            }
        }
        self.snapshot()
    }

    pub fn rescan_snapshot(&self) -> Result<WorkspaceSnapshot, AppError> {
        let Some(dir) = self.mods_dir() else {
            self.artifacts.write().clear();
            return self.snapshot();
        };
        let artifacts = JarScanner::scan_dir(&dir)?;
        *self.artifacts.write() = artifacts;
        self.snapshot()
    }

    pub fn snapshot(&self) -> Result<WorkspaceSnapshot, AppError> {
        let categories = self.repo.list_categories()?;
        let Some(dir) = self.mods_dir() else {
            return Ok(WorkspaceSnapshot { mods_dir: None, categories, mods: vec![] });
        };

        // IMPORTANT: snapshot() is intentionally cheap. JAR parsing is performed only
        // by rescan_snapshot() (startup, watcher events, explicit refresh, folder change).
        // Drag-and-drop only changes SQLite layout and must never rescan every archive.
        let artifacts = self.artifacts.read().clone();
        let mut layouts = self.repo.list_mod_layouts()?;
        let mut mods = Vec::with_capacity(artifacts.len());
        for artifact in artifacts {
            let (category_id, sort_index) = layouts
                .remove(&artifact.layout_key)
                .unwrap_or((None, i64::MAX / 4));
            mods.push(ModEntryDto {
                id: artifact.id,
                layout_key: artifact.layout_key,
                mod_id: artifact.descriptor.mod_id,
                name: artifact.descriptor.name,
                version: artifact.descriptor.version,
                loader: artifact.descriptor.loader,
                enabled: artifact.enabled,
                file_name: artifact.file_name,
                path: artifact.path.display().to_string(),
                icon_data_url: artifact.icon_data_url,
                packaged_mods: artifact.packaged_mods.into_iter().map(|descriptor| PackagedModDto {
                    mod_id: descriptor.mod_id,
                    name: descriptor.name,
                    version: descriptor.version,
                    loader: descriptor.loader,
                }).collect(),
                category_id,
                sort_index,
            });
        }
        mods.sort_by(|a, b| a.category_id.cmp(&b.category_id)
            .then(a.sort_index.cmp(&b.sort_index))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));

        Ok(WorkspaceSnapshot {
            mods_dir: Some(dir.display().to_string()),
            categories,
            mods,
        })
    }
}

fn is_descendant(candidate: &str, ancestor: &str, categories: &[Category]) -> bool {
    let mut cursor = Some(candidate);
    while let Some(id) = cursor {
        if id == ancestor { return true; }
        cursor = categories.iter().find(|c| c.id == id).and_then(|c| c.parent_id.as_deref());
    }
    false
}

fn toggled_path(current: &Path, enabled: bool) -> PathBuf {
    let s = current.to_string_lossy();
    if enabled {
        if let Some(stripped) = s.strip_suffix(".disabled") {
            PathBuf::from(stripped)
        } else {
            current.to_path_buf()
        }
    } else if s.ends_with(".jar") {
        PathBuf::from(format!("{s}.disabled"))
    } else {
        current.to_path_buf()
    }
}
