use std::sync::Arc;

use cosmic::iced::futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct Api {
    client: Client,
    port: u32,
    model: Option<String>,
}

#[derive(Serialize)]
struct GenerateQuery {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GenerateResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<u64>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u64>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
}

impl Api {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            port: 11434,
            model: None,
        }
    }

    pub fn change_port(&mut self, port: u32) -> &mut Self {
        self.port = port;
        self
    }

    pub fn set_model(&mut self, model: String) -> &mut Self {
        self.model = Some(model);
        self
    }
}

pub async fn prompt_req(api: Api, prompt: Arc<String>) -> GenerateResponse {
    let model = api.model.as_ref().unwrap().clone();
    let query_params = GenerateQuery {
        model,
        prompt: prompt.to_string(),
        stream: false,
    };

    api.client
        .post(format!("http://localhost:{}/api/generate", api.port))
        .json::<GenerateQuery>(&query_params)
        .send()
        .await
        .unwrap()
        .json::<GenerateResponse>()
        .await
        .unwrap()
}
