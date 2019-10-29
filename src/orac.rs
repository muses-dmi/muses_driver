//! Description: 
//!    Simple cmd linetool to run the Muses xyz ORAC driver, automatically connecting to xyz. Waits for user to 
//!    enter return, then disconnects and termiates.
//! 
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.

#![allow(dead_code)]
#![warn(unused_variables)]
#![allow(warnings)]

#![feature(const_fn)]
#![feature(deadline_api)]

#[macro_use]
extern crate log;
//extern crate stderrlog;

extern crate simple_logger;

// extern crate libc;

// extern crate net2;

#[macro_use]
extern crate serde_derive;

use serde::{Deserialize, Serialize};
use serde_json::{Value};

use serialport::prelude::*;
use serialport::SerialPortType;

use std::time::{Duration, Instant};

use std::sync::mpsc::{channel, Sender, Receiver};

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use std::fs::File;
//use std::io::Read;

use rosc::{OscPacket, OscMessage, OscType};

extern crate muses_sensel;
use muses_sensel::*;
use muses_sensel::sensel::*;

mod orac_serial_device;
mod transport;
mod transport_rec;
mod osc;
mod orac_term_display;
mod osc_utils;

extern crate getopts;
use getopts::Options;
use std::env;

extern crate termion;
use termion::terminal_size;

use termion::color;
use termion::raw::IntoRawMode;
use std::io::{Read, Write, stdout, stdin};

extern crate muses_driver;
use muses_driver::*;

use orac_term_display::*;

//use std::io::stdin;

use crate::LIVE_DRIVERS;

// need to move to config
const ORAC_SEND_ADDR: &'static str = "192.168.2.2:6100"; //"127.0.0.1:6100";
const ORAC_SEND_PORT: i32 = 6101;
const ORAC_RECEIVE_ADDR: &'static str = "192.168.2.1:6101"; // ""192.168.2.2:6101";

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("i", "interface", "Use termimal interface");
    opts.optflag("", "pi", "Use Statis PI with PiSound for Orac");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    //simple_logger::init().unwrap();
    stderrlog::new()
        .module(module_path!())
        .verbosity(2)
        .init()
        .unwrap();
    
    info!("Muses ORAC Driver Rust Component initilaized");

    // connect to xyz
    let (osc_r, osc_s) = connecting(matches.opt_present("pi"));

    if matches.opt_present("interface") {
        display(osc_r, osc_s);
    }
    else {
        println!("Press Enter to exit driver");
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
    }

    // disconnect from xyz
    muses_driver::disconnect_rust();

    // close down driver
}

pub fn connecting(using_pi: bool) -> (Receiver<(OscPacket, Option<String>)>, Sender<(OscPacket, Option<String>)>) {

    // check if already connected, LIVE_DRIVERS would be zero if not
    // if LIVE_DRIVERS.load( Ordering::SeqCst) != 0 {
    //     return;
    // }

    // Read config file
    // TODO: this is not portable to non POSIX systems
    let config_path = format!("{}{}", env::var("HOME").expect("Failed to read $(HOME)"), muses_config);
    let mut config = String::new();
    let mut f = 
        File::open(config_path)
        .expect("Unable open config file");
    f.read_to_string(&mut config).expect("Unable to read config file");

    // Deserialize config
    let config : Config  = serde_json::from_str(&config).expect("Invalid config file");

    let mut from_addr: String = config.osc_from_addr.clone();
    let mut to_addr: String = config.osc_to_addr.clone();
    let mut orac_send_addr: String = config.orac_send_addr.clone();
    let mut orac_receive_addr: String = config.orac_receive_addr.clone();
    let orac_send_port: i32 = config.orac_send_port;

    if (using_pi) {
        from_addr = config.osc_pi_from_addr.clone();
        to_addr = config.osc_to_addr.clone();
        orac_send_addr = config.orac_pi_send_addr.clone();
        orac_receive_addr = config.orac_pi_receive_addr.clone();
    }

    //--------------
    // OSC receiver
    //--------------

    // create out going OSC thread, which receives events from MEC
    let (osc_rec_s, osc_rec_r)    = channel();
    // let osc_rec_s_copy    = osc_rec_s.clone();
    // match transport_rec::Transport::new(&orac_receive_addr[..], osc_rec_s) {
    //     Ok(transport) => {
    //         std::thread::Builder::new()
    //             .spawn(move || {
    //                 info!("ORAC RECEIVE thread is running");
                    
    //                 // increment LIVE_DRIVERS, to register us
    //                 LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);
                    
    //                 transport.run();

    //                 // decrement LIVE_DRIVERS, to deregister us
    //                 LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

    //                 info!("ORAC RECEIVE is disconnected");
    //             }).unwrap();
    //     },
    //     Err(s) => {
    //         error!("ERROR ORAC RECEIVE: {}", s)
    //     }
    // }

    //--------------
    // OSC producer
    //--------------

    // create out going OSC thread, which receives events from the muses hardware
    let (osc_s, osc_r)    = channel();
    let osc_s_return      = osc_s.clone();
    let orac_receive_addr_clone = orac_receive_addr.clone();

    // from address 127.0.0.1:8001
    // to address 127.0.0.1:8338
    // match transport::Transport::new(&from_addr[..], &orac_send_addr[..]) {
    //     Ok(transport) => {
    //         std::thread::Builder::new()
    //             .spawn(move || {
    //                 info!("OSC thread is running");

    //                 // increment LIVE_DRIVERS, to register us
    //                 LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);
                    
    //                 let two = std::time::Duration::from_secs(2);
    //                 std::thread::sleep(two);

    //                 // Send /Connect to ORAC
    //                 transport.send(
    //                     &OscPacket::Message(OscMessage {
    //                         addr: "/Connect".to_string(),
    //                         args: Some(vec![OscType::Int(orac_send_port)]),
    //                     }));
    //                     //transport::Transport::get_addr_from_arg(ORAC_RECEIVE_ADDR).unwrap());

    //                 let s = osc::Osc::new(osc_r);

    //                 // run driver
    //                 s.run(&transport);

    //                 // send message to OSC receiver thread to terminate
    //                 transport.send_to(
    //                     &OscPacket::Message(OscMessage {
    //                         addr: "/terminate".to_string(),
    //                         args: None,
    //                     }),
    //                 transport::Transport::get_addr_from_arg(&orac_receive_addr_clone[..]).unwrap());
                    
    //                 // decrement LIVE_DRIVERS, to deregister us
    //                 LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

    //                 info!("OSC thread is disconnected");
    //             }).unwrap();
    //     }
    //     Err(s) => {
    //         error!("ERROR: {}", s)
    //     }
    // }

    // Read SVG IR
    let mut data = String::new();
    let mut f = 
        File::open(config.svg_ir_path)
        .expect("Unable to open Sensel SVG JSON IR");
    f.read_to_string(&mut data).expect("Unable to read string from Sensel SVG JSON");

    //-----------
    // STM32/Arduino, buttons, and encoders, Sensel, ...
    //-----------
    // select and open serial port, if no port found, then simple return,
    
    let mut serial_for_write : Option<Box<dyn SerialPort>> = None;

    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            // FIXME: allow user to select via JSON configure
            match (p.port_type) {
                serialport::SerialPortType::UsbPort(usb_port) => {
                    //println!("{:?} {:?} {:?}", usb_port.manufacturer, usb_port.product, usb_port.pid);
                    //if p.port_name == config.arduino_serial_port { //"/dev/tty.usbmodem141401" {
                    if usb_port.pid == config.serial_pid as u16 {
                        info!("Opening serial port {}", p.port_name);

                        let s = SerialPortSettings {
                            baud_rate: config.serial_baud,
                            data_bits: DataBits::Eight,
                            flow_control: FlowControl::None,
                            parity: Parity::None,
                            stop_bits: StopBits::One,
                            timeout: Duration::from_millis(1),
                        };
                        if let Ok(serial) = serialport::open_with_settings(&p.port_name, &s) {
                            // be sure not to move send channel
                            let oo = osc_s.clone();

                            // we want to pass the serial port into the OSC rec
                            // so we create it once we have the serial port
                            serial_for_write = Some(serial.try_clone().expect("Failed to clone"));

                            std::thread::Builder::new()
                                .spawn(move || {
                                    let interface = 
                                        interface_direct::InterfaceBuilder::new(data)
                                        .build();

                                    match interface {
                                        Ok(interface) => {
                                            info!("serial thread is running");
                                            
                                            // increment LIVE_DRIVERS, to register us
                                            LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);

                                            let s = orac_serial_device::Serial::new(interface, oo, serial);
                                            
                                            // run driver
                                            orac_serial_device::Serial::run(s);
                                            
                                            // decrement LIVE_DRIVERS to deregister us
                                            LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                                            info!("serial (Arduino) thread is disconnected");
                                        },
                                        Err(s) => {
                                            error!("ERROR: {}", s)
                                        }
                                    }
                                }).unwrap();
                            break;
                        }
                        else {
                            error!("Failed to open {}", p.port_name);
                            //return;
                        }
                    }
                },
                _ => { }
            }
        }
    } else {
        error!("Error listing serial ports");
    }


    let serial_send = orac_serial_device::SerialSend::new(serial_for_write.unwrap());
    match transport_rec::Transport::new(&orac_receive_addr[..], osc_rec_s, serial_send) {
        Ok(mut transport) => {
            std::thread::Builder::new()
                .spawn(move || {
                    info!("ORAC RECEIVE thread is running");
                    
                    // increment LIVE_DRIVERS, to register us
                    LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);
                    
                    transport.run();

                    // decrement LIVE_DRIVERS, to deregister us
                    LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                    info!("ORAC RECEIVE is disconnected");
                }).unwrap();
        },
        Err(s) => {
            error!("ERROR ORAC RECEIVE: {}", s)
        }
    }

    // finally we create the out going OSC thread (that sends messages to MEC)
    match transport::Transport::new(&from_addr[..], &orac_send_addr[..]) {
        Ok(transport) => {
            std::thread::Builder::new()
                .spawn(move || {
                    info!("OSC thread is running");

                    // increment LIVE_DRIVERS, to register us
                    LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);
                    
                    // Send /Connect to ORAC
                    transport.send(
                        &OscPacket::Message(OscMessage {
                            addr: "/Connect".to_string(),
                            args: Some(vec![OscType::Int(orac_send_port)]),
                        }));
                        //transport::Transport::get_addr_from_arg(ORAC_RECEIVE_ADDR).unwrap());

                    let s = osc::Osc::new(osc_r);

                    // run driver
                    s.run(&transport);

                    // send message to OSC receiver thread to terminate
                    transport.send_to(
                        &OscPacket::Message(OscMessage {
                            addr: "/terminate".to_string(),
                            args: None,
                        }),
                    transport::Transport::get_addr_from_arg(&orac_receive_addr_clone[..]).unwrap());
                    
                    // decrement LIVE_DRIVERS, to deregister us
                    LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                    info!("OSC thread is disconnected");
                }).unwrap();
        }
        Err(s) => {
            error!("ERROR: {}", s)
        }
    }

    (osc_rec_r, osc_s_return)
}