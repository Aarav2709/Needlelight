use crate::{
    backend::{
        errors::AppResult,
        installed_mods::InstalledModsStore,
        installer,
        mod_database::CatalogCache,
        modpacks::{self, LaunchExtras},
        profiles,
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

// turns low level errors into something a player can act on, the original text goes to the install log
fn friendly_error(message: &str) -> Option<String> {
    if message.starts_with("Network error") {
        installer::write_install_log(message);
        return Some(
            "Couldn't reach the download server. Check your internet connection and try again."
                .to_string(),
        );
    }
    if message.starts_with("Zip error") {
        installer::write_install_log(message);
        return Some("A download was damaged. Try again.".to_string());
    }
    if message.starts_with("Failed to parse") {
        installer::write_install_log(message);
        return Some("The mod catalog sent something unexpected. Try again later.".to_string());
    }
    if let Some(detail) = message.strip_prefix("I/O error: ") {
        installer::write_install_log(message);
        return Some(format!(
            "Couldn't update files on disk ({}).",
            detail.trim_end_matches('.')
        ));
    }
    None
}

// error messages are often lowercase log fragments, so capitalize them and end them with punctuation
fn format_user_error(message: &str) -> String {
    if let Some(friendly) = friendly_error(message.trim()) {
        return friendly;
    }
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "Something went wrong.".to_string();
    }
    let mut chars = trimmed.chars();
    let mut out: String = chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_default();
    out.push_str(chars.as_str());
    if !out.ends_with('.') && !out.ends_with('!') && !out.ends_with('?') && !out.ends_with(':') {
        out.push('.');
    }
    out
}

// applies the legacy folder and catalog migrations to a settings copy
fn sync_managed_folder(mut settings: AppSettings) -> AppSettings {
    settings.sync_managed_folder();
    settings.sync_custom_modlinks();
    settings
}

async fn ensure_no_running_games(state: &State<'_, AppState>) -> Result<(), String> {
    let running = state.running_games.read().await;
    if running.is_empty() {
        return Ok(());
    }
    let active = running
        .keys()
        .map(|key| {
            GameKey::from_key(key)
                .map(|g| g.display_name())
                .unwrap_or("The game")
        })
        .collect::<Vec<_>>()
        .join(" and ");
    Err(format!("Close {active} before changing mods."))
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let mut settings = sync_managed_folder(state.settings.read().await.clone());

    // finds the game on first load if no folder is set, at most once per game per session since it can walk whole drives
    let first_attempt = settings.managed_folder.trim().is_empty()
        && state
            .auto_detected
            .write()
            .await
            .insert(settings.game.as_str().to_string());
    if first_attempt {
        if let Some(path) = map_err(AppSettings::auto_detect(&settings.game).await)? {
            settings.managed_folder = AppSettings::normalize_managed_folder(&path, &settings.game);
            let game = settings.game.clone();
            let folder = settings.managed_folder.clone();
            settings.set_managed_folder_for(&game, folder);
            map_err(settings.save().await)?;
            {
                let mut shared = state.settings.write().await;
                *shared = settings.clone();
            }
            let reloaded = map_err(InstalledModsStore::load(&settings).await)?;
            let mut installed = state.installed.write().await;
            *installed = reloaded;
        }
    }

    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
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
        // switching games without a new path, so restore the saved path for the new game
        let stored = incoming.managed_folder_for(&incoming_game);
        incoming.managed_folder = stored;
    }

    if incoming.game != previous.game {
        // the form was loaded for the previous game, so keep its catalog values under that game
        incoming.custom_modlinks_by_game.insert(
            previous.game.as_str().to_string(),
            crate::backend::settings::CustomModlinksConfig {
                enabled: incoming.use_custom_modlinks,
                uri: incoming.custom_modlinks_uri.clone(),
            },
        );
        incoming.sync_custom_modlinks();
    }

    incoming.managed_folder =
        AppSettings::normalize_managed_folder(&incoming.managed_folder, &incoming_game);
    let folder = incoming.managed_folder.clone();
    incoming.set_managed_folder_for(&incoming_game, folder);
    incoming.set_custom_modlinks_for_current();

    map_err(incoming.save().await)?;

    *state.settings.write().await = incoming.clone();

    let reloaded = map_err(InstalledModsStore::load(&incoming).await)?;
    *state.installed.write().await = reloaded;

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
pub async fn install_mod(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    ensure_no_running_games(&state).await?;
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog =
        map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    let result =
        installer::install_mod(&app, &settings, &mut installed, &catalog.response, &name).await;
    if let Err(error) = &result {
        installer::write_install_log(format!("Mod install failed for {name}: {error}"));
    }
    map_err(result)
}

#[tauri::command]
pub async fn uninstall_mod(state: State<'_, AppState>, name: String) -> Result<(), String> {
    ensure_no_running_games(&state).await?;
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    map_err(installer::uninstall_mod(&settings, &mut installed, &name).await)
}

#[tauri::command]
pub async fn toggle_mod(
    state: State<'_, AppState>,
    name: String,
    enable: bool,
) -> Result<(), String> {
    ensure_no_running_games(&state).await?;
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    map_err(installer::toggle_mod(&settings, &mut installed, &name, enable).await)
}

#[tauri::command]
pub async fn install_api(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    ensure_no_running_games(&state).await?;
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog =
        map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    let result = installer::install_api(&app, &settings, &mut installed, &catalog.response).await;
    if let Err(error) = &result {
        installer::write_install_log(format!("Modding API install failed: {error}"));
    }
    map_err(result)
}

#[tauri::command]
pub async fn parse_download_command(
    raw: String,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let (_, data) = url_scheme::decode_command(&raw);
    map_err(url_scheme::parse_download_command(&data))
}

#[tauri::command]
pub async fn launch_game(state: State<'_, AppState>, modded: bool) -> Result<String, String> {
    let settings = sync_managed_folder(state.settings.read().await.clone());
    launch_with_settings(&state, settings, modded, LaunchExtras::default()).await
}

// the game's executable inside its install folder, if the game is there
fn find_game_exe(settings: &AppSettings) -> Option<std::path::PathBuf> {
    if settings.managed_folder.trim().is_empty() {
        return None;
    }
    let game_root = settings.game_root_path();
    if !game_root.is_dir() {
        return None;
    }
    let candidates: Vec<std::path::PathBuf> = match settings.game {
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
    candidates.into_iter().find(|path| path.is_file())
}

// drops a launch reservation (pid 0) that never turned into a running process
async fn release_launch_reservation(state: &State<'_, AppState>, game_key: &str) {
    let mut running = state.running_games.write().await;
    if running.get(game_key).copied() == Some(0) {
        running.remove(game_key);
    }
}

// launches the game in settings, modpack launches pass extra doorstop arguments through extras
async fn launch_with_settings(
    state: &State<'_, AppState>,
    settings: AppSettings,
    modded: bool,
    extras: LaunchExtras,
) -> Result<String, String> {
    let not_found = format!(
        "{} wasn't found on this computer. Open Settings → Games to locate it.",
        settings.game.display_name()
    );
    if settings.managed_folder.trim().is_empty() {
        installer::write_install_log("Game launch failed: no managed folder is configured.");
        return Err(not_found);
    }

    let game_root = settings.game_root_path();
    let exe = find_game_exe(&settings).ok_or_else(|| {
        installer::write_install_log(format!(
            "Game launch failed: no game executable in {}",
            game_root.display()
        ));
        not_found.clone()
    })?;

    let game_key = settings.game.as_str().to_string();
    {
        let mut running = state.running_games.write().await;
        if running.contains_key(&game_key) {
            return Err(format!(
                "{} is already running. Close it before launching again.",
                settings.game.display_name()
            ));
        }
        // pid 0 reserves the launch so concurrent calls can't race between this check and the spawn
        running.insert(game_key.clone(), 0);
    }

    let is_vanilla_hk = settings.game == GameKey::HollowKnight && !modded;
    if settings.game == GameKey::HollowKnight {
        // a vanilla launch swaps in the original files while the game runs and restores the api after it exits
        let prepared = if modded {
            installer::ensure_hk_api_enabled(&settings).await
        } else {
            match installer::ensure_hk_api_disabled(&settings).await {
                Ok(()) => installer::mark_pending_hk_api_restore(&settings).await,
                Err(error) => Err(error),
            }
        };
        if let Err(error) = prepared {
            release_launch_reservation(state, &game_key).await;
            return Err(format_user_error(&error.to_string()));
        }
        installer::write_install_log(if modded {
            "Prepared Hollow Knight in modded/API-enabled state."
        } else {
            "Prepared Hollow Knight in vanilla state."
        });
    }

    installer::write_install_log(format!(
        "Launching {} from {} (modded={modded}).",
        exe.display(),
        game_root.display()
    ));
    let child = match Command::new(&exe)
        .current_dir(&game_root)
        // the standard steamworks workaround that stops the game restarting itself through steam
        .env("SteamAppId", settings.game.steam_app_id())
        .env("SteamGameId", settings.game.steam_app_id())
        .args(&extras.args)
        .envs(
            extras
                .envs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let message = format!("Failed to launch game: {e}");
            installer::write_install_log(format!("Game launch failed: {message}"));
            if is_vanilla_hk {
                if let Err(error) = installer::ensure_hk_api_enabled(&settings).await {
                    installer::write_install_log(format!(
                        "Failed to restore Hollow Knight API state after launch failure: {error}"
                    ));
                }
                if let Err(error) = installer::clear_pending_hk_api_restore(&settings).await {
                    installer::write_install_log(format!(
                        "Failed to clear pending Hollow Knight restore marker: {error}"
                    ));
                }
            }
            release_launch_reservation(state, &game_key).await;
            return Err(message);
        }
    };

    let pid = child.id();
    state
        .running_games
        .write()
        .await
        .insert(game_key.clone(), pid);

    let restore_settings = settings.clone();
    let running_games = state.running_games.clone();
    let running_key = game_key.clone();
    tokio::spawn(async move {
        let mut child = child;
        let _ = tokio::task::spawn_blocking(move || child.wait()).await;

        if is_vanilla_hk {
            if let Err(error) = installer::ensure_hk_api_enabled(&restore_settings).await {
                installer::write_install_log(format!(
                    "Failed to restore Hollow Knight API state after vanilla launch: {error}"
                ));
            } else {
                installer::write_install_log(
                    "Restored Hollow Knight to modded/API-enabled state after vanilla launch.",
                );
            }
            if let Err(error) = installer::clear_pending_hk_api_restore(&restore_settings).await {
                installer::write_install_log(format!(
                    "Failed to clear pending Hollow Knight restore marker: {error}"
                ));
            }
        }

        let mut running = running_games.write().await;
        if running.get(&running_key).copied() == Some(pid) {
            running.remove(&running_key);
        }
    });

    let mode = if modded { "modded" } else { "vanilla" };
    Ok(format!(
        "Launched {} ({mode}, process {pid}).",
        exe.file_name().unwrap_or_default().to_string_lossy()
    ))
}

// modpacks are profile folders that the regular installer works in, see backend modpacks for launching

fn modpack_dir(path: &str) -> Result<std::path::PathBuf, String> {
    let dir = std::path::PathBuf::from(path);
    if !dir.is_dir() {
        return Err("This modpack no longer exists.".to_string());
    }
    Ok(dir)
}

// settings rooted at the modpack, the modpack's own installed mods store, and its game
async fn modpack_context(
    state: &State<'_, AppState>,
    path: &str,
) -> Result<(std::path::PathBuf, AppSettings, InstalledModsStore, GameKey), String> {
    let dir = modpack_dir(path)?;
    let meta = map_err(profiles::load_profile_meta(&dir))?;
    let base = state.settings.read().await.clone();
    let settings = modpacks::profile_settings(&base, &dir, &meta.game);
    let installed = map_err(InstalledModsStore::load(&settings).await)?;
    Ok((dir, settings, installed, meta.game))
}

// reloads the global installed mods store when the game is active so the app sees changes to its real mods folder
async fn reload_active_store(state: &State<'_, AppState>, game: &GameKey) -> Result<(), String> {
    let base = sync_managed_folder(state.settings.read().await.clone());
    if &base.game == game {
        let store = map_err(InstalledModsStore::load(&base).await)?;
        *state.installed.write().await = store;
    }
    Ok(())
}

// silksong modpacks carry their own bepinex, so reinstall it if it went missing
async fn ensure_modpack_bepinex(
    settings: &AppSettings,
    dir: &std::path::Path,
    game: &GameKey,
) -> Result<(), String> {
    if game.is_silksong() && modpacks::silksong_preloader(dir).is_none() {
        crate::profile_create_plugin::install_bepinex_pack(settings, dir).await?;
    }
    Ok(())
}

// one game's catalog with nothing installed, the ui merges in each modpack's state so it is fetched once
#[tauri::command]
pub async fn game_catalog(
    state: State<'_, AppState>,
    game: GameKey,
) -> Result<crate::backend::models::CatalogResponse, String> {
    let base = state.settings.read().await.clone();
    let settings = modpacks::game_settings(&base, &game);
    let fetch_official = !settings.use_custom_modlinks;
    let cache = map_err(
        CatalogCache::build(&settings, &InstalledModsStore::default(), fetch_official).await,
    )?;
    // catalog sources return an empty list on failure, so report it and let the ui offer a retry
    if cache.response.items.is_empty() {
        return Err(format!(
            "Couldn't load the {} mod catalog. Check your internet connection and try again.",
            game.display_name()
        ));
    }
    Ok(cache.response)
}

// a modpack's installed mods, reconciled with what is actually on disk
#[tauri::command]
pub async fn modpack_installed(
    state: State<'_, AppState>,
    path: String,
) -> Result<crate::backend::models::PersistedInstalled, String> {
    let (_, _, installed, _) = modpack_context(&state, &path).await?;
    Ok(installed.db)
}

// the catalog for a modpack's game with install state from that modpack
#[tauri::command]
pub async fn modpack_catalog(
    state: State<'_, AppState>,
    path: String,
) -> Result<crate::backend::models::CatalogResponse, String> {
    let (_, settings, installed, _) = modpack_context(&state, &path).await?;
    let fetch_official = !settings.use_custom_modlinks;
    let cache = map_err(CatalogCache::build(&settings, &installed, fetch_official).await)?;
    Ok(cache.response)
}

// installs or updates a mod and its dependencies into a modpack
#[tauri::command]
pub async fn modpack_install_mod(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> Result<(), String> {
    modpack_install_mods(app, state, path, vec![name]).await
}

// installs or updates several mods and their dependencies into a modpack with one catalog fetch
#[tauri::command]
pub async fn modpack_install_mods(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    names: Vec<String>,
) -> Result<(), String> {
    let _guard = state.modpack_lock.lock().await;
    let (dir, settings, mut installed, game) = modpack_context(&state, &path).await?;
    ensure_modpack_bepinex(&settings, &dir, &game).await?;
    let fetch_official = !settings.use_custom_modlinks;
    let catalog = map_err(CatalogCache::build(&settings, &installed, fetch_official).await)?;
    let mut visited = std::collections::HashSet::new();
    for name in &names {
        let result = installer::install_mod_with_deps(
            &app,
            &settings,
            &mut installed,
            &catalog.response,
            name,
            &mut visited,
        )
        .await;
        if let Err(error) = &result {
            installer::write_install_log(format!("Modpack install failed for {name}: {error}"));
            return map_err(result);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn modpack_uninstall_mod(
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> Result<(), String> {
    let _guard = state.modpack_lock.lock().await;
    let (_, settings, mut installed, _) = modpack_context(&state, &path).await?;
    map_err(installer::uninstall_mod(&settings, &mut installed, &name).await)
}

#[tauri::command]
pub async fn modpack_toggle_mod(
    state: State<'_, AppState>,
    path: String,
    name: String,
    enable: bool,
) -> Result<(), String> {
    let _guard = state.modpack_lock.lock().await;
    let (_, settings, mut installed, _) = modpack_context(&state, &path).await?;
    map_err(installer::toggle_mod(&settings, &mut installed, &name, enable).await)
}

// launches a modpack, silksong runs it in place through doorstop and hollow knight mirrors its mods first
#[tauri::command]
pub async fn modpack_launch(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    ensure_no_running_games(&state).await?;
    let _guard = state.modpack_lock.lock().await;
    let (dir, profile, _, game) = modpack_context(&state, &path).await?;
    let base = state.settings.read().await.clone();
    let real = modpacks::game_settings(&base, &game);
    if real.managed_folder.trim().is_empty() {
        return Err(format!(
            "{} wasn't found on this computer. Open Settings → Games to locate it.",
            game.display_name()
        ));
    }

    let extras = if game.is_silksong() {
        ensure_modpack_bepinex(&profile, &dir, &game).await?;
        map_err(modpacks::silksong_launch_extras(&real, &dir))?
    } else {
        let mut store = map_err(InstalledModsStore::load(&real).await)?;
        if !installer::is_api_installed(&real, &store) {
            let catalog =
                map_err(CatalogCache::build(&real, &store, !real.use_custom_modlinks).await)?;
            map_err(installer::install_api(&app, &real, &mut store, &catalog.response).await)?;
        }
        map_err(modpacks::apply_hk_modpack(&real, &dir))?;
        reload_active_store(&state, &game).await?;
        LaunchExtras::default()
    };

    let mut meta = map_err(profiles::load_profile_meta(&dir))?;
    meta.last_played = Some(chrono::Utc::now());
    map_err(profiles::save_profile_meta(&dir, &meta))?;

    launch_with_settings(&state, real, true, extras).await
}

// hollow knight only, puts back the mods the game had before the first modpack
#[tauri::command]
pub async fn modpack_restore_original_mods(state: State<'_, AppState>) -> Result<bool, String> {
    ensure_no_running_games(&state).await?;
    let base = state.settings.read().await.clone();
    let real = modpacks::game_settings(&base, &GameKey::HollowKnight);
    let restored = map_err(modpacks::restore_hk_original(&real))?;
    reload_active_store(&state, &GameKey::HollowKnight).await?;
    Ok(restored)
}

// hollow knight only, the modpack currently applied to the game folder
#[tauri::command]
pub async fn modpack_active_hk() -> Result<Option<String>, String> {
    Ok(modpacks::active_hk_modpack())
}

#[derive(serde::Serialize)]
pub struct AppPaths {
    pub config_dir: String,
    pub install_log: String,
    pub profiles_dir: String,
}

// where needlelight keeps its data, for the show in folder actions in settings
#[tauri::command]
pub async fn app_paths() -> Result<AppPaths, String> {
    let config = map_err(AppSettings::config_dir())?;
    Ok(AppPaths {
        config_dir: config.to_string_lossy().to_string(),
        install_log: config
            .join("Needlelight-install.log")
            .to_string_lossy()
            .to_string(),
        profiles_dir: config.join("profiles").to_string_lossy().to_string(),
    })
}

#[derive(serde::Serialize)]
pub struct GameAvailability {
    pub game: GameKey,
    // the game was found at its saved location
    pub found: bool,
}

// whether each supported game is installed where needlelight expects it
#[tauri::command]
pub async fn game_availability(
    state: State<'_, AppState>,
) -> Result<Vec<GameAvailability>, String> {
    let base = state.settings.read().await.clone();
    Ok([GameKey::HollowKnight, GameKey::Silksong]
        .into_iter()
        .map(|game| {
            let settings = modpacks::game_settings(&base, &game);
            let found = find_game_exe(&settings).is_some();
            GameAvailability { game, found }
        })
        .collect())
}

// checks that a folder the player picked really contains the game
#[tauri::command]
pub async fn game_folder_valid(
    state: State<'_, AppState>,
    game: GameKey,
    folder: String,
) -> Result<bool, String> {
    let mut settings = modpacks::game_settings(&state.settings.read().await.clone(), &game);
    settings.managed_folder = AppSettings::normalize_managed_folder(&folder, &game);
    Ok(find_game_exe(&settings).is_some())
}

#[derive(serde::Serialize)]
pub struct ModReadme {
    pub markdown: String,
    // base for resolving relative image paths in the markdown
    pub image_base: Option<String>,
    // base for resolving relative links in the markdown
    pub link_base: Option<String>,
}

// a mod's readme from thunderstore or its github repository, none when it has no readable one
#[tauri::command]
pub async fn mod_readme(url: String, version: String) -> Result<Option<ModReadme>, String> {
    let client = map_err(installer::http_client())?;
    let timeout = std::time::Duration::from_secs(20);

    if let Some(rest) = url.strip_prefix("https://thunderstore.io/c/") {
        // package pages look like thunderstore.io/c/community/p/owner/name
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() < 4 || parts[1] != "p" || version.trim().is_empty() {
            return Ok(None);
        }
        let api = format!(
            "https://thunderstore.io/api/experimental/package/{}/{}/{}/readme/",
            parts[2],
            parts[3],
            version.trim()
        );
        #[derive(serde::Deserialize)]
        struct Readme {
            markdown: String,
        }
        let response = client.get(&api).timeout(timeout).send().await;
        let Ok(response) = response else {
            return Ok(None);
        };
        if !response.status().is_success() {
            return Ok(None);
        }
        return Ok(response
            .json::<Readme>()
            .await
            .ok()
            .filter(|r| !r.markdown.trim().is_empty())
            .map(|r| ModReadme {
                markdown: r.markdown,
                image_base: None,
                link_base: None,
            }));
    }

    if let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            return Ok(None);
        }
        let owner = parts[0];
        let repo = parts[1].trim_end_matches(".git");
        let raw_base = format!("https://raw.githubusercontent.com/{owner}/{repo}/HEAD/");
        for file in ["README.md", "readme.md", "Readme.md", "README.MD", "README"] {
            let response = client
                .get(format!("{raw_base}{file}"))
                .timeout(timeout)
                .send()
                .await;
            let Ok(response) = response else {
                return Ok(None);
            };
            if !response.status().is_success() {
                continue;
            }
            let Ok(markdown) = response.text().await else {
                return Ok(None);
            };
            if markdown.trim().is_empty() {
                return Ok(None);
            }
            return Ok(Some(ModReadme {
                markdown,
                image_base: Some(raw_base.clone()),
                link_base: Some(format!("https://github.com/{owner}/{repo}/blob/HEAD/")),
            }));
        }
    }

    Ok(None)
}
