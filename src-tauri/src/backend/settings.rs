use super::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameKey {
    HollowKnight,
    Silksong,
}

impl Default for GameKey {
    fn default() -> Self {
        Self::HollowKnight
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub managed_folder: String,
    pub game: GameKey,
    #[serde(default)]
    pub use_custom_modlinks: bool,
    #[serde(default)]
    pub custom_modlinks_uri: String,
    #[serde(default)]
    pub use_github_mirror: bool,
    #[serde(default)]
    pub github_mirror_format: String,
    #[serde(default)]
    pub low_storage_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            managed_folder: String::new(),
            game: GameKey::HollowKnight,
            use_custom_modlinks: false,
            custom_modlinks_uri: String::new(),
            use_github_mirror: false,
            github_mirror_format: String::new(),
            low_storage_mode: false,
        }
    }
}

impl AppSettings {
    fn game_data_dir_name(game: &GameKey) -> &'static str {
        match game {
            GameKey::HollowKnight => "Hollow Knight_Data",
            GameKey::Silksong => "Hollow Knight Silksong_Data",
        }
    }

    fn normalize_managed_folder(raw: &str, game: &GameKey) -> String {
        let path = PathBuf::from(raw.trim());
        if raw.trim().is_empty() {
            return String::new();
        }

        if path.file_name().and_then(|n| n.to_str()) == Some("Managed") {
            return path.to_string_lossy().to_string();
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("_Data") {
                return path.join("Managed").to_string_lossy().to_string();
            }
        }

        let data_name = Self::game_data_dir_name(game);
        let data_candidate = path.join(data_name);
        if data_candidate.exists() {
            return data_candidate.join("Managed").to_string_lossy().to_string();
        }

        path.to_string_lossy().to_string()
    }

    fn game_root_path(&self) -> PathBuf {
        let managed = PathBuf::from(&self.managed_folder);

        if managed.file_name().and_then(|n| n.to_str()) == Some("Managed") {
            if let Some(data_folder) = managed.parent() {
                if data_folder
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| name.ends_with("_Data"))
                    .unwrap_or(false)
                {
                    if let Some(root) = data_folder.parent() {
                        return root.to_path_buf();
                    }
                }
            }
        }

        if managed
            .file_name()
            .and_then(|n| n.to_str())
            .map(|name| name.ends_with("_Data"))
            .unwrap_or(false)
        {
            if let Some(root) = managed.parent() {
                return root.to_path_buf();
            }
        }

        let data_candidate = managed.join(Self::game_data_dir_name(&self.game));
        if data_candidate.exists() {
            return managed;
        }

        managed
    }

    pub fn normalized(&self) -> Self {
        let mut copy = self.clone();
        copy.managed_folder = Self::normalize_managed_folder(&self.managed_folder, &self.game);
        copy
    }

    pub fn config_dir() -> AppResult<PathBuf> {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOME")
                    .map(|home| Path::new(&home).join(".config"))
                    .map_err(|_| AppError::InvalidInput("cannot resolve config directory".to_string()))
            })?;

        Ok(base.join("HKModInstaller"))
    }

    pub fn config_path() -> AppResult<PathBuf> {
        Ok(Self::config_dir()?.join("HKInstallerSettings.json"))
    }

    pub fn cache_folder(&self) -> AppResult<PathBuf> {
        Ok(Self::config_dir()?.join("HKInstallerCache"))
    }

    pub fn mods_folder(&self) -> PathBuf {
        if matches!(self.game, GameKey::Silksong) {
            return self.game_root_path().join("BepInEx").join("plugins");
        }

        let managed = PathBuf::from(&self.managed_folder);
        if managed.file_name().and_then(|n| n.to_str()) == Some("Managed") {
            return managed.join("Mods");
        }

        managed.join("Mods")
    }

    pub fn disabled_folder(&self) -> PathBuf {
        self.mods_folder().join("Disabled")
    }

    pub fn installed_mods_path(&self) -> AppResult<PathBuf> {
        let key = match self.game {
            GameKey::HollowKnight => "hollow_knight",
            GameKey::Silksong => "silksong",
        };
        Ok(Self::config_dir()?.join(format!("InstalledMods.{key}.json")))
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
        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = match game {
            GameKey::HollowKnight => vec![
                ".local/share/Steam/steamapps/common/Hollow Knight/Hollow Knight_Data/Managed",
                ".steam/steam/steamapps/common/Hollow Knight/Hollow Knight_Data/Managed",
                ".steam/root/steamapps/common/Hollow Knight/Hollow Knight_Data/Managed",
                ".var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/Hollow Knight/Hollow Knight_Data/Managed",
            ],
            GameKey::Silksong => vec![
                ".local/share/Steam/steamapps/common/Hollow Knight Silksong/Hollow Knight Silksong_Data/Managed",
                ".steam/steam/steamapps/common/Hollow Knight Silksong/Hollow Knight Silksong_Data/Managed",
                ".steam/root/steamapps/common/Hollow Knight Silksong/Hollow Knight Silksong_Data/Managed",
                ".var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/Hollow Knight Silksong/Hollow Knight Silksong_Data/Managed",
            ],
        };

        for relative in candidates {
            let path = Path::new(&home).join(relative);
            if path.exists() {
                return Ok(Some(path.to_string_lossy().to_string()));
            }
        }

        Ok(None)
    }
}
