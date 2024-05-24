use std::hash::Hash;

use cosmic::{
    iced::futures::{Stream, StreamExt},
    iced_futures::MaybeSend,
};
use futures::SinkExt;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::models::Models;

#[derive(Debug, Clone)]
pub enum Event {
    Ready(mpsc::Sender<Request>),
    Response(BotResponse),
    PullResponse(PullModelResponse),
    PullDone,
    RemoveStatus(String),
    RemovedModel,
    Done,
}

#[derive(Debug, Clone)]
pub enum Request {
    Ask((Models, String)),
    AskWithContext((Models, String, Option<Vec<u64>>)),
    PullModel(Models),
    RemoveModel(Models),
}

pub fn subscription<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
) -> cosmic::iced::Subscription<Event> {
    use cosmic::iced::subscription;

    subscription::channel(id, 1, |mut output| async move {
        loop {
            let mut responses = std::pin::pin!(service());
            while let Some(message) = responses.next().await {
                let _res = output.send(message).await;
            }
        }
    })
}

pub fn service() -> impl Stream<Item = Event> + MaybeSend {
    let (requests_tx, mut requests_rx) = mpsc::channel(4);
    let (responses_tx, mut responses_rx) = mpsc::channel(4);

    let service_future = async move {
        let _res = responses_tx.send(Event::Ready(requests_tx.clone())).await;

        let client = &mut None;
        let pull_client = &mut None;

        while let Some(request) = requests_rx.recv().await {
            match request {
                Request::Ask((model, text)) => {
                    _ = client_request(model, text, None, &responses_tx, client).await
                }
                Request::AskWithContext((model, text, context)) => {
                    _ = client_request(model, text, context, &responses_tx, client).await
                }
                Request::PullModel(model) => {
                    _ = pull_request(model.to_string(), &responses_tx, pull_client).await
                }
                Request::RemoveModel(model) => {
                    _ = remove_request(model.to_string(), &responses_tx).await
                }
            }
        }
    };

    let _res = tokio::task::spawn(service_future);

    async_stream::stream! {
        while let Some(message) = responses_rx.recv().await {
            yield message;
        }
    }
}

async fn client_request<'a>(
    model: Models,
    prompt: String,
    context: Option<Vec<u64>>,
    tx: &mpsc::Sender<Event>,
    client: &'a mut Option<(Bot, oneshot::Sender<()>)>,
) -> &'a mut Option<(Bot, oneshot::Sender<()>)> {
    if client.is_none() {
        *client = match Bot::new(model.to_string(), prompt, context).await {
            Ok((new_client, responses)) => {
                let tx = tx.clone();

                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
                let listener = async {
                    let listener = Box::pin(async move {
                        let mut responses = std::pin::pin!(responses);
                        while let Some(Ok(response)) = responses.next().await {
                            let data = serde_json::from_slice::<BotResponse>(&response);

                            if let Ok(res) = data {
                                let _res = tx.send(Event::Response(res)).await;
                            }
                        }
                        let _ = tx.send(Event::Done).await;
                    });

                    let killswitch = Box::pin(async move {
                        let _res = kill_rx.await;
                    });

                    futures::future::select(listener, killswitch).await;
                };

                let _res = tokio::task::spawn(listener);

                Some((new_client, kill_tx))
            }
            Err(_why) => None,
        }
    };

    client
}

#[derive(Serialize)]
struct GenerateNonContext {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Serialize)]
struct GenerateWithContext {
    model: String,
    prompt: String,
    stream: bool,
    context: Vec<u64>,
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
                stream: true,
                context: context.unwrap(),
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

async fn pull_request<'a>(
    model: String,
    tx: &mpsc::Sender<Event>,
    client: &'a mut Option<(PullModel, oneshot::Sender<()>)>,
) -> &'a mut Option<(PullModel, oneshot::Sender<()>)> {
    if client.is_none() {
        *client = match PullModel::new(model).await {
            Ok((new_client, responses)) => {
                let tx = tx.clone();

                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
                let listener = async {
                    let listener = Box::pin(async move {
                        let mut responses = std::pin::pin!(responses);
                        while let Some(Ok(response)) = responses.next().await {
                            let data = serde_json::from_slice::<PullModelResponse>(&response);

                            if let Ok(res) = data {
                                let _res = tx.send(Event::PullResponse(res)).await;
                            }
                        }
                        let _ = tx.send(Event::PullDone).await;
                    });

                    let killswitch = Box::pin(async move {
                        let _res = kill_rx.await;
                    });

                    futures::future::select(listener, killswitch).await;
                };

                let _res = tokio::task::spawn(listener);

                Some((new_client, kill_tx))
            }
            Err(_why) => None,
        }
    };

    client
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

async fn remove_request<'a>(model: String, tx: &mpsc::Sender<Event>) -> anyhow::Result<()> {
    if let Ok((_new_client, status_code)) = RemoveModel::new(model).await {
        if status_code.is_success() {
            _ = tx.send(Event::RemoveStatus(String::from("Removed successfully")));
        } else {
            _ = tx.send(Event::RemoveStatus(String::from("Can't remove model")));
        };
    };

    let _ = tx.send(Event::RemovedModel).await;

    Ok(())
}
