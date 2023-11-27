mod handle_message;

pub use handle_message::handle_message;

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub type Address = i16;
pub type Velocity = i16;
pub type Function = u8;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum Direction {
    Reverse = 0,
    Forward = 1,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("R{}", *self as u8))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WiMessageType {
    AddAddress,
    RemoveAddress,
    Velocity(Velocity),
    FunctionPressed(Function),
    FunctionReleased(Function),
    Direction(Direction),
}

impl WiMessageType {
    pub fn is_address(&self) -> bool {
        matches!(self, Self::AddAddress | Self::RemoveAddress)
    }
}

impl Display for WiMessageType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WiMessageType::Velocity(throttle) => format!("V{throttle}"),
            WiMessageType::FunctionPressed(func) => format!("F1{func}"),
            WiMessageType::FunctionReleased(func) => format!("F0{func}"),
            WiMessageType::Direction(dir) => dir.to_string(),
            WiMessageType::AddAddress => '+'.into(),
            WiMessageType::RemoveAddress => '-'.into(),
        };

        f.write_str(&s)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WiMessage {
    message_type: WiMessageType,
    address: Address,
}

impl Display for WiMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = if self.message_type.is_address() {
            format!("MT{}L{}<;>", self.message_type, self.address)
        } else {
            format!("MTAL{}<;>{}", self.address, self.message_type)
        };

        f.write_str(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_display() {
        assert_eq!(format!("{}", Direction::Reverse), "R0");
        assert_eq!(format!("{}", Direction::Forward), "R1");
    }

    #[test]
    fn wi_message_type_display() {
        assert_eq!(format!("{}", WiMessageType::AddAddress), "+");
        assert_eq!(format!("{}", WiMessageType::RemoveAddress), "-");
        assert_eq!(format!("{}", WiMessageType::Velocity(5)), "V5");
        assert_eq!(format!("{}", WiMessageType::FunctionPressed(5)), "F15");
        assert_eq!(format!("{}", WiMessageType::FunctionReleased(5)), "F05");
        assert_eq!(
            format!("{}", WiMessageType::Direction(Direction::Reverse)),
            "R0"
        );
    }

    #[test]
    fn wi_message_display() {
        let wi_message = WiMessage {
            message_type: WiMessageType::AddAddress,
            address: 5,
        };
        assert_eq!(format!("{}", wi_message), "MT+L5<;>");
        let wi_message = WiMessage {
            message_type: WiMessageType::FunctionReleased(10),
            address: 6,
        };
        assert_eq!(format!("{}", wi_message), "MTAL6<;>F010");
    }

    #[test]
    fn wi_message_type_is_address() {
        assert!(WiMessageType::AddAddress.is_address());
        assert!(WiMessageType::RemoveAddress.is_address());
        assert!(!WiMessageType::Velocity(5).is_address());
    }
}
