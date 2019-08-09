//
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//

use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;
use std::time::{Duration, Instant};

use std::{thread, time};

use rosc::{OscPacket, OscMessage, OscType};

use std::sync::mpsc::{Receiver};

use std::io::stdin;
use std::thread::spawn;
use std::sync::mpsc::channel;

use std::sync::atomic::{AtomicBool, Ordering};

use super::transport::*;

// Shared AtomicBool, when true listening threads should shutdown
use crate::DISCONNECT;

pub struct Osc {
    // channel to receive OSC packets from arduino, sensel, and lightpad on
    osc_reciver: Receiver<OscPacket>,
}

unsafe impl Send for Osc {
}

impl Osc {
    pub fn new(osc_r: Receiver<OscPacket>) -> Self {
        Osc {
            osc_reciver: osc_r,
        }
    }

    pub fn run(mut self, transport: Transport) {
        // process osc messages, until disconnect request
        while !DISCONNECT.load(Ordering::SeqCst) {
            // set a timeout of 2secs, so we can check we are not supposed to exit
            // hmm... there seems to be a bug that cause the timeout to panic (see #39364)
            match self.osc_reciver.recv() { //_timeout(Duration::from_secs(2)) {
                Ok(packet) => {
                    // workaround for panic bug, is to have serial thread send a FAKE packet
                    if DISCONNECT.load(Ordering::SeqCst) {
                        return;
                    }
                    transport.send(&packet);
                },
                Err(_)     => { }
            }
        }
    }
}