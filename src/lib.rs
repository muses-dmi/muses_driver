//! Copyright © 2019 Benedict Gaster. All rights reserved.

#![allow(dead_code)]
#![warn(unused_variables)]
#![allow(warnings)]

#![feature(const_fn)]
#![feature(deadline_api)]

#[macro_use]
extern crate log;
//extern crate env_logger;
extern crate stderrlog;

use serialport::prelude::*;
use serialport::SerialPortType;

use std::time::{Duration, Instant};

use std::sync::mpsc::{channel, Sender, Receiver};

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

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
extern "C" fn init_rust() {
    stderrlog::new().module(module_path!()).init().unwrap();
}

#[no_mangle]
extern "C" fn disconnect_rust() {
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
extern "C" fn connect_rust() {

    // check if already connected, LIVE_DRIVERS would be zero if not
    if LIVE_DRIVERS.load( Ordering::SeqCst) != 0 {
        return;
    }

    // create out going OSC thread, which receives events from the muses hardware
    let (osc_s, osc_r)    = channel();

    // from address 127.0.0.1:8001
    // to address 127.0.0.1:8338
    match transport::Transport::new(from_addr, to_addr) {
        Ok(transport) => {
            std::thread::Builder::new()
                .spawn(move || {
                    error!("OSC thread is running");
                    
                    // increment LIVE_DRIVERS, to register us
                    LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);
                    
                    let s = osc::Osc::new(osc_r);
                    
                    // run driver
                    s.run(transport);
                    
                    // decrement LIVE_DRIVERS, to deregister us
                    LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                    error!("OSC thread is disconnected");
                }).unwrap();
        }
        Err(s) => {
            error!("ERROR: {}", s)
        }
    }

    // open Serial port on Arduino, buttons, and encoders

    // select and open serial port, if no port found, then simple return
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            // FIXME: allow user to select via JSON configure
            if p.port_name == "/dev/tty.usbmodem143401" { //"/dev/tty.usbmodem141401" {
                error!("Opening serial port {}", p.port_name);

                let s = SerialPortSettings {
                    baud_rate: 9600,
                    data_bits: DataBits::Eight,
                    flow_control: FlowControl::None,
                    parity: Parity::None,
                    stop_bits: StopBits::One,
                    timeout: Duration::from_millis(1),
                };
                if let Ok(serial) = serialport::open_with_settings(&p.port_name, &s) {
                    std::thread::Builder::new()
                        .spawn(move || {
                            error!("serial (Arduino) thread is running");
                            
                            // increment LIVE_DRIVERS, to register us
                            LIVE_DRIVERS.fetch_add(1, Ordering::SeqCst);

                            let s = serial_device::Serial::new(osc_s.clone(), serial);
                            
                            // run driver
                            serial_device::Serial::run(s);
                            
                            // decrement LIVE_DRIVERS to deregister us
                            LIVE_DRIVERS.fetch_add(-1, Ordering::SeqCst);

                            error!("serial (Arduino) thread is disconnected");
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

    // open sensel

    // open ROLI lightpad
}