use futures::Stream;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

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
        let client = Client::new().post("http://localhost:11434/api/generate");

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
        let client = Client::new().post("http://localhost:11434/api/pull");

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
        let client = Client::new().delete("http://localhost:11434/api/delete");

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
    pub name: Option<String>,
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
        let client = reqwest::blocking::Client::new().get("http://localhost:11434/api/tags");

        let request = client.send().unwrap().json::<Tags>();

        Self { result: request }
    }
}
