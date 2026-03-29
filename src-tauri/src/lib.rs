mod api_client;
mod docker;
mod ollama;
mod task_runner;
mod config;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state
pub struct AppState {
    pub config: Mutex<config::SandboxConfig>,
    pub api: api_client::ApiClient,
    pub runner: Mutex<task_runner::TaskRunner>,
    pub is_online: Mutex<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub is_online: bool,
    pub current_load: u32,
    pub max_concurrent: u32,
    pub docker_available: bool,
    pub ollama_available: bool,
    pub model_loaded: String,
    pub tasks_completed: u64,
    pub uptime_seconds: u64,
}

// ============================================================
// TAURI COMMANDS — exposed to React frontend via invoke()
// ============================================================

/// Get current sandbox status
#[tauri::command]
async fn get_status(state: tauri::State<'_, Arc<AppState>>) -> Result<StatusResponse, String> {
    let config = state.config.lock().await;
    let is_online = *state.is_online.lock().await;
    let runner = state.runner.lock().await;

    let docker_ok = docker::check_docker().await.unwrap_or(false);
    let ollama_ok = ollama::check_ollama(&config.ollama_url).await.unwrap_or(false);

    Ok(StatusResponse {
        is_online,
        current_load: runner.current_load(),
        max_concurrent: config.max_concurrent,
        docker_available: docker_ok,
        ollama_available: ollama_ok,
        model_loaded: config.model_name.clone(),
        tasks_completed: runner.tasks_completed(),
        uptime_seconds: runner.uptime_seconds(),
    })
}

/// Go online — start polling for tasks
#[tauri::command]
async fn go_online(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut is_online = state.is_online.lock().await;

    // Pre-flight checks
    let config = state.config.lock().await;
    if !docker::check_docker().await.unwrap_or(false) {
        return Err("Docker is not running. Start Docker Desktop first.".into());
    }
    if !ollama::check_ollama(&config.ollama_url).await.unwrap_or(false) {
        return Err(format!("Cannot reach Ollama at {}. Is it running?", config.ollama_url));
    }

    // Verify model is loaded
    let models = ollama::list_models(&config.ollama_url).await
        .map_err(|e| format!("Failed to list models: {}", e))?;
    if !models.iter().any(|m| m.contains(&config.model_name)) {
        return Err(format!(
            "Model '{}' not found. Available: {}",
            config.model_name,
            models.join(", ")
        ));
    }

    *is_online = true;

    // Notify platform
    state.api.set_online(true).await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Go offline — stop accepting tasks, finish current ones
#[tauri::command]
async fn go_offline(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut is_online = state.is_online.lock().await;
    *is_online = false;
    state.api.set_online(false).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Update operator settings
#[tauri::command]
async fn update_settings(
    state: tauri::State<'_, Arc<AppState>>,
    settings: config::SandboxConfig,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    *config = settings.clone();
    config::save_config(&settings).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get available Ollama models
#[tauri::command]
async fn list_models(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    let config = state.config.lock().await;
    ollama::list_models(&config.ollama_url)
        .await
        .map_err(|e| e.to_string())
}

/// Run certification benchmark for a category
#[tauri::command]
async fn run_benchmark(
    state: tauri::State<'_, Arc<AppState>>,
    category: String,
) -> Result<f64, String> {
    let config = state.config.lock().await;
    let score = state.api.run_certification(&category, &config.model_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(score)
}

// ============================================================
// APP ENTRY POINT
// ============================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let config = config::load_config().unwrap_or_default();
    let api = api_client::ApiClient::new(
        config.platform_url.clone(),
        config.api_token.clone(),
    );

    let state = Arc::new(AppState {
        config: Mutex::new(config),
        api,
        runner: Mutex::new(task_runner::TaskRunner::new()),
        is_online: Mutex::new(false),
    });

    let state_clone = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            get_status,
            go_online,
            go_offline,
            update_settings,
            list_models,
            run_benchmark,
        ])
        .setup(move |app| {
            // Spawn background task poller
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                task_runner::poll_loop(state_clone, handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
