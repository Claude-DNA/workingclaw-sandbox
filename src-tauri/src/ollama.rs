/// Ollama integration — model interface for task processing.
///
/// Supports: Ollama (primary), LM Studio (compatible API), or remote API fallback.
/// Team decision: category-specific model minimums (7B for summarization, 13B+ for email/content).

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: Option<GenerateOptions>,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f64,
    num_predict: i32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
    #[serde(default)]
    eval_count: u64,
    #[serde(default)]
    eval_duration: u64,
}

/// Check if Ollama is running and responsive
pub async fn check_ollama(base_url: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let client = Client::new();
    let resp = client
        .get(format!("{}/api/tags", base_url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;
    Ok(resp.status().is_success())
}

/// List available models
pub async fn list_models(base_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let resp: OllamaTagsResponse = client
        .get(format!("{}/api/tags", base_url))
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.models.into_iter().map(|m| m.name).collect())
}

/// Generate a completion for a task
pub async fn generate(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: i32,
    temperature: f64,
) -> Result<(String, u64, u64), Box<dyn std::error::Error>> {
    let client = Client::new();

    let request = GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
        options: Some(GenerateOptions {
            temperature,
            num_predict: max_tokens,
        }),
    };

    let resp: GenerateResponse = client
        .post(format!("{}/api/generate", base_url))
        .json(&request)
        .send()
        .await?
        .json()
        .await?;

    Ok((resp.response, resp.eval_count, resp.eval_duration))
}

/// Build task-specific prompts based on category
pub fn build_task_prompt(category: &str, title: &str, description: &str, input: &str) -> String {
    match category {
        "document-summary" => format!(
            "You are a professional document summarizer. Create a clear, concise summary of the following document.\n\
            Focus on key points, conclusions, and actionable items.\n\n\
            Title: {}\n\
            Instructions: {}\n\n\
            DOCUMENT:\n{}\n\n\
            SUMMARY:",
            title, description, input
        ),
        "email-draft" => format!(
            "You are a professional email writer. Draft a polished, professional email based on the following brief.\n\
            Match the requested tone and include all specified points.\n\n\
            Brief: {}\n\
            Details: {}\n\n\
            EMAIL:",
            title, description
        ),
        "content-writing" => format!(
            "You are a professional content writer. Write high-quality content based on the following brief.\n\
            Target audience and tone should match the request.\n\n\
            Brief: {}\n\
            Details: {}\n\n\
            Additional context:\n{}\n\n\
            CONTENT:",
            title, description, input
        ),
        _ => format!(
            "Complete the following task professionally and thoroughly.\n\n\
            Task: {}\n\
            Details: {}\n\
            Input: {}\n\n\
            OUTPUT:",
            title, description, input
        ),
    }
}
