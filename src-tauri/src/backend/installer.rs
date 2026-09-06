use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{CatalogResponse, ModState},
    settings::AppSettings,
};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs::{File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}};
use tauri::{AppHandle, Emitter};
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

pub(crate) fn write_install_log(message: impl AsRef<str>) {
    let message = message.as_ref();
    log::info!("{message}");

    let result = (|| -> std::io::Result<()> {
        let log_path = AppSettings::config_dir()
            .map_err(|error| std::io::Error::other(error.to_string()))?
            .join("Needlelight-install.log");
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(log_path)?;
        writeln!(file, "{} {message}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))
    })();

    if let Err(error) = result {
        log::warn!("Could not write installation diagnostic: {error}");
    }
}

pub async fn install_mod<R: tauri::Runtime>(
    app: &AppHandle<R>,
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    catalog: &CatalogResponse,
    mod_name: &str,
) -> AppResult<()> {
    write_install_log(format!("Starting mod install: {mod_name} (game: {})", settings.game.as_str()));
    if settings.managed_folder.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Game folder not configured. Go to Settings > Game to set it up.".to_string(),
        ));
    }

    if !settings.game.is_silksong() {
        ensure_valid_hk_managed_folder(settings)?;
        if !is_api_installed(settings, installed) {
            install_api(app, settings, installed, catalog).await?;
        }
        ensure_hk_api_enabled(settings).await?;
    } else if !is_api_installed(settings, installed) {
        write_install_log("Modding API is missing; installing it before the mod.");
        install_api(app, settings, installed, catalog).await?;
    }

    let mut visited = HashSet::new();
    install_mod_with_deps(settings, installed, catalog, mod_name, &mut visited).await
}

pub async fn uninstall_mod(
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    mod_name: &str,
) -> AppResult<()> {
    write_install_log(format!("Starting mod uninstall: {mod_name} (game: {})", settings.game.as_str()));
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

#[derive(Clone, serde::Serialize)]
struct ApiInstallProgress {
    progress: u8,
    stage: String,
}

fn emit_api_progress<R: tauri::Runtime>(app: &AppHandle<R>, progress: u8, stage: impl Into<String>) {
    let _ = app.emit("api-install-progress", ApiInstallProgress {
        progress: progress.min(100),
        stage: stage.into(),
    });
}

pub async fn install_api<R: tauri::Runtime>(
    app: &AppHandle<R>,
    settings: &AppSettings,
    installed: &mut InstalledModsStore,
    catalog: &CatalogResponse,
) -> AppResult<()> {
    write_install_log(format!("Starting Modding API install (game: {}).", settings.game.as_str()));
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

    emit_api_progress(app, 0, "Downloading Modding API...");
    let client = reqwest::Client::builder().user_agent("Needlelight").build()?;
    let mut response = client
        .get(&api.url)
        .send()
        .await?
        .error_for_status()?;
    let total = response.content_length();
    let mut downloaded = 0u64;
    let mut bytes = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let progress = total
            .filter(|size| *size > 0)
            .map(|size| ((downloaded.saturating_mul(70) / size).min(70)) as u8)
            .unwrap_or(35);
        emit_api_progress(app, progress, "Downloading Modding API...");
    }

    write_install_log(format!("Downloaded Modding API archive ({} bytes).", bytes.len()));
    emit_api_progress(app, 72, "Verifying download...");

    if !api.sha256.trim().is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode_upper(hasher.finalize());
        if actual != api.sha256.to_uppercase() {
            return Err(AppError::HashMismatch);
        }
    }

    emit_api_progress(app, 82, "Installing API files...");
    if settings.game.is_silksong() {
        let target = settings.game_root_path();
        tokio::fs::create_dir_all(&target).await?;
        extract_zip_guarded(bytes.as_ref(), &target, &["BepInEx"])?;
    } else {
        install_hk_api_payload(settings, bytes.as_ref()).await?;
    }

    emit_api_progress(app, 94, "Verifying installation...");
    if !is_api_installed(settings, installed) {
        write_install_log(format!(
            "Modding API verification failed after extraction to {}.",
            settings.managed_folder
        ));
        return Err(AppError::InvalidInput(
            "Modding API files were not found after installation. See Needlelight-install.log for details.".to_string(),
        ));
    }

    installed.db.api_install = Some(super::models::PersistedModState {
        enabled: true,
        version: api.version.clone(),
        pinned: false,
    });
    installed.db.has_vanilla = !settings.game.is_silksong();
    installed.save(settings).await?;
    write_install_log(format!("Modding API installed successfully in {}.", settings.managed_folder));
    emit_api_progress(app, 100, "Installation complete");

    Ok(())
}

async fn install_hk_api_payload(settings: &AppSettings, data: &[u8]) -> AppResult<()> {
    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    let vanilla = managed.join("Assembly-CSharp.dll.v");
    let modded = managed.join("Assembly-CSharp.dll.m");

    if !current.exists() {
        return Err(AppError::InvalidInput(
            "Managed folder is invalid (Assembly-CSharp.dll not found).".to_string(),
        ));
    }

    // The first install captures the pristine game assembly exactly once.
    // If the API is installed but currently disabled, Current is vanilla and .m
    // contains the API. Re-enable it first so reinstall always ends in the same
    // clean layout: Current=API, .v=vanilla, and no stale .m file.
    if modded.exists() && !is_hk_api_current(settings)? {
        ensure_hk_api_enabled(settings).await?;
    }

    if !vanilla.exists() {
        tokio::fs::copy(&current, &vanilla).await?;
    }

    // Never destroy a known-good vanilla backup. When reinstalling while the API is
    // enabled, current is the modded assembly and can be replaced in place.
    extract_zip_guarded(data, &managed, &[])?;

    // Make sure an API-enabled state has the API assembly as Current and the vanilla
    // backup remains untouched. A fresh install intentionally has no .m yet.
    if is_hk_api_current(settings)? {
        // Keep current as the newly extracted API payload.
    }

    Ok(())
}

pub async fn ensure_hk_api_enabled(settings: &AppSettings) -> AppResult<()> {
    if settings.game.is_silksong() || settings.managed_folder.trim().is_empty() {
        return Ok(());
    }

    if is_hk_api_current(settings)? {
        return Ok(());
    }

    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    let vanilla = managed.join("Assembly-CSharp.dll.v");
    let modded = managed.join("Assembly-CSharp.dll.m");

    if !modded.exists() {
        return Err(AppError::InvalidInput(
            "The Modding API is installed but its modded Assembly-CSharp backup is missing.".to_string(),
        ));
    }

    // In the disabled state, Current is vanilla and .m is the API assembly.
    // Move vanilla to .v, then restore the API into Current.
    replace_file(&current, &vanilla).await?;
    replace_file(&modded, &current).await?;
    Ok(())
}

pub async fn ensure_hk_api_disabled(settings: &AppSettings) -> AppResult<()> {
    if settings.game.is_silksong() || settings.managed_folder.trim().is_empty() {
        return Ok(());
    }

    if !is_hk_api_current(settings)? {
        return Ok(());
    }

    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    let vanilla = managed.join("Assembly-CSharp.dll.v");
    let modded = managed.join("Assembly-CSharp.dll.m");

    if !vanilla.exists() {
        return Err(AppError::InvalidInput(
            "Cannot disable the Modding API because the vanilla Assembly-CSharp backup is missing.".to_string(),
        ));
    }

    replace_file(&current, &modded).await?;
    replace_file(&vanilla, &current).await?;
    Ok(())
}

async fn replace_file(source: &Path, destination: &Path) -> AppResult<()> {
    if destination.exists() {
        tokio::fs::remove_file(destination).await?;
    }
    tokio::fs::rename(source, destination).await?;
    Ok(())
}

fn is_hk_api_current(settings: &AppSettings) -> AppResult<bool> {
    if settings.game.is_silksong() || settings.managed_folder.trim().is_empty() {
        return Ok(false);
    }
    let current = PathBuf::from(&settings.managed_folder).join("Assembly-CSharp.dll");
    Ok(matches!(detect_api_version(&current), Ok(Some(_))))
}

fn extract_zip_guarded(data: &[u8], destination: &Path, preserve_roots: &[&str]) -> AppResult<()> {
    let names = {
        let reader = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(reader)?;
        let mut collected = Vec::new();
        for i in 0..archive.len() {
            collected.push(archive.by_index(i)?.name().to_string());
        }
        collected
    };

    let wrapped_root = detect_wrapped_root(&names, preserve_roots);

    let reader = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| AppError::InvalidInput("zip entry path traversal blocked".to_string()))?
            .to_path_buf();

        let relative = if let Some(root) = wrapped_root.as_ref() {
            strip_wrapped_root(&enclosed, root)
        } else {
            enclosed
        };
        let output = destination.join(relative);
        if !output.starts_with(destination) {
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

fn detect_wrapped_root(names: &[String], preserve_roots: &[&str]) -> Option<String> {
    let mut first_root: Option<String> = None;

    for name in names {
        let path = Path::new(name);
        let mut components = path.components();
        let root = components.next()?.as_os_str().to_str()?;
        if components.next().is_none() {
            return None;
        }

        if let Some(existing) = &first_root {
            if !existing.eq_ignore_ascii_case(root) {
                return None;
            }
        } else {
            first_root = Some(root.to_string());
        }
    }

    let root = first_root?;
    if preserve_roots.iter().any(|candidate| candidate.eq_ignore_ascii_case(&root)) {
        None
    } else {
        Some(root)
    }
}

fn strip_wrapped_root(path: &Path, wrapped_root: &str) -> PathBuf {
    let mut components = path.components();
    let Some(first) = components.next() else {
        return path.to_path_buf();
    };

    if first.as_os_str().to_str().is_some_and(|segment| segment.eq_ignore_ascii_case(wrapped_root)) {
        components.collect()
    } else {
        path.to_path_buf()
    }
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

pub fn is_hk_api_enabled(settings: &AppSettings) -> bool {
    if settings.game.is_silksong() || settings.managed_folder.trim().is_empty() {
        return false;
    }
    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    matches!(detect_api_version(&current), Ok(Some(_)))
}

pub fn is_api_installed(settings: &AppSettings, _installed: &InstalledModsStore) -> bool {
    if settings.game.is_silksong() {
        let bepinex = settings.game_root_path().join("BepInEx/core/BepInEx.dll");
        return bepinex.exists();
    }

    if settings.managed_folder.trim().is_empty() {
        return false;
    }

    let managed = PathBuf::from(&settings.managed_folder);
    if managed.join("ModdingApi.dll").exists() || is_hk_api_enabled(settings) {
        return true;
    }

    // API may currently be disabled. In that state the vanilla Assembly-CSharp.dll
    // is Current and the installed API is held in Assembly-CSharp.dll.m.
    managed.join("Assembly-CSharp.dll.m").is_file()
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
                    if state.version == item.version && installed_mod_exists_on_disk(settings, &item.name) {
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

            let bytes = download_mod_bytes(&item_link, &item_sha256).await?;
            write_install_log(format!("Downloaded mod {item_name} ({} bytes).", bytes.len()));

            if is_silksong {
                if looks_like_zip(bytes.as_ref()) {
                    install_silksong_mod_archive(settings, &item_name, bytes.as_ref()).await?;
                } else {
                    let folder = settings.mods_folder().join(&item_name);
                    tokio::fs::create_dir_all(&folder).await?;
                    let file_name = filename_from_url(&item_link)
                        .filter(|name| name.to_lowercase().ends_with(".dll"))
                        .unwrap_or_else(|| format!("{item_name}.dll"));
                    tokio::fs::write(folder.join(file_name), bytes.as_slice()).await?;
                }
            } else {
                let folder = InstalledModsStore::mod_folder(settings, &item_name, true);
                if folder.exists() {
                    tokio::fs::remove_dir_all(&folder).await?;
                }
                tokio::fs::create_dir_all(&folder).await?;
                if looks_like_zip(bytes.as_ref()) {
                    extract_zip_guarded(bytes.as_ref(), &folder, &[])?;
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
            if !installed_mod_exists_on_disk(settings, &item_name) {
                write_install_log(format!("Mod verification failed: {item_name} is missing after extraction."));
                return Err(AppError::InvalidInput(format!("{item_name} was not found after installation. See Needlelight-install.log for details.")));
            }
            write_install_log(format!("Installed mod {item_name} version {item_version}."));
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

fn installed_mod_exists_on_disk(settings: &AppSettings, mod_name: &str) -> bool {
    if settings.game.is_silksong() {
        return silksong_mod_paths(settings, mod_name)
            .into_iter()
            .any(|path| path.exists());
    }

    InstalledModsStore::mod_folder(settings, mod_name, true).exists()
        || InstalledModsStore::mod_folder(settings, mod_name, false).exists()
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
