use super::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

// how deep the fallback drive scan looks for the game's managed folder
const SCAN_MAX_DEPTH: usize = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameKey {
    #[default]
    HollowKnight,
    Silksong,
}

impl GameKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameKey::HollowKnight => "hollow_knight",
            GameKey::Silksong => "silksong",
        }
    }

    pub fn is_silksong(&self) -> bool {
        matches!(self, GameKey::Silksong)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GameKey::HollowKnight => "Hollow Knight",
            GameKey::Silksong => "Hollow Knight: Silksong",
        }
    }

    pub fn from_key(key: &str) -> Option<GameKey> {
        match key {
            "hollow_knight" => Some(GameKey::HollowKnight),
            "silksong" => Some(GameKey::Silksong),
            _ => None,
        }
    }

    pub fn steam_app_id(&self) -> &'static str {
        match self {
            GameKey::HollowKnight => "367520",
            GameKey::Silksong => "1030300",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub managed_folder: String,
    pub game: GameKey,
    #[serde(default)]
    pub managed_folders: HashMap<String, String>,
    #[serde(default)]
    pub use_custom_modlinks: bool,
    #[serde(default)]
    pub custom_modlinks_uri: String,
    // per game custom catalogs, the two fields above mirror the selected game for the frontend and old configs
    #[serde(default)]
    pub custom_modlinks_by_game: HashMap<String, CustomModlinksConfig>,
    #[serde(default)]
    pub use_github_mirror: bool,
    #[serde(default)]
    pub github_mirror_format: String,
    #[serde(default)]
    pub low_storage_mode: bool,
    // runtime only override for the installed mods database, modpacks point it inside their own folder
    #[serde(skip)]
    pub installed_db_override: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomModlinksConfig {
    pub enabled: bool,
    pub uri: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            managed_folder: String::new(),
            game: GameKey::HollowKnight,
            managed_folders: HashMap::new(),
            use_custom_modlinks: false,
            custom_modlinks_uri: String::new(),
            custom_modlinks_by_game: HashMap::new(),
            use_github_mirror: false,
            github_mirror_format: String::new(),
            low_storage_mode: false,
            installed_db_override: None,
        }
    }
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

impl AppSettings {
    fn game_data_dir_name(game: &GameKey) -> &'static str {
        match game {
            GameKey::HollowKnight => "Hollow Knight_Data",
            GameKey::Silksong => "Hollow Knight Silksong_Data",
        }
    }

    pub fn normalize_managed_folder(raw: &str, game: &GameKey) -> String {
        let raw = raw.trim();
        if raw.is_empty() {
            return String::new();
        }
        let path = PathBuf::from(raw);

        if file_name(&path) == Some("Managed") {
            return path.to_string_lossy().to_string();
        }

        if file_name(&path).is_some_and(|name| name.ends_with("_Data")) {
            return path.join("Managed").to_string_lossy().to_string();
        }

        let data_candidate = path.join(Self::game_data_dir_name(game));
        if data_candidate.exists() {
            return data_candidate.join("Managed").to_string_lossy().to_string();
        }

        path.to_string_lossy().to_string()
    }

    pub fn game_root_path(&self) -> PathBuf {
        let managed = PathBuf::from(&self.managed_folder);

        if file_name(&managed) == Some("Managed") {
            if let Some(data_folder) = managed.parent() {
                // windows and linux keep it in the game folder under name_data/managed
                if file_name(data_folder).is_some_and(|name| name.ends_with("_Data")) {
                    if let Some(root) = data_folder.parent() {
                        return root.to_path_buf();
                    }
                }
                // macos keeps it inside the app bundle under contents/resources/data/managed
                if file_name(data_folder) == Some("Data") {
                    let app = data_folder
                        .parent()
                        .filter(|resources| file_name(resources) == Some("Resources"))
                        .and_then(Path::parent)
                        .filter(|contents| file_name(contents) == Some("Contents"))
                        .and_then(Path::parent)
                        .filter(|app| file_name(app).is_some_and(|name| name.ends_with(".app")));
                    if let Some(root) = app.and_then(Path::parent) {
                        return root.to_path_buf();
                    }
                }
            }
        }

        if file_name(&managed).is_some_and(|name| name.ends_with("_Data")) {
            if let Some(root) = managed.parent() {
                return root.to_path_buf();
            }
        }

        managed
    }

    pub fn normalized(&self) -> Self {
        let mut copy = self.clone();
        copy.managed_folder = Self::normalize_managed_folder(&self.managed_folder, &self.game);
        copy
    }

    // routes a github download through the mirror, the format holds {url} or is a prefix for the url
    pub fn mirrored_url(&self, url: &str) -> String {
        let format = self.github_mirror_format.trim();
        if !self.use_github_mirror || format.is_empty() {
            return url.to_string();
        }
        let is_github = [
            "https://github.com/",
            "https://raw.githubusercontent.com/",
            "https://objects.githubusercontent.com/",
            "https://codeload.github.com/",
        ]
        .iter()
        .any(|prefix| url.starts_with(prefix));
        if !is_github {
            return url.to_string();
        }
        if format.contains("{url}") {
            format.replace("{url}", url)
        } else {
            format!("{}/{}", format.trim_end_matches('/'), url)
        }
    }

    pub fn managed_folder_for(&self, game: &GameKey) -> String {
        self.managed_folders
            .get(game.as_str())
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_managed_folder_for(&mut self, game: &GameKey, path: String) {
        self.managed_folders.insert(game.as_str().to_string(), path);
    }

    pub fn sync_managed_folder(&mut self) {
        if self.managed_folders.is_empty() && !self.managed_folder.is_empty() {
            let game = self.game.clone();
            let folder = self.managed_folder.clone();
            self.set_managed_folder_for(&game, folder);
        }

        let stored = self.managed_folder_for(&self.game);
        if !stored.is_empty() {
            self.managed_folder = stored;
        }
    }

    pub fn sync_custom_modlinks(&mut self) {
        if self.custom_modlinks_by_game.is_empty()
            && (self.use_custom_modlinks || !self.custom_modlinks_uri.trim().is_empty())
        {
            self.set_custom_modlinks_for_current();
        }

        if let Some(config) = self.custom_modlinks_by_game.get(self.game.as_str()) {
            self.use_custom_modlinks = config.enabled;
            self.custom_modlinks_uri = config.uri.clone();
        } else {
            self.use_custom_modlinks = false;
            self.custom_modlinks_uri.clear();
        }
    }

    pub fn set_custom_modlinks_for_current(&mut self) {
        self.custom_modlinks_by_game.insert(
            self.game.as_str().to_string(),
            CustomModlinksConfig {
                enabled: self.use_custom_modlinks,
                uri: self.custom_modlinks_uri.clone(),
            },
        );
    }

    pub fn config_dir() -> AppResult<PathBuf> {
        let unresolved = || AppError::InvalidInput("cannot resolve config directory".to_string());

        if cfg!(target_os = "windows") {
            return std::env::var("APPDATA")
                .or_else(|_| std::env::var("LOCALAPPDATA"))
                .map(|base| PathBuf::from(base).join("HKModInstaller"))
                .map_err(|_| unresolved());
        }

        if cfg!(target_os = "macos") {
            return std::env::var("HOME")
                .map(|home| {
                    Path::new(&home)
                        .join("Library")
                        .join("Application Support")
                        .join("HKModInstaller")
                })
                .map_err(|_| unresolved());
        }

        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .map(|home| Path::new(&home).join(".config"))
                    .map_err(|_| unresolved())
            })?;

        Ok(base.join("HKModInstaller"))
    }

    pub fn config_path() -> AppResult<PathBuf> {
        Ok(Self::config_dir()?.join("HKInstallerSettings.json"))
    }

    pub fn mods_folder(&self) -> PathBuf {
        if self.game.is_silksong() {
            return self.game_root_path().join("BepInEx").join("plugins");
        }
        PathBuf::from(&self.managed_folder).join("Mods")
    }

    pub fn disabled_folder(&self) -> PathBuf {
        self.mods_folder().join("Disabled")
    }

    pub fn installed_mods_path(&self) -> AppResult<PathBuf> {
        if let Some(path) = &self.installed_db_override {
            return Ok(path.clone());
        }
        Ok(Self::config_dir()?.join(format!("InstalledMods.{}.json", self.game.as_str())))
    }

    pub async fn load() -> AppResult<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = tokio::fs::read_to_string(path).await?;
        let parsed = serde_json::from_str::<Self>(&content)?;
        Ok(parsed)
    }

    pub async fn save(&self) -> AppResult<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub async fn auto_detect(game: &GameKey) -> AppResult<Option<String>> {
        // the search touches the disk a lot, so it runs off the async runtime
        let game = game.clone();
        tokio::task::spawn_blocking(move || Self::detect_blocking(&game))
            .await
            .map_err(|error| AppError::InvalidInput(format!("game search failed: {error}")))
    }

    fn detect_blocking(game: &GameKey) -> Option<String> {
        let data_dir = Self::game_data_dir_name(game);

        let candidates: Vec<PathBuf> = if cfg!(target_os = "windows") {
            let windows_paths = match game {
                GameKey::HollowKnight => vec![
                    "Program Files/Steam/steamapps/common/Hollow Knight",
                    "Program Files (x86)/Steam/steamapps/common/Hollow Knight",
                    "SteamLibrary/steamapps/common/Hollow Knight",
                    "Program Files/GOG Galaxy/Games/Hollow Knight",
                    "Program Files (x86)/GOG Galaxy/Games/Hollow Knight",
                    "XboxGames/Hollow Knight/Content",
                    "Steam/steamapps/common/Hollow Knight",
                    "GOG Galaxy/Games/Hollow Knight",
                ],
                GameKey::Silksong => vec![
                    "Program Files/Steam/steamapps/common/Hollow Knight Silksong",
                    "Program Files (x86)/Steam/steamapps/common/Hollow Knight Silksong",
                    "SteamLibrary/steamapps/common/Hollow Knight Silksong",
                    "Program Files/GOG Galaxy/Games/Hollow Knight Silksong",
                    "Program Files (x86)/GOG Galaxy/Games/Hollow Knight Silksong",
                    "XboxGames/Hollow Knight Silksong/Content",
                    "XboxGames/Hollow Knight- Silksong/Content",
                    "Steam/steamapps/common/Hollow Knight Silksong",
                    "GOG Galaxy/Games/Hollow Knight Silksong",
                ],
            };

            Self::drive_roots()
                .into_iter()
                .flat_map(|root| {
                    windows_paths
                        .iter()
                        .map(move |relative| root.join(relative).join(data_dir).join("Managed"))
                })
                .collect()
        } else if cfg!(target_os = "macos") {
            let home = std::env::var("HOME").unwrap_or_default();
            let (folder, app_name) = match game {
                GameKey::HollowKnight => ("Hollow Knight", "Hollow Knight.app"),
                GameKey::Silksong => ("Hollow Knight Silksong", "Hollow Knight Silksong.app"),
            };
            vec![Path::new(&home)
                .join("Library/Application Support/Steam/steamapps/common")
                .join(folder)
                .join(app_name)
                .join("Contents/Resources/Data/Managed")]
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            let folder = match game {
                GameKey::HollowKnight => "Hollow Knight",
                GameKey::Silksong => "Hollow Knight Silksong",
            };
            [
                ".local/share/Steam/steamapps/common",
                ".steam/steam/steamapps/common",
                ".steam/root/steamapps/common",
                ".var/app/com.valvesoftware.Steam/data/Steam/steamapps/common",
            ]
            .iter()
            .map(|library| {
                Path::new(&home)
                    .join(library)
                    .join(folder)
                    .join(data_dir)
                    .join("Managed")
            })
            .collect()
        };

        if let Some(path) = candidates.into_iter().find(|path| path.exists()) {
            return Some(path.to_string_lossy().to_string());
        }

        Self::scan_for_managed_folder(data_dir).map(|path| path.to_string_lossy().to_string())
    }

    // drive letters that exist on this machine
    fn drive_roots() -> Vec<PathBuf> {
        (b'A'..=b'Z')
            .map(|drive| PathBuf::from(format!("{}:\\", drive as char)))
            .filter(|root| root.exists())
            .collect()
    }

    fn scan_for_managed_folder(data_dir: &str) -> Option<PathBuf> {
        let roots: Vec<PathBuf> = if cfg!(target_os = "windows") {
            Self::drive_roots()
        } else if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/Applications"),
                PathBuf::from(std::env::var("HOME").unwrap_or_default()),
            ]
        } else {
            vec![
                PathBuf::from(std::env::var("HOME").unwrap_or_default()),
                PathBuf::from("/mnt"),
                PathBuf::from("/media"),
            ]
        };

        for root in roots.into_iter().filter(|root| root.exists()) {
            let found = WalkDir::new(&root)
                .follow_links(false)
                .max_depth(SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(Result::ok)
                .find(|entry| {
                    entry.file_type().is_dir()
                        && entry
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case("Managed")
                        && entry
                            .path()
                            .parent()
                            .and_then(file_name)
                            .is_some_and(|parent| parent.eq_ignore_ascii_case(data_dir))
                });
            if let Some(entry) = found {
                return Some(entry.into_path());
            }
        }

        None
    }
}
