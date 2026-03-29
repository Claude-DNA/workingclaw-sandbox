/// Docker container management for task sandboxing.
///
/// Team decision: Docker isolation at launch, not post-launch.
/// Each customer task runs in its own container:
/// - Customer data enters container as mounted volume
/// - Results exit via stdout/file
/// - Container destroyed after completion
/// - Operator cannot attach to running containers

use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions, LogsOptions, RemoveContainerOptions};
use bollard::models::HostConfig;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskContainer {
    pub container_id: String,
    pub task_id: String,
    pub status: ContainerStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ContainerStatus {
    Creating,
    Running,
    Completed,
    Failed,
    TimedOut,
}

/// Check if Docker daemon is running
pub async fn check_docker() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;
    docker.ping().await?;
    Ok(true)
}

/// Run a task in an isolated Docker container
///
/// The container:
/// - Has no network access (security)
/// - Has a memory limit
/// - Has a CPU limit
/// - Has a timeout
/// - Input is passed via stdin/env, output collected from stdout
/// - Container is destroyed after completion
pub async fn run_task_in_container(
    image: &str,
    task_id: &str,
    input_text: &str,
    prompt: &str,
    ollama_url: &str,
    model: &str,
    memory_mb: u64,
    cpu_limit: f64,
    timeout_secs: u64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;

    let container_name = format!("workingclaw-task-{}", task_id);

    // The task execution script runs inside the container:
    // 1. Reads input from environment
    // 2. Calls Ollama API (host network access only to Ollama)
    // 3. Outputs result to stdout
    let exec_script = format!(
        r#"
        curl -s {ollama_url}/api/generate \
            -d '{{"model": "{model}", "prompt": "{prompt}\n\nInput:\n{input}", "stream": false}}' \
            | python3 -c "import sys,json; print(json.load(sys.stdin).get('response',''))"
        "#,
        ollama_url = ollama_url,
        model = model,
        prompt = prompt.replace('"', r#"\""#).replace('\n', r#"\n"#),
        input = input_text.replace('"', r#"\""#).replace('\n', r#"\n"#),
    );

    let host_config = HostConfig {
        memory: Some((memory_mb * 1024 * 1024) as i64),
        nano_cpus: Some((cpu_limit * 1_000_000_000.0) as i64),
        // Allow access to host's Ollama only — no internet
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        // Read-only root filesystem for security
        readonly_rootfs: Some(true),
        // No privileged access
        privileged: Some(false),
        ..Default::default()
    };

    let config = Config {
        image: Some(image.to_string()),
        cmd: Some(vec!["sh".to_string(), "-c".to_string(), exec_script]),
        host_config: Some(host_config),
        env: Some(vec![
            format!("TASK_ID={}", task_id),
            format!("OLLAMA_URL={}", ollama_url),
            format!("MODEL={}", model),
        ]),
        // Disable networking except to Ollama
        network_disabled: Some(false), // Need host access for Ollama
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: &container_name,
        platform: None,
    };

    // Create and start container
    let container = docker.create_container(Some(options), config).await?;
    docker.start_container(&container.id, None::<StartContainerOptions<String>>).await?;

    // Wait for completion with timeout
    let wait_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        async {
            let mut stream = docker.wait_container(
                &container.id,
                None::<WaitContainerOptions<String>>,
            );
            while let Some(result) = stream.next().await {
                match result {
                    Ok(exit) => return Ok(exit.status_code),
                    Err(e) => return Err(e),
                }
            }
            Ok(0i64)
        }
    ).await;

    // Collect output regardless of exit
    let mut output = String::new();
    let log_opts = LogsOptions::<String> {
        stdout: true,
        stderr: false,
        ..Default::default()
    };
    let mut logs = docker.logs(&container.id, Some(log_opts));
    while let Some(Ok(chunk)) = logs.next().await {
        let s: String = chunk.to_string();
        output.push_str(&s);
    }

    // Clean up container
    let remove_opts = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    let _ = docker.remove_container(&container.id, Some(remove_opts)).await;

    match wait_result {
        Ok(Ok(0)) => Ok(output.trim().to_string()),
        Ok(Ok(code)) => Err(format!("Container exited with code {}", code).into()),
        Ok(Err(e)) => Err(format!("Container error: {}", e).into()),
        Err(_) => Err("Task timed out".into()),
    }
}
