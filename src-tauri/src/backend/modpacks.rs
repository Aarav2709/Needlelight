// modpacks are isolated sets of mods for one game, installed by the regular installer via profile_settings
// silksong follows cogfly: each modpack has its own bepinex tree and doorstop is pointed at it per launch
// hollow knight follows lumafly: launching mirrors the modpack's enabled mods into the game's mods folder

use super::{
    errors::{AppError, AppResult},
    settings::{AppSettings, GameKey},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// installed mods database kept inside each modpack folder
const PROFILE_DB_FILE: &str = "installed.json";

// extra process arguments and environment variables for a launch
#[derive(Debug, Default, Clone)]
pub struct LaunchExtras {
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
}

// settings for the real install of a game, whichever game is currently active
pub fn game_settings(base: &AppSettings, game: &GameKey) -> AppSettings {
    let mut settings = base.clone();
    // run the legacy single folder migrations under the original game before retargeting
    settings.sync_managed_folder();
    settings.sync_custom_modlinks();
    settings.game = game.clone();
    // normalize like startup does, the stored folder may be the game root instead of the managed folder
    settings.managed_folder =
        AppSettings::normalize_managed_folder(&settings.managed_folder_for(game), game);
    settings.sync_custom_modlinks();
    // modpacks always use each game's official catalog, even if an older version left a custom one on
    settings.use_custom_modlinks = false;
    settings.installed_db_override = None;
    settings
}

// settings rooted at a modpack folder so every path helper and the regular installer work inside it
pub fn profile_settings(base: &AppSettings, profile_dir: &Path, game: &GameKey) -> AppSettings {
    let mut settings = game_settings(base, game);
    settings.managed_folder = profile_dir.to_string_lossy().to_string();
    settings.installed_db_override = Some(profile_dir.join(PROFILE_DB_FILE));
    settings
}

// hollow knight, lumafly model

#[derive(Debug, Default, Serialize, Deserialize)]
struct HkModpackState {
    // modpack folder mirrored into the game, none while the game holds the player's own mods
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

// path of the hollow knight modpack currently applied to the game, if any
pub fn active_hk_modpack() -> Option<String> {
    hk_state_dir()
        .ok()
        .and_then(|dir| read_hk_state(&dir).active_profile)
}

// mirrors a hollow knight modpack's enabled mods into the game's mods folder
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
        // first modpack, so set the player's own mods aside once without ever overwriting an older backup
        let original = dir.join("original");
        let target = if has_entries(&original) {
            dir.join(format!(
                "original-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ))
        } else {
            original
        };
        move_children(&game_mods, &target)?;
    } else {
        // the folder holds an earlier modpack, keep one generation in case it was edited by hand
        let last = dir.join("last-replaced");
        if last.exists() {
            std::fs::remove_dir_all(&last)?;
        }
        move_children(&game_mods, &last)?;
    }

    // disabled mods live in the modpack's disabled folder and simply aren't copied
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

// puts back the mods the game had before the first modpack, false when there was nothing to restore
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

// silksong, cogfly model

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

// the bepinex preloader inside a silksong modpack, if its bepinex install is intact
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

// arguments and environment that make the game load bepinex from the modpack instead of the game folder
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
        // match the argument style to the proxy in the game folder, which may predate this modpack's pack
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

    // native linux and macos builds load doorstop through the dynamic linker instead
    let (lib_names, preload_var): (&[&str], &str) = if cfg!(target_os = "macos") {
        (
            &["libdoorstop.dylib", "doorstop_libs/libdoorstop.dylib"],
            "DYLD_INSERT_LIBRARIES",
        )
    } else {
        (
            &["libdoorstop.so", "doorstop_libs/libdoorstop_x64.so"],
            "LD_PRELOAD",
        )
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

    let mut envs = if doorstop_major(profile_dir).unwrap_or(4) >= 4 {
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
    envs.push((preload_var.to_string(), preload));
    Ok(LaunchExtras { args: vec![], envs })
}

// puts the doorstop proxy in the game folder without replacing files, with a disabled config so steam launches stay vanilla
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

// file helpers

fn has_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

// moves every entry of src into dst, creating dst if needed and leaving src itself in place
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

// renames, or copies then deletes when source and target are on different drives
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

// copies a file or a whole folder tree
pub(crate) fn copy_path(src: &Path, dst: &Path) -> AppResult<()> {
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
