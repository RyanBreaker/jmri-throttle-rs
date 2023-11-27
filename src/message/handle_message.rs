use crate::message::WiMessage;
use crate::TO_JMRI;
use log::{debug, error};
use uuid::Uuid;
use warp::ws::Message;

pub async fn handle_message(id: Uuid, message: Message) {
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
    debug!("Received message(uid={id}, message={message:?})");

    // let client = match CLIENTS.read().await.get(&id) {
    //     None => {
    //         error!("Unable to find client '{id}` from CLIENTS");
    //         return;
    //     }
    //     Some(client) => client,
    // };

    let jmri_chan = TO_JMRI.tx.read().await;
    jmri_chan.send(message.to_string()).unwrap();
}
