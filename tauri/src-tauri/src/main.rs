use std::process::{Child, Command};
use std::sync::Mutex;

use tauri::Manager;

struct ApiChild(Mutex<Option<Child>>);

fn start_api(app: &tauri::AppHandle) -> Result<Child, String> {
    if let Ok(path) = std::env::var("DWATA_API_PATH") {
        return Command::new(path)
            .spawn()
            .map_err(|e| format!("failed to spawn DWATA_API_PATH: {e}"));
    }

    let mut candidates = Vec::new();

    if let Ok(exe_dir) = app.path().executable_dir() {
        candidates.push(exe_dir.join("dwata-api"));
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("dwata-api"));
    }

    if cfg!(target_os = "windows") {
        for candidate in &mut candidates {
            candidate.set_extension("exe");
        }
    }

    let candidate = candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            "dwata-api sidecar not found in executable or resource dir (set DWATA_API_PATH for dev)"
                .to_string()
        })?;

    Command::new(candidate)
        .spawn()
        .map_err(|e| format!("failed to spawn dwata-api sidecar: {e}"))
}

fn stop_api(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<ApiChild>() {
        if let Some(mut child) = state.0.lock().ok().and_then(|mut g| g.take()) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            match start_api(app.handle()) {
                Ok(child) => {
                    app.manage(ApiChild(Mutex::new(Some(child))));
                }
                Err(err) => {
                    eprintln!("[dwata] {err}");
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                stop_api(&window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
