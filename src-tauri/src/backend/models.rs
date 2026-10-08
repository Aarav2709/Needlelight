use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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
    // minimum version per dependency, only filled in by catalogs that publish it (thunderstore)
    #[serde(default)]
    pub dependency_versions: BTreeMap<String, String>,
    // icon url when the catalog provides one (thunderstore)
    #[serde(default)]
    pub icon: Option<String>,
    // total downloads across all versions when the catalog provides them (thunderstore)
    #[serde(default)]
    pub downloads: Option<u64>,
    // last update time in rfc 3339 when the catalog provides it
    #[serde(default)]
    pub updated_at: Option<String>,
    // the project's own website when listed separately from the repository (thunderstore)
    #[serde(default)]
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub url: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogResponse {
    pub items: Vec<ModItem>,
    pub api: ApiInfo,
    #[serde(default)]
    pub api_installed: bool,
    #[serde(default)]
    pub api_enabled: bool,
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
