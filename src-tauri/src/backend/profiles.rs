use super::{errors::AppResult, settings::{AppSettings, GameKey}};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Hooks {
    #[serde(default)]
    pub pre_launch: Option<String>,
    #[serde(default)]
    pub wrapper: Option<String>,
    #[serde(default)]
    pub post_exit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInstance {
    pub path: String,
    pub install_stage: String,

    pub name: String,
    #[serde(default)]
    pub icon_path: Option<String>,

    pub game_version: String,
    pub loader: String,
    #[serde(default)]
    pub loader_version: Option<String>,

    #[serde(default)]
    pub groups: Vec<String>,

    #[serde(default)]
    pub linked_data: Option<serde_json::Value>,

    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,

    #[serde(default)]
    pub submitted_time_played: i64,
    #[serde(default)]
    pub recent_time_played: i64,

    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub extra_launch_args: Option<Vec<String>>,
    #[serde(default)]
    pub custom_env_vars: Option<Vec<(String, String)>>,

    #[serde(default)]
    pub memory: Option<serde_json::Value>,
    #[serde(default)]
    pub force_fullscreen: Option<bool>,
    #[serde(default)]
    pub game_resolution: Option<(i32, i32)>,

    #[serde(default)]
    pub hooks: Hooks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMeta {
    pub name: String,
    pub game: GameKey,
    #[serde(default)]
    pub groups: Vec<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    pub icon_file: Option<String>,
}

pub fn profiles_root(game: &GameKey) -> AppResult<PathBuf> {
    Ok(AppSettings::config_dir()?.join("profiles").join(game.as_str()))
}

pub fn profile_meta_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join("profile.json")
}

pub fn sanitize_profile_name(raw: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "profile".to_string();
    }

    trimmed
        .chars()
        .map(|c| if invalid.contains(&c) { '_' } else { c })
        .collect::<String>()
}

pub fn load_profile_meta(profile_dir: &Path) -> AppResult<ProfileMeta> {
    let path = profile_meta_path(profile_dir);
    if !path.exists() {
        let now = Utc::now();
        return Ok(ProfileMeta {
            name: profile_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Profile")
                .to_string(),
            game: GameKey::HollowKnight,
            groups: vec![],
            created: now,
            modified: now,
            last_played: None,
            icon_file: None,
        });
    }

    let content = fs::read_to_string(path)?;
    let meta = serde_json::from_str::<ProfileMeta>(&content)?;
    Ok(meta)
}

pub fn save_profile_meta(profile_dir: &Path, meta: &ProfileMeta) -> AppResult<()> {
    let path = profile_meta_path(profile_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(meta)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn profile_to_instance(profile_dir: &Path, meta: &ProfileMeta) -> GameInstance {
    let icon_path = meta.icon_file.as_ref().map(|file| profile_dir.join(file).to_string_lossy().to_string());

    GameInstance {
        path: profile_dir.to_string_lossy().to_string(),
        install_stage: "installed".to_string(),
        name: meta.name.clone(),
        icon_path,
        game_version: "".to_string(),
        loader: "vanilla".to_string(),
        loader_version: None,
        groups: meta.groups.clone(),
        linked_data: None,
        created: meta.created,
        modified: meta.modified,
        last_played: meta.last_played,
        submitted_time_played: 0,
        recent_time_played: 0,
        java_path: None,
        extra_launch_args: None,
        custom_env_vars: None,
        memory: None,
        force_fullscreen: None,
        game_resolution: None,
        hooks: Hooks::default(),
    }
}

pub fn ensure_profile_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path)?;
    Ok(())
}
