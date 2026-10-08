use crate::{
    backend::{
        profiles::{self, GameInstance},
        settings::GameKey,
    },
    AppState,
};
use chrono::Utc;
use std::path::{Path, PathBuf};
use tauri::State;

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
pub async fn profile_edit(
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
    meta.modified = Utc::now();

    profiles::save_profile_meta(&profile_dir, &meta).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn profile_remove(state: State<'_, AppState>, path: String) -> Result<(), String> {
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

    Ok(())
}
