//!
//! Description:
//!    Simple ORAC serial driver that talks to the Muses instrument to handle
//!    button presses, encoder rotations, and Sensel touches.
//! 
//!    This is designed specfically for comunicating with MEC/ORAC and stores 
//!    some additional information about controllers and so on.
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

use std::net::{UdpSocket, SocketAddrV4};

extern crate serialport;
extern crate num;

use std::sync::mpsc::{Sender, Receiver};
use std::io::{self, Write};
use std::time::Duration;

use serialport::prelude::*;

use rosc::{OscPacket, OscMessage, OscType};
use rosc::encoder;

use std::str::FromStr;

use std::sync::atomic::{AtomicBool, Ordering};

use muses_sensel::*;
use super::osc_utils::*;
use super::transport::*;

// Shared AtomicBool, when true listening threads should shutdown
use crate::DISCONNECT;

// ORAC/MEC currently supports a maximum of 8 controllers
const NUM_CONTROLLERS: usize = 8;

/// Prefix for encoder messages from MCU
const ENCODER_PREFIX: &'static str = "/e";
const ENCODER_PREFIX_LENGTH: usize = 2;
const ENCODER_NUM_OFFSET_START: usize = 3;
const ENCODER_NUM_OFFSET_END: usize = 4;

/// Prefix of /key messages
const KEY_PREFIX: &'static str = "/key";
const KEY_PREFIX_LENGTH: usize = 4;

/// Prefix for PD /knobs message
const KNOBS_PREFIX: &'static str = "/knobs";
const KNOBS_PREFIX_LENGTH: usize = 6;

/// Prefix for /s Sensel messages
const SENSEL_PREFIX: &'static str = "/s";
const SENSEL_PREFIX_LENGTH: usize = 2;

const SENSEL_MSG_LENGTH: usize = 5;

/// ORAC Controllers, represented as controls on the Muses xyx
struct Controller {
    /// Integer than is maintained as representation of hardware and mapped to ORAC representation, which is (0,1]
    value: i32,
    /// OSC Address messsage for outgoing messages to ORAC
    osc_address: String,
}

impl Controller {
    const MAX_CONTROLLER_VALUE: i32 = 127;
    const MIN_CONTROLLER_VALUE: i32 = 0;

    pub fn new(osc_address: String) -> Self {
        Controller {
            value: 0,
            osc_address: osc_address,
        }
    }

    #[inline(always)]
    pub fn inc(&mut self, value: i32) {
        if value < 0 {
            self.value = self.value - 1;
        }
        else {
            self.value = (self.value + 1);
        }

        // clamp value
        self.value = num::clamp(self.value, Controller::MIN_CONTROLLER_VALUE, Controller::MAX_CONTROLLER_VALUE);
    }

    /// Send Controller message to ORAC/MEC
    pub fn send(&self, osc_sender: &Sender<(OscPacket, Option<SocketAddrV4>)>) {
        let tmp = (self.value - Controller::MIN_CONTROLLER_VALUE) as f32 / 
                    (Controller::MAX_CONTROLLER_VALUE -  Controller::MIN_CONTROLLER_VALUE) as f32;

        let mut packet = OscPacket::Message(OscMessage {
                addr: self.osc_address.clone(),
                //args: Some(vec![OscType::Int(self.value)]),
                args: Some(vec![OscType::Float(tmp)]),
            });
            info!("{:?}", packet);
            osc_sender.send((packet, None)).unwrap();
    }
}

pub struct Serial {
    inferface: interface_direct::InterfaceDirect,
    /// OSC messages to be sent out with optional address
    osc_sender: Sender<(OscPacket, Option<SocketAddrV4>)>,
    /// IPv4 address to send OSC messages direct to ORAC PD patch
    pd_send_addr: SocketAddrV4,
    /// IP Addr to send direct to ORAC
    port: Box<dyn SerialPort>,
    controllers: [Controller; NUM_CONTROLLERS],
}

unsafe impl Send for Serial {

}

impl Serial {
    /// Buffer for incoming RX messages from hardware
    const BUFFER_SIZE: usize = 1000;
    
    pub fn new(
            inferface: interface_direct::InterfaceDirect, 
            osc: Sender<(OscPacket, Option<SocketAddrV4>)>, 
            orac_direct_addr: SocketAddrV4,
            port: Box<dyn SerialPort>) -> Self {
        Serial {
            inferface: inferface,
            osc_sender: osc,
            pd_send_addr: orac_direct_addr,
            port: port,
            controllers: [
                Controller::new("/P1Ctrl".to_string()), Controller::new("/P2Ctrl".to_string()), 
                Controller::new("/P3Ctrl".to_string()), Controller::new("/P4Ctrl".to_string()),
                Controller::new("/P5Ctrl".to_string()), Controller::new("/P6Ctrl".to_string()), 
                Controller::new("/P8Ctrl".to_string()), Controller::new("/P8Ctrl".to_string())
            ],
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
        // /key message has 2 arguments
        // /b message has 1 argument
        // /e message has 1 argument
        let mut message: [Vec<u8>; 7] = [ Vec::new(), Vec::new(), 
                                          Vec::new() , Vec::new(), 
                                          Vec::new(), Vec::new(),
                                          Vec::new() ];
        let mut index: usize = 0;

        // set timeout to 2 secs so we don't miss requests to disconnect
        serial.port.set_timeout(Duration::from_secs(2));

        // process osc messages over serial, until disconnect request
        while !DISCONNECT.load(Ordering::SeqCst) {
            match serial.port.read(serial_buf.as_mut_slice()) {
                Ok(t) => {
                    for x in &serial_buf[..t] {

                        //print!("{}", *x as char);
            
                        // end of message, so transmit
                        if *x as char == '\n' {
                            let address = String::from_utf8_lossy(&message[0][..]).to_string();
                            
                            // sensel message
                            //if index == SENSEL_MSG_LENGTH {
                            if  &address[..SENSEL_PREFIX_LENGTH] == SENSEL_PREFIX {
                                let contact = sensel::contact::Contact {
                                    id: String::from_utf8_lossy(&message[1][..]).parse::<i32>().unwrap_or(0) as u8,
                                    state: Serial::toState(
                                        String::from_utf8_lossy(&message[2][..]).parse::<i32>().unwrap_or(0) as u8),
                                    x: String::from_utf8_lossy(&message[3][..]).parse::<i32>().unwrap_or(0) as f32,
                                    y: String::from_utf8_lossy(&message[4][..]).parse::<i32>().unwrap_or(0) as f32,
                                    total_force: String::from_utf8_lossy(
                                                    &message[5][..]).parse::<i32>().unwrap_or(0) as f32,
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

                                // is an encoder?
                                if  &address[..ENCODER_PREFIX_LENGTH] == ENCODER_PREFIX {
                                    // extract the encoder number
                                    let index = address[ENCODER_NUM_OFFSET_START..ENCODER_NUM_OFFSET_END]
                                                    .trim().parse::<i32>().unwrap() as usize;

                                    serial.controllers[index-1].inc(arg);
                                    serial.controllers[index-1].send(&serial.osc_sender);
                                }
                                // handle /key messages as these are sent directly to orac and not via mec
                                else if  &address[..KEY_PREFIX_LENGTH] == KEY_PREFIX {
                                    let arg2 = String::from_utf8_lossy(&message[2][..]).parse::<i32>().unwrap_or(0); 
                                    let mut packet = OscPacket::Message(OscMessage {
                                        addr: address,
                                        args: Some(vec![OscType::Int(arg), OscType::Int(arg2)]),
                                    });
                                    info!("{:?}", packet);
                                    serial.osc_sender.send((packet, Some(serial.pd_send_addr))).unwrap();
                                }
                                // handle /knobs messages as these are sent directly to orac and not via mec
                                else if  &address[..KNOBS_PREFIX_LENGTH] == KNOBS_PREFIX {
                                    let arg2 = String::from_utf8_lossy(&message[2][..]).parse::<i32>().unwrap_or(0); 
                                    let arg3 = String::from_utf8_lossy(&message[3][..]).parse::<i32>().unwrap_or(0); 
                                    let arg4 = String::from_utf8_lossy(&message[4][..]).parse::<i32>().unwrap_or(0); 
                                    let arg5 = String::from_utf8_lossy(&message[5][..]).parse::<i32>().unwrap_or(0); 
                                    let arg6 = String::from_utf8_lossy(&message[6][..]).parse::<i32>().unwrap_or(0); 
                                    
                                    let value = if arg5 == 0 {
                                        0
                                    }
                                    else {
                                        1
                                    };

                                    let mut packet = OscPacket::Message(OscMessage {
                                        addr: "/encbut".to_string(),
                                        args: Some(vec![OscType::Int(value)]),
                                    });

                                    // let mut packet = OscPacket::Message(OscMessage {
                                    //     addr: address,
                                    //     args: Some(vec![OscType::Int(arg), 
                                    //                     OscType::Int(arg2),
                                    //                     OscType::Int(arg3),
                                    //                     OscType::Int(arg4),
                                    //                     OscType::Int(arg5),
                                    //                     OscType::Int(arg6)]),
                                    // });
                                    info!("{:?}", packet);
                                    serial.osc_sender.send((packet, Some(serial.pd_send_addr))).unwrap();
                                }
                                else {                                
                                    // build and transmit packet
                                    let mut packet = OscPacket::Message(OscMessage {
                                        addr: address,
                                        args: Some(vec![OscType::Int(arg)]),
                                    });
                                    info!("{:?}", packet);
                                    serial.osc_sender.send((packet, None)).unwrap();
                                }
                            }

                            // setup for next message
                            for i in 0..index+1 {
                                message[i].clear();
                            }
                            index = 0;
                        }
                        // move to argument processing or next argument
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

        // },
        //     Err(s) => {
        //         error!("ERROR Send to ORAC/PD: {}", s)
        //     }
        // }

        // due to bug with revc_timeout panicing on OSC thread, we pass a fake packet to enable terminating that thread
        let mut fake_packet = OscPacket::Message(OscMessage {
            addr: "/fakepacket".to_string(),
            args: None,
        });
        serial.osc_sender.send((fake_packet, None)).unwrap();
    }
}


pub struct SerialSend {
    /// Handle to serial port for TX communication
    port: Box<dyn SerialPort>,
    /// ORAC controller descriptions are stored here, and then conjoined with value to form outgoing message
    desc: [String; 8],
}

impl SerialSend {
    pub fn new(port: Box<dyn SerialPort>) -> Self {
        SerialSend {
            port: port,
            desc: [ "".to_string(), "".to_string(), "".to_string(), "".to_string(),
                    "".to_string(), "".to_string(), "".to_string(), "".to_string() ]
        }
    }

    /// send module/page and controller data to muses xyx hardware for OLED display
    pub fn send(&mut self, packet: &OscPacket) {
        //let addr = 
        let mut d = "".to_string();
        match packet {
            OscPacket::Message(msg) => {
                match &msg.addr[..] {
                    "/module"  => { d = "m".to_owned() + &osc_string(&msg) + &"\n"; },
                    "/page"    => { d = "p".to_owned() + &osc_string(&msg) + &"\n"; },
                    "/P1Desc"  => { 
                        self.desc[0] = "1".to_owned() + &osc_string(&msg) + &" ";
                        return;
                    },
                    "/P1Value" => { d = self.desc[0].clone() + &osc_string(&msg) + &"\n"; },
                    "/P2Desc"  => { 
                        self.desc[1] = "2".to_owned() + &osc_string(&msg) + &" "; 
                        return; 
                    },
                    "/P2Value" => { d = self.desc[1].clone() + &osc_string(&msg) + &"\n"; },
                    "/P3Desc"  => { 
                        self.desc[2] = "3".to_owned() + &osc_string(&msg) + &" "; 
                        return;
                    },
                    "/P3Value" => { d = self.desc[2].clone() + &osc_string(&msg) + &"\n"; },
                    "/P4Desc"  => { 
                        self.desc[3] = "4".to_owned() + &osc_string(&msg) + &" ";
                        return;
                    },
                    "/P4Value" => { d = self.desc[3].clone() + &osc_string(&msg) + &"\n"; },
                    "/P5Desc"  => { 
                        self.desc[4] = "5".to_owned() + &osc_string(&msg) + &" "; 
                        return; 
                    },
                    "/P5Value" => { d = self.desc[4].clone() + &osc_string(&msg) + &"\n"; },
                    "/P6Desc"  => { 
                        self.desc[5] = "6 ".to_owned() + &osc_string(&msg) + &" "; 
                        return; 
                    },
                    "/P6Value" => { d = self.desc[5].clone() + &osc_string(&msg) + &"\n"; },
                    "/P7Desc"  => { 
                        self.desc[6] = "7".to_owned() + &osc_string(&msg) + &" "; 
                        return;
                    },
                    "/P7Value" => { d = self.desc[6].clone() + &osc_string(&msg) + &"\n"; },
                    "/P8Desc"  => { 
                        self.desc[7] = "8".to_owned() + &osc_string(&msg) + &" "; 
                        return; 
                    },
                    "/P8Value" => { d = self.desc[7].clone() + &osc_string(&msg) + &"\n"; },
                    _ => {
                        return;
                    }
                }
            },
            OscPacket::Bundle(_) => {
                error!("{}", "no support OSC bundles");
                return;
            }
        }
        // now send message
        self.port.write(d.as_bytes())
                .expect("Failed to write to serial port");
    }
}