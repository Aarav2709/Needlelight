use super::{
    errors::AppResult,
    settings::{AppSettings, GameKey},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

// what the frontend receives for each modpack
#[derive(Debug, Clone, Serialize)]
pub struct GameInstance {
    pub path: String,
    pub game: GameKey,
    pub name: String,
    pub description: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMeta {
    pub name: String,
    pub game: GameKey,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    // icons are no longer used, the field stays so older profile files keep it
    pub icon_file: Option<String>,
}

pub fn profiles_root(game: &GameKey) -> AppResult<PathBuf> {
    Ok(AppSettings::config_dir()?
        .join("profiles")
        .join(game.as_str()))
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
            description: None,
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
    GameInstance {
        path: profile_dir.to_string_lossy().to_string(),
        game: meta.game.clone(),
        name: meta.name.clone(),
        description: meta.description.clone(),
        created: meta.created,
        modified: meta.modified,
        last_played: meta.last_played,
    }
}

pub fn ensure_profile_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path)?;
    Ok(())
}
