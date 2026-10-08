pub mod backend;
pub mod commands;
mod profile_create_plugin;
mod profile_plugin;

use backend::{
    installer,
    settings::{AppSettings, GameKey},
};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    // running games by game key, with their process id (0 while a launch is starting)
    pub running_games: Arc<RwLock<BTreeMap<String, u32>>>,
    // games whose install folder was already auto detected this session
    pub auto_detected: Arc<RwLock<HashSet<String>>>,
    // serializes modpack edits so two operations never overwrite each other's installed mods file
    pub modpack_lock: Arc<tokio::sync::Mutex<()>>,
}

impl AppState {
    pub async fn new() -> Self {
        let mut settings = AppSettings::load().await.unwrap_or_default();
        settings.sync_managed_folder();
        settings.sync_custom_modlinks();
        let mut hk_settings = settings.clone();
        hk_settings.game = GameKey::HollowKnight;
        hk_settings.managed_folder = AppSettings::normalize_managed_folder(
            &settings.managed_folder_for(&GameKey::HollowKnight),
            &GameKey::HollowKnight,
        );
        if let Err(error) = installer::recover_pending_hk_api_restore(&hk_settings).await {
            log::warn!("Could not recover pending Hollow Knight API restore: {error}");
        }
        Self {
            settings: Arc::new(RwLock::new(settings)),
            running_games: Arc::new(RwLock::new(BTreeMap::new())),
            auto_detected: Arc::new(RwLock::new(HashSet::new())),
            modpack_lock: Arc::new(tokio::sync::Mutex::new(())),
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
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_window_state::Builder::new().build())
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                commands::load_settings,
                commands::save_settings,
                commands::auto_detect_managed_folder,
                commands::launch_game,
                commands::game_catalog,
                commands::modpack_installed,
                commands::modpack_install_mods,
                commands::modpack_uninstall_mod,
                commands::modpack_toggle_mod,
                commands::modpack_launch,
                commands::modpack_restore_original_mods,
                commands::modpack_active_hk,
                commands::open_app_folder,
                commands::game_availability,
                commands::game_folder_valid,
                commands::mod_readme,
                // modpack commands live here because tauri rejects inline plugins without permissions
                profile_plugin::profile_list,
                profile_plugin::profile_edit,
                profile_plugin::profile_remove,
                profile_create_plugin::profile_create,
                profile_create_plugin::profile_duplicate,
            ])
            .run(tauri::generate_context!())
            .expect("failed to run tauri app");
    })
}
