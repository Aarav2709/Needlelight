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
use tauri::{AppHandle, State};

fn map_err<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(|e| format_user_error(&e.to_string()))
}

// most AppError messages are lowercase fragments meant for logs, not people;
// capitalize the first letter and make sure it ends with punctuation
fn format_user_error(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "Something went wrong.".to_string();
    }
    let mut chars = trimmed.chars();
    let mut out: String = chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
    out.push_str(chars.as_str());
    if !out.ends_with(['.', '!', '?', ':']) {
        out.push('.');
    }
    out
}

// delegates to AppSettings::sync_managed_folder, single source of truth
fn sync_managed_folder(mut settings: AppSettings) -> AppSettings {
    settings.sync_managed_folder();
    settings.sync_custom_modlinks();
    settings
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = sync_managed_folder(state.settings.read().await.clone());

    // auto-detect the managed folder on cold load if nothing is configured
    if settings.managed_folder.trim().is_empty() {
        if let Some(path) = map_err(AppSettings::auto_detect(&settings.game).await)? {
            settings.managed_folder = AppSettings::normalize_managed_folder(&path, &settings.game);
            let game = settings.game.clone();
            let folder = settings.managed_folder.clone();
            settings.set_managed_folder_for(&game, folder);
            map_err(settings.save().await)?;
            let mut shared = state.settings.write().await;
            *shared = settings.clone();
        }
    }

    Ok(settings)
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
        // switching games without updating the path: restore saved path for new game
        let stored = incoming.managed_folder_for(&incoming_game);
        incoming.managed_folder = stored;
    }

    if incoming.game != previous.game {
        // form was loaded for the previous game; preserve its catalog values first
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
    cache.response.api_enabled = if settings.game.is_silksong() {
        api_installed
    } else {
        installer::is_hk_api_enabled(&settings)
    };
    if !api_installed {
        cache.response.api.url = String::new();
    }

    Ok(cache.response)
}

#[tauri::command]
pub async fn install_mod(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog = map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    let result = installer::install_mod(&app, &settings, &mut installed, &catalog.response, &name).await;
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
pub async fn install_api(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog = map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    let result = installer::install_api(&app, &settings, &mut installed, &catalog.response).await;
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
    let settings = sync_managed_folder(state.settings.read().await.clone());
    if settings.managed_folder.trim().is_empty() {
        installer::write_install_log("Game launch failed: no managed folder is configured.");
        return Err("Game folder not configured. Go to Settings > Game to set it up.".to_string());
    }

    // refuse a second launch of the same game instead of letting the OS/game
    // handle it, since that just surfaces a confusing native dialog
    {
        let running = state.running_game.read().await;
        if running.as_ref() == Some(&settings.game) {
            return Err(format!(
                "{} is already running. Close it before launching again.",
                settings.game.display_name()
            ));
        }
    }

    let game_root = settings.game_root_path();
    if !game_root.is_dir() {
        installer::write_install_log(format!("Game launch failed: game root is not a directory: {}", game_root.display()));
        return Err(format!("Game folder is invalid: {}", game_root.display()));
    }

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

    let exe = exe_candidates.iter().find(|p| p.is_file()).ok_or_else(|| {
        let message = format!("Could not find game executable in {}", game_root.display());
        installer::write_install_log(format!("Game launch failed: {message}"));
        message
    })?.clone();

    if settings.game == GameKey::HollowKnight {
        // vanilla launch swaps Current for the pristine backup for the
        // process lifetime, then restores modded state after it exits
        if modded {
            installer::ensure_hk_api_enabled(&settings).await.map_err(|e| format_user_error(&e.to_string()))?;
            installer::write_install_log("Prepared Hollow Knight in modded/API-enabled state.");
        } else {
            installer::ensure_hk_api_disabled(&settings).await.map_err(|e| format_user_error(&e.to_string()))?;
            installer::write_install_log("Prepared Hollow Knight in vanilla state.");
        }
    }

    installer::write_install_log(format!("Launching {} from {} (modded={modded}).", exe.display(), game_root.display()));
    let child = Command::new(&exe)
        .current_dir(&game_root)
        // the game checks for this before falling back to "not launched
        // through Steam" self-restart behavior when its exe is run directly
        // instead of through the Steam client; this is the standard
        // Steamworks workaround for launchers that spawn the exe themselves
        .env("SteamAppId", settings.game.steam_app_id())
        .env("SteamGameId", settings.game.steam_app_id())
        .spawn()
        .map_err(|e| {
            let message = format!("Failed to launch game: {e}");
            installer::write_install_log(format!("Game launch failed: {message}"));
            message
        })?;

    let pid = child.id();
    {
        let mut running = state.running_game.write().await;
        *running = Some(settings.game.clone());
    }

    let restore_settings = settings.clone();
    let is_vanilla_hk = settings.game == GameKey::HollowKnight && !modded;
    let running_game = state.running_game.clone();
    tokio::spawn(async move {
        let mut child = child;
        let _ = tokio::task::spawn_blocking(move || child.wait()).await;

        if is_vanilla_hk {
            if let Err(error) = installer::ensure_hk_api_enabled(&restore_settings).await {
                installer::write_install_log(format!("Failed to restore Hollow Knight API state after vanilla launch: {error}"));
            } else {
                installer::write_install_log("Restored Hollow Knight to modded/API-enabled state after vanilla launch.");
            }
        }

        let mut running = running_game.write().await;
        *running = None;
    });

    let mode = if modded { "modded" } else { "vanilla" };
    Ok(format!("Launched {} ({mode}, process {pid}).", exe.file_name().unwrap_or_default().to_string_lossy()))
}
