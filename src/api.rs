use futures::Stream;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::Settings;

#[derive(Serialize)]
struct GenerateNonContext {
    model: String,
    prompt: String,
    stream: bool,
    images: Vec<String>,
}

#[derive(Serialize)]
struct GenerateWithContext {
    model: String,
    prompt: String,
    images: Vec<String>,
    context: Vec<u64>,
    stream: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BotResponse {
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

#[derive(Debug)]
pub struct Bot {
    pub prompt: String,
}

impl Bot {
    pub async fn new(
        model: String,
        prompt: String,
        images: Vec<String>,
        context: Option<Vec<u64>>,
    ) -> anyhow::Result<(
        Self,
        impl Stream<Item = anyhow::Result<bytes::Bytes, reqwest::Error>>,
    )> {
        let settings = Settings::load();
        let client = Client::new().post(format!("http://{}/api/generate", settings.ollama_address));

        let stream = if context.is_none() {
            let no_context_query = GenerateNonContext {
                model,
                prompt,
                images,
                stream: true,
            };

            client
                .json::<GenerateNonContext>(&no_context_query)
                .send()
                .await
                .unwrap()
                .bytes_stream()
        } else {
            let context_query = GenerateWithContext {
                model,
                prompt,
                images,
                context: context.unwrap(),
                stream: true,
            };

            client
                .json::<GenerateWithContext>(&context_query)
                .send()
                .await
                .unwrap()
                .bytes_stream()
        };

        let bot = Self {
            prompt: String::new(),
        };

        Ok((bot, stream))
    }
}

#[derive(Serialize)]
pub struct PullModelQuery {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PullModelResponse {
    pub status: String,
    pub digest: Option<String>,
    pub total: Option<u64>,
    pub completed: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PullModel {
    pub model: String,
}

impl PullModel {
    pub async fn new(
        model: String,
    ) -> anyhow::Result<(
        Self,
        impl Stream<Item = anyhow::Result<bytes::Bytes, reqwest::Error>>,
    )> {
        let settings = Settings::load();
        let client = Client::new().post(format!("http://{}/api/pull", settings.ollama_address));

        let pull_query = PullModelQuery { name: model };

        let stream = client
            .json::<PullModelQuery>(&pull_query)
            .send()
            .await
            .unwrap()
            .bytes_stream();

        let pull = Self {
            model: String::new(),
        };

        Ok((pull, stream))
    }
}

#[derive(Debug, Serialize)]
pub struct RemoveModelQuery {
    name: String,
}

#[derive(Debug, Clone)]
pub struct RemoveModel {
    pub model: String,
}

impl RemoveModel {
    pub async fn new(model: String) -> anyhow::Result<(Self, StatusCode)> {
        let settings = Settings::load();
        let client = Client::new().delete(format!("http://{}/api/delete", settings.ollama_address));

        let remove_query = RemoveModelQuery { name: model };

        let request = client
            .json::<RemoveModelQuery>(&remove_query)
            .send()
            .await?
            .status();

        let remove = RemoveModel {
            model: String::new(),
        };

        Ok((remove, request))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tags {
    pub models: Vec<Model>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub model: String,
    pub modified_at: Option<String>,
    pub size: Option<u64>,
    pub digest: Option<String>,
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelDetails {
    pub format: Option<String>,
    pub family: Option<String>,
    pub families: Option<Vec<String>>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
}

#[derive(Debug)]
pub struct ListModels {
    pub result: anyhow::Result<Tags, reqwest::Error>,
}

impl ListModels {
    pub fn new() -> Self {
        let settings = Settings::load();
        let client = reqwest::blocking::Client::new()
            .get(format!("http://{}/api/tags", settings.ollama_address));

        let request = client.send();

        if let Ok(result) = request {
            Self {
                result: result.json::<Tags>(),
            }
        } else {
            Self {
                result: Ok(Tags { models: Vec::new() }),
            }
        }
    }
}
