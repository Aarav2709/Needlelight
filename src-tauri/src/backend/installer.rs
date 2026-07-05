use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{CatalogResponse, ModState},
    settings::AppSettings,
};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs::File, io::{Cursor, Read}, path::{Path, PathBuf}};
use walkdir::WalkDir;
use zip::ZipArchive;

fn ensure_valid_hk_managed_folder(settings: &AppSettings) -> AppResult<()> {
    let managed = PathBuf::from(&settings.managed_folder);
    if !managed.exists() {
        return Err(AppError::InvalidInput(
            "Managed folder does not exist. Go to Settings > Game to set it up.".to_string(),
        ));
    }

    let assembly = managed.join("Assembly-CSharp.dll");
    if !assembly.exists() {
        return Err(AppError::InvalidInput(
            "Managed folder is invalid (Assembly-CSharp.dll not found). Go to Settings > Game to set it up.".to_string(),
        ));
    }

    Ok(())
}

fn looks_like_zip(data: &[u8]) -> bool {
    data.len() >= 4 && data[0] == b'P' && data[1] == b'K'
}

fn filename_from_url(url: &str) -> Option<String> {
    let base = url.split('/').last()?;
    let base = base.split('?').next().unwrap_or(base);
    let base = base.split('#').next().unwrap_or(base);
    let base = base.trim();
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

pub async fn install_mod(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    catalog: &CatalogResponse,
    mod_name: &str,
) -> AppResult<()> {
    if settings.managed_folder.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Game folder not configured. Go to Settings > Game to set it up.".to_string(),
        ));
    }

    if !settings.game.is_silksong() {
        ensure_valid_hk_managed_folder(settings)?;
        if !is_api_installed(settings, installed) {
            install_api(settings, installed, catalog).await?;
        }
    }

    let mut visited = HashSet::new();
    install_mod_with_deps(settings, installed, catalog, mod_name, &mut visited).await
}

pub async fn uninstall_mod(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    mod_name: &str,
) -> AppResult<()> {
    if settings.managed_folder.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Game folder not configured. Go to Settings > Game to set it up.".to_string(),
        ));
    }
    if settings.game.is_silksong() {
        remove_silksong_mod(settings, mod_name).await?;
        installed.mark_uninstalled(mod_name);
        installed.save(settings).await?;
        return Ok(());
    }

    let enabled_folder = InstalledModsStore::mod_folder(settings, mod_name, true);
    let disabled_folder = InstalledModsStore::mod_folder(settings, mod_name, false);

    if enabled_folder.exists() {
        tokio::fs::remove_dir_all(enabled_folder).await?;
    }
    if disabled_folder.exists() {
        tokio::fs::remove_dir_all(disabled_folder).await?;
    }

    installed.mark_uninstalled(mod_name);
    installed.save(settings).await?;
    Ok(())
}

pub async fn toggle_mod(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    mod_name: &str,
    enable: bool,
) -> AppResult<()> {
    if settings.managed_folder.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Game folder not configured. Go to Settings > Game to set it up.".to_string(),
        ));
    }
    if settings.game.is_silksong() {
        set_silksong_mod_enabled(settings, mod_name, enable)?;
        installed.set_enabled(mod_name, enable);
        installed.save(settings).await?;
        return Ok(());
    }

    InstalledModsStore::move_mod_folder(settings, mod_name, enable).await?;
    installed.set_enabled(mod_name, enable);
    installed.save(settings).await?;
    Ok(())
}

pub async fn install_api(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    catalog: &CatalogResponse,
) -> AppResult<()> {
    if settings.managed_folder.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Game folder not configured. Go to Settings > Game to set it up.".to_string(),
        ));
    }

    if !settings.game.is_silksong() {
        ensure_valid_hk_managed_folder(settings)?;
    }

    let api = &catalog.api;
    if api.url.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "modding api download is unavailable".to_string(),
        ));
    }

    let client = reqwest::Client::builder().user_agent("Needlelight").build()?;
    let bytes = client
        .get(&api.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    if !api.sha256.trim().is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode_upper(hasher.finalize());
        if actual != api.sha256.to_uppercase() {
            return Err(AppError::HashMismatch);
        }
    }

    let target = if settings.game.is_silksong() {
        settings.game_root_path()
    } else {
        ensure_hk_api_backup(settings).await?;
        determine_hk_api_target(settings, bytes.as_ref())?
    };

    tokio::fs::create_dir_all(&target).await?;
    extract_zip_guarded(bytes.as_ref(), &target)?;

    installed.db.api_install = Some(super::models::PersistedModState {
        enabled: true,
        version: api.version.clone(),
        pinned: false,
    });
    installed.save(settings).await?;

    Ok(())
}

async fn ensure_hk_api_backup(settings: &AppSettings) -> AppResult<()> {
    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    let vanilla = managed.join("Assembly-CSharp.dll.v");

    if current.exists() && !vanilla.exists() {
        tokio::fs::copy(&current, &vanilla).await?;
    }

    Ok(())
}

fn extract_zip_guarded(data: &[u8], destination: &Path) -> AppResult<()> {
    let reader = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| AppError::InvalidInput("zip entry path traversal blocked".to_string()))?
            .to_path_buf();

        let output = destination.join(enclosed);
        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&output)?;
            continue;
        }

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&output)?;
        std::io::copy(&mut entry, &mut file)?;
    }

    Ok(())
}

pub fn detect_api_version(path: &Path) -> AppResult<Option<i32>> {
    if !path.exists() {
        return Ok(None);
    }

    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let ascii = String::from_utf8_lossy(&buf);
    let marker = "_modVersion";
    if let Some(pos) = ascii.find(marker) {
        let rest = &ascii[pos..];
        let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).take(3).collect();
        if let Ok(v) = digits.parse::<i32>() {
            return Ok(Some(v));
        }
    }

    Ok(None)
}

pub fn map_state_after_install(current: &ModState, version: &str) -> ModState {
    match current {
        ModState::NotInModlinks { modlinks_mod, .. } => ModState::NotInModlinks {
            enabled: true,
            pinned: false,
            installed: true,
            modlinks_mod: *modlinks_mod,
        },
        _ => ModState::Installed {
            enabled: true,
            pinned: false,
            version: version.to_string(),
            updated: true,
        },
    }
}

pub fn is_api_installed(settings: &AppSettings, _installed: &InstalledModsStore) -> bool {
    if settings.game.is_silksong() {
        let bepinex = settings.game_root_path().join("BepInEx/core/BepInEx.dll");
        return bepinex.exists();
    }

    if settings.managed_folder.trim().is_empty() {
        return false;
    }

    let managed_dir = PathBuf::from(&settings.managed_folder);
    if managed_dir.join("ModdingApi.dll").exists() {
        return true;
    }

    let managed = managed_dir.join("Assembly-CSharp.dll");
    matches!(detect_api_version(&managed), Ok(Some(_)))
}

fn determine_hk_api_target(settings: &AppSettings, data: &[u8]) -> AppResult<PathBuf> {
    let managed = PathBuf::from(&settings.managed_folder);
    let managed_parent = managed
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| managed.clone());

    let reader = Cursor::new(data);
    let mut archive = ZipArchive::new(reader)?;

    let mut has_data_managed_prefix = false;
    let mut has_managed_prefix = false;
    let mut has_mods_prefix = false;
    let mut has_root_assembly = false;

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name();
        let parts: Vec<&str> = name.split('/').collect();
        let first = parts.first().copied().unwrap_or("");
        if parts.len() >= 2
            && parts[1].eq_ignore_ascii_case("Managed")
            && first.to_lowercase().ends_with("_data")
        {
            has_data_managed_prefix = true;
        }
        if first.eq_ignore_ascii_case("Managed") {
            has_managed_prefix = true;
        }
        if first.eq_ignore_ascii_case("Mods") {
            has_mods_prefix = true;
        }
        if first.eq_ignore_ascii_case("Assembly-CSharp.dll") {
            has_root_assembly = true;
        }
    }

    if has_data_managed_prefix {
        return Ok(settings.game_root_path());
    }

    if has_managed_prefix {
        return Ok(managed_parent);
    }
    if has_mods_prefix || has_root_assembly {
        return Ok(managed);
    }

    Ok(managed_parent)
}

async fn install_mod_with_deps(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    catalog: &CatalogResponse,
    mod_name: &str,
    visited: &mut HashSet<String>,
) -> AppResult<()> {
    let mut stack: Vec<(String, bool)> = vec![(mod_name.to_string(), false)];

    while let Some((current, ready)) = stack.pop() {
        if ready {
            let (item_name, item_version, item_link, item_sha256, is_silksong) = {
                let item = catalog
                    .items
                    .iter()
                    .find(|x| x.name == current)
                    .ok_or_else(|| AppError::NotFound(format!("mod '{current}' not found")))?;

                if let Some(state) = installed.db.mods.get(&item.name) {
                    if state.version == item.version {
                        continue;
                    }
                }

                (
                    item.name.clone(),
                    item.version.clone(),
                    item.link.clone(),
                    item.sha256.clone(),
                    settings.game.is_silksong(),
                )
            };

            if item_link.trim().is_empty() {
                return Err(AppError::InvalidInput("mod has no download link".to_string()));
            }

            if is_silksong {
                ensure_silksong_bepinex(settings).await?;
            }

            let bytes = download_mod_bytes(&item_link, &item_sha256).await?;

            if is_silksong {
                install_silksong_mod_archive(settings, &item_name, bytes.as_ref()).await?;
            } else {
                let folder = InstalledModsStore::mod_folder(settings, &item_name, true);
                if folder.exists() {
                    tokio::fs::remove_dir_all(&folder).await?;
                }
                tokio::fs::create_dir_all(&folder).await?;
                if looks_like_zip(bytes.as_ref()) {
                    extract_zip_guarded(bytes.as_ref(), &folder)?;
                } else {
                    let mut file_name = filename_from_url(&item_link)
                        .unwrap_or_else(|| format!("{item_name}.dll"));
                    if !file_name.to_lowercase().ends_with(".dll") {
                        file_name = format!("{item_name}.dll");
                    }
                    let target = folder.join(file_name);
                    tokio::fs::write(target, bytes.as_slice()).await?;
                }
            }

            installed.mark_installed(&item_name, &item_version, true);
            installed.save(settings).await?;
            continue;
        }

        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());

        let item = catalog
            .items
            .iter()
            .find(|x| x.name == current)
            .ok_or_else(|| AppError::NotFound(format!("mod '{current}' not found")))?;

        if let Some(state) = installed.db.mods.get(&item.name) {
            if state.version == item.version {
                continue;
            }
        }

        let dependencies: Vec<String> = item
            .dependencies
            .iter()
            .filter(|dep| !dep.contains("BepInExPack") && !dep.trim().is_empty())
            .cloned()
            .collect();

        stack.push((current.clone(), true));
        for dep in dependencies.into_iter().rev() {
            stack.push((dep, false));
        }
    }

    Ok(())
}

async fn download_mod_bytes(url: &str, sha256: &str) -> AppResult<Vec<u8>> {
    let client = reqwest::Client::builder().user_agent("Needlelight").build()?;
    let bytes = client.get(url).send().await?.error_for_status()?.bytes().await?;

    if !sha256.trim().is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode_upper(hasher.finalize());
        if actual != sha256.to_uppercase() {
            return Err(AppError::HashMismatch);
        }
    }

    Ok(bytes.to_vec())
}

async fn ensure_silksong_bepinex(settings: &AppSettings) -> AppResult<()> {
    let bepinex = settings.game_root_path().join("BepInEx/core/BepInEx.dll");
    if bepinex.exists() {
        return Ok(());
    }

    // Reuse the Thunderstore BepInEx pack from the catalog if available
    let api = super::mod_database::CatalogCache::build(settings, &InstalledModsStore::default(), true)
        .await
        .map(|cache| cache.response.api)
        .unwrap_or_else(|_| super::models::ApiInfo {
            url: String::new(),
            version: String::new(),
            sha256: String::new(),
        });

    if api.url.trim().is_empty() {
        return Err(AppError::InvalidInput("BepInEx pack not available".to_string()));
    }

    let bytes = download_mod_bytes(&api.url, &api.sha256).await?;
    let target = settings.game_root_path();
    tokio::fs::create_dir_all(&target).await?;
    extract_zip_guarded(bytes.as_ref(), &target)?;
    Ok(())
}

async fn install_silksong_mod_archive(
    settings: &AppSettings,
    mod_name: &str,
    data: &[u8],
) -> AppResult<()> {
    let bepinex_root = settings.game_root_path().join("BepInEx");
    tokio::fs::create_dir_all(&bepinex_root).await?;

    for folder in silksong_mod_paths(settings, mod_name) {
        if folder.exists() {
            std::fs::remove_dir_all(&folder)?;
        }
    }

    let reader = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let mut path = entry
            .enclosed_name()
            .ok_or_else(|| AppError::InvalidInput("zip entry path traversal blocked".to_string()))?
            .to_path_buf();

        if path.components().count() == 0 {
            continue;
        }

        // Strip leading BepInEx folder if present
        if let Some(first) = path.components().next().and_then(|c| c.as_os_str().to_str()) {
            if first.eq_ignore_ascii_case("BepInEx") && path.components().count() > 1 {
                path = path.components().skip(1).collect();
            }
        }

        if path.components().count() == 0 {
            continue;
        }

        let root = path.components().next().and_then(|c| c.as_os_str().to_str()).unwrap_or("");
        let (target_base, relative) = match root {
            "plugins" | "patchers" | "core" | "monomod" => {
                let rel = path.components().skip(1).collect::<PathBuf>();
                if rel.components().count() == 0 {
                    continue;
                }
                (bepinex_root.join(root).join(mod_name), rel)
            }
            _ => (bepinex_root.join("plugins").join(mod_name), path),
        };

        let output = target_base.join(relative).to_path_buf();
        if !output.starts_with(&target_base) {
            return Err(AppError::InvalidInput("zip entry path traversal blocked".to_string()));
        }

        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&output)?;
            continue;
        }

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&output)?;
        std::io::copy(&mut entry, &mut file)?;
    }

    Ok(())
}

fn set_silksong_mod_enabled(settings: &AppSettings, mod_name: &str, enabled: bool) -> AppResult<()> {
    let paths = silksong_mod_paths(settings, mod_name);
    for folder in paths {
        if !folder.exists() {
            continue;
        }
        for entry in WalkDir::new(&folder).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "manifest.json" {
                continue;
            }
            let path = entry.path();
            if enabled {
                if name.ends_with(".old") {
                    let target = path.with_file_name(name.trim_end_matches(".old"));
                    if target.exists() {
                        std::fs::remove_file(&target)?;
                    }
                    std::fs::rename(path, target)?;
                }
            } else if !name.ends_with(".old") {
                let target = path.with_file_name(format!("{name}.old"));
                if target.exists() {
                    std::fs::remove_file(&target)?;
                }
                std::fs::rename(path, target)?;
            }
        }
    }

    Ok(())
}

async fn remove_silksong_mod(settings: &AppSettings, mod_name: &str) -> AppResult<()> {
    for folder in silksong_mod_paths(settings, mod_name) {
        if folder.exists() {
            tokio::fs::remove_dir_all(folder).await?;
        }
    }
    Ok(())
}

fn silksong_mod_paths(settings: &AppSettings, mod_name: &str) -> Vec<PathBuf> {
    let bepinex = settings.game_root_path().join("BepInEx");
    vec![
        bepinex.join("plugins").join(mod_name),
        bepinex.join("patchers").join(mod_name),
        bepinex.join("core").join(mod_name),
        bepinex.join("monomod").join(mod_name),
    ]
}
