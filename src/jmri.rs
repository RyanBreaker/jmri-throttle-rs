use crate::TO_JMRI;
use futures::future::join3;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Notify;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::codec::{Framed, LinesCodec};
use uuid::Uuid;

pub async fn jmri_conn(
    notify: Arc<Notify>,
    tx: UnboundedSender<String>,
    mut rx: UnboundedReceiverStream<String>,
) -> Result<(), Box<dyn Error>> {
    let my_id = Uuid::new_v4();
    debug!("Server's ID: {my_id}");

    let jmri_server = &env::var("JMRI_SERVER").unwrap_or("localhost:12090".to_string());
    let throttle_name = &env::var("JMRI_THROTTLE_NAME").unwrap_or("TestThrottleRs".to_string());

    let jmri_conn = TcpStream::connect(jmri_server).await.unwrap_or_else(|e| {
        error!("Error connecting to JMRI at '{jmri_server}': {e}");
        std::process::exit(1);
    });
    let (mut jmri_tx, mut jmri_rx) = Framed::new(jmri_conn, LinesCodec::new()).split::<String>();

    // Notify we're connected and main init can continue
    info!("Successfully connected to JMRI at: {jmri_server}");
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

    // Initial setup message to JMRI
    jmri_tx
        .send(format!("HU{my_id}\nN{throttle_name}\n"))
        .await
        .unwrap();

    let write_handle2 = tokio::spawn(async move {
        let mut rx = TO_JMRI.rx.write().await;
        while let Some(line) = rx.next().await {
            debug!("Message to send to JMRI: {line}");
        }
    });

    // TODO: figure out this one
    let write_handle = tokio::spawn(async move {
        while let Some(line) = rx.next().await {
            if let Err(e) = jmri_tx.send(line).await {
                error!("Error sending message to JMRI: {e}");
            }
        }
    });

    let _ = join3(read_handle, write_handle, write_handle2).await;

    Ok(())
}
