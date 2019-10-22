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

mod serial_device;
mod transport;
mod transport_rec;
mod osc;
mod orac_term_display;

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

const ORAC_SEND_ADDR: &'static str = "127.0.0.1:6100";
const ORAC_SEND_PORT: i32 = 6101;
const ORAC_RECEIVE_ADDR: &'static str = "127.0.0.1:6101";

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("", "sensel-only", "Only run sensel driver not full muses instrument");
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
    //muses_driver::connect_rust(matches.opt_present("sensel-only"));
    let (osc_r, osc_s) = connecting();

    display(osc_r, osc_s);

    // println!("Press Enter to exit driver");
    // let mut input = String::new();
    // stdin().read_line(&mut input).unwrap();

    // disconnect from xyz
    muses_driver::disconnect_rust();

    // close down driver
}

pub fn connecting() -> (Receiver<OscPacket>, Sender<OscPacket>) {

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

    //--------------
    // OSC receiver
    //--------------

    // create out going OSC thread, which receives events from MEC
    let (osc_rec_s, osc_rec_r)    = channel();
    let osc_rec_s_copy    = osc_rec_s.clone();
    match transport_rec::Transport::new(ORAC_RECEIVE_ADDR, osc_rec_s) {
        Ok(transport) => {
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

    //--------------
    // OSC producer
    //--------------

    // create out going OSC thread, which receives events from the muses hardware
    let (osc_s, osc_r)    = channel();
    let osc_s_return      = osc_s.clone();

    // from address 127.0.0.1:8001
    // to address 127.0.0.1:8338
    match transport::Transport::new(&config.osc_from_addr[..], ORAC_SEND_ADDR) {
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
                            args: Some(vec![OscType::Int(ORAC_SEND_PORT)]),
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
                    transport::Transport::get_addr_from_arg(ORAC_RECEIVE_ADDR).unwrap());
                    
                    // decrement LIVE_DRIVERS, to deregister us
                    LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                    info!("OSC thread is disconnected");
                }).unwrap();
        }
        Err(s) => {
            error!("ERROR: {}", s)
        }
    }

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
    
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            // FIXME: allow user to select via JSON configure
            match (p.port_type) {
                serialport::SerialPortType::UsbPort(usb_port) => {
                    println!("{:?} {:?} {:?}", usb_port.manufacturer, usb_port.product, usb_port.pid);
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

                                            let s = serial_device::Serial::new(interface, oo, serial);
                                            
                                            // run driver
                                            serial_device::Serial::run(s);
                                            
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

    (osc_rec_r, osc_s_return)
}