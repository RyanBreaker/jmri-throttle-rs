use crate::Direction::Forward;
use futures::StreamExt;
use log::Level::Debug;
use log::{debug, error, log_enabled};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::ws::{Message, WebSocket};
use warp::Filter;

pub struct Client {
    pub id: Uuid,
}

type Clients = Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<Message>>>>;

const SERVER: &str = "localhost:12090";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let my_id = Uuid::new_v4();
    debug!("Server's ID: {my_id}");

    let (jmri_tx, jmri_rx) = mpsc::unbounded_channel::<String>();
    let mut jmri_rx = UnboundedReceiverStream::new(jmri_rx);

    let mut jmri_conn = TcpStream::connect(SERVER).await.unwrap_or_else(|e| {
        error!("Error connecting to JMRI at '{SERVER}': {e}");
        panic!();
    });

    // TODO: set up async task for sending and receiving messages from JMRI
    jmri_conn.write_all(b"HU{}").await?;

    let mut buffer = [0; 1024];
    let n = jmri_conn.read(&mut buffer).await?;
    if n > 0 {
        debug!("Received: {}", String::from_utf8_lossy(&buffer));
    }

    let health = warp::path!("health")
        .and(warp::get())
        .map(|| warp::reply::with_status("Healthy", StatusCode::OK));

    let clients = Clients::default();
    let clients = warp::any().map(move || clients.clone());

    let ws =
        warp::path("ws")
            .and(warp::ws())
            .and(clients)
            .map(|ws: warp::ws::Ws, clients: Clients| {
                ws.on_upgrade(move |socket| connected(socket, clients))
            });

    let routes = health.or(ws);

    warp::serve(health.or(routes))
        .run(([0, 0, 0, 0], 5000))
        .await;

    Ok(())
}

async fn connected(ws: WebSocket, clients: Clients) {
    let id = Uuid::new_v4();
    debug!("New id: {id}");

    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel::<Message>();
    let mut rx = UnboundedReceiverStream::new(rx);

    clients.write().await.insert(id, tx);
    if log_enabled!(Debug) {
        let clients = clients.read().await;
        debug!("Number of clients: {}", clients.len());
        debug!("Current clients: {:?}", clients.keys());
    }

    while let Some(result) = ws_rx.next().await {
        let message = match result {
            Ok(message) => message,
            Err(e) => {
                error!("Websocket error(uid={id}, e={e})");
                break;
            }
        };
        handle_message(message, id).await;
    }

    clients.write().await.remove(&id);
}

async fn handle_message(message: Message, id: Uuid) {
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

#[derive(Serialize, Deserialize, Debug)]
enum Direction {
    Reverse = 0,
    Forward = 1,
}

impl Direction {
    pub fn as_num(&self) -> usize {
        match self {
            Direction::Reverse => 0,
            Forward => 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum WiMessageType {
    AddEngine(String),
    RemoveEngine(String),
    Throttle(usize),
    Function(usize),
    Direction(Direction),
}

#[derive(Serialize, Deserialize, Debug)]
struct WiMessage {
    message_type: WiMessageType,
}
