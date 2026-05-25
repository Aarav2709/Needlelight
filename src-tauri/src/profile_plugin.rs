use crate::{
    backend::{
        profiles::{self, GameInstance, ProfileMeta},
        settings::GameKey,
    },
    AppState,
};
use chrono::Utc;
use serde::Serialize;
use std::{collections::HashMap, io::{self, ErrorKind}, path::{Path, PathBuf}};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
struct ContentFile {
    pub hash: String,
    pub file_name: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_version_id: Option<String>,
    pub project_type: String,
}

#[derive(Debug, Serialize)]
struct ProfileEventPayload {
    pub uuid: String,
    pub name: String,
    pub profile_path: String,
    pub path: String,
    pub event: String,
}

fn resolve_profile_dir(settings: &crate::backend::settings::AppSettings, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return path;
    }

    profiles::profiles_root(&settings.game)
        .unwrap_or_else(|_| PathBuf::from(raw))
        .join(raw)
}

fn emit_profile_event(app: &AppHandle, profile_dir: &Path, meta: &ProfileMeta, event: &str) {
    let payload = ProfileEventPayload {
        uuid: profile_dir.to_string_lossy().to_string(),
        name: meta.name.clone(),
        profile_path: profile_dir.to_string_lossy().to_string(),
        path: profile_dir.to_string_lossy().to_string(),
        event: event.to_string(),
    };
    let _ = app.emit("profile", payload);
}

fn mods_dir(profile_dir: &Path, game: &GameKey) -> PathBuf {
    if game.is_silksong() {
        profile_dir.join("BepInEx").join("plugins")
    } else {
        profile_dir.join("Mods")
    }
}

fn directory_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += directory_size(&path);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn silksong_disabled(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if silksong_disabled(&path) {
                    return true;
                }
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".old") {
                    return true;
                }
            }
        }
    }
    false
}

fn build_content_file(_path: &Path, file_name: String, size: u64) -> ContentFile {
    ContentFile {
        hash: String::new(),
        file_name,
        size,
        metadata: None,
        update_version_id: None,
        project_type: "mod".to_string(),
    }
}

#[tauri::command]
pub async fn profile_list(state: State<'_, AppState>) -> Result<Vec<GameInstance>, String> {
    let settings = state.settings.read().await.clone();
    let root = profiles::profiles_root(&settings.game).map_err(|e| e.to_string())?;

    if !root.exists() {
        return Ok(vec![]);
    }

    let mut output = Vec::new();
    let entries = std::fs::read_dir(&root).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let mut meta = profiles::load_profile_meta(&path).map_err(|e| e.to_string())?;
        if meta.game != settings.game {
            meta.game = settings.game.clone();
        }
        output.push(profiles::profile_to_instance(&path, &meta));
    }

    Ok(output)
}

#[tauri::command]
pub async fn profile_get(state: State<'_, AppState>, path: String) -> Result<GameInstance, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
    Ok(profiles::profile_to_instance(&profile_dir, &meta))
}

#[tauri::command]
pub async fn profile_get_many(state: State<'_, AppState>, paths: Vec<String>) -> Result<Vec<GameInstance>, String> {
    let settings = state.settings.read().await.clone();
    let mut output = Vec::new();

    for path in paths {
        let profile_dir = resolve_profile_dir(&settings, &path);
        let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
        output.push(profiles::profile_to_instance(&profile_dir, &meta));
    }

    Ok(output)
}

#[tauri::command]
pub async fn profile_get_full_path(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    Ok(profile_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn profile_get_mod_full_path(
    state: State<'_, AppState>,
    path: String,
    project_path: String,
) -> Result<String, String> {
    let settings = state.settings.read().await.clone();
    let _ = resolve_profile_dir(&settings, &path);
    Ok(project_path)
}

#[tauri::command]
pub async fn profile_get_optimal_jre_key(_path: String) -> Result<String, String> {
    Ok("default".to_string())
}

#[tauri::command]
pub async fn profile_check_installed(state: State<'_, AppState>, path: String, project_id: String) -> Result<bool, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let mods = mods_dir(&profile_dir, &settings.game);
    Ok(mods.join(project_id).exists())
}

#[tauri::command]
pub async fn profile_install(state: State<'_, AppState>, path: String, _force: Option<bool>) -> Result<bool, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);

    profiles::ensure_profile_dir(&profile_dir).map_err(|e| e.to_string())?;
    if settings.game.is_silksong() {
        let bepinex = profile_dir.join("BepInEx/core/BepInEx.dll");
        if !bepinex.exists() {
            crate::profile_create_plugin::install_bepinex_pack(&settings, &profile_dir).await?;
        }
    } else {
        std::fs::create_dir_all(profile_dir.join("Mods")).map_err(|e| e.to_string())?;
    }

    Ok(true)
}

#[tauri::command]
pub async fn profile_update_all(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn profile_update_project(_path: String, _project_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn profile_add_project_from_version(
    _path: String,
    _version_id: String,
) -> Result<String, String> {
    Err("Adding projects by version is not supported for Needlelight profiles.".to_string())
}

#[tauri::command]
pub async fn profile_add_project_from_path(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    project_path: String,
) -> Result<String, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let mods_root = mods_dir(&profile_dir, &settings.game);
    std::fs::create_dir_all(&mods_root).map_err(|e| e.to_string())?;

    let source = PathBuf::from(&project_path);
    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mod")
        .to_string();
    let target = mods_root.join(&file_name);

    if source.is_dir() {
        copy_dir_all(&source, &target).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(&source, &target).map_err(|e| e.to_string())?;
    }

    let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "synced");

    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn profile_toggle_disable_project(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    project_path: String,
) -> Result<String, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let project = PathBuf::from(&project_path);

    if project.is_dir() {
        toggle_directory(project.as_path(), settings.game.is_silksong()).map_err(|e| e.to_string())?;
        let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
        emit_profile_event(&app, &profile_dir, &meta, "synced");
        return Ok(project_path);
    }

    let file_name = project
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mod")
        .to_string();

    if file_name.ends_with(".disabled") {
        let new_name = file_name.trim_end_matches(".disabled");
        let new_path = project.with_file_name(new_name);
        std::fs::rename(&project, &new_path).map_err(|e| e.to_string())?;
        let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
        emit_profile_event(&app, &profile_dir, &meta, "synced");
        return Ok(new_path.to_string_lossy().to_string());
    }

    let new_path = project.with_file_name(format!("{file_name}.disabled"));
    std::fs::rename(&project, &new_path).map_err(|e| e.to_string())?;
    let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "synced");
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn profile_remove_project(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    project_path: String,
) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let project = PathBuf::from(&project_path);

    if project.is_dir() {
        std::fs::remove_dir_all(&project).map_err(|e| e.to_string())?;
    } else if project.exists() {
        std::fs::remove_file(&project).map_err(|e| e.to_string())?;
    }

    let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "synced");

    Ok(())
}

#[tauri::command]
pub async fn profile_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    edit_profile: serde_json::Value,
) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let mut meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;

    if let Some(name) = edit_profile.get("name").and_then(|v| v.as_str()) {
        meta.name = name.trim().to_string();
    }
    if let Some(groups) = edit_profile.get("groups").and_then(|v| v.as_array()) {
        meta.groups = groups
            .iter()
            .filter_map(|g| g.as_str().map(|s| s.to_string()))
            .collect();
    }
    meta.modified = Utc::now();

    profiles::save_profile_meta(&profile_dir, &meta).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "edited");
    Ok(())
}

#[tauri::command]
pub async fn profile_edit_icon(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    icon_path: Option<String>,
) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let mut meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;

    if let Some(icon_path) = icon_path {
        if let Some(existing) = meta.icon_file.as_ref() {
            let existing_path = profile_dir.join(existing);
            if existing_path.exists() {
                let _ = std::fs::remove_file(existing_path);
            }
        }
        let source = PathBuf::from(&icon_path);
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let file_name = format!("icon.{ext}");
        let target = profile_dir.join(&file_name);
        std::fs::copy(&source, &target).map_err(|e| e.to_string())?;
        meta.icon_file = Some(file_name);
    } else {
        if let Some(existing) = meta.icon_file.as_ref() {
            let existing_path = profile_dir.join(existing);
            if existing_path.exists() {
                let _ = std::fs::remove_file(existing_path);
            }
        }
        meta.icon_file = None;
    }

    meta.modified = Utc::now();
    profiles::save_profile_meta(&profile_dir, &meta).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "edited");
    Ok(())
}

#[tauri::command]
pub async fn profile_remove(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;

    if profile_dir.exists() {
        std::fs::remove_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    }

    emit_profile_event(&app, &profile_dir, &meta, "removed");
    Ok(())
}

#[tauri::command]
pub async fn profile_get_projects(
    state: State<'_, AppState>,
    path: String,
    _cache_behaviour: Option<String>,
) -> Result<HashMap<String, ContentFile>, String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let mods_root = mods_dir(&profile_dir, &settings.game);
    std::fs::create_dir_all(&mods_root).map_err(|e| e.to_string())?;

    let mut output = HashMap::new();
    let entries = std::fs::read_dir(&mods_root).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("mod")
                .to_string();
            if name == "Disabled" {
                continue;
            }
            let disabled = if settings.game.is_silksong() {
                silksong_disabled(&path)
            } else {
                name.ends_with(".disabled")
            };
            let file_name = if disabled { format!("{name}.disabled") } else { name };
            let size = directory_size(&path);
            output.insert(path.to_string_lossy().to_string(), build_content_file(&path, file_name, size));
            continue;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("mod")
            .to_string();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        output.insert(path.to_string_lossy().to_string(), build_content_file(&path, name, size));
    }

    Ok(output)
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("profile")
        .invoke_handler(tauri::generate_handler![
            profile_list,
            profile_get,
            profile_get_many,
            profile_get_projects,
            profile_get_full_path,
            profile_get_mod_full_path,
            profile_get_optimal_jre_key,
            profile_check_installed,
            profile_install,
            profile_update_all,
            profile_update_project,
            profile_add_project_from_version,
            profile_add_project_from_path,
            profile_toggle_disable_project,
            profile_remove_project,
            profile_edit,
            profile_edit_icon,
            profile_remove,
        ])
        .build()
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
        let rel = entry
            .path()
            .strip_prefix(src)
            .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn toggle_directory(dir: &Path, use_old_extension: bool) -> io::Result<()> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "manifest.json" {
            continue;
        }
        let path = entry.path();
        if use_old_extension {
            if name.ends_with(".old") {
                let target = path.with_file_name(name.trim_end_matches(".old"));
                std::fs::rename(path, target)?;
            } else {
                let target = path.with_file_name(format!("{name}.old"));
                std::fs::rename(path, target)?;
            }
        } else if name.ends_with(".disabled") {
            let target = path.with_file_name(name.trim_end_matches(".disabled"));
            std::fs::rename(path, target)?;
        } else {
            let target = path.with_file_name(format!("{name}.disabled"));
            std::fs::rename(path, target)?;
        }
    }
    Ok(())
}
