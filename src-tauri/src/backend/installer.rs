use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{CatalogResponse, ModState},
    settings::AppSettings,
};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};
use zip::ZipArchive;

pub async fn install_mod(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    catalog: &CatalogResponse,
    mod_name: &str,
) -> AppResult<()> {
    let item = catalog
        .items
        .iter()
        .find(|x| x.name == mod_name)
        .ok_or_else(|| AppError::NotFound(format!("mod '{mod_name}' not found")))?
        .clone();

    if item.link.trim().is_empty() {
        return Err(AppError::InvalidInput("mod has no download link".to_string()));
    }

    let client = reqwest::Client::builder().user_agent("Needlelight").build()?;
    let bytes = client
        .get(&item.link)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    if !item.sha256.trim().is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode_upper(hasher.finalize());
        if actual != item.sha256.to_uppercase() {
            return Err(AppError::HashMismatch);
        }
    }

    let folder = InstalledModsStore::mod_folder(settings, &item.name, true);
    if folder.exists() {
        tokio::fs::remove_dir_all(&folder).await?;
    }
    tokio::fs::create_dir_all(&folder).await?;

    extract_zip_guarded(bytes.as_ref(), &folder)?;

    installed.mark_installed(&item.name, &item.version, true);
    installed.save(settings).await?;
    Ok(())
}

pub async fn uninstall_mod(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    mod_name: &str,
) -> AppResult<()> {
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
    let api = &catalog.api;
    if api.url.trim().is_empty() {
        return Ok(());
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

    let managed = Path::new(&settings.managed_folder);
    tokio::fs::create_dir_all(managed).await?;
    extract_zip_guarded(bytes.as_ref(), managed)?;

    installed.db.api_install = Some(super::models::PersistedModState {
        enabled: true,
        version: api.version.to_string(),
        pinned: false,
    });
    installed.save(settings).await?;

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
