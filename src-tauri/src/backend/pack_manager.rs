use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    settings::AppSettings,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const PACK_INFO_FILE: &str = "packMods.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEntry {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackDocument {
    pub name: String,
    pub description: String,
    pub authors: String,
    pub mods: Vec<PackEntry>,
}

fn packs_root(settings: &AppSettings) -> PathBuf {
    settings.mods_folder().join("..")
}

pub async fn list_packs(settings: &AppSettings) -> AppResult<Vec<String>> {
    let root = packs_root(settings);
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut output = Vec::new();
    let mut entries = tokio::fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !entry.path().is_dir() {
            continue;
        }
        if entry.path().join(PACK_INFO_FILE).exists() {
            output.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    output.sort();
    Ok(output)
}

pub async fn save_pack(
    settings: &AppSettings,
    installed: &InstalledModsStore,
    name: &str,
    description: &str,
    authors: &str,
) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidInput("pack name cannot be empty".to_string()));
    }

    let dir = packs_root(settings).join(name);
    tokio::fs::create_dir_all(&dir).await?;

    let mut mods = Vec::new();
    for (mod_name, state) in &installed.db.mods {
        mods.push(PackEntry {
            name: mod_name.clone(),
            enabled: state.enabled,
        });
    }
    for (mod_name, state) in &installed.db.not_in_modlinks_mods {
        mods.push(PackEntry {
            name: mod_name.clone(),
            enabled: state.enabled,
        });
    }
    mods.sort_by(|a, b| a.name.cmp(&b.name));

    let doc = PackDocument {
        name: name.to_string(),
        description: description.to_string(),
        authors: authors.to_string(),
        mods,
    };

    let payload = serde_json::to_string_pretty(&doc)?;
    tokio::fs::write(dir.join(PACK_INFO_FILE), payload).await?;
    Ok(())
}

pub async fn load_pack(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    name: &str,
    additive: bool,
) -> AppResult<bool> {
    let file = packs_root(settings).join(name).join(PACK_INFO_FILE);
    if !file.exists() {
        return Ok(false);
    }

    let payload = tokio::fs::read_to_string(file).await?;
    let pack = serde_json::from_str::<PackDocument>(&payload)?;

    if !additive {
        let existing: Vec<String> = installed
            .db
            .mods
            .keys()
            .chain(installed.db.not_in_modlinks_mods.keys())
            .cloned()
            .collect();
        for name in existing {
            super::installer::uninstall_mod(settings, installed, &name).await?;
        }
    }

    for entry in pack.mods {
        if !installed.has_mod(&entry.name) {
            installed.mark_not_in_modlinks(&entry.name, entry.enabled, false);
        } else {
            installed.set_enabled(&entry.name, entry.enabled);
        }
    }

    installed.save(settings).await?;
    Ok(true)
}

pub async fn import_pack(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    code: &str,
) -> AppResult<Option<String>> {
    let url = format!("https://pastebin.com/raw/{code}");
    let text = reqwest::Client::builder()
        .user_agent("Needlelight")
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let parsed = serde_json::from_str::<PackDocument>(&text)?;
    let pack_name = parsed.name.clone();

    let dir = packs_root(settings).join(&pack_name);
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(dir.join(PACK_INFO_FILE), serde_json::to_string_pretty(&parsed)?).await?;

    load_pack(settings, installed, &pack_name, true).await?;
    Ok(Some(pack_name))
}
