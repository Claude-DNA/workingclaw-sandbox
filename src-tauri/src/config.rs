use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct SandboxConfig {
    /// WorkingClaw Market platform URL
    pub platform_url: String,
    /// Operator API token (from platform login)
    pub api_token: String,
    /// Ollama endpoint
    pub ollama_url: String,
    /// Model to use for task processing
    pub model_name: String,
    /// Max concurrent tasks (team agreed: 5-15 realistic for consumer hardware)
    pub max_concurrent: u32,
    /// Docker image for task sandboxing
    pub docker_image: String,
    /// Container memory limit (MB)
    pub container_memory_mb: u64,
    /// Container CPU limit (cores)
    pub container_cpu_limit: f64,
    /// Task timeout (seconds)
    pub task_timeout_secs: u64,
    /// Poll interval for new tasks (seconds)
    pub poll_interval_secs: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            platform_url: "https://market.workingclaw.com".into(),
            api_token: String::new(),
            ollama_url: "http://localhost:11434".into(),
            model_name: "llama3.1:8b".into(),
            max_concurrent: 5,
            docker_image: "workingclaw/sandbox:latest".into(),
            container_memory_mb: 512,
            container_cpu_limit: 1.0,
            task_timeout_secs: 300,
            poll_interval_secs: 5,
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("workingclaw-sandbox")
        .join("config.json")
}

pub fn load_config() -> Result<SandboxConfig, Box<dyn std::error::Error>> {
    let path = config_path();
    if !path.exists() {
        return Ok(SandboxConfig::default());
    }
    let data = std::fs::read_to_string(&path)?;
    let config: SandboxConfig = serde_json::from_str(&data)?;
    Ok(config)
}

pub fn save_config(config: &SandboxConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, data)?;
    Ok(())
}
