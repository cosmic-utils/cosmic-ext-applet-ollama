use cosmic::{
    iced::futures::{Stream, StreamExt},
    iced_futures::MaybeSend,
};
use tokio::sync::{mpsc, oneshot};

use crate::api::{Bot, BotResponse, PullModel, PullModelResponse, RemoveModel};

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
    Ask((String, String, Vec<String>, String)),
    AskWithContext((String, String, Vec<String>, Option<Vec<u64>>, String)),
    PullModel(String),
    RemoveModel(String),
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
                Request::Ask((model, text, images, keep_alive)) => {
                    _ = client_request(model, text, images, None, keep_alive, &responses_tx, client)
                        .await
                }
                Request::AskWithContext((model, text, images, context, keep_alive)) => {
                    _ = client_request(
                        model,
                        text,
                        images,
                        context,
                        keep_alive,
                        &responses_tx,
                        client,
                    )
                    .await
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
    model: String,
    prompt: String,
    images: Vec<String>,
    context: Option<Vec<u64>>,
    keep_alive_model: String,
    tx: &mpsc::Sender<Event>,
    client: &'a mut Option<(Bot, oneshot::Sender<()>)>,
) -> &'a mut Option<(Bot, oneshot::Sender<()>)> {
    if client.is_none() {
        *client = match Bot::new(model.to_string(), prompt, images, context, keep_alive_model).await
        {
            Ok((new_client, responses)) => {
                let tx = tx.clone();

                let (kill_tx, kill_rx) = oneshot::channel();
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

async fn pull_request<'a>(
    model: String,
    tx: &mpsc::Sender<Event>,
    client: &'a mut Option<(PullModel, oneshot::Sender<()>)>,
) -> &'a mut Option<(PullModel, oneshot::Sender<()>)> {
    if client.is_none() {
        *client = match PullModel::new(model).await {
            Ok((new_client, responses)) => {
                let tx = tx.clone();

                let (kill_tx, kill_rx) = oneshot::channel();
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

async fn remove_request(model: String, tx: &mpsc::Sender<Event>) -> anyhow::Result<()> {
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
