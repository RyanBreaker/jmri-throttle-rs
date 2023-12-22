mod handle_message;

pub use handle_message::handle_message;

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

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

impl FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dir = match s {
            "0" => Direction::Reverse,
            _ => Direction::Forward,
        };
        Ok(dir)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WiMessageType {
    AddAddress,
    RemoveAddress,
    Velocity(Velocity),
    FunctionPressed(Function),
    FunctionReleased(Function), // TODO: Maybe remove FunctionReleased as FunctionPressed always toggles in JMRI
    Direction(Direction),
    Time(i64),
}

impl WiMessageType {
    pub fn is_address(&self) -> bool {
        matches!(
            self,
            WiMessageType::AddAddress | WiMessageType::RemoveAddress
        )
    }
}

impl FromStr for WiMessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let first = chars.next().unwrap();
        let nums: String = chars.filter(|c| c.is_numeric()).collect();

        match first {
            'V' => Ok(WiMessageType::Velocity(nums.parse().unwrap())),
            'F' => {
                let mut nums = nums.chars();
                let is_pressed = nums.next().unwrap_or('0') == '1';
                let nums = nums.collect::<String>().parse().unwrap();
                if is_pressed {
                    Ok(WiMessageType::FunctionPressed(nums))
                } else {
                    Ok(WiMessageType::FunctionReleased(nums))
                }
            }
            'R' => {
                let dir = Direction::from_str(&nums).unwrap();
                Ok(WiMessageType::Direction(dir))
            }
            _ => Err("No action found".into()),
        }
    }
}

impl Display for WiMessageType {
    // Formatting for outbound messages to JMRI
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use WiMessageType::*;
        let s = match self {
            Velocity(throttle) => format!("V{throttle}"),
            FunctionPressed(func) => format!("F1{func}"),
            FunctionReleased(func) => format!("F0{func}"),
            Direction(dir) => dir.to_string(),
            AddAddress => '+'.into(),
            RemoveAddress => '-'.into(),
            _ => String::new(),
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
            format!(
                "MT{}L{}<;>L{}",
                self.message_type, self.address, self.address
            )
        } else {
            format!("MTAL{}<;>{}", self.address, self.message_type)
        };

        f.write_str(&s)
    }
}

impl FromStr for WiMessage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("<;>");
        let address = split.next().unwrap();

        let address_action = if address.contains('-') {
            Some(WiMessageType::RemoveAddress)
        } else if address.contains('+') {
            Some(WiMessageType::AddAddress)
        } else {
            None
        };

        let address: String = address.chars().filter(|c| c.is_numeric()).collect();
        let address: Address = address
            .parse()
            .map_err(|e| format!("Couldn't translate address: {e}"))?;

        if let Some(message_type) = address_action {
            return Ok(WiMessage {
                message_type,
                address,
            });
        };

        let action = split.next();
        if action.is_none() {
            return Err(format!("Empty action: {s}"));
        }

        let message_type = WiMessageType::from_str(action.unwrap())?;
        Ok(WiMessage {
            message_type,
            address,
        })
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
        assert_eq!(format!("{}", wi_message), "MT+L5<;>L5");
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
