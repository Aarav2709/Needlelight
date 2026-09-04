use crate::{
    backend::{
        errors::AppResult,
        installer,
        mod_database::CatalogCache,
        pack_manager,
        settings::{AppSettings, GameKey},
        url_scheme,
    },
    AppState,
};
use std::collections::BTreeMap;
use std::process::Command;
use tauri::State;

fn map_err<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

fn sync_managed_folder(mut settings: AppSettings) -> AppSettings {
    if settings.managed_folders.is_empty() && !settings.managed_folder.is_empty() {
        let game = settings.game.clone();
        let folder = settings.managed_folder.clone();
        settings.set_managed_folder_for(&game, folder);
    }

    let stored = settings.managed_folder_for(&settings.game);
    if !stored.is_empty() {
        settings.managed_folder = stored;
    }

    settings.sync_custom_modlinks();
    settings
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(sync_managed_folder(state.settings.read().await.clone()))
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    let mut incoming = settings.normalized();
    let previous = state.settings.read().await.clone();
    let previous = sync_managed_folder(previous);

    if incoming.managed_folders.is_empty() {
        incoming.managed_folders = previous.managed_folders.clone();
    }
    if incoming.custom_modlinks_by_game.is_empty() {
        incoming.custom_modlinks_by_game = previous.custom_modlinks_by_game.clone();
    }

    let prev_path = previous.managed_folder;
    let incoming_game = incoming.game.clone();
    if incoming.game != previous.game && incoming.managed_folder == prev_path {
        // Switching games without updating the path: restore saved path for new game.
        let stored = incoming.managed_folder_for(&incoming_game);
        incoming.managed_folder = stored;
    }

    if incoming.game != previous.game {
        // The form was loaded for the previous game, so preserve its catalog
        // values before exposing the selected game's independent values.
        incoming.custom_modlinks_by_game.insert(
            previous.game.as_str().to_string(),
            crate::backend::settings::CustomModlinksConfig {
                enabled: incoming.use_custom_modlinks,
                uri: incoming.custom_modlinks_uri.clone(),
            },
        );
        incoming.sync_custom_modlinks();
    } else {
        incoming.set_custom_modlinks_for_current();
    }

    incoming.managed_folder = AppSettings::normalize_managed_folder(&incoming.managed_folder, &incoming_game);
    let folder = incoming.managed_folder.clone();
    incoming.set_managed_folder_for(&incoming_game, folder);
    incoming.set_custom_modlinks_for_current();

    map_err(incoming.save().await)?;

    {
        let mut shared = state.settings.write().await;
        *shared = incoming.clone();
    }

    let reloaded = map_err(crate::backend::installed_mods::InstalledModsStore::load(&incoming).await)?;
    {
        let mut installed = state.installed.write().await;
        *installed = reloaded;
    }

    Ok(())
}

#[tauri::command]
pub async fn auto_detect_managed_folder(game: GameKey) -> Result<Option<String>, String> {
    map_err(AppSettings::auto_detect(&game).await)
}

#[tauri::command]
pub async fn refresh_catalog(
    state: State<'_, AppState>,
    fetch_official: bool,
) -> Result<crate::backend::models::CatalogResponse, String> {
    let settings = state.settings.read().await.clone();
    let installed = state.installed.read().await.clone();

    let fetch_official = fetch_official && !settings.use_custom_modlinks;
    let mut cache = map_err(CatalogCache::build(&settings, &installed, fetch_official).await)?;
    let api_installed = installer::is_api_installed(&settings, &installed);
    cache.response.api_installed = api_installed;
    cache.response.api_enabled = api_installed;
    if !api_installed {
        cache.response.api.url = String::new();
    }

    Ok(cache.response)
}

#[tauri::command]
pub async fn install_mod(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog = map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    let result = installer::install_mod(&settings, &mut installed, &catalog.response, &name).await;
    if let Err(error) = &result {
        installer::write_install_log(format!("Mod install failed for {name}: {error}"));
    }
    map_err(result)
}

#[tauri::command]
pub async fn uninstall_mod(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    map_err(installer::uninstall_mod(&settings, &mut installed, &name).await)
}

#[tauri::command]
pub async fn toggle_mod(state: State<'_, AppState>, name: String, enable: bool) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    map_err(installer::toggle_mod(&settings, &mut installed, &name, enable).await)
}

#[tauri::command]
pub async fn install_api(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog = map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    let result = installer::install_api(&settings, &mut installed, &catalog.response).await;
    if let Err(error) = &result {
        installer::write_install_log(format!("Modding API install failed: {error}"));
    }
    map_err(result)
}

#[tauri::command]
pub async fn parse_download_command(raw: String) -> Result<BTreeMap<String, Option<String>>, String> {
    let (_, data) = url_scheme::decode_command(&raw);
    map_err(url_scheme::parse_download_command(&data))
}

#[tauri::command]
pub async fn list_packs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let settings = state.settings.read().await.clone();
    map_err(pack_manager::list_packs(&settings).await)
}

#[tauri::command]
pub async fn save_pack(
    state: State<'_, AppState>,
    name: String,
    description: String,
    authors: String,
) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let installed = state.installed.read().await.clone();
    map_err(pack_manager::save_pack(&settings, &installed, &name, &description, &authors).await)
}

#[tauri::command]
pub async fn load_pack(state: State<'_, AppState>, name: String, additive: bool) -> Result<bool, String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    map_err(pack_manager::load_pack(&settings, &mut installed, &name, additive).await)
}

#[tauri::command]
pub async fn import_pack(state: State<'_, AppState>, code: String) -> Result<Option<String>, String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    map_err(pack_manager::import_pack(&settings, &mut installed, &code).await)
}

#[tauri::command]
pub async fn launch_game(state: State<'_, AppState>, modded: bool) -> Result<String, String> {
    let settings = state.settings.read().await.clone();
    if settings.managed_folder.is_empty() {
        installer::write_install_log("Game launch failed: no managed folder is configured.");
        return Err("Game folder not configured. Go to Settings > Game to set it up.".to_string());
    }

    let game_root = settings.game_root_path();
    if !game_root.is_dir() {
        installer::write_install_log(format!("Game launch failed: game root is not a directory: {}", game_root.display()));
        return Err(format!("Game folder is invalid: {}", game_root.display()));
    }

    // Find executable
    let exe_candidates: Vec<std::path::PathBuf> = match settings.game {
        GameKey::HollowKnight => vec![
            game_root.join("hollow_knight.x86_64"),
            game_root.join("hollow_knight"),
            game_root.join("hollow_knight.exe"),
            game_root.join("Hollow Knight.exe"),
            game_root.join("HollowKnight.exe"),
            game_root.join("Hollow Knight.app/Contents/MacOS/Hollow Knight"),
        ],
        GameKey::Silksong => vec![
            game_root.join("hollowknightsilksong.x86_64"),
            game_root.join("hollow_knight_silksong.x86_64"),
            game_root.join("Hollow Knight Silksong.x86_64"),
            game_root.join("hollowknightsilksong"),
            game_root.join("hollowknightsilksong.exe"),
            game_root.join("Hollow Knight Silksong.exe"),
            game_root.join("Hollow Knight Silksong.app/Contents/MacOS/Hollow Knight Silksong"),
        ],
    };

    let exe = exe_candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            let message = format!("Could not find game executable in {}", game_root.display());
            installer::write_install_log(format!("Game launch failed: {message}"));
            message
        })?;

    // If vanilla, temporarily disable mods by renaming Assembly-CSharp.dll.modded
    // For now just launch the game directly - vanilla/modded distinction is handled by
    // whether the API is installed
    installer::write_install_log(format!("Launching {} from {}.", exe.display(), game_root.display()));
    let child = Command::new(exe.as_os_str())
        .current_dir(&game_root)
        .spawn()
        .map_err(|e| {
            let message = format!("Failed to launch game: {e}");
            installer::write_install_log(format!("Game launch failed: {message}"));
            message
        })?;

    let mode = if modded { "modded" } else { "vanilla" };
    Ok(format!("Launched {} ({mode}, process {}).", exe.file_name().unwrap_or_default().to_string_lossy(), child.id()))
}
