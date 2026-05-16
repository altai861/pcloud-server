use crate::relay::{
    config::RelayClientConfig,
    protocol::{
        BodyChunk, DeviceToRelay, HeaderPair, RelayRequestStart, RelayResponseStart, RelayToDevice,
        ResponseEnd, ResponseError,
    },
};
use axum::body::Bytes;
use base64::{Engine as _, engine::general_purpose};
use futures_util::{SinkExt, StreamExt};
use reqwest::{
    Client, Method,
    header::{
        CONNECTION, CONTENT_LENGTH, HOST, HeaderName, HeaderValue, TRANSFER_ENCODING, UPGRADE,
    },
};
use std::{collections::HashMap, io, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const STREAM_CHUNK_BYTES: usize = 64 * 1024;
const REQUEST_BODY_BUFFER_CHUNKS: usize = 16;
const OUTGOING_BUFFER_MESSAGES: usize = 128;

type RequestBodySender = mpsc::Sender<Result<Bytes, io::Error>>;
type RequestBodies = Arc<Mutex<HashMap<String, RequestBodySender>>>;
type OutgoingSender = mpsc::Sender<Message>;

pub async fn run_relay_client(config: RelayClientConfig) {
    let http_client = match Client::builder()
        .connect_timeout(config.local_request_timeout)
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("relay client could not initialize local HTTP client: {error}");
            return;
        }
    };

    loop {
        let connect_url = config.device_connect_url();
        println!("connecting to relay server as device {}", config.device_id);

        match connect_async(&connect_url).await {
            Ok((socket, _)) => {
                println!("relay tunnel connected for device {}", config.device_id);
                if let Err(error) = handle_relay_socket(&config, &http_client, socket).await {
                    eprintln!(
                        "relay tunnel ended for device {}: {error}",
                        config.device_id
                    );
                }
            }
            Err(error) => {
                eprintln!("relay tunnel connection failed: {error}");
            }
        }

        tokio::time::sleep(config.reconnect_delay).await;
    }
}

async fn handle_relay_socket(
    config: &RelayClientConfig,
    http_client: &Client,
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> anyhow::Result<()> {
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (outgoing_sender, mut outgoing_receiver) =
        mpsc::channel::<Message>(OUTGOING_BUFFER_MESSAGES);
    let request_bodies: RequestBodies = Arc::new(Mutex::new(HashMap::new()));

    let writer_task = tokio::spawn(async move {
        while let Some(message) = outgoing_receiver.recv().await {
            if socket_sender.send(message).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = socket_receiver.next().await {
        match message? {
            Message::Text(text) => {
                handle_relay_text(
                    config,
                    http_client,
                    &text,
                    outgoing_sender.clone(),
                    request_bodies.clone(),
                )
                .await?;
            }
            Message::Ping(payload) => {
                outgoing_sender.send(Message::Pong(payload)).await.ok();
            }
            Message::Close(frame) => {
                outgoing_sender.send(Message::Close(frame)).await.ok();
                break;
            }
            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }

    writer_task.abort();
    Ok(())
}

async fn handle_relay_text(
    config: &RelayClientConfig,
    http_client: &Client,
    text: &str,
    outgoing_sender: OutgoingSender,
    request_bodies: RequestBodies,
) -> anyhow::Result<()> {
    let relay_message = match serde_json::from_str::<RelayToDevice>(text) {
        Ok(message) => message,
        Err(error) => {
            send_device_message(
                &outgoing_sender,
                DeviceToRelay::ResponseError(ResponseError {
                    request_id: "unknown".to_owned(),
                    status: 400,
                    message: format!("Invalid relay message: {error}"),
                }),
            )
            .await?;
            return Ok(());
        }
    };

    match relay_message {
        RelayToDevice::RequestStart(request) => {
            let request_id = request.request_id.clone();
            let (body_sender, body_receiver) = mpsc::channel(REQUEST_BODY_BUFFER_CHUNKS);
            request_bodies
                .lock()
                .await
                .insert(request_id.clone(), body_sender);

            let config = config.clone();
            let http_client = http_client.clone();
            let outgoing_sender = outgoing_sender.clone();
            let request_bodies = request_bodies.clone();

            tokio::spawn(async move {
                let result = execute_local_request(
                    &config,
                    &http_client,
                    request,
                    body_receiver,
                    &outgoing_sender,
                )
                .await;

                if let Err(error) = result {
                    let _ = send_device_message(
                        &outgoing_sender,
                        DeviceToRelay::ResponseError(ResponseError {
                            request_id: request_id.clone(),
                            status: 502,
                            message: error.to_string(),
                        }),
                    )
                    .await;
                }

                request_bodies.lock().await.remove(&request_id);
            });
        }
        RelayToDevice::RequestBodyChunk(chunk) => {
            let body_sender = {
                let request_bodies = request_bodies.lock().await;
                request_bodies.get(&chunk.request_id).cloned()
            };

            let Some(body_sender) = body_sender else {
                return Ok(());
            };

            match decode_body(&chunk.body_base64) {
                Ok(body) => {
                    let _ = body_sender.send(Ok(Bytes::from(body))).await;
                }
                Err(error) => {
                    let _ = body_sender
                        .send(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid request body chunk: {error}"),
                        )))
                        .await;
                }
            }
        }
        RelayToDevice::RequestEnd(end) => {
            request_bodies.lock().await.remove(&end.request_id);
        }
        RelayToDevice::RequestAbort(abort) => {
            if let Some(body_sender) = request_bodies.lock().await.remove(&abort.request_id) {
                let _ = body_sender
                    .send(Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        abort.message,
                    )))
                    .await;
            }
        }
    }

    Ok(())
}

async fn execute_local_request(
    config: &RelayClientConfig,
    http_client: &Client,
    request: RelayRequestStart,
    body_receiver: mpsc::Receiver<Result<Bytes, io::Error>>,
    outgoing_sender: &OutgoingSender,
) -> anyhow::Result<()> {
    let method = request.method.parse::<Method>()?;
    let url = local_url(&config.local_base_url, &request.path);
    println!(
        "relay client request {}: {} {} -> {}",
        request.request_id, method, request.path, url
    );

    let body_stream = futures_util::stream::unfold(body_receiver, |mut receiver| async {
        receiver.recv().await.map(|item| (item, receiver))
    });

    let mut builder = http_client
        .request(method, url)
        .body(reqwest::Body::wrap_stream(body_stream));

    for header in request.headers {
        let Ok(name) = HeaderName::from_bytes(header.name.as_bytes()) else {
            continue;
        };
        if !should_forward_header(&name) {
            continue;
        }
        let Ok(value) = HeaderValue::from_str(&header.value) else {
            continue;
        };
        builder = builder.header(name, value);
    }

    let mut local_response = builder.send().await?;
    let status = local_response.status().as_u16();
    println!(
        "relay client response {}: local server returned {}",
        request.request_id, status
    );

    let headers = relay_headers(local_response.headers());
    send_device_message(
        outgoing_sender,
        DeviceToRelay::ResponseStart(RelayResponseStart {
            request_id: request.request_id.clone(),
            status,
            headers,
        }),
    )
    .await?;

    while let Some(chunk) = local_response.chunk().await? {
        for body_chunk in chunk.chunks(STREAM_CHUNK_BYTES) {
            send_device_message(
                outgoing_sender,
                DeviceToRelay::ResponseBodyChunk(BodyChunk {
                    request_id: request.request_id.clone(),
                    body_base64: encode_body(body_chunk),
                }),
            )
            .await?;
        }
    }

    send_device_message(
        outgoing_sender,
        DeviceToRelay::ResponseEnd(ResponseEnd {
            request_id: request.request_id,
        }),
    )
    .await?;

    Ok(())
}

fn local_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

fn relay_headers(headers: &reqwest::header::HeaderMap) -> Vec<HeaderPair> {
    headers
        .iter()
        .filter(|(name, _)| should_forward_header(name))
        .filter_map(|(name, value)| {
            Some(HeaderPair {
                name: name.as_str().to_owned(),
                value: value.to_str().ok()?.to_owned(),
            })
        })
        .collect()
}

fn should_forward_header(name: &HeaderName) -> bool {
    !matches!(
        name,
        &HOST | &CONNECTION | &UPGRADE | &TRANSFER_ENCODING | &CONTENT_LENGTH
    )
}

fn send_device_message(
    sender: &OutgoingSender,
    message: DeviceToRelay,
) -> impl std::future::Future<Output = anyhow::Result<()>> + '_ {
    async move {
        let serialized = serde_json::to_string(&message)?;
        sender
            .send(Message::Text(serialized))
            .await
            .map_err(|_| anyhow::anyhow!("Relay tunnel is closed"))
    }
}

fn encode_body(body: &[u8]) -> String {
    general_purpose::STANDARD.encode(body)
}

fn decode_body(body_base64: &str) -> Result<Vec<u8>, base64::DecodeError> {
    general_purpose::STANDARD.decode(body_base64)
}
