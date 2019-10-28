//! Description: 
//! 
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 
use rosc::{OscPacket, OscMessage, OscType};

/// extract a string from 1st OSC argument, fallback ""
pub fn osc_string(msg: &OscMessage) -> String {
    if let Some(vec) = &msg.args {
        if let OscType::String(value) = &vec[0] {
            return value.clone();
        }
    }
    "".to_string()
}

/// extract a float from 1st OSC argument, fallback 0.0
pub fn osc_float(msg: &OscMessage) -> f32 {
    if let Some(vec) = &msg.args {
        if let OscType::Float(value) = &vec[0] {
            return *value;
        }
    }
    0.0
}