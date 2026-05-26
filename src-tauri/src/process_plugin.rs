use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub uuid: String,
    pub pid: u32,
    pub profile_path: String,
}

#[derive(Default)]
pub struct ProcessStore {
    pub processes: Mutex<HashMap<String, ProcessInfo>>,
}

impl ProcessStore {
    #[allow(dead_code)]
    pub async fn insert(&self, info: ProcessInfo) {
        let mut guard = self.processes.lock().await;
        guard.insert(info.uuid.clone(), info);
    }

    #[allow(dead_code)]
    pub async fn remove(&self, uuid: &str) {
        let mut guard = self.processes.lock().await;
        guard.remove(uuid);
    }
}

#[tauri::command]
pub async fn process_get_by_profile_path(
    state: State<'_, ProcessStore>,
    path: String,
) -> Result<Vec<u32>, String> {
    let guard = state.processes.lock().await;
    Ok(guard
        .values()
        .filter(|info| info.profile_path == path)
        .map(|info| info.pid)
        .collect())
}

#[tauri::command]
pub async fn process_get_all(state: State<'_, ProcessStore>) -> Result<Vec<u32>, String> {
    let guard = state.processes.lock().await;
    Ok(guard.values().map(|info| info.pid).collect())
}

#[tauri::command]
pub async fn process_kill(state: State<'_, ProcessStore>, uuid: String) -> Result<(), String> {
    let mut guard = state.processes.lock().await;
    let info = guard
        .remove(&uuid)
        .ok_or_else(|| "process not found".to_string())?;

    if cfg!(target_os = "windows") {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &info.pid.to_string(), "/T", "/F"])
            .status();
    } else {
        let _ = std::process::Command::new("kill")
            .args(["-9", &info.pid.to_string()])
            .status();
    }

    Ok(())
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("process")
        .invoke_handler(tauri::generate_handler![
            process_get_by_profile_path,
            process_get_all,
            process_kill,
        ])
        .build()
}
