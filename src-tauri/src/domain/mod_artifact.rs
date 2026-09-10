use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ModDescriptor {
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub loader: ModLoader,
    pub icon_path_in_jar: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModArtifact {
    pub id: String,
    pub layout_key: String,
    pub descriptor: ModDescriptor,
    pub packaged_mods: Vec<ModDescriptor>,
    pub enabled: bool,
    pub file_name: String,
    pub path: PathBuf,
    pub icon_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackagedModDto {
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub loader: ModLoader,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntryDto {
    pub id: String,
    pub layout_key: String,
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub loader: ModLoader,
    pub enabled: bool,
    pub file_name: String,
    pub path: String,
    pub icon_data_url: Option<String>,
    pub packaged_mods: Vec<PackagedModDto>,
    pub category_id: Option<String>,
    pub sort_index: i64,
}
