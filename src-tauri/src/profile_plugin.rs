use crate::{
    backend::{
        profiles::{self, GameInstance, ProfileMeta},
        settings::GameKey,
    },
    AppState,
};
use chrono::Utc;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfileEventPayload {
    pub uuid: String,
    pub name: String,
    pub profile_path: String,
    pub path: String,
    pub event: String,
}

// absolute paths are used as is, anything else is a folder name under the active game's profiles
pub(crate) fn resolve_profile_dir(
    settings: &crate::backend::settings::AppSettings,
    raw: &str,
) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return path;
    }

    profiles::profiles_root(&settings.game)
        .unwrap_or_else(|_| PathBuf::from(raw))
        .join(raw)
}

pub(crate) fn emit_profile_event<R: tauri::Runtime>(
    app: &AppHandle<R>,
    profile_dir: &Path,
    meta: &ProfileMeta,
    event: &str,
) {
    let payload = ProfileEventPayload {
        uuid: profile_dir.to_string_lossy().to_string(),
        name: meta.name.clone(),
        profile_path: profile_dir.to_string_lossy().to_string(),
        path: profile_dir.to_string_lossy().to_string(),
        event: event.to_string(),
    };
    let _ = app.emit("profile", payload);
}

#[tauri::command]
pub async fn profile_list(game: Option<GameKey>) -> Result<Vec<GameInstance>, String> {
    // profiles live in one folder per game, so no game means list every modpack of both games
    let games: Vec<GameKey> = match game {
        Some(g) => vec![g],
        None => vec![GameKey::HollowKnight, GameKey::Silksong],
    };

    let mut output = Vec::new();
    for g in games {
        let root = profiles::profiles_root(&g).map_err(|e| e.to_string())?;
        if !root.exists() {
            continue;
        }

        let entries = std::fs::read_dir(&root).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // one unreadable profile.json shouldn't hide every other modpack
            let mut meta = match profiles::load_profile_meta(&path) {
                Ok(meta) => meta,
                Err(error) => {
                    log::warn!("Skipping modpack {}: {error}", path.display());
                    continue;
                }
            };
            meta.game = g.clone();
            output.push(profiles::profile_to_instance(&path, &meta));
        }
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
pub async fn profile_edit<R: tauri::Runtime>(
    app: AppHandle<R>,
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
    if let Some(description) = edit_profile.get("description") {
        meta.description = description
            .as_str()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
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
pub async fn profile_edit_icon<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: String,
    icon_path: Option<String>,
) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let mut meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;

    if let Some(existing) = meta.icon_file.take() {
        let existing_path = profile_dir.join(existing);
        if existing_path.exists() {
            let _ = std::fs::remove_file(existing_path);
        }
    }
    if let Some(icon_path) = icon_path {
        let file_name = crate::profile_create_plugin::copy_icon(&profile_dir, &icon_path)
            .map_err(|e| e.to_string())?;
        meta.icon_file = Some(file_name);
    }

    meta.modified = Utc::now();
    profiles::save_profile_meta(&profile_dir, &meta).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "edited");
    Ok(())
}

#[tauri::command]
pub async fn profile_remove<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let _guard = state.modpack_lock.lock().await;
    let settings = state.settings.read().await.clone();
    let profile_dir = resolve_profile_dir(&settings, &path);
    let meta = profiles::load_profile_meta(&profile_dir).map_err(|e| e.to_string())?;

    if profile_dir.exists() {
        // only ever delete a modpack folder sitting directly in a game's profiles folder
        let inside_profiles = profiles::profiles_root(&meta.game)
            .ok()
            .and_then(|root| root.parent().map(Path::to_path_buf))
            .and_then(|root| root.canonicalize().ok())
            .zip(profile_dir.canonicalize().ok())
            .is_some_and(|(root, dir)| dir.parent().and_then(Path::parent) == Some(root.as_path()));
        if !inside_profiles {
            return Err("This folder isn't a Needlelight modpack.".to_string());
        }
        std::fs::remove_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    }

    emit_profile_event(&app, &profile_dir, &meta, "removed");
    Ok(())
}
