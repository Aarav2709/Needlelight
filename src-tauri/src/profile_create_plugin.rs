use crate::{
    backend::{
        mod_database::CatalogCache,
        modpacks::copy_path,
        profiles::{self, GameInstance, ProfileMeta},
        settings::GameKey,
    },
    profile_plugin::resolve_profile_dir,
    AppState,
};
use chrono::Utc;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn profile_create(
    state: State<'_, AppState>,
    name: String,
    game: GameKey,
    description: Option<String>,
) -> Result<GameInstance, String> {
    let root = profiles::profiles_root(&game).map_err(|e| e.to_string())?;
    profiles::ensure_profile_dir(&root).map_err(|e| e.to_string())?;

    let display_name = name.trim().to_string();
    let safe_name = profiles::sanitize_profile_name(&display_name);
    let profile_dir = root.join(&safe_name);
    if profile_dir.exists() {
        return Err("A modpack with this name already exists.".to_string());
    }

    profiles::ensure_profile_dir(&profile_dir).map_err(|e| e.to_string())?;

    if game.is_silksong() {
        // resolve the loader for the modpack's game, not whichever game is active
        let base = state.settings.read().await.clone();
        let settings = crate::backend::modpacks::game_settings(&base, &game);
        if let Err(error) = install_bepinex_pack(&settings, &profile_dir).await {
            // don't leave a half created modpack behind
            let _ = std::fs::remove_dir_all(&profile_dir);
            return Err(error);
        }
    } else {
        std::fs::create_dir_all(profile_dir.join("Mods")).map_err(|e| e.to_string())?;
    }

    let now = Utc::now();
    let meta = ProfileMeta {
        name: display_name,
        game,
        description: description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty()),
        groups: vec![],
        created: now,
        modified: now,
        last_played: None,
        icon_file: None,
    };

    profiles::save_profile_meta(&profile_dir, &meta).map_err(|e| e.to_string())?;

    Ok(profiles::profile_to_instance(&profile_dir, &meta))
}

#[tauri::command]
pub async fn profile_duplicate(
    state: State<'_, AppState>,
    path: String,
) -> Result<GameInstance, String> {
    let _guard = state.modpack_lock.lock().await;
    let settings = state.settings.read().await.clone();
    let source_dir = resolve_profile_dir(&settings, &path);
    let meta = profiles::load_profile_meta(&source_dir).map_err(|e| e.to_string())?;

    let root = profiles::profiles_root(&meta.game).map_err(|e| e.to_string())?;
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

    copy_path(&source_dir, &candidate).map_err(|e| e.to_string())?;

    let now = Utc::now();
    let mut new_meta = meta.clone();
    new_meta.name = display_name;
    new_meta.created = now;
    new_meta.modified = now;
    new_meta.last_played = None;

    profiles::save_profile_meta(&candidate, &new_meta).map_err(|e| e.to_string())?;

    Ok(profiles::profile_to_instance(&candidate, &new_meta))
}

pub(crate) async fn install_bepinex_pack(
    settings: &crate::backend::settings::AppSettings,
    profile_dir: &Path,
) -> Result<(), String> {
    let cache = CatalogCache::build(
        settings,
        &crate::backend::installed_mods::InstalledModsStore::default(),
        true,
    )
    .await
    .map_err(|e| e.to_string())?;
    let api = cache.response.api;
    if api.url.trim().is_empty() {
        return Err(
            "Couldn't download the mod loader for Hollow Knight: Silksong. Check your internet \
             connection and try again."
                .to_string(),
        );
    }

    let client = crate::backend::installer::http_client().map_err(|e| e.to_string())?;
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

    // thunderstore packs nest everything in a wrapper folder, the installer's extractor strips it so bepinex lands in the modpack root
    crate::backend::installer::extract_zip_guarded(bytes.as_ref(), profile_dir, &["BepInEx"])
        .map_err(|e| e.to_string())?;
    Ok(())
}
