#[forbid(unsafe_code)]
mod client;
mod jmri;
mod ws;

use crate::jmri::jmri_conn;
use crate::ws::handle_connection;
use futures::future::join;
use log::error;
use once_cell::sync::Lazy;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{mpsc, Notify, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::http::StatusCode;
use warp::Filter;

struct JmriChannel {
    pub tx: RwLock<UnboundedSender<String>>,
    pub rx: RwLock<UnboundedReceiverStream<String>>,
}

static TO_JMRI: Lazy<JmriChannel> = make_chan();
static FROM_JMRI: Lazy<JmriChannel> = make_chan();

const fn make_chan() -> Lazy<JmriChannel> {
    Lazy::new(|| {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let tx = RwLock::new(tx);
        let rx = RwLock::new(UnboundedReceiverStream::new(rx));
        JmriChannel { tx, rx }
    })
}

static TIME: Lazy<RwLock<i64>> = Lazy::new(|| RwLock::new(0));

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let jmri_notify = Arc::new(Notify::new());
    let jmri_handle = {
        let jmri_notify = jmri_notify.clone();
        tokio::spawn(async move {
            if let Err(e) = jmri_conn(jmri_notify).await {
                error!("Error on jmri_conn: {e}");
            }
        })
    };

    // Lets us know we're connected to JMRI and can continue
    jmri_notify.notified().await;

    let health = warp::path!("health")
        .and(warp::get())
        .map(|| warp::reply::with_status("Healthy", StatusCode::OK));

    let ws = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(handle_connection));

    let routes = health.or(ws);

    let warp_handle = warp::serve(routes).run(([0, 0, 0, 0], 4000));

    let _ = join(jmri_handle, warp_handle).await;

    Ok(())
}
