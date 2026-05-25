use crate::{
    backend::{
        mod_database::CatalogCache,
        profiles::{self, GameInstance, ProfileMeta},
    },
    AppState,
};
use chrono::Utc;
use serde::Serialize;
use std::{io::{self, ErrorKind}, path::{Path, PathBuf}};
use tauri::{AppHandle, Emitter, State};
use zip::ZipArchive;

#[derive(Debug, Serialize)]
struct ProfileEventPayload {
    pub uuid: String,
    pub name: String,
    pub profile_path: String,
    pub path: String,
    pub event: String,
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

fn resolve_profile_dir(settings: &crate::backend::settings::AppSettings, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return path;
    }

    profiles::profiles_root(&settings.game)
        .unwrap_or_else(|_| PathBuf::from(raw))
        .join(raw)
}

#[tauri::command]
pub async fn profile_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    _game_version: String,
    _modloader: String,
    _loader_version: Option<String>,
    icon: Option<String>,
    skip_install: Option<bool>,
) -> Result<GameInstance, String> {
    let settings = state.settings.read().await.clone();
    let root = profiles::profiles_root(&settings.game).map_err(|e| e.to_string())?;
    profiles::ensure_profile_dir(&root).map_err(|e| e.to_string())?;

    let display_name = name.trim().to_string();
    let safe_name = profiles::sanitize_profile_name(&display_name);
    let profile_dir = root.join(&safe_name);
    if profile_dir.exists() {
        return Err("Profile already exists.".to_string());
    }

    profiles::ensure_profile_dir(&profile_dir).map_err(|e| e.to_string())?;

    if settings.game.is_silksong() {
        if !skip_install.unwrap_or(false) {
            install_bepinex_pack(&settings, &profile_dir).await?;
        } else {
            std::fs::create_dir_all(profile_dir.join("BepInEx"))
                .map_err(|e| e.to_string())?;
        }
    } else {
        std::fs::create_dir_all(profile_dir.join("Mods")).map_err(|e| e.to_string())?;
    }

    let icon_file = icon
        .and_then(|path| copy_icon(&profile_dir, &path).ok());

    let now = Utc::now();
    let meta = ProfileMeta {
        name: display_name,
        game: settings.game.clone(),
        groups: vec![],
        created: now,
        modified: now,
        last_played: None,
        icon_file,
    };

    profiles::save_profile_meta(&profile_dir, &meta).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &profile_dir, &meta, "created");

    Ok(profiles::profile_to_instance(&profile_dir, &meta))
}

#[tauri::command]
pub async fn profile_duplicate(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<GameInstance, String> {
    let settings = state.settings.read().await.clone();
    let source_dir = resolve_profile_dir(&settings, &path);
    let meta = profiles::load_profile_meta(&source_dir).map_err(|e| e.to_string())?;

    let root = profiles::profiles_root(&settings.game).map_err(|e| e.to_string())?;
    profiles::ensure_profile_dir(&root).map_err(|e| e.to_string())?;

    let base_name = format!("{} copy", meta.name);
    let mut index = 1;
    let mut candidate = root.join(profiles::sanitize_profile_name(&base_name));
    let mut display_name = base_name.clone();
    while candidate.exists() {
        index += 1;
        display_name = format!("{base_name} {index}");
        candidate = root.join(profiles::sanitize_profile_name(&display_name));
    }

    copy_dir_all(&source_dir, &candidate).map_err(|e| e.to_string())?;

    let now = Utc::now();
    let mut new_meta = meta.clone();
    new_meta.name = display_name;
    new_meta.created = now;
    new_meta.modified = now;
    new_meta.last_played = None;

    profiles::save_profile_meta(&candidate, &new_meta).map_err(|e| e.to_string())?;
    emit_profile_event(&app, &candidate, &new_meta, "created");

    Ok(profiles::profile_to_instance(&candidate, &new_meta))
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("profile-create")
        .invoke_handler(tauri::generate_handler![profile_create, profile_duplicate])
        .build()
}

fn copy_icon(profile_dir: &Path, icon_path: &str) -> io::Result<String> {
    let source = PathBuf::from(icon_path);
    let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let file_name = format!("icon.{ext}");
    let target = profile_dir.join(&file_name);
    std::fs::copy(source, target)?;
    Ok(file_name)
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

pub(crate) async fn install_bepinex_pack(
    settings: &crate::backend::settings::AppSettings,
    profile_dir: &Path,
) -> Result<(), String> {
    let cache = CatalogCache::build(settings, &crate::backend::installed_mods::InstalledModsStore::default(), true)
        .await
        .map_err(|e| e.to_string())?;
    let api = cache.response.api;
    if api.url.trim().is_empty() {
        return Err("BepInEx pack not available".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Needlelight")
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = client
        .get(&api.url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    extract_zip_guarded(bytes.as_ref(), profile_dir).map_err(|e| e.to_string())?;
    Ok(())
}

fn extract_zip_guarded(data: &[u8], destination: &Path) -> io::Result<()> {
    let reader = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(reader)
        .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "zip entry path traversal blocked"))?
            .to_path_buf();

        let output = destination.join(enclosed);
        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&output)?;
            continue;
        }

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(&output)?;
        std::io::copy(&mut entry, &mut file)?;
    }

    Ok(())
}
