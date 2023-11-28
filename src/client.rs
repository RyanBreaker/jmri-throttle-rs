use crate::message::Address;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use uuid::Uuid;
use warp::ws::Message;

pub type Clients = Arc<RwLock<HashMap<Uuid, Client>>>;

pub static CLIENTS: Lazy<Clients> = Lazy::new(Clients::default);

#[derive(Debug)]
pub struct Client {
    pub id: Uuid,
    pub addresses: Vec<Address>,
    pub sender: UnboundedSender<Message>,
}

impl Client {
    pub fn new(id: Uuid, sender: UnboundedSender<Message>) -> Self {
        Self {
            id,
            sender,
            addresses: Vec::new(),
        }
    }
}
