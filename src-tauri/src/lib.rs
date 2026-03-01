pub mod backend;
pub mod commands;

use backend::{installed_mods::InstalledModsStore, settings::AppSettings};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub settings: Arc<RwLock<AppSettings>>,
    pub installed: Arc<RwLock<InstalledModsStore>>,
}

impl AppState {
    pub async fn new() -> Self {
        let settings = AppSettings::load().await.unwrap_or_default();
        let installed = InstalledModsStore::load(&settings).await.unwrap_or_default();
        Self {
            settings: Arc::new(RwLock::new(settings)),
            installed: Arc::new(RwLock::new(installed)),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::async_runtime::block_on(async {
        let state = AppState::new().await;

        tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_os::init())
            .plugin(tauri_plugin_window_state::Builder::new().build())
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
