//!
//! Description:
//!    Simple serial driver that talks to the Muses instrument arduino to handle
//!    button presses and encoder rotations.
//! 
//!    The protocol is two types of OSC messages of the form:
//! 
//!       /b/n i
//! 
//!    Button messages, n is button index, and i is an integer value
//! 
//!       /e/n i
//! 
//!    Encoder messages, m is encoder index, and i is an integer value
//! 
//!    All messages are terminated with newline, i.e. '\n'.
//! 
//!    The only valid whitespace between OSC address and argument is space, i.e. ' '.
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 
extern crate serialport;

use std::sync::mpsc::{Sender};
use std::io::{self, Write};
use std::time::Duration;

use serialport::prelude::*;

use rosc::{OscPacket, OscMessage, OscType};
use rosc::encoder;

use std::str::FromStr;

use std::sync::atomic::{AtomicBool, Ordering};

// Shared AtomicBool, when true listening threads should shutdown
use crate::DISCONNECT;

pub struct Serial {
    osc_sender: Sender<OscPacket>,
    port: Box<dyn SerialPort>,
}

unsafe impl Send for Serial {

}

impl Serial {
    const BUFFER_SIZE: usize = 1000;
    
    pub fn new(osc: Sender<OscPacket>, port: Box<dyn SerialPort>) -> Self {
        Serial {
            osc_sender: osc,
            port: port,
        }
    }

    pub fn run(mut serial: Serial) {
        let mut serial_buf: Vec<u8> = vec![0; Serial::BUFFER_SIZE];

        // as noted above messages are a very simple fixed format:
        //
        //      "address int_argument\n"
        //
        let mut message: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
        let mut index: usize = 0;

        // set timeout to 2 secs so we don't miss requests to disconnect
        serial.port.set_timeout(Duration::from_secs(2));

        // process osc messages over serial, until disconnected request
        while !DISCONNECT.load(Ordering::SeqCst) {
            match serial.port.read(serial_buf.as_mut_slice()) {
                Ok(t) => {
                    for x in &serial_buf[..t] {

                        //print!("{}", *x as char);
                        // end of message, so transmit
                        if *x as char == '\n' {
                            // convert argument to int, if not valid we return 0 to avoid exception
                            let arg = String::from_utf8_lossy(&message[1][..]).parse::<i32>().unwrap_or(0); 
                            let address = String::from_utf8_lossy(&message[0][..]).to_string();

                            // build and transmit packet
                            let mut packet = OscPacket::Message(OscMessage {
                                addr: address,
                                args: Some(vec![OscType::Int(arg)]),
                            });
                            serial.osc_sender.send(packet).unwrap();

                            // setup for next message
                            index = 0;
                            message[0].clear();
                            message[1].clear();
                        }
                        // move to argument processing
                        else if *x as char == ' ' {
                            index = 1;
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
        serial.osc_sender.send(fake_packet).unwrap();
    }
}