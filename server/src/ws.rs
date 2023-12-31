use crate::client::{Client, CLIENTS};
use crate::jmri::handle_message;
use crate::{TIME, TO_JMRI};

use futures::{SinkExt, StreamExt};
use jmri_throttle_rs::message::{WiMessage, WiMessageType};
use log::Level::Debug;
use log::{debug, error, log_enabled};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};

pub async fn handle_connection(ws: WebSocket) {
    let id = Uuid::new_v4();
    debug!("New id: {id}");

    // WebSocket streams
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Client channels
    let (to_client_tx, to_client_rx) = mpsc::unbounded_channel::<String>();
    let mut to_client_rx = UnboundedReceiverStream::new(to_client_rx);

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
        let time_message =
            serde_json::to_string(&WiMessage::new(0, WiMessageType::Time(*TIME.read().await)))
                .unwrap();
        ws_tx.send(Message::text(time_message)).await.unwrap();

        while let Some(message) = to_client_rx.next().await {
            if let Err(e) = ws_tx.send(Message::text(message)).await {
                error!("Error sending to client '{id}': {e}");
            };
        }
    });

    client_receive_handle.await.ok();
    drop(client_send_handle);

    if let Some(client) = CLIENTS.write().await.remove(&id) {
        let mut messages: Vec<String> = Vec::new();
        for address in client.addresses {
            messages.push(WiMessage::new(address, WiMessageType::RemoveAddress).to_string())
        }
        TO_JMRI.tx.write().await.send(messages.join("\n")).unwrap();
    }
    debug!("Removed client '{id}'");
}
