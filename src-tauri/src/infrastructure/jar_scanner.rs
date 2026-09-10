use crate::domain::mod_artifact::{ModArtifact, ModDescriptor, ModLoader};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value as JsonValue;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

pub struct JarScanner;

impl JarScanner {
    pub fn scan_dir(dir: &Path) -> Result<Vec<ModArtifact>, ScanError> {
        let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_mod_file(p))
            .collect();
        paths.sort();

        let mut out = Vec::new();
        for path in paths {
            match Self::scan_file(&path) {
                Ok(mod_artifact) => out.push(mod_artifact),
                Err(err) => eprintln!("Skipping {}: {err}", path.display()),
            }
        }
        Ok(out)
    }

    pub fn scan_file(path: &Path) -> Result<ModArtifact, ScanError> {
        let enabled = path.to_string_lossy().ends_with(".jar");
        let file_name = path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("unknown.jar")
            .to_string();
        let stable_path = strip_disabled(path);
        let artifact_id = Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            stable_path.to_string_lossy().as_bytes(),
        )
        .to_string();

        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut packaged_mods = scan_archive_descriptors(&mut archive, 0);

        let descriptor = if packaged_mods.is_empty() {
            ModDescriptor {
                mod_id: strip_jar_suffix(&file_name),
                name: strip_jar_suffix(&file_name),
                version: "?".into(),
                loader: ModLoader::Unknown,
                icon_path_in_jar: None,
            }
        } else {
            packaged_mods[0].clone()
        };

        if packaged_mods.is_empty() {
            packaged_mods.push(descriptor.clone());
        }

        let icon_data_url = descriptor
            .icon_path_in_jar
            .as_deref()
            .and_then(|icon_path| read_bytes(&mut archive, icon_path))
            .map(|bytes| {
                format!(
                    "data:{};base64,{}",
                    mime_for_icon(descriptor.icon_path_in_jar.as_deref()),
                    STANDARD.encode(bytes)
                )
            });

        let layout_key = format!("{:?}:{}", descriptor.loader, descriptor.mod_id);
        Ok(ModArtifact {
            id: artifact_id,
            layout_key,
            descriptor,
            packaged_mods,
            enabled,
            file_name,
            path: path.to_path_buf(),
            icon_data_url,
        })
    }
}

fn scan_archive_descriptors<R: Read + Seek>(archive: &mut ZipArchive<R>, depth: usize) -> Vec<ModDescriptor> {
    // A small depth cap protects the UI scanner from pathological nested archives.
    if depth > 3 {
        return Vec::new();
    }

    let mut descriptors = Vec::new();
    let mut declared_nested_paths = Vec::new();

    if let Some(text) = read_text(archive, "fabric.mod.json") {
        descriptors.push(parse_fabric(&text));
        declared_nested_paths.extend(parse_declared_jars(&text));
    } else if let Some(text) = read_text(archive, "quilt.mod.json") {
        descriptors.push(parse_quilt(&text));
        declared_nested_paths.extend(parse_declared_jars(&text));
    } else if let Some(text) = read_text(archive, "META-INF/neoforge.mods.toml") {
        descriptors.extend(parse_forge_like(&text, ModLoader::NeoForge));
    } else if let Some(text) = read_text(archive, "META-INF/mods.toml") {
        descriptors.extend(parse_forge_like(&text, ModLoader::Forge));
    }

    if depth < 3 {
        let mut nested_paths = declared_nested_paths;
        nested_paths.extend(archive_nested_jar_paths(archive));
        nested_paths.sort();
        nested_paths.dedup();

        for nested_path in nested_paths {
            let Some(bytes) = read_bytes(archive, &nested_path) else {
                continue;
            };
            let Ok(mut nested_archive) = ZipArchive::new(Cursor::new(bytes)) else {
                continue;
            };
            descriptors.extend(scan_archive_descriptors(&mut nested_archive, depth + 1));
        }
    }

    dedupe_descriptors(descriptors)
}

fn parse_fabric(text: &str) -> ModDescriptor {
    let v: JsonValue = serde_json::from_str(text).unwrap_or(JsonValue::Null);
    let mod_id = str_at(&v, &["id"]).unwrap_or("unknown").to_string();
    let name = str_at(&v, &["name"]).unwrap_or(&mod_id).to_string();
    let version = str_at(&v, &["version"]).unwrap_or("?").to_string();
    let icon_path_in_jar = v.get("icon").and_then(icon_from_json);
    ModDescriptor {
        mod_id,
        name,
        version,
        loader: ModLoader::Fabric,
        icon_path_in_jar,
    }
}

fn parse_quilt(text: &str) -> ModDescriptor {
    let v: JsonValue = serde_json::from_str(text).unwrap_or(JsonValue::Null);
    let loader = v.get("quilt_loader");
    let mod_id = loader.and_then(|l| str_at(l, &["id"])).unwrap_or("unknown").to_string();
    let version = loader.and_then(|l| str_at(l, &["version"])).unwrap_or("?").to_string();
    let name = loader
        .and_then(|l| str_at(l, &["metadata", "name"]))
        .unwrap_or(&mod_id)
        .to_string();
    let icon_path_in_jar = loader
        .and_then(|l| l.get("metadata"))
        .and_then(|m| m.get("icon"))
        .and_then(icon_from_json);
    ModDescriptor {
        mod_id,
        name,
        version,
        loader: ModLoader::Quilt,
        icon_path_in_jar,
    }
}

fn parse_forge_like(text: &str, loader: ModLoader) -> Vec<ModDescriptor> {
    let v: toml::Value = toml::from_str(text).unwrap_or(toml::Value::Table(Default::default()));
    let Some(mods) = v.get("mods").and_then(|x| x.as_array()) else {
        return Vec::new();
    };

    mods.iter()
        .map(|m| {
            let mod_id = m
                .get("modId")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let name = m
                .get("displayName")
                .and_then(|x| x.as_str())
                .unwrap_or(&mod_id)
                .to_string();
            let version = m
                .get("version")
                .and_then(|x| x.as_str())
                .unwrap_or("?")
                .to_string();
            let icon_path_in_jar = m
                .get("logoFile")
                .and_then(|x| x.as_str())
                .map(str::to_string);
            ModDescriptor {
                mod_id,
                name,
                version,
                loader: loader.clone(),
                icon_path_in_jar,
            }
        })
        .collect()
}

fn parse_declared_jars(text: &str) -> Vec<String> {
    let v: JsonValue = serde_json::from_str(text).unwrap_or(JsonValue::Null);
    v.get("jars")
        .and_then(|jars| jars.as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .or_else(|| entry.get("file").and_then(|v| v.as_str()).map(str::to_string))
        })
        .filter(|path| path.to_ascii_lowercase().ends_with(".jar"))
        .collect()
}

fn archive_nested_jar_paths<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Vec<String> {
    let mut paths = Vec::new();
    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        let name = file.name().replace('\\', "/");
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".jar")
            && (lower.starts_with("meta-inf/jars/") || lower.starts_with("meta-inf/jarjar/"))
        {
            paths.push(name);
        }
    }
    paths
}

fn dedupe_descriptors(descriptors: Vec<ModDescriptor>) -> Vec<ModDescriptor> {
    let mut seen = HashSet::new();
    descriptors
        .into_iter()
        .filter(|descriptor| {
            let key = format!(
                "{:?}\u{1f}{}\u{1f}{}",
                descriptor.loader,
                descriptor.mod_id.to_ascii_lowercase(),
                descriptor.version
            );
            seen.insert(key)
        })
        .collect()
}

fn str_at<'a>(v: &'a JsonValue, path: &[&str]) -> Option<&'a str> {
    let mut current = v;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn icon_from_json(v: &JsonValue) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    let obj = v.as_object()?;
    obj.iter()
        .filter_map(|(k, v)| Some((k.parse::<u32>().ok()?, v.as_str()?)))
        .max_by_key(|(size, _)| *size)
        .map(|(_, path)| path.to_string())
}

fn read_text<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut file = archive.by_name(name).ok()?;
    let mut s = String::new();
    file.read_to_string(&mut s).ok()?;
    Some(s)
}

fn read_bytes<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<Vec<u8>> {
    let mut file = archive.by_name(name).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn is_mod_file(path: &Path) -> bool {
    let s = path
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    s.ends_with(".jar") || s.ends_with(".jar.disabled")
}

fn strip_disabled(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.strip_suffix(".disabled").unwrap_or(&s))
}

fn strip_jar_suffix(name: &str) -> String {
    name.trim_end_matches(".disabled")
        .trim_end_matches(".jar")
        .to_string()
}

fn mime_for_icon(path: Option<&str>) -> &'static str {
    match path.unwrap_or("").to_ascii_lowercase().as_str() {
        p if p.ends_with(".jpg") || p.ends_with(".jpeg") => "image/jpeg",
        p if p.ends_with(".webp") => "image/webp",
        p if p.ends_with(".gif") => "image/gif",
        _ => "image/png",
    }
}
