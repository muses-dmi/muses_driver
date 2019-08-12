
//! Description: 
//! 
//!    This is a "C" only API designed to be called from a native OS host application. 
//!    Currently this is tested with statusBar app, which is a Mac OS Swift application that runs as a native 
//!    status bar app. 
//! 
//!    It should be straightforward to use this API on other OSes, but to this point no testing has been done.
//! TODO: 
//!     Test Sensel Driver
//!     Add ~/$(HOME)/.muses/driver_init.json
//!     Get Sensel presets from config
//!     ROLI Lightpad driver
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.

#![allow(dead_code)]
#![warn(unused_variables)]
#![allow(warnings)]

#![feature(const_fn)]
#![feature(deadline_api)]

#[macro_use]
extern crate log;
extern crate simple_logger;

use serialport::prelude::*;
use serialport::SerialPortType;

use std::time::{Duration, Instant};

use std::sync::mpsc::{channel, Sender, Receiver};

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use std::fs::File;
use std::io::Read;

extern crate muses_sensel;
use muses_sensel::*;

mod serial_device;
mod transport;
mod osc;

//------------------------------------------------------------------------------

/// Globally shared variable use to inform drivers they should disconnect
pub static DISCONNECT: AtomicBool = AtomicBool::new(false);

/// Globaly shared variable that tracks number of live (i.e. connected) drivers
static LIVE_DRIVERS: AtomicI32 = AtomicI32::new(0);

//------------------------------------------------------------------------------

/// UDP address that OSC messages are sent from
const from_addr: &'static str = "127.0.0.1:8001";

/// UDP address that OSC messages are sent to
const to_addr: &'static str = "127.0.0.1:8338";

//------------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn init_rust() {
     // logging is only enabled for debug build
     //#[cfg(debug_assertions)]
     simple_logger::init().unwrap();

     info!("Muses Driver Rust Component initilaized");
}

#[no_mangle]
pub extern "C" fn disconnect_rust() {
    // check if already connected, LIVE_DRIVERS would be zero if not
    if LIVE_DRIVERS.load( Ordering::SeqCst) == 0 {
        return;
    }

    // tell drivers to disconnect
    DISCONNECT.store(true, Ordering::SeqCst);

    // wait for drivers to disconnect
    while LIVE_DRIVERS.load(Ordering::SeqCst) != 0 {
    }

    // at this point all drivers have disconnected

    // clear disconnect request, ready for a new connection request
    DISCONNECT.store(false, Ordering::SeqCst);
}

#[no_mangle]
/// call to connected to Muses instrument
pub extern "C" fn connect_rust() {

    // check if already connected, LIVE_DRIVERS would be zero if not
    if LIVE_DRIVERS.load( Ordering::SeqCst) != 0 {
        return;
    }

    // TODO: add ~/$(HOME)/.muses/driver_init.json


    // create out going OSC thread, which receives events from the muses hardware
    let (osc_s, osc_r)    = channel();

    // from address 127.0.0.1:8001
    // to address 127.0.0.1:8338
    match transport::Transport::new(from_addr, to_addr) {
        Ok(transport) => {
            std::thread::Builder::new()
                .spawn(move || {
                    info!("OSC thread is running");
                    
                    // increment LIVE_DRIVERS, to register us
                    LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);
                    
                    let s = osc::Osc::new(osc_r);
                    
                    // run driver
                    s.run(transport);
                    
                    // decrement LIVE_DRIVERS, to deregister us
                    LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                    info!("OSC thread is disconnected");
                }).unwrap();
        }
        Err(s) => {
            error!("ERROR: {}", s)
        }
    }

    // Arduino, buttons, and encoders

    // select and open serial port, if no port found, then simple return
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            // FIXME: allow user to select via JSON configure
            if p.port_name == "/dev/tty.usbmodem143401" { //"/dev/tty.usbmodem141401" {
                info!("Opening serial port {}", p.port_name);

                let s = SerialPortSettings {
                    baud_rate: 9600,
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
                            info!("serial (Arduino) thread is running");
                            
                            // increment LIVE_DRIVERS, to register us
                            LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);

                            let s = serial_device::Serial::new(oo, serial);
                            
                            // run driver
                            serial_device::Serial::run(s);
                            
                            // decrement LIVE_DRIVERS to deregister us
                            LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                            info!("serial (Arduino) thread is disconnected");
                        }).unwrap();
                    break;
                }
                else {
                    error!("Failed to open {}", p.port_name);
                    return;
                }
            }
        }
    } else {
        error!("Error listing serial ports");
        return;
    }

    // Sensel

    // TODO: process JSON file(s) for Sensel presets from config
    let mut data = String::new();
    let mut f = 
        File::open("/Users/br-gaster/dev/audio/muses_rust/external/github/svg_interface/farm1.json")
        .expect("Unable to JSON IR");
    f.read_to_string(&mut data).expect("Unable to read string");

    // be sure not to move send channel
    let o_s = osc_s.clone();
    std::thread::Builder::new()
        .spawn(move || {
            // create an interface with the SVG JSON IR
            let interface = 
                interface::InterfaceBuilder::new(data)
                .build();

            match interface {
                Ok(interface) => {
                
                        info!("Sensel thread is running");
                        
                        // increment LIVE_DRIVERS, to register us
                        LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);

                        interface.run(150, o_s, &DISCONNECT);
                        
                        // decrement LIVE_DRIVERS to deregister us
                        LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                        info!("Sensel thread is disconnected");
                },
                Err(s) => {
                    error!("ERROR: {}", s)
                }
            }
        }).unwrap();

    // open ROLI lightpad
}