pub mod backend;
pub mod commands;
mod logs_plugin;
mod process_plugin;
mod profile_create_plugin;
mod profile_plugin;

use backend::{installed_mods::InstalledModsStore, settings::{AppSettings, GameKey}};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    pub installed: Arc<RwLock<InstalledModsStore>>,
    // tracks the game Needlelight itself last spawned, so a second launch
    // click can't stack another process on top of one still running
    pub running_game: Arc<RwLock<Option<GameKey>>>,
}

impl AppState {
    pub async fn new() -> Self {
        let mut settings = AppSettings::load().await.unwrap_or_default();
        settings.sync_managed_folder();
        settings.sync_custom_modlinks();
        let installed = InstalledModsStore::load(&settings).await.unwrap_or_default();
        Self {
            settings: Arc::new(RwLock::new(settings)),
            installed: Arc::new(RwLock::new(installed)),
            running_game: Arc::new(RwLock::new(None)),
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
