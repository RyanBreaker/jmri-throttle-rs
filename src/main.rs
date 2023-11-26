mod message;

use crate::message::WiMessage;
use futures::future::join;
use futures::{SinkExt, StreamExt};
use log::Level::Debug;
use log::{debug, error, info, log_enabled};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, Notify, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::codec::{Framed, LinesCodec};
use uuid::Uuid;
use warp::http::StatusCode;
use warp::ws::{Message, WebSocket};
use warp::Filter;

type Clients = Arc<RwLock<HashMap<Uuid, UnboundedSender<Message>>>>;

static CLIENTS: Lazy<Clients> = Lazy::new(Clients::default);

const SERVER: &str = "localhost:12090";
const THROTTLE_NAME: &str = "TestThrottleRs";

async fn jmri_conn(
    notify: Arc<Notify>,
    tx: UnboundedSender<String>,
    mut rx: UnboundedReceiverStream<String>,
) -> Result<(), Box<dyn Error>> {
    let my_id = Uuid::new_v4();
    debug!("Server's ID: {my_id}");

    let jmri_conn = TcpStream::connect(SERVER).await.unwrap_or_else(|e| {
        error!("Error connecting to JMRI at '{SERVER}': {e}");
        std::process::exit(1);
    });
    let (mut jmri_tx, mut jmri_rx) = Framed::new(jmri_conn, LinesCodec::new()).split::<String>();

    // Notify we're connected and main init can continue
    info!("Successfully connected to JMRI at: {SERVER}");
    notify.notify_one();

    let read_handle = tokio::spawn(async move {
        while let Some(line) = jmri_rx.next().await {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    error!("Error reading from JMRI: {e}");
                    break;
                }
            };
            if let Err(e) = tx.send(line) {
                error!("Error sending message from JMRI: {e}");
            }
        }
    });

    // Setup message to JMRI
    jmri_tx
        .send(format!("HU{my_id}\nN{THROTTLE_NAME}\n"))
        .await
        .unwrap();

    let write_handle = tokio::spawn(async move {
        while let Some(line) = rx.next().await {
            if let Err(e) = jmri_tx.send(line).await {
                error!("Error sending message to JMRI: {e}");
            }
        }
    });

    let _ = join(read_handle, write_handle).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let (jmri_tx, jmri_rx) = mpsc::unbounded_channel::<String>();
    let jmri_rx = UnboundedReceiverStream::new(jmri_rx);

    let jmri_notify = Arc::new(Notify::new());
    let jmri_up2 = jmri_notify.clone();
    let jmri_handle = tokio::spawn(async move {
        if let Err(e) = jmri_conn(jmri_up2, jmri_tx, jmri_rx).await {
            error!("Error on jmri_conn: {e}");
        }
    });

    // Lets us know we're connected to JMRI and can continue
    jmri_notify.notified().await;

    let health = warp::path!("health")
        .and(warp::get())
        .map(|| warp::reply::with_status("Healthy", StatusCode::OK));

    let ws =
        warp::path("ws")
            .and(warp::ws())
            .map(|ws: warp::ws::Ws| {
                ws.on_upgrade(connected)
            });

    let routes = health.or(ws);

    let warp_handle = warp::serve(routes).run(([0, 0, 0, 0], 6000));

    let _ = join(jmri_handle, warp_handle).await;

    Ok(())
}

async fn connected(ws: WebSocket) {
    let id = Uuid::new_v4();
    debug!("New id: {id}");

    // WebSocket streams
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Client channels
    let (to_client_tx, to_client_rx) = mpsc::unbounded_channel::<Message>();
    let mut to_client_rx = UnboundedReceiverStream::new(to_client_rx);

    to_client_tx.send(Message::text("Test123")).unwrap();
    CLIENTS.write().await.insert(id, to_client_tx);
    if log_enabled!(Debug) {
        let clients = CLIENTS.read().await;
        debug!("Number of clients: {}", clients.len());
        debug!("Current clients: {:?}", clients.keys());
    }

    let client_receive_handle = tokio::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            let message = match result {
                Ok(message) => message,
                Err(e) => {
                    error!("Websocket error(uid={id}, e={e})");
                    break;
                }
            };
            // If we were sent a close, return to start cleanup at the end of this fn
            if message.is_close() {
                return;
            }
            handle_message(id, message);
        }
    });

    let client_send_handle = tokio::spawn(async move {
        while let Some(message) = to_client_rx.next().await {
            if let Err(e) = ws_tx.send(message).await {
                error!("Error sending to client '{id}': {e}");
            };
        }
    });

    client_receive_handle.await.unwrap();
    drop(client_send_handle);

    CLIENTS.write().await.remove(&id);
    debug!("Removed client '{id}'");
}

fn handle_message(id: Uuid, message: Message) {
    if !message.is_text() {
        debug!("Text not received to '{id}': {message:?}");
        return;
    }
    let message = message.to_str().unwrap();
    let message = match serde_json::from_str::<WiMessage>(message) {
        Ok(message) => message,
        Err(e) => {
            error!("Deserialize error(uid={id}, e={e})");
            return;
        }
    };
    debug!("Received message(uid={id}, message={message:?})")
}
