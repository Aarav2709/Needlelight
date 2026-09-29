//! Modpacks: named, isolated sets of mods for one game.
//!
//! The two games load mods differently, so they follow the launcher each community already
//! uses:
//!
//! * **Silksong** follows Cogfly (and r2modman / Thunderstore Mod Manager): every modpack keeps
//!   its own complete BepInEx tree, and launching points Unity Doorstop at that tree through
//!   per-process arguments. The game folder's mods are never touched; the only thing placed in
//!   the game folder is the Doorstop proxy DLL, and only if it isn't already there.
//! * **Hollow Knight** follows Lumafly: the Modding API only loads mods from the game's real
//!   `Managed/Mods` folder, so launching a modpack mirrors its enabled mods into that folder.
//!   Whatever was in there before the first modpack was applied is moved aside once, and can be
//!   restored at any time.
//!
//! Mods are installed into a modpack by the regular installer, pointed at the modpack folder
//! through [`profile_settings`], so dependency resolution, hash checks and archive layout rules
//! are shared with the rest of the app instead of being reimplemented here.

use super::{
    errors::{AppError, AppResult},
    settings::{AppSettings, GameKey},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Installed-mods database kept inside each modpack folder.
const PROFILE_DB_FILE: &str = "installed.json";

/// Extra process arguments / environment variables for a launch.
#[derive(Debug, Default, Clone)]
pub struct LaunchExtras {
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

/// Settings for the real install of `game`, independent of which game is currently active.
pub fn game_settings(base: &AppSettings, game: &GameKey) -> AppSettings {
    let mut settings = base.clone();
    // Run the legacy single-folder migrations under the original game before retargeting,
    // otherwise they would attach the old game's folder/catalog to the new one.
    settings.sync_managed_folder();
    settings.sync_custom_modlinks();
    settings.game = game.clone();
    // Normalize like startup does: the stored folder may be the game root rather than
    // `<game>_Data/Managed`, and the Hollow Knight Mods folder lives under Managed.
    settings.managed_folder =
        AppSettings::normalize_managed_folder(&settings.managed_folder_for(game), game);
    settings.sync_custom_modlinks();
    // Custom catalogs aren't part of the modpack workflow: modpacks always use each game's
    // supported catalog (ModLinks for Hollow Knight, Thunderstore for Silksong), even if an
    // older version of the app left a custom catalog switched on.
    settings.use_custom_modlinks = false;
    settings.installed_db_override = None;
    settings
}

/// Settings rooted at a modpack folder. Every path helper (`mods_folder`, `game_root_path`,
/// `installed_mods_path`) then resolves inside the modpack, which lets the regular installer
/// install, update, toggle and uninstall mods in a modpack unchanged.
pub fn profile_settings(base: &AppSettings, profile_dir: &Path, game: &GameKey) -> AppSettings {
    let mut settings = game_settings(base, game);
    settings.managed_folder = profile_dir.to_string_lossy().to_string();
    settings.installed_db_override = Some(profile_dir.join(PROFILE_DB_FILE));
    settings
}

// ─── Hollow Knight (Lumafly model) ──────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
struct HkModpackState {
    /// Modpack folder currently mirrored into the game, or None when the game holds the
    /// player's own mods.
    active_profile: Option<String>,
}

fn hk_state_dir() -> AppResult<PathBuf> {
    Ok(AppSettings::config_dir()?
        .join("modpacks")
        .join(GameKey::HollowKnight.as_str()))
}

fn read_hk_state(dir: &Path) -> HkModpackState {
    std::fs::read_to_string(dir.join("state.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_hk_state(dir: &Path, state: &HkModpackState) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("state.json"), serde_json::to_string_pretty(state)?)?;
    Ok(())
}

/// Path of the Hollow Knight modpack currently applied to the game, if any.
pub fn active_hk_modpack() -> Option<String> {
    hk_state_dir()
        .ok()
        .and_then(|dir| read_hk_state(&dir).active_profile)
}

/// Mirror a Hollow Knight modpack's enabled mods into the game's `Managed/Mods` folder.
pub fn apply_hk_modpack(real: &AppSettings, profile_dir: &Path) -> AppResult<()> {
    if real.managed_folder.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Hollow Knight wasn't found on this computer. Open Settings → Games to locate it."
                .to_string(),
        ));
    }

    let game_mods = real.mods_folder();
    std::fs::create_dir_all(&game_mods)?;

    let dir = hk_state_dir()?;
    std::fs::create_dir_all(&dir)?;
    let mut state = read_hk_state(&dir);

    if state.active_profile.is_none() {
        // First modpack: set the player's own mods aside exactly once so they can be restored.
        // Never overwrite an existing backup; fall back to a timestamped folder instead.
        let original = dir.join("original");
        let target = if has_entries(&original) {
            dir.join(format!("original-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")))
        } else {
            original
        };
        move_children(&game_mods, &target)?;
    } else {
        // The folder holds a previously applied modpack. Keep one generation as a safety net
        // in case anything was changed in the game folder directly in the meantime.
        let last = dir.join("last-replaced");
        if last.exists() {
            std::fs::remove_dir_all(&last)?;
        }
        move_children(&game_mods, &last)?;
    }

    // Disabled mods live in Mods/Disabled inside the modpack and simply aren't copied.
    let profile_mods = profile_dir.join("Mods");
    if profile_mods.is_dir() {
        for entry in std::fs::read_dir(&profile_mods)?.flatten() {
            let name = entry.file_name();
            if name == "Disabled" {
                continue;
            }
            copy_path(&entry.path(), &game_mods.join(&name))?;
        }
    }

    state.active_profile = Some(profile_dir.to_string_lossy().to_string());
    write_hk_state(&dir, &state)
}

/// Put back the mods the game had before the first modpack was applied.
/// Returns false when no modpack was applied (nothing to restore).
pub fn restore_hk_original(real: &AppSettings) -> AppResult<bool> {
    let dir = hk_state_dir()?;
    let mut state = read_hk_state(&dir);
    if state.active_profile.is_none() {
        return Ok(false);
    }

    let game_mods = real.mods_folder();
    std::fs::create_dir_all(&game_mods)?;

    let last = dir.join("last-replaced");
    if last.exists() {
        std::fs::remove_dir_all(&last)?;
    }
    move_children(&game_mods, &last)?;

    let original = dir.join("original");
    if original.is_dir() {
        move_children(&original, &game_mods)?;
        let _ = std::fs::remove_dir(&original);
    }

    state.active_profile = None;
    write_hk_state(&dir, &state)?;
    Ok(true)
}

// ─── Silksong (Cogfly model) ────────────────────────────────────────────────

const DOORSTOP4_DISABLED_INI: &str = "[General]
enabled = false
target_assembly = BepInEx\\core\\BepInEx.Preloader.dll
redirect_output_log = false
boot_config_override =
ignore_disable_switch = false

[UnityMono]
dll_search_path_override =
debug_enabled = false
debug_address = 127.0.0.1:10000
debug_suspend = false
";

const DOORSTOP3_DISABLED_INI: &str = "[UnityDoorstop]
enabled=false
targetAssembly=BepInEx\\core\\BepInEx.Preloader.dll
redirectOutputLog=false
ignoreDisableSwitch=false
dllSearchPathOverride=
";

/// The BepInEx preloader inside a Silksong modpack, if its BepInEx install is intact.
pub fn silksong_preloader(profile_dir: &Path) -> Option<PathBuf> {
    let core = profile_dir.join("BepInEx").join("core");
    ["BepInEx.Preloader.dll", "BepInEx.Unity.Mono.Preloader.dll"]
        .iter()
        .map(|name| core.join(name))
        .find(|path| path.is_file())
}

fn doorstop_major(dir: &Path) -> Option<u32> {
    std::fs::read_to_string(dir.join(".doorstop_version"))
        .ok()
        .and_then(|version| version.trim().chars().next())
        .and_then(|c| c.to_digit(10))
}

/// Arguments / environment that make the game load BepInEx from `profile_dir` instead of the
/// game folder. Requires the modpack's BepInEx to be installed (see [`silksong_preloader`]).
pub fn silksong_launch_extras(real: &AppSettings, profile_dir: &Path) -> AppResult<LaunchExtras> {
    let preloader = silksong_preloader(profile_dir).ok_or_else(|| {
        AppError::InvalidInput(
            "This modpack's mod loader is incomplete. Check your connection and try again."
                .to_string(),
        )
    })?;
    let target = preloader.to_string_lossy().to_string();
    let game_root = real.game_root_path();

    if cfg!(target_os = "windows") {
        ensure_doorstop_proxy(profile_dir, &game_root)?;
        // Match the argument style to the proxy that will actually run, which is the one in
        // the game folder (it may predate this modpack's BepInEx pack).
        let major = doorstop_major(&game_root)
            .or_else(|| doorstop_major(profile_dir))
            .unwrap_or(4);
        let args = if major >= 4 {
            vec!["--doorstop-enabled", "true", "--doorstop-target-assembly"]
        } else {
            vec!["--doorstop-enable", "true", "--doorstop-target"]
        };
        let mut args: Vec<String> = args.into_iter().map(String::from).collect();
        args.push(target);
        return Ok(LaunchExtras { args, envs: vec![] });
    }

    // Native Linux / macOS builds load Doorstop through the dynamic linker instead.
    let (lib_names, preload_var): (&[&str], &str) = if cfg!(target_os = "macos") {
        (&["libdoorstop.dylib", "doorstop_libs/libdoorstop.dylib"], "DYLD_INSERT_LIBRARIES")
    } else {
        (&["libdoorstop.so", "doorstop_libs/libdoorstop_x64.so"], "LD_PRELOAD")
    };
    let lib = lib_names
        .iter()
        .map(|name| profile_dir.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            AppError::InvalidInput(
                "Hollow Knight: Silksong modpacks can't be launched on this system yet."
                    .to_string(),
            )
        })?;

    let mut preload = lib.to_string_lossy().to_string();
    if let Ok(existing) = std::env::var(preload_var) {
        if !existing.trim().is_empty() {
            preload = format!("{preload}:{existing}");
        }
    }

    let envs = if doorstop_major(profile_dir).unwrap_or(4) >= 4 {
        vec![
            ("DOORSTOP_ENABLED".to_string(), "1".to_string()),
            ("DOORSTOP_TARGET_ASSEMBLY".to_string(), target),
        ]
    } else {
        vec![
            ("DOORSTOP_ENABLE".to_string(), "TRUE".to_string()),
            ("DOORSTOP_INVOKE_DLL_PATH".to_string(), target),
        ]
    };
    let mut envs = envs;
    envs.push((preload_var.to_string(), preload));
    Ok(LaunchExtras { args: vec![], envs })
}

/// Make sure the Doorstop proxy DLL exists in the game folder. Existing files are never
/// replaced. A config is only written when none exists, and it ships with Doorstop disabled so
/// starting the game from Steam stays vanilla - modpack launches enable it per process.
fn ensure_doorstop_proxy(profile_dir: &Path, game_root: &Path) -> AppResult<()> {
    let proxy = ["winhttp.dll", "version.dll"]
        .iter()
        .find(|name| game_root.join(name).is_file() || profile_dir.join(name).is_file())
        .ok_or_else(|| {
            AppError::InvalidInput(
                "This modpack's mod loader is missing files. Create the modpack again, or try \
                 again later."
                    .to_string(),
            )
        })?;

    if !game_root.join(proxy).is_file() {
        std::fs::copy(profile_dir.join(proxy), game_root.join(proxy))?;
        let version = profile_dir.join(".doorstop_version");
        if version.is_file() && !game_root.join(".doorstop_version").exists() {
            std::fs::copy(&version, game_root.join(".doorstop_version"))?;
        }
    }

    let ini = game_root.join("doorstop_config.ini");
    if !ini.exists() {
        let major = doorstop_major(game_root)
            .or_else(|| doorstop_major(profile_dir))
            .unwrap_or(4);
        let body = if major >= 4 {
            DOORSTOP4_DISABLED_INI
        } else {
            DOORSTOP3_DISABLED_INI
        };
        std::fs::write(ini, body)?;
    }
    Ok(())
}

// ─── File helpers ───────────────────────────────────────────────────────────

fn has_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

/// Move every entry of `src` into `dst` (created if needed). `src` itself stays in place.
fn move_children(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;
    if !src.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(src)?.flatten() {
        move_path(&entry.path(), &dst.join(entry.file_name()))?;
    }
    Ok(())
}

/// Rename, falling back to copy + delete when source and target are on different drives
/// (the game library and the app's config folder often are).
fn move_path(src: &Path, dst: &Path) -> AppResult<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_path(src, dst)?;
    if src.is_dir() {
        std::fs::remove_dir_all(src)?;
    } else {
        std::fs::remove_file(src)?;
    }
    Ok(())
}

fn copy_path(src: &Path, dst: &Path) -> AppResult<()> {
    if src.is_dir() {
        for entry in walkdir::WalkDir::new(src) {
            let entry = entry.map_err(|e| AppError::InvalidInput(e.to_string()))?;
            let relative = entry
                .path()
                .strip_prefix(src)
                .map_err(|e| AppError::InvalidInput(e.to_string()))?;
            let target = dst.join(relative);
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(entry.path(), &target)?;
            }
        }
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
    }
    Ok(())
}
