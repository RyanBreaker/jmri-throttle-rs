use crate::{FROM_JMRI, TO_JMRI};
use futures::future::join3;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio::time::sleep;
use tokio_util::codec::{Framed, LinesCodec};
use uuid::Uuid;

const NEWLINE: char = '\n';

pub async fn jmri_conn(notify: Arc<Notify>) -> Result<(), Box<dyn Error>> {
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

    let heartbeat_handle = tokio::spawn(async move {
        if let Err(e) = TO_JMRI.tx.read().await.send("*".into()) {
            error!("Error sending heartbeat to JMRI: {e}");
        }
        sleep(Duration::from_secs(3)).await;
    });

    let read_handle = tokio::spawn(async move {
        while let Some(line) = jmri_rx.next().await {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    error!("Error reading from JMRI: {e}");
                    break;
                }
            };
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            debug!("Message from JMRI (len={}): {line}", line.len());
            if let Err(e) = FROM_JMRI.tx.read().await.send(line.into()) {
                error!("Error sending message from JMRI: {e}");
            }
        }
    });

    // Initial setup message to JMRI
    jmri_tx
        .send(format!("HU{my_id}{NEWLINE}N{throttle_name}"))
        .await
        .unwrap();

    let write_handle = tokio::spawn(async move {
        while let Some(line) = TO_JMRI.rx.write().await.next().await {
            if line.is_empty() {
                continue;
            }
            debug!("Sending message to JMRI: {line}");
            jmri_tx.send(line).await.unwrap();
        }
    });

    let _ = join3(read_handle, write_handle, heartbeat_handle).await;

    Ok(())
}
