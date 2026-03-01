use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{ApiInfo, CatalogResponse, ModItem},
    settings::{AppSettings, GameKey},
    url_scheme::normalize_custom_modlinks_uri,
};
use roxmltree::Document;
use std::time::Duration;

const HK_MODLINKS: &str = "https://raw.githubusercontent.com/hk-modding/modlinks/main/ModLinks.xml";
const HK_APILINKS: &str = "https://raw.githubusercontent.com/hk-modding/modlinks/main/ApiLinks.xml";
const HK_MODLINKS_FALLBACK: &str = "https://cdn.jsdelivr.net/gh/hk-modding/modlinks@latest/ModLinks.xml";
const HK_APILINKS_FALLBACK: &str = "https://cdn.jsdelivr.net/gh/hk-modding/modlinks@latest/ApiLinks.xml";

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

#[derive(Debug, Clone)]
pub struct CatalogCache {
    pub response: CatalogResponse,
}

impl CatalogCache {
    pub async fn build(settings: &AppSettings, installed: &InstalledModsStore, fetch_official: bool) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Needlelight")
            .timeout(Duration::from_secs(30))
            .build()?;

        let modlinks_xml = fetch_modlinks_xml(&client, settings, fetch_official).await?;
        let api_xml = fetch_apilinks_xml(&client, settings).await;

        let api = api_xml
            .ok()
            .and_then(|xml| parse_api_info(&xml).ok())
            .unwrap_or_else(|| ApiInfo {
                url: String::new(),
                version: 0,
                sha256: String::new(),
            });

        let mut items = parse_mod_items(&modlinks_xml, installed)?;

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
            });
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self {
            response: CatalogResponse { items, api },
        })
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
        let text = client.get(uri).send().await?.error_for_status()?.text().await?;
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

    fetch_first_ok(client, &urls).await
}

async fn fetch_apilinks_xml(client: &reqwest::Client, settings: &AppSettings) -> AppResult<String> {
    if let Some(urls) = get_env_urls(&settings.game, false) {
        return fetch_first_ok(client, &urls).await;
    }

    let urls: Vec<String> = match settings.game {
        GameKey::HollowKnight => vec![HK_APILINKS.to_string(), HK_APILINKS_FALLBACK.to_string()],
        GameKey::Silksong => SS_APILINKS.iter().map(|url| url.to_string()).collect(),
    };

    fetch_first_ok(client, &urls).await
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

    Err(AppError::NotFound("unable to fetch remote resource".to_string()))
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
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0);

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

    Ok(ApiInfo { url, version, sha256 })
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
            let selected = target.ok_or_else(|| AppError::InvalidInput("invalid Links node".to_string()))?;
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
