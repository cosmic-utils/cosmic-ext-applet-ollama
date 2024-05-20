use std::sync::Arc;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::{is_installed, Models};

pub struct Api {
    client: Client,
    port: u32,
    model: Arc<Models>,
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
            model: Arc::new(Models::NoModel),
        }
    }

    pub fn change_port(&mut self, port: u32) -> &mut Self {
        self.port = port;
        self
    }

    pub fn set_model(&mut self, model: Arc<Models>) -> &mut Self {
        if *model != Models::NoModel && is_installed(&model) {
            self.model = model;
        }
        self
    }
}

pub async fn prompt_req(api: Api, prompt: Arc<String>) -> Option<GenerateResponse> {
    let query_params = GenerateQuery {
        model: api.model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
    };

    if api.model.to_string() != "none" && is_installed(&api.model) {
        return Some(
            api.client
                .post(format!("http://localhost:{}/api/generate", api.port))
                .json::<GenerateQuery>(&query_params)
                .send()
                .await
                .unwrap()
                .json::<GenerateResponse>()
                .await
                .unwrap(),
        );
    }
    None
}
