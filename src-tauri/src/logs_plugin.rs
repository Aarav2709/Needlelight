use serde::Serialize;
use std::{path::{Path, PathBuf}};

#[derive(Debug, Serialize)]
pub struct Logs {
    pub filename: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize)]
pub struct LogCursor {
    pub cursor: u64,
    pub output: String,
    pub new_file: bool,
}

fn resolve_log_file(profile_path: &Path) -> Option<PathBuf> {
    let candidates = [
        profile_path.join("BepInEx").join("LogOutput.log"),
        profile_path.join("Logs").join("ModLog.txt"),
        profile_path.join("logs").join("latest.log"),
    ];

    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[tauri::command]
pub async fn logs_get_logs(profile_path: String, _clear_contents: Option<bool>) -> Result<Vec<Logs>, String> {
    let profile = PathBuf::from(profile_path);
    let Some(file) = resolve_log_file(&profile) else {
        return Ok(vec![]);
    };

    let content = std::fs::read_to_string(&file).unwrap_or_default();
    Ok(vec![Logs {
        filename: file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("log")
            .to_string(),
        stdout: content,
        stderr: String::new(),
    }])
}

#[tauri::command]
pub async fn logs_get_logs_by_filename(
    profile_path: String,
    _log_type: Option<String>,
    filename: String,
) -> Result<Vec<Logs>, String> {
    let profile = PathBuf::from(profile_path);
    let file = profile.join(filename);
    let content = std::fs::read_to_string(&file).unwrap_or_default();
    Ok(vec![Logs {
        filename: file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("log")
            .to_string(),
        stdout: content,
        stderr: String::new(),
    }])
}

#[tauri::command]
pub async fn logs_get_output_by_filename(
    profile_path: String,
    _log_type: Option<String>,
    filename: String,
) -> Result<String, String> {
    let profile = PathBuf::from(profile_path);
    let file = profile.join(filename);
    Ok(std::fs::read_to_string(&file).unwrap_or_default())
}

#[tauri::command]
pub async fn logs_delete_logs_by_filename(
    profile_path: String,
    _log_type: Option<String>,
    filename: String,
) -> Result<(), String> {
    let profile = PathBuf::from(profile_path);
    let file = profile.join(filename);
    if file.exists() {
        std::fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn logs_delete_logs(profile_path: String) -> Result<(), String> {
    let profile = PathBuf::from(profile_path);
    if let Some(file) = resolve_log_file(&profile) {
        let _ = std::fs::remove_file(file);
    }
    Ok(())
}

#[tauri::command]
pub async fn logs_get_latest_log_cursor(
    profile_path: String,
    cursor: u64,
) -> Result<LogCursor, String> {
    let profile = PathBuf::from(profile_path);
    let Some(file) = resolve_log_file(&profile) else {
        return Ok(LogCursor {
            cursor,
            output: String::new(),
            new_file: false,
        });
    };

    let content = std::fs::read_to_string(&file).unwrap_or_default();
    let bytes = content.as_bytes();
    let mut new_file = false;
    let start = if cursor as usize > bytes.len() {
        new_file = true;
        0
    } else {
        cursor as usize
    };

    let output = String::from_utf8_lossy(&bytes[start..]).to_string();
    Ok(LogCursor {
        cursor: bytes.len() as u64,
        output,
        new_file,
    })
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("logs")
        .invoke_handler(tauri::generate_handler![
            logs_get_logs,
            logs_get_logs_by_filename,
            logs_get_output_by_filename,
            logs_delete_logs_by_filename,
            logs_delete_logs,
            logs_get_latest_log_cursor,
        ])
        .build()
}
