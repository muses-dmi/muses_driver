//!
//! Description:
//!    Simple serial driver that talks to the Muses instrument arduino to handle
//!    button presses and encoder rotations.
//! 
//!    The protocol is two types of OSC messages and a special Sensel message.
//!    The two OSC messages are of the form:
//! 
//!       /b/n i
//! 
//!    Button messages, n is button index, and i is an integer value
//! 
//!       /e/n i
//! 
//!    Encoder messages, m is encoder index, and i is an integer value
//! 
//!    The Sensel message is of the form
//! 
//!       /s id type x y pressure
//! 
//!    Type is 0 invalid contact (should not happen)
//!            1 Contact start
//!            2 contact move
//!            3 contact end
//!    
//!    All messages are terminated with newline, i.e. '\n'.
//! 
//!    The only valid whitespace between OSC address and argument is space, i.e. ' '.
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 
extern crate serialport;


use std::net::{SocketAddrV4};

use std::sync::mpsc::{Sender};
use std::io::{self, Write};
use std::time::Duration;

use serialport::prelude::*;

use rosc::{OscPacket, OscMessage, OscType};
use rosc::encoder;

use std::str::FromStr;

use std::sync::atomic::{AtomicBool, Ordering};

use muses_sensel::*;

// Shared AtomicBool, when true listening threads should shutdown
use crate::DISCONNECT;

pub struct Serial {
    inferface: interface_direct::InterfaceDirect,
    osc_sender: Sender<(OscPacket, Option<SocketAddrV4>)>,
    port: Box<dyn SerialPort>,
}

unsafe impl Send for Serial {

}

impl Serial {
    const BUFFER_SIZE: usize = 1000;
    
    pub fn new(
            inferface: interface_direct::InterfaceDirect, 
            osc: Sender<(OscPacket, Option<SocketAddrV4>)>, 
            port: Box<dyn SerialPort>) -> Self {
        Serial {
            inferface: inferface,
            osc_sender: osc,
            port: port,
        }
    }

    fn toState(state: u8) -> bindings::SenselContactState {
        match state {
            1 => bindings::SenselContactState::CONTACT_START,
            2 => bindings::SenselContactState::CONTACT_MOVE,
            3 => bindings::SenselContactState::CONTACT_END,
            _ => bindings::SenselContactState::CONTACT_INVALID,
        }
    }

    pub fn run(mut serial: Serial) {
        let mut serial_buf: Vec<u8> = vec![0; Serial::BUFFER_SIZE];

        // as noted above messages are a very simple fixed format:
        //
        //      "address int_argument\n"
        //
        // Sensel message has 6 arguments, including message header
        let mut message: [Vec<u8>; 6] = [ Vec::new(), Vec::new(), Vec::new() , Vec::new() , Vec::new(), Vec::new() ];
        let mut index: usize = 0;

        // set timeout to 2 secs so we don't miss requests to disconnect
        serial.port.set_timeout(Duration::from_secs(2));

        // process osc messages over serial, until disconnected request
        while !DISCONNECT.load(Ordering::SeqCst) {
            match serial.port.read(serial_buf.as_mut_slice()) {
                Ok(t) => {
                    for x in &serial_buf[..t] {

                        print!("{}", *x as char);
            
                        // end of message, so transmit
                        if *x as char == '\n' {
                            let address = String::from_utf8_lossy(&message[0][..]).to_string();
                            // sensel message
                            if index == 5 {
                                let contact = sensel::contact::Contact {
                                    id: String::from_utf8_lossy(&message[1][..]).parse::<i32>().unwrap_or(0) as u8,
                                    state: Serial::toState(String::from_utf8_lossy(&message[2][..]).parse::<i32>().unwrap_or(0) as u8),
                                    x: String::from_utf8_lossy(&message[3][..]).parse::<i32>().unwrap_or(0) as f32,
                                    y: String::from_utf8_lossy(&message[4][..]).parse::<i32>().unwrap_or(0) as f32,
                                    total_force: String::from_utf8_lossy(&message[5][..]).parse::<i32>().unwrap_or(0) as f32,
                                    area: 0.0,
                                    ellipse: None,
                                    delta: None,
                                    bounding_box: None,
                                    peak: None,
                                };
                                //info!("contact {:?}", contact);
                                //info!("f = {}", String::from_utf8_lossy(&message[5][..]));
                                // handle contact with interface
                                serial.inferface.handleContact(&contact, &serial.osc_sender);
                            }
                            else { // button and encoder messages
                                // convert argument to int, if not valid we return 0 to avoid exception
                                let arg = String::from_utf8_lossy(&message[1][..]).parse::<i32>().unwrap_or(0); 

                                // build and transmit packet
                                let mut packet = OscPacket::Message(OscMessage {
                                    addr: address,
                                    args: Some(vec![OscType::Int(arg)]),
                                });
                                //info!("{:?}", packet);
                                serial.osc_sender.send((packet, None)).unwrap();
                                
                            }

                            // setup for next message
                            for i in 0..index+1 {
                                message[i].clear();
                            }
                            index = 0;
                        }
                        // move to argument processing
                        else if *x as char == ' ' {
                            index = index + 1;
                        }
                        // store char
                        else {
                            message[index].push(*x);
                        }
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => error!("{:?}", e),
            }
        }

        // due to bug with revc_timeout panicing on OSC thread, we pass a fake packet to enable terminating that thread
        let mut fake_packet = OscPacket::Message(OscMessage {
            addr: "/fakepacket".to_string(),
            args: None,
        });
        serial.osc_sender.send((fake_packet, None)).unwrap();
    }
}