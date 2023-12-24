use jmri_throttle_rs::message::Address;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type Clients = Arc<RwLock<HashMap<Uuid, Client>>>;

pub static CLIENTS: Lazy<Clients> = Lazy::new(Clients::default);

#[derive(Debug)]
pub struct Client {
    pub id: Uuid,
    pub addresses: HashSet<Address>,
    pub sender: UnboundedSender<String>,
}

impl Client {
    pub fn new(id: Uuid, sender: UnboundedSender<String>) -> Self {
        Self {
            id,
            sender,
            addresses: HashSet::new(),
        }
    }
}
