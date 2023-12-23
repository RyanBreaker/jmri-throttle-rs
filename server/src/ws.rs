use crate::client::{Client, CLIENTS};
use crate::jmri::handle_message;
use crate::FROM_JMRI;
use futures::{SinkExt, StreamExt};
use jmri_throttle_rs::message::WiMessage;
use log::Level::Debug;
use log::{debug, error, info, log_enabled};
use std::str::FromStr;
use tokio::sync::mpsc;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

pub async fn handle_connection(ws: WebSocket) {
    let id = Uuid::new_v4();
    debug!("New id: {id}");

    // WebSocket streams
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Client channels
    // TODO: Allow more channels for multiple clients, or just create a JMRI connections per client
    let (to_client_tx, _to_client_rx) = mpsc::unbounded_channel::<Message>();
    // let mut to_client_rx = UnboundedReceiverStream::new(to_client_rx);

    CLIENTS
        .write()
        .await
        .insert(id, Client::new(id, to_client_tx));

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
            // If we were sent a close, return to start cleanup at the end of handle_connection
            if message.is_close() {
                return;
            }
            handle_message(id, message).await;
        }
    });

    let client_send_handle = tokio::spawn(async move {
        while let Some(message) = FROM_JMRI.rx.write().await.next().await {
            if let Ok(message) = WiMessage::from_str(&message) {
                let message = serde_json::to_string(&message).unwrap();
                ws_tx.send(Message::text(message)).await.unwrap();
            } else {
                info!("Couldn't parse message: {message}");
            }
        }
    });

    client_receive_handle.await.unwrap();
    drop(client_send_handle);

    CLIENTS.write().await.remove(&id);
    debug!("Removed client '{id}'");
}
