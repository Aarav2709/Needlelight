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
use tauri::State;

fn map_err<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    let settings = settings.normalized();
    map_err(settings.save().await)?;

    {
        let mut shared = state.settings.write().await;
        *shared = settings.clone();
    }

    let reloaded = map_err(crate::backend::installed_mods::InstalledModsStore::load(&settings).await)?;
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

    let cache = map_err(CatalogCache::build(&settings, &installed, fetch_official).await)?;
    Ok(cache.response)
}

#[tauri::command]
pub async fn install_mod(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let settings = state.settings.read().await.clone();
    let mut installed = state.installed.write().await;
    let catalog = map_err(CatalogCache::build(&settings, &installed, !settings.use_custom_modlinks).await)?;

    map_err(installer::install_mod(&settings, &mut installed, &catalog.response, &name).await)
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

    map_err(installer::install_api(&settings, &mut installed, &catalog.response).await)
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
