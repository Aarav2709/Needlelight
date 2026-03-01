use super::{
    errors::AppResult,
    models::{PersistedInstalled, PersistedModState, PersistedNotInModlinks},
    settings::AppSettings,
};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Default)]
pub struct InstalledModsStore {
    pub db: PersistedInstalled,
}

impl InstalledModsStore {
    pub async fn load(settings: &AppSettings) -> AppResult<Self> {
        let game_path = settings.installed_mods_path()?;
        let legacy_path = AppSettings::config_dir()?.join("InstalledMods.json");
        let path = if game_path.exists() { game_path } else { legacy_path };

        let mut parsed = if path.exists() {
            let content = tokio::fs::read_to_string(path).await?;
            serde_json::from_str::<PersistedInstalled>(&content).unwrap_or_default()
        } else {
            PersistedInstalled::default()
        };

        Self::reconcile_with_disk(&mut parsed, settings).await?;

        let store = Self { db: parsed };
        store.save(settings).await?;
        Ok(store)
    }

    async fn reconcile_with_disk(db: &mut PersistedInstalled, settings: &AppSettings) -> AppResult<()> {
        let mods_folder = settings.mods_folder();
        let disabled_folder = settings.disabled_folder();

        tokio::fs::create_dir_all(&mods_folder).await?;
        tokio::fs::create_dir_all(&disabled_folder).await?;

        let mut discovered: HashMap<String, bool> = HashMap::new();
        for (dir, enabled) in [(&mods_folder, true), (&disabled_folder, false)] {
            if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name != "Disabled" {
                            discovered.insert(name, enabled);
                        }
                    }
                }
            }
        }

        db.mods.retain(|name, st| {
            discovered
                .get(name)
                .map(|enabled| {
                    st.enabled = *enabled;
                    true
                })
                .unwrap_or(false)
        });

        db.not_in_modlinks_mods.retain(|name, st| {
            discovered
                .get(name)
                .map(|enabled| {
                    st.enabled = *enabled;
                    true
                })
                .unwrap_or(false)
        });

        for (name, enabled) in discovered {
            if db.mods.contains_key(&name) || db.not_in_modlinks_mods.contains_key(&name) {
                continue;
            }
            db.not_in_modlinks_mods.insert(
                name,
                PersistedNotInModlinks {
                    enabled,
                    pinned: false,
                    installed: true,
                    modlinks_mod: false,
                },
            );
        }

        Ok(())
    }

    pub async fn save(&self, settings: &AppSettings) -> AppResult<()> {
        let path = settings.installed_mods_path()?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let payload = serde_json::to_string_pretty(&self.db)?;
        tokio::fs::write(path, payload).await?;
        Ok(())
    }

    pub fn state_for_manifest(&self, name: &str, version: &str) -> super::models::ModState {
        if let Some(st) = self.db.mods.get(name) {
            return super::models::ModState::Installed {
                enabled: st.enabled,
                pinned: st.pinned,
                version: st.version.clone(),
                updated: st.version == version,
            };
        }

        if let Some(st) = self.db.not_in_modlinks_mods.get(name) {
            return super::models::ModState::NotInModlinks {
                enabled: st.enabled,
                pinned: st.pinned,
                installed: st.installed,
                modlinks_mod: st.modlinks_mod,
            };
        }

        super::models::ModState::NotInstalled { installing: false }
    }

    pub fn mark_installed(&mut self, name: &str, version: &str, enabled: bool) {
        self.db.not_in_modlinks_mods.remove(name);
        self.db.mods.insert(
            name.to_string(),
            PersistedModState {
                enabled,
                version: version.to_string(),
                pinned: false,
            },
        );
    }

    pub fn mark_not_in_modlinks(&mut self, name: &str, enabled: bool, modlinks_mod: bool) {
        self.db.mods.remove(name);
        self.db.not_in_modlinks_mods.insert(
            name.to_string(),
            PersistedNotInModlinks {
                enabled,
                pinned: false,
                installed: true,
                modlinks_mod,
            },
        );
    }

    pub fn mark_uninstalled(&mut self, name: &str) {
        self.db.mods.remove(name);
        self.db.not_in_modlinks_mods.remove(name);
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(item) = self.db.mods.get_mut(name) {
            item.enabled = enabled;
        }
        if let Some(item) = self.db.not_in_modlinks_mods.get_mut(name) {
            item.enabled = enabled;
        }
    }

    pub fn has_mod(&self, name: &str) -> bool {
        self.db.mods.contains_key(name) || self.db.not_in_modlinks_mods.contains_key(name)
    }

    pub async fn move_mod_folder(settings: &AppSettings, name: &str, enable: bool) -> AppResult<()> {
        let from = if enable {
            settings.disabled_folder().join(name)
        } else {
            settings.mods_folder().join(name)
        };
        let to = if enable {
            settings.mods_folder().join(name)
        } else {
            settings.disabled_folder().join(name)
        };

        if !from.exists() {
            return Ok(());
        }
        if let Some(parent) = to.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        if to.exists() {
            if to.is_dir() {
                tokio::fs::remove_dir_all(&to).await?;
            } else {
                tokio::fs::remove_file(&to).await?;
            }
        }

        tokio::fs::rename(from, to).await?;
        Ok(())
    }

    pub fn mod_folder<'a>(settings: &'a AppSettings, name: &'a str, enabled: bool) -> std::path::PathBuf {
        let root = if enabled { settings.mods_folder() } else { settings.disabled_folder() };
        Path::new(&root).join(name)
    }
}
