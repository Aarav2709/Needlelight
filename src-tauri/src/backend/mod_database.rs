use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{ApiInfo, CatalogResponse, ModItem},
    settings::{AppSettings, GameKey},
};
use regex::Regex;
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

// thunderstore api responses
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

        // silksong uses thunderstore unless a custom modlinks feed (same schema as hollow knight) is on
        if settings.game == GameKey::Silksong && !settings.use_custom_modlinks {
            return Self::build_silksong(&client, settings, installed).await;
        }

        let modlinks_xml = fetch_modlinks_xml(&client, settings, fetch_official).await;
        let api_xml = fetch_apilinks_xml(&client, settings).await;

        let api = api_xml
            .ok()
            .and_then(|xml| parse_api_info(&xml).ok())
            .unwrap_or_else(empty_api);

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

    // builds the silksong catalog from the thunderstore api
    async fn build_silksong(
        client: &reqwest::Client,
        settings: &AppSettings,
        installed: &InstalledModsStore,
    ) -> AppResult<Self> {
        let packages: Vec<ThunderstorePackage> = match client.get(THUNDERSTORE_SS_URL).send().await
        {
            Ok(resp) => match resp.error_for_status() {
                Ok(ok) => ok.json().await.unwrap_or_else(|e| {
                    log::error!("Thunderstore sent an unreadable package list: {e}");
                    Vec::new()
                }),
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

        // the silksong bepinex pack is published under silksong_modding as well as bepinex, like cogfly checks
        const LOADER_PACKAGES: [&str; 2] = [
            "BepInEx-BepInExPack_Silksong",
            "silksong_modding-BepInExPack_Silksong",
        ];
        // other mod managers that thunderstore lists as packages
        const MANAGER_PACKAGES: [&str; 2] = ["ebkr-r2modman", "Kesomannen-GaleModManager"];

        let mut api: Option<ApiInfo> = None;
        let mut items: Vec<ModItem> = Vec::new();
        for pkg in packages
            .into_iter()
            .filter(|p| !p.is_deprecated && !p.versions.is_empty())
        {
            if LOADER_PACKAGES.contains(&pkg.full_name.as_str()) {
                let latest = &pkg.versions[0];
                api = Some(ApiInfo {
                    url: latest.download_url.clone(),
                    version: latest.version_number.clone(),
                    sha256: String::new(),
                });
                continue;
            }
            if MANAGER_PACKAGES.contains(&pkg.full_name.as_str()) {
                continue;
            }

            let latest = &pkg.versions[0];
            let name = pkg.full_name.clone();

            // keep dependencies as owner and mod pairs and remember the minimum version each asks for
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
                // thunderstore doesn't publish a sha256 in the listing
                sha256: String::new(),
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
                homepage: Some(latest.website_url.trim().to_string())
                    .filter(|url| url.starts_with("https://") || url.starts_with("http://")),
            });
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));

        // the apilinks feeds are only a fallback for when thunderstore doesn't list the loader
        let api = match api {
            Some(api) => api,
            None => fetch_apilinks_xml(client, settings)
                .await
                .ok()
                .and_then(|xml| parse_api_info(&xml).ok())
                .unwrap_or_else(empty_api),
        };

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

fn empty_api() -> ApiInfo {
    ApiInfo {
        url: String::new(),
        version: String::new(),
        sha256: String::new(),
    }
}

// turns github blob and pastebin page links into their raw file urls
fn normalize_custom_modlinks_uri(input: &str) -> String {
    let mut value = input.trim().to_string();
    let github_regex = Regex::new(r"^(http(s?)://)?(www\.)?github.com").unwrap();
    let pastebin_regex = Regex::new(r"^(http(s?)://)?(www\.)?pastebin.com").unwrap();

    if github_regex.is_match(&value) {
        value = value
            .replace("https://github.com", "https://raw.githubusercontent.com")
            .replace("http://github.com", "https://raw.githubusercontent.com")
            .replace("/blob/", "/");
    }

    if pastebin_regex.is_match(&value) {
        value = value
            .replace("https://pastebin.com", "https://pastebin.com/raw")
            .replace("http://pastebin.com", "https://pastebin.com/raw");
    }

    value
}

// the links entry name for this platform in modlinks and apilinks
fn platform_key() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Mac"
    } else {
        "Linux"
    }
}

// thunderstore prefixes legacy categories with (deprecated category), strip it so tags read cleanly
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

// with the github mirror on, try each mirrored url first and keep the originals as fallbacks
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

    let os_link = links
        .children()
        .find(|n| n.has_tag_name(platform_key()))
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

        // a malformed links entry only costs that mod its download, not the whole catalog
        let selected = match links_node {
            Some(links) => links
                .children()
                .find(|n| n.has_tag_name(platform_key()))
                .or_else(|| links.children().find(|n| n.has_tag_name("Windows"))),
            None => link_node,
        };
        if links_node.is_some() && selected.is_none() {
            log::warn!("ModLinks entry {name} has no download for this platform");
        }
        let (link, sha256) = selected
            .map(|node| {
                (
                    node.text().unwrap_or_default().trim().to_string(),
                    node.attribute("SHA256").unwrap_or_default().to_string(),
                )
            })
            .unwrap_or_default();

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
