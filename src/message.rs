use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    Reverse = 0,
    Forward = 1,
}

impl Direction {
    pub fn as_num(&self) -> usize {
        match self {
            Direction::Reverse => 0,
            Direction::Forward => 1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WiMessageType {
    AddEngine(String),
    RemoveEngine(String),
    Throttle(usize),
    Function(usize),
    Direction(Direction),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WiMessage {
    message_type: WiMessageType,
}
