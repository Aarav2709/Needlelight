use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModState {
    Installed {
        enabled: bool,
        pinned: bool,
        version: String,
        updated: bool,
    },
    NotInstalled {
        #[serde(default)]
        installing: bool,
    },
    NotInModlinks {
        enabled: bool,
        pinned: bool,
        installed: bool,
        modlinks_mod: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModItem {
    pub name: String,
    pub description: String,
    pub version: String,
    pub dependencies: Vec<String>,
    pub link: String,
    pub sha256: String,
    pub repository: String,
    pub issues: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub integrations: Vec<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub state: ModState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub url: String,
    pub version: i32,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogResponse {
    pub items: Vec<ModItem>,
    pub api: ApiInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedInstalled {
    #[serde(default)]
    pub mods: HashMap<String, PersistedModState>,
    #[serde(default)]
    pub not_in_modlinks_mods: HashMap<String, PersistedNotInModlinks>,
    #[serde(default)]
    pub api_install: Option<PersistedModState>,
    #[serde(default)]
    pub has_vanilla: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedModState {
    pub enabled: bool,
    pub version: String,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedNotInModlinks {
    pub enabled: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default = "default_true")]
    pub installed: bool,
    pub modlinks_mod: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgressArgs {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProgressArgs {
    pub completed: bool,
    pub item_name: Option<String>,
    pub download: Option<DownloadProgressArgs>,
}
