/// Task execution pipeline — the core loop.
///
/// 1. Poll platform for assigned tasks
/// 2. For each task, spin up a Docker container
/// 3. Feed input to Ollama via container
/// 4. Collect output, submit to platform
/// 5. Destroy container
///
/// Respects max_concurrent limit (team agreed: 5-15 for consumer hardware).

use crate::{AppState, docker, ollama, api_client::PlatformTask};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::Semaphore;

pub struct TaskRunner {
    start_time: Instant,
    tasks_completed: u64,
    current_load: u32,
}

impl TaskRunner {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            tasks_completed: 0,
            current_load: 0,
        }
    }

    pub fn current_load(&self) -> u32 {
        self.current_load
    }

    pub fn tasks_completed(&self) -> u64 {
        self.tasks_completed
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Main polling loop — runs in background
pub async fn poll_loop(state: Arc<AppState>, app: tauri::AppHandle) {
    log::info!("Task poller started");

    loop {
        let poll_interval = {
            let config = state.config.lock().await;
            config.poll_interval_secs
        };

        let is_online = *state.is_online.lock().await;

        if is_online {
            match state.api.poll_tasks().await {
                Ok(tasks) => {
                    if !tasks.is_empty() {
                        log::info!("Received {} tasks", tasks.len());
                        // Emit to frontend
                        let _ = app.emit("tasks-received", tasks.len());
                    }

                    let config = state.config.lock().await;
                    let max = config.max_concurrent as usize;
                    let semaphore = Arc::new(Semaphore::new(max));

                    for task in tasks {
                        let state = state.clone();
                        let app = app.clone();
                        let sem = semaphore.clone();

                        tokio::spawn(async move {
                            let _permit = match sem.acquire().await {
                                Ok(p) => p,
                                Err(_) => return,
                            };
                            execute_task(state, app, task).await;
                        });
                    }
                }
                Err(e) => {
                    log::error!("Poll error: {}", e);
                    let _ = app.emit("poll-error", e.to_string());
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(poll_interval)).await;
    }
}

/// Execute a single task in a Docker container
async fn execute_task(state: Arc<AppState>, app: tauri::AppHandle, task: PlatformTask) {
    let task_id = task.id.clone();
    log::info!("Starting task {}", task_id);

    // Update load
    {
        let mut runner = state.runner.lock().await;
        runner.current_load += 1;
    }
    let _ = app.emit("task-started", &task_id);

    // Notify platform
    if let Err(e) = state.api.task_started(&task_id).await {
        log::error!("Failed to notify task started: {}", e);
    }

    let config = state.config.lock().await;
    let model = config.model_name.clone();
    let ollama_url = config.ollama_url.clone();
    let docker_image = config.docker_image.clone();
    let memory_mb = config.container_memory_mb;
    let cpu_limit = config.container_cpu_limit;
    let timeout = config.task_timeout_secs;
    drop(config);

    // Build the prompt
    let prompt = ollama::build_task_prompt(
        &task.category_name,
        &task.title,
        task.description.as_deref().unwrap_or(""),
        task.input_text.as_deref().unwrap_or(""),
    );

    let start = Instant::now();

    // Execute in Docker container
    let result = docker::run_task_in_container(
        &docker_image,
        &task_id,
        task.input_text.as_deref().unwrap_or(""),
        &prompt,
        &ollama_url,
        &model,
        memory_mb,
        cpu_limit,
        timeout,
    )
    .await;

    let elapsed = start.elapsed().as_secs();

    match result {
        Ok(output) => {
            log::info!("Task {} completed in {}s", task_id, elapsed);

            // Submit result to platform
            if let Err(e) = state
                .api
                .submit_result(&task_id, &output, &model, 0, elapsed)
                .await
            {
                log::error!("Failed to submit result for {}: {}", task_id, e);
            }

            {
                let mut runner = state.runner.lock().await;
                runner.tasks_completed += 1;
            }

            let _ = app.emit("task-completed", serde_json::json!({
                "task_id": task_id,
                "duration": elapsed,
                "success": true,
            }));
        }
        Err(e) => {
            log::error!("Task {} failed: {}", task_id, e);

            if let Err(e2) = state.api.task_failed(&task_id, &e.to_string()).await {
                log::error!("Failed to report failure for {}: {}", task_id, e2);
            }

            let _ = app.emit("task-failed", serde_json::json!({
                "task_id": task_id,
                "error": e.to_string(),
            }));
        }
    }

    // Decrement load
    {
        let mut runner = state.runner.lock().await;
        runner.current_load = runner.current_load.saturating_sub(1);
    }
}
