//
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//

//use net2::{unix::UnixUdpBuilderExt, UdpBuilder};

use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;

use std::sync::mpsc::{Sender};

use rosc::{OscPacket, OscMessage, OscType};
use rosc::encoder;

use std::time::Duration;

use std::sync::atomic::{AtomicBool, Ordering};
use crate::DISCONNECT;

#[derive(Debug)]
pub struct Transport {
    socket: UdpSocket,
    from_addr: SocketAddrV4,
    osc_sender: Sender<OscPacket>,
}

impl Transport {
    pub fn new(from_addr: &str, osc_sender: Sender<OscPacket>) -> Result<Self, &'static str> {
        // let addr = match SocketAddrV4::from_str(from_addr) {
        //     Ok(addr) => addr,
        //     Err(_) => panic!("moo"),
        // };

        // // we need to be able to reuse the socket as it will be open
        // let sock = UdpBuilder::new_v4().unwrap()
        //     .reuse_address(true).unwrap()
        //     .reuse_port(true).unwrap()
        //     .bind(from_addr).unwrap();

        // //let sock = UdpSocket::bind(addr).unwrap();

        // Ok(Transport { 
        //     socket: sock, 
        //     from_addr: addr,
        //     osc_sender: osc_sender, 
        // })
        

         Transport::get_addr_from_arg(from_addr)
            .and_then(|from_addr| 
                    UdpSocket::bind(from_addr)
                    .and_then(|sock| Ok(Transport { 
                        socket: sock, 
                        from_addr: from_addr,
                        osc_sender: osc_sender, }))
                    .map_err(|_| "failed to open socket"))
    }

    fn get_addr_from_arg(arg: &str) -> Result<SocketAddrV4, &'static str> {
        match SocketAddrV4::from_str(arg) {
            Ok(addr) => Ok(addr),
            Err(_)   => Err("failed to create socket addr")
        }
    }

    pub fn run(&self) {
        // timeout read every 3 secs to check quit
        //self.socket.set_read_timeout(Some(Duration::new(3, 0)));
        let mut buf = [0u8; rosc::decoder::MTU];
        while !DISCONNECT.load(Ordering::SeqCst) {
            match self.socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    //info!("Received packet with size {} from: {}", size, addr);
                    let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                    Transport::info_packet(&packet);
                    self.osc_sender.send(packet);
                }
                Err(e) => {
                    //error!("Error receiving from socket: {}", e);
                }
            }
        }
    }

    pub fn info_packet(packet: &OscPacket) {
        match packet {
            OscPacket::Message(msg) => {
                info!("OSC address: {}", msg.addr);
                match &msg.args {
                    Some(args) => {
                        info!("OSC arguments: {:?}", args);
                    }
                    None => info!("No arguments in message."),
                }
            }
            OscPacket::Bundle(bundle) => {
                println!("OSC Bundle: {:?}", bundle);
            }
        }
    }

    // fn enable_port_reuse(socket: &UdpSocket) -> std::io::Result<()> {
    //     use std::os::unix::prelude::*;
    //     use std::mem;
    //     use libc;

    //     unsafe {
    //         let optval: libc::c_int = 1;
    //         let ret = libc::setsockopt(
    //             socket.as_raw_fd(),
    //             libc::SOL_SOCKET,
    //             libc::SO_REUSEPORT,
    //             &optval as *const _ as *const libc::c_void,
    //             mem::size_of_val(&optval) as libc::socklen_t,
    //         );
    //         if ret != 0 {
    //             return Err(std::io::Error::last_os_error());
    //         }
    //     }

    //     Ok(())
    // }
}