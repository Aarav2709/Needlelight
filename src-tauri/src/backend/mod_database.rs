use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{ApiInfo, CatalogResponse, ModItem},
    settings::{AppSettings, GameKey},
    url_scheme::normalize_custom_modlinks_uri,
};
use roxmltree::Document;
use serde::Deserialize;
use std::time::Duration;

const HK_MODLINKS: &str = "https://raw.githubusercontent.com/hk-modding/modlinks/main/ModLinks.xml";
const HK_APILINKS: &str = "https://raw.githubusercontent.com/hk-modding/modlinks/main/ApiLinks.xml";
const HK_MODLINKS_FALLBACK: &str =
    "https://cdn.jsdelivr.net/gh/hk-modding/modlinks@latest/ModLinks.xml";
const HK_APILINKS_FALLBACK: &str =
    "https://cdn.jsdelivr.net/gh/hk-modding/modlinks@latest/ApiLinks.xml";

const SS_MODLINKS: &[&str] = &[
    "https://raw.githubusercontent.com/silksong-modding/modlinks/main/ModLinks.xml",
    "https://raw.githubusercontent.com/hk-modding/silksong-modlinks/main/ModLinks.xml",
    "https://raw.githubusercontent.com/hk-modding/modlinks/main/silksong/ModLinks.xml",
    "https://cdn.jsdelivr.net/gh/silksong-modding/modlinks@latest/ModLinks.xml",
];

const SS_APILINKS: &[&str] = &[
    "https://raw.githubusercontent.com/silksong-modding/modlinks/main/ApiLinks.xml",
    "https://raw.githubusercontent.com/hk-modding/silksong-modlinks/main/ApiLinks.xml",
    "https://raw.githubusercontent.com/hk-modding/modlinks/main/silksong/ApiLinks.xml",
    "https://cdn.jsdelivr.net/gh/silksong-modding/modlinks@latest/ApiLinks.xml",
];

const THUNDERSTORE_SS_URL: &str =
    "https://thunderstore.io/c/hollow-knight-silksong/api/v1/package/";

// ─── Thunderstore DTOs ──────────────────────────────────────────────────────
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ThunderstorePackage {
    pub name: String,
    pub full_name: String,
    pub owner: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub rating_score: i32,
    #[serde(default)]
    pub is_deprecated: bool,
    #[serde(default)]
    pub date_updated: String,
    #[serde(default)]
    pub versions: Vec<ThunderstoreVersion>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ThunderstoreVersion {
    pub version_number: String,
    pub description: String,
    #[serde(default)]
    pub icon: String,
    pub download_url: String,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub website_url: String,
}

#[derive(Debug, Clone)]
pub struct CatalogCache {
    pub response: CatalogResponse,
}

impl CatalogCache {
    pub async fn build(
        settings: &AppSettings,
        installed: &InstalledModsStore,
        fetch_official: bool,
    ) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Needlelight")
            .timeout(Duration::from_secs(30))
            .build()?;

        // A custom Silksong ModLinks feed follows the same schema as Hollow
        // Knight's catalog. Use it when requested; Thunderstore remains the
        // default Silksong source.
        if settings.game == GameKey::Silksong && !settings.use_custom_modlinks {
            return Self::build_silksong(&client, settings, installed).await;
        }

        let modlinks_xml = fetch_modlinks_xml(&client, settings, fetch_official).await;
        let api_xml = fetch_apilinks_xml(&client, settings).await;

        let api = api_xml
            .ok()
            .and_then(|xml| parse_api_info(&xml).ok())
            .unwrap_or_else(|| ApiInfo {
                url: String::new(),
                version: String::new(),
                sha256: String::new(),
            });

        let mut items = match modlinks_xml {
            Ok(xml) => parse_mod_items(&xml, installed)?,
            Err(e) => {
                log::error!("Could not fetch modlinks: {e}");
                Vec::new()
            }
        };

        for (name, state) in &installed.db.not_in_modlinks_mods {
            if items.iter().any(|x| x.name == *name) {
                continue;
            }
            items.push(ModItem {
                name: name.clone(),
                description: "This mod is not from official modlinks".to_string(),
                version: "0.0.0.0".to_string(),
                dependencies: vec![],
                link: String::new(),
                sha256: String::new(),
                repository: String::new(),
                issues: String::new(),
                tags: vec![],
                integrations: vec![],
                authors: vec![],
                state: super::models::ModState::NotInModlinks {
                    enabled: state.enabled,
                    pinned: state.pinned,
                    installed: state.installed,
                    modlinks_mod: state.modlinks_mod,
                },
                dependency_versions: Default::default(),
                icon: None,
                downloads: None,
                updated_at: None,
                homepage: None,
            });
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self {
            response: CatalogResponse {
                items,
                api,
                api_installed: false,
                api_enabled: false,
            },
        })
    }

    /// Build catalog from Thunderstore API for Silksong
    async fn build_silksong(
        client: &reqwest::Client,
        settings: &AppSettings,
        installed: &InstalledModsStore,
    ) -> AppResult<Self> {
        let packages: Vec<ThunderstorePackage> = match client.get(THUNDERSTORE_SS_URL).send().await
        {
            Ok(resp) => match resp.error_for_status() {
                Ok(ok) => ok.json().await.unwrap_or_default(),
                Err(e) => {
                    log::error!("Thunderstore API error: {e}");
                    Vec::new()
                }
            },
            Err(e) => {
                log::error!("Could not reach Thunderstore: {e}");
                Vec::new()
            }
        };

        // the Silksong BepInEx pack is published under silksong_modding, not
        // BepInEx; check both, matching Cogfly's reference implementation
        let excluded = [
            "BepInEx-BepInExPack_Silksong",
            "silksong_modding-BepInExPack_Silksong",
            "ebkr-r2modman",
            "Kesomannen-GaleModManager",
        ];

        let mut api = fetch_apilinks_xml(client, settings)
            .await
            .ok()
            .and_then(|xml| parse_api_info(&xml).ok())
            .unwrap_or_else(|| ApiInfo {
                url: String::new(),
                version: String::new(),
                sha256: String::new(),
            });

        let mut items: Vec<ModItem> = Vec::new();
        for pkg in packages
            .into_iter()
            .filter(|p| !p.is_deprecated && !p.versions.is_empty())
        {
            if excluded.contains(&pkg.full_name.as_str()) {
                if pkg.full_name == "BepInEx-BepInExPack_Silksong"
                    || pkg.full_name == "silksong_modding-BepInExPack_Silksong"
                {
                    if let Some(latest) = pkg.versions.first() {
                        api = ApiInfo {
                            url: latest.download_url.clone(),
                            version: latest.version_number.clone(),
                            sha256: String::new(),
                        };
                    }
                }
                continue;
            }

            let latest = &pkg.versions[0];
            let name = pkg.full_name.clone();

            // Keep dependency identifiers as owner-mod pairs (full_name without version), and
            // remember the minimum version each one asks for.
            let mut dependencies: Vec<String> = Vec::with_capacity(latest.dependencies.len());
            let mut dependency_versions = std::collections::BTreeMap::new();
            for dep in &latest.dependencies {
                let parts: Vec<&str> = dep.split('-').collect();
                if parts.len() >= 2 {
                    let key = format!("{}-{}", parts[0], parts[1]);
                    if parts.len() >= 3 && !parts[2].is_empty() {
                        dependency_versions.insert(key.clone(), parts[2..].join("-"));
                    }
                    dependencies.push(key);
                } else {
                    dependencies.push(dep.clone());
                }
            }
            let downloads: u64 = pkg.versions.iter().map(|v| v.downloads.max(0) as u64).sum();

            let state = installed.state_for_manifest(&name, &latest.version_number);

            items.push(ModItem {
                name,
                description: latest.description.clone(),
                version: latest.version_number.clone(),
                dependencies,
                link: latest.download_url.clone(),
                sha256: String::new(), // Thunderstore doesn't provide SHA256 in the listing
                repository: format!(
                    "https://thunderstore.io/c/hollow-knight-silksong/p/{}/{}/",
                    pkg.owner, pkg.name
                ),
                issues: String::new(),
                tags: pkg
                    .categories
                    .iter()
                    .map(|tag| strip_deprecated_category_prefix(tag))
                    .collect(),
                integrations: vec![],
                authors: vec![pkg.owner.clone()],
                state,
                dependency_versions,
                icon: Some(latest.icon.clone()).filter(|icon| !icon.trim().is_empty()),
                downloads: Some(downloads),
                updated_at: Some(pkg.date_updated.clone()).filter(|date| !date.trim().is_empty()),
                homepage: Some(latest.website_url.trim().to_string()).filter(|url| {
                    url.starts_with("https://") || url.starts_with("http://")
                }),
            });
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self {
            response: CatalogResponse {
                items,
                api,
                api_installed: false,
                api_enabled: false,
            },
        })
    }
}

// Thunderstore returns literal category names like "(Deprecated category) Misc"
// for legacy categories; strip that prefix so tags just read "Misc"
fn strip_deprecated_category_prefix(tag: &str) -> String {
    const PREFIX: &str = "(Deprecated category)";
    let trimmed = tag.trim();
    if let Some(rest) = trimmed.strip_prefix(PREFIX) {
        rest.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

async fn fetch_modlinks_xml(
    client: &reqwest::Client,
    settings: &AppSettings,
    fetch_official: bool,
) -> AppResult<String> {
    if !fetch_official && settings.use_custom_modlinks {
        let uri = normalize_custom_modlinks_uri(&settings.custom_modlinks_uri);
        if uri.is_empty() {
            return Err(AppError::InvalidModlinks);
        }
        let text = client
            .get(uri)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        if text.trim().is_empty() {
            return Err(AppError::InvalidModlinks);
        }
        return Ok(text);
    }

    if let Some(urls) = get_env_urls(&settings.game, true) {
        return fetch_first_ok(client, &urls).await;
    }

    let urls: Vec<String> = match settings.game {
        GameKey::HollowKnight => vec![HK_MODLINKS.to_string(), HK_MODLINKS_FALLBACK.to_string()],
        GameKey::Silksong => SS_MODLINKS.iter().map(|url| url.to_string()).collect(),
    };

    fetch_first_ok(client, &with_mirror(settings, urls)).await
}

async fn fetch_apilinks_xml(client: &reqwest::Client, settings: &AppSettings) -> AppResult<String> {
    if let Some(urls) = get_env_urls(&settings.game, false) {
        return fetch_first_ok(client, &urls).await;
    }

    let urls: Vec<String> = match settings.game {
        GameKey::HollowKnight => vec![HK_APILINKS.to_string(), HK_APILINKS_FALLBACK.to_string()],
        GameKey::Silksong => SS_APILINKS.iter().map(|url| url.to_string()).collect(),
    };

    fetch_first_ok(client, &with_mirror(settings, urls)).await
}

/// When the GitHub mirror is enabled, try the mirrored copy of each URL first and keep the
/// originals as fallbacks.
fn with_mirror(settings: &AppSettings, urls: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(urls.len() * 2);
    for url in &urls {
        let mirrored = settings.mirrored_url(url);
        if mirrored != *url && !out.contains(&mirrored) {
            out.push(mirrored);
        }
    }
    out.extend(urls);
    out
}

fn parse_env_urls(var_name: &str) -> Option<Vec<String>> {
    std::env::var(var_name).ok().and_then(|raw| {
        let urls: Vec<String> = raw
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        if urls.is_empty() {
            None
        } else {
            Some(urls)
        }
    })
}

fn get_env_urls(game: &GameKey, modlinks: bool) -> Option<Vec<String>> {
    match (game, modlinks) {
        (GameKey::HollowKnight, true) => parse_env_urls("NEEDLELIGHT_HK_MODLINKS_URLS"),
        (GameKey::HollowKnight, false) => parse_env_urls("NEEDLELIGHT_HK_APILINKS_URLS"),
        (GameKey::Silksong, true) => parse_env_urls("NEEDLELIGHT_SS_MODLINKS_URLS"),
        (GameKey::Silksong, false) => parse_env_urls("NEEDLELIGHT_SS_APILINKS_URLS"),
    }
}

async fn fetch_first_ok(client: &reqwest::Client, urls: &[String]) -> AppResult<String> {
    for url in urls {
        if let Ok(response) = client.get(url).send().await {
            if let Ok(ok) = response.error_for_status() {
                if let Ok(text) = ok.text().await {
                    if !text.trim().is_empty() {
                        return Ok(text);
                    }
                }
            }
        }
    }

    Err(AppError::NotFound(
        "unable to fetch remote resource".to_string(),
    ))
}

fn parse_api_info(xml: &str) -> AppResult<ApiInfo> {
    let doc = Document::parse(xml).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    let manifest = doc
        .descendants()
        .find(|n| n.has_tag_name("Manifest"))
        .ok_or_else(|| AppError::InvalidInput("ApiLinks has no Manifest".to_string()))?;

    let version = manifest
        .children()
        .find(|n| n.has_tag_name("Version"))
        .and_then(|v| v.text())
        .unwrap_or("0")
        .trim()
        .to_string();

    let links = manifest
        .children()
        .find(|n| n.has_tag_name("Links"))
        .ok_or_else(|| AppError::InvalidInput("ApiLinks has no Links".to_string()))?;

    let os_key = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Mac"
    } else {
        "Linux"
    };

    let os_link = links
        .children()
        .find(|n| n.has_tag_name(os_key))
        .ok_or_else(|| AppError::InvalidInput("ApiLinks missing platform link".to_string()))?;

    let url = os_link.text().unwrap_or_default().trim().to_string();
    let sha256 = os_link.attribute("SHA256").unwrap_or_default().to_string();

    Ok(ApiInfo {
        url,
        version,
        sha256,
    })
}

fn parse_mod_items(xml: &str, installed: &InstalledModsStore) -> AppResult<Vec<ModItem>> {
    let doc = Document::parse(xml).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    let mut items = Vec::new();

    for manifest in doc.descendants().filter(|n| n.has_tag_name("Manifest")) {
        let name = text_child(manifest, "Name");
        if name.is_empty() {
            continue;
        }

        let version = text_child(manifest, "Version");
        let description = text_child(manifest, "Description");
        let repository = text_child(manifest, "Repository");
        let issues = text_child(manifest, "Issues");

        let links_node = manifest.children().find(|n| n.has_tag_name("Links"));
        let link_node = manifest.children().find(|n| n.has_tag_name("Link"));

        let (link, sha256) = if let Some(links) = links_node {
            let os_key = if cfg!(target_os = "windows") {
                "Windows"
            } else if cfg!(target_os = "macos") {
                "Mac"
            } else {
                "Linux"
            };
            let target = links
                .children()
                .find(|n| n.has_tag_name(os_key))
                .or_else(|| links.children().find(|n| n.has_tag_name("Windows")));
            let selected =
                target.ok_or_else(|| AppError::InvalidInput("invalid Links node".to_string()))?;
            (
                selected.text().unwrap_or_default().trim().to_string(),
                selected.attribute("SHA256").unwrap_or_default().to_string(),
            )
        } else if let Some(link) = link_node {
            (
                link.text().unwrap_or_default().trim().to_string(),
                link.attribute("SHA256").unwrap_or_default().to_string(),
            )
        } else {
            (String::new(), String::new())
        };

        let dependencies = list_children(manifest, "Dependencies", "Dependency");
        let tags = list_children(manifest, "Tags", "Tag");
        let integrations = list_children(manifest, "Integrations", "Integration");
        let authors = list_children(manifest, "Authors", "Author");

        items.push(ModItem {
            state: installed.state_for_manifest(&name, &version),
            name,
            description,
            version,
            dependencies,
            link,
            sha256,
            repository,
            issues,
            tags,
            integrations,
            authors,
            dependency_versions: Default::default(),
            icon: None,
            downloads: None,
            updated_at: None,
            homepage: None,
        });
    }

    Ok(items)
}

fn text_child(node: roxmltree::Node<'_, '_>, child: &str) -> String {
    node.children()
        .find(|n| n.has_tag_name(child))
        .and_then(|n| n.text())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn list_children(node: roxmltree::Node<'_, '_>, container: &str, item: &str) -> Vec<String> {
    node.children()
        .find(|n| n.has_tag_name(container))
        .map(|container_node| {
            container_node
                .children()
                .filter(|n| n.has_tag_name(item))
                .filter_map(|n| n.text())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
