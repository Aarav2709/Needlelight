pub mod backend;
pub mod commands;
mod logs_plugin;
mod process_plugin;
mod profile_create_plugin;
mod profile_plugin;

use backend::{
    installed_mods::InstalledModsStore,
    installer,
    settings::{AppSettings, GameKey},
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;

pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    pub installed: Arc<RwLock<InstalledModsStore>>,
    // tracks active launches by game key and process id
    pub running_games: Arc<RwLock<BTreeMap<String, u32>>>,
}

impl AppState {
    pub async fn new() -> Self {
        let mut settings = AppSettings::load().await.unwrap_or_default();
        settings.sync_managed_folder();
        settings.sync_custom_modlinks();
        let mut hk_settings = settings.clone();
        hk_settings.game = GameKey::HollowKnight;
        hk_settings.managed_folder =
            AppSettings::normalize_managed_folder(&settings.managed_folder_for(&GameKey::HollowKnight), &GameKey::HollowKnight);
        if let Err(error) = installer::recover_pending_hk_api_restore(&hk_settings).await {
            log::warn!("Could not recover pending Hollow Knight API restore: {error}");
        }
        let installed = InstalledModsStore::load(&settings)
            .await
            .unwrap_or_default();
        Self {
            settings: Arc::new(RwLock::new(settings)),
            installed: Arc::new(RwLock::new(installed)),
            running_games: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::async_runtime::block_on(async {
        let state = AppState::new().await;

        tauri::Builder::default()
            .plugin(tauri_plugin_log::Builder::new().build())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_window_state::Builder::new().build())
            .plugin(profile_plugin::init())
            .plugin(profile_create_plugin::init())
            .plugin(process_plugin::init())
            .plugin(logs_plugin::init())
            .manage(process_plugin::ProcessStore::default())
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                commands::load_settings,
                commands::save_settings,
                commands::auto_detect_managed_folder,
                commands::refresh_catalog,
                commands::install_mod,
                commands::uninstall_mod,
                commands::toggle_mod,
                commands::install_api,
                commands::parse_download_command,
                commands::list_packs,
                commands::save_pack,
                commands::load_pack,
                commands::import_pack,
                commands::launch_game,
            ])
            .run(tauri::generate_context!())
            .expect("failed to run tauri app");
    })
}
