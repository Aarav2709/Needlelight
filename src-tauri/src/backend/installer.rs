use super::{
    errors::{AppError, AppResult},
    installed_mods::InstalledModsStore,
    models::{CatalogResponse, ModState},
    settings::AppSettings,
};
use serde::{Deserialize, Serialize};
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
    install_mod_with_deps(app, settings, installed, catalog, mod_name, &mut visited).await
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

#[derive(Clone, serde::Serialize)]
struct ModInstallProgress {
    item_name: String,
    progress: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HkApiBackupFile {
    relative_path: String,
    vanilla_existed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HkApiBackupManifest {
    version: String,
    files: Vec<HkApiBackupFile>,
}

fn emit_mod_progress<R: tauri::Runtime>(app: &AppHandle<R>, item_name: &str, progress: u8) {
    let _ = app.emit("mod-install-progress", ModInstallProgress {
        item_name: item_name.to_string(),
        progress: progress.min(100),
    });
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
        log_extracted_tree(&target.join("BepInEx"), 3);
    } else {
        install_hk_api_payload(settings, bytes.as_ref(), &api.version).await?;
    }

    emit_api_progress(app, 94, "Verifying installation...");
    if !is_api_installed(settings, installed) {
        let install_target = if settings.game.is_silksong() {
            settings.game_root_path().display().to_string()
        } else {
            settings.managed_folder.clone()
        };
        write_install_log(format!(
            "Modding API verification failed after extraction to {install_target}."
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

// logs the extracted BepInEx tree (bounded depth) so a verification
// failure shows exactly what actually landed on disk instead of a guess
fn log_extracted_tree(root: &Path, max_depth: usize) {
    if !root.is_dir() {
        write_install_log(format!("Extraction check: {} does not exist.", root.display()));
        return;
    }
    let mut lines = vec![format!("Extracted tree under {}:", root.display())];
    for entry in WalkDir::new(root).max_depth(max_depth).into_iter().flatten() {
        if let Ok(relative) = entry.path().strip_prefix(root) {
            if !relative.as_os_str().is_empty() {
                lines.push(format!("  {}", relative.display()));
            }
        }
    }
    write_install_log(lines.join("\n"));
}

async fn install_hk_api_payload(settings: &AppSettings, data: &[u8], api_version: &str) -> AppResult<()> {
    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    if !current.exists() {
        return Err(AppError::InvalidInput(
            "Managed folder is invalid (Assembly-CSharp.dll not found).".to_string(),
        ));
    }

    let state_dir = hk_api_state_dir(settings);
    let manifest_path = state_dir.join("manifest.json");

    // manifest-backed installs can always be safely refreshed
    if manifest_path.is_file() {
        restore_hk_vanilla(settings).await?;
        tokio::fs::remove_dir_all(&state_dir).await?;
    } else if managed.join("Assembly-CSharp.dll.m").exists() {
        return Err(AppError::InvalidInput(
            "An older API installation without a complete backup was found. Verify Hollow Knight's files in Steam, then reinstall the Modding API.".to_string(),
        ));
    }

    let relative_files = collect_api_archive_files(data)?;
    if relative_files.is_empty() {
        return Err(AppError::InvalidInput(
            "The Modding API archive contains no installable files.".to_string(),
        ));
    }

    let vanilla_dir = state_dir.join("vanilla");
    let modded_dir = state_dir.join("modded");
    tokio::fs::create_dir_all(&vanilla_dir).await?;
    tokio::fs::create_dir_all(&modded_dir).await?;

    let mut manifest = HkApiBackupManifest {
        version: api_version.to_string(),
        files: Vec::with_capacity(relative_files.len()),
    };

    for relative in &relative_files {
        let current_path = managed.join(relative);
        let vanilla_path = vanilla_dir.join(relative);
        if let Some(parent) = vanilla_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let vanilla_existed = current_path.is_file();
        if vanilla_existed {
            tokio::fs::copy(&current_path, &vanilla_path).await?;
        }

        manifest.files.push(HkApiBackupFile {
            relative_path: relative.to_string_lossy().replace('\\', "/"),
            vanilla_existed,
        });
    }

    extract_zip_guarded(data, &managed, &[])?;

    for file in &manifest.files {
        let relative = PathBuf::from(&file.relative_path);
        let installed_path = managed.join(&relative);
        if !installed_path.is_file() {
            return Err(AppError::InvalidInput(format!(
                "Modding API installation is missing {}.",
                file.relative_path
            )));
        }
        let modded_path = modded_dir.join(&relative);
        if let Some(parent) = modded_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::copy(&installed_path, &modded_path).await?;
    }

    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| AppError::InvalidInput(format!("could not write API backup manifest: {e}")))?;
    tokio::fs::write(&manifest_path, manifest_bytes).await?;

    Ok(())
}

fn hk_api_state_dir(settings: &AppSettings) -> PathBuf {
    PathBuf::from(&settings.managed_folder).join(".needlelight-api")
}

fn hk_api_manifest(settings: &AppSettings) -> PathBuf {
    hk_api_state_dir(settings).join("manifest.json")
}

// must match extract_zip_guarded's wrapped-root stripping, or the manifest
// paths won't match what actually lands on disk
fn collect_api_archive_files(data: &[u8]) -> AppResult<Vec<PathBuf>> {
    let names = {
        let reader = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(reader)?;
        let mut collected = Vec::new();
        for i in 0..archive.len() {
            collected.push(archive.by_index(i)?.name().to_string());
        }
        collected
    };

    let wrapped_root = detect_wrapped_root(&names, &[]);

    let reader = std::io::Cursor::new(data);
    let mut archive = ZipArchive::new(reader)?;
    let mut files = Vec::new();

    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if entry.name().ends_with('/') {
            continue;
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| AppError::InvalidInput("zip entry path traversal blocked".to_string()))?
            .to_path_buf();
        let relative = if let Some(root) = wrapped_root.as_ref() {
            strip_wrapped_root(&enclosed, root)
        } else {
            enclosed
        };
        if relative.components().count() > 0 {
            files.push(relative);
        }
    }

    Ok(files)
}

async fn restore_hk_vanilla(settings: &AppSettings) -> AppResult<()> {
    let manifest_path = hk_api_manifest(settings);
    if !manifest_path.is_file() {
        return Err(AppError::InvalidInput("Hollow Knight API backup manifest is missing.".to_string()));
    }

    let manifest_bytes = tokio::fs::read(&manifest_path).await?;
    let manifest: HkApiBackupManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| AppError::InvalidInput(format!("could not read API backup manifest: {e}")))?;
    let managed = PathBuf::from(&settings.managed_folder);
    let vanilla_dir = hk_api_state_dir(settings).join("vanilla");

    for file in manifest.files {
        let relative = PathBuf::from(file.relative_path);
        let current = managed.join(&relative);
        if file.vanilla_existed {
            let backup = vanilla_dir.join(&relative);
            if !backup.is_file() {
                return Err(AppError::InvalidInput(format!(
                    "Missing vanilla API backup for {}.",
                    relative.display()
                )));
            }
            if let Some(parent) = current.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(backup, current).await?;
        } else if current.exists() {
            tokio::fs::remove_file(current).await?;
        }
    }

    Ok(())
}

async fn restore_hk_modded(settings: &AppSettings) -> AppResult<()> {
    let manifest_path = hk_api_manifest(settings);
    if !manifest_path.is_file() {
        return Err(AppError::InvalidInput("Hollow Knight API backup manifest is missing.".to_string()));
    }

    let manifest_bytes = tokio::fs::read(&manifest_path).await?;
    let manifest: HkApiBackupManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| AppError::InvalidInput(format!("could not read API backup manifest: {e}")))?;
    let managed = PathBuf::from(&settings.managed_folder);
    let modded_dir = hk_api_state_dir(settings).join("modded");

    for file in manifest.files {
        let relative = PathBuf::from(file.relative_path);
        let backup = modded_dir.join(&relative);
        let current = managed.join(&relative);
        if !backup.is_file() {
            return Err(AppError::InvalidInput(format!(
                "Missing Modding API backup for {}.",
                relative.display()
            )));
        }
        if let Some(parent) = current.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::copy(backup, current).await?;
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

    if hk_api_manifest(settings).is_file() {
        restore_hk_modded(settings).await?;
        return Ok(());
    }

    // legacy single-file backup, kept for old installs
    let managed = PathBuf::from(&settings.managed_folder);
    let current = managed.join("Assembly-CSharp.dll");
    let vanilla = managed.join("Assembly-CSharp.dll.v");
    let modded = managed.join("Assembly-CSharp.dll.m");
    if !modded.exists() {
        return Err(AppError::InvalidInput(
            "The Modding API is installed but its API backup is missing.".to_string(),
        ));
    }
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

    if hk_api_manifest(settings).is_file() {
        restore_hk_vanilla(settings).await?;
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
    let mut found_nested_entry = false;

    for name in names {
        let name = name.trim_end_matches('/');

        if name.is_empty() {
            continue;
        }

        let path = Path::new(name);
        let mut components = path.components();

        let Some(root) = components.next() else {
            continue;
        };

        // Ignore top-level directory entries like "BepInExPack/".
        if components.next().is_none() {
            continue;
        }

        found_nested_entry = true;

        let Some(root) = root.as_os_str().to_str() else {
            return None;
        };

        if let Some(existing) = &first_root {
            if !existing.eq_ignore_ascii_case(root) {
                return None;
            }
        } else {
            first_root = Some(root.to_string());
        }
    }

    if !found_nested_entry {
        return None;
    }

    let root = first_root?;

    if preserve_roots
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&root))
    {
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
        let core = settings.game_root_path().join("BepInEx").join("core");
        if !core.is_dir() {
            return false;
        }
        // different BepInEx releases have shipped different core dll names;
        // accept any of them, or fall back to "core has any file in it" so a
        // successful extraction is never rejected over one hardcoded name
        let known_markers = ["BepInEx.dll", "BepInEx.Core.dll", "BepInEx.Preloader.dll", "BepInEx.Unity.dll"];
        if known_markers.iter().any(|name| core.join(name).is_file()) {
            return true;
        }
        return std::fs::read_dir(&core).map(|mut entries| entries.next().is_some()).unwrap_or(false);
    }

    if settings.managed_folder.trim().is_empty() {
        return false;
    }

    if is_hk_api_enabled(settings) {
        return true;
    }

    // installed-but-disabled: check the backup's real content, not just
    // that a manifest file exists, so a stale backup can't be trusted
    let managed = PathBuf::from(&settings.managed_folder);
    let manifest_modded = hk_api_state_dir(settings).join("modded").join("Assembly-CSharp.dll");
    let legacy_modded = managed.join("Assembly-CSharp.dll.m");

    matches!(detect_api_version(&manifest_modded), Ok(Some(_)))
        || matches!(detect_api_version(&legacy_modded), Ok(Some(_)))
}

async fn install_mod_with_deps<R: tauri::Runtime>(
    app: &AppHandle<R>,
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

            let bytes = download_mod_bytes(app, &item_name, &item_link, &item_sha256).await?;
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

async fn download_mod_bytes<R: tauri::Runtime>(
    app: &AppHandle<R>,
    item_name: &str,
    url: &str,
    sha256: &str,
) -> AppResult<Vec<u8>> {
    emit_mod_progress(app, item_name, 0);
    let client = reqwest::Client::builder().user_agent("Needlelight").build()?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    let total = response.content_length();
    let mut downloaded = 0u64;
    let mut bytes = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        let progress = total
            .filter(|size| *size > 0)
            .map(|size| ((downloaded.saturating_mul(92) / size).min(92)) as u8)
            .unwrap_or(0);
        if total.is_some() {
            emit_mod_progress(app, item_name, progress);
        }
    }

    emit_mod_progress(app, item_name, 96);

    if !sha256.trim().is_empty() {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode_upper(hasher.finalize());
        if actual != sha256.to_uppercase() {
            emit_mod_progress(app, item_name, 0);
            return Err(AppError::HashMismatch);
        }
    }

    emit_mod_progress(app, item_name, 100);
    Ok(bytes)
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
