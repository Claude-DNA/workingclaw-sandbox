/// Platform API client — communicates with WorkingClaw Market backend.
///
/// Endpoints match the Next.js API routes in workingclaw-market.

use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PlatformTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub category_name: String,
    pub input_text: Option<String>,
    pub input_file_url: Option<String>,
    pub price: i64,
    pub operator_payout: i64,
}

#[derive(Serialize)]
struct TaskResultPayload {
    output: String,
    model_used: String,
    tokens_used: u64,
    processing_time: u64,
}

#[derive(Serialize)]
struct OnlinePayload {
    is_online: bool,
}

#[derive(Deserialize)]
pub struct CertificationResult {
    pub score: f64,
    pub passed: bool,
}

impl ApiClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Poll for available tasks assigned to this operator
    pub async fn poll_tasks(&self) -> Result<Vec<PlatformTask>, Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .get(format!("{}/api/operators/tasks/pending", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Poll failed ({}): {}", status, body).into());
        }

        let tasks: Vec<PlatformTask> = resp.json().await?;
        Ok(tasks)
    }

    /// Notify platform that task processing has started
    pub async fn task_started(&self, task_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .post(format!("{}/api/tasks/{}/started", self.base_url, task_id))
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        Ok(())
    }

    /// Submit task result to platform
    pub async fn submit_result(
        &self,
        task_id: &str,
        output: &str,
        model_used: &str,
        tokens_used: u64,
        processing_time_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = TaskResultPayload {
            output: output.to_string(),
            model_used: model_used.to_string(),
            tokens_used,
            processing_time: processing_time_secs,
        };

        let resp = self
            .client
            .post(format!("{}/api/tasks/{}/result", self.base_url, task_id))
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Submit result failed: {}", body).into());
        }

        Ok(())
    }

    /// Report task failure to platform
    pub async fn task_failed(
        &self,
        task_id: &str,
        error: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({ "error": error });

        self.client
            .post(format!("{}/api/tasks/{}/failed", self.base_url, task_id))
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    /// Set operator online/offline status
    pub async fn set_online(&self, online: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .post(format!("{}/api/operators/status", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&OnlinePayload { is_online: online })
            .send()
            .await?;
        Ok(())
    }

    /// Run certification benchmark for a task category
    pub async fn run_certification(
        &self,
        category: &str,
        model: &str,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let payload = serde_json::json!({
            "category": category,
            "model": model,
        });

        let resp: CertificationResult = self
            .client
            .post(format!("{}/api/operators/certify", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.score)
    }
}
