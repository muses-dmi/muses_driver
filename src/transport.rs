//
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//

use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;

use rosc::{OscPacket, OscMessage, OscType};
use rosc::encoder;

#[derive(Debug)]
pub struct Transport {
    socket: UdpSocket,
    to_addr: SocketAddrV4,
}

impl Transport {
    pub fn new(host_addr: &str, to_addr: &str) -> Result<Self, &'static str> {
         Transport::get_addr_from_arg(host_addr)
            .and_then(|host_addr| 
                Transport::get_addr_from_arg(to_addr)
                .and_then(|to_addr| 
                    UdpSocket::bind(host_addr)
                    .and_then(|sock| Ok(Transport { socket: sock, to_addr: to_addr }))
                    .map_err(|_| "failed to open socket")))
    }

    pub fn get_addr_from_arg(arg: &str) -> Result<SocketAddrV4, &'static str> {
        match SocketAddrV4::from_str(arg) {
            Ok(addr) => Ok(addr),
            Err(_)   => Err("failed to create socket addr")
        }
    }

    /// send using send address created with
    pub fn send(&self, packet: &OscPacket) -> Result<(), &'static str> {
        let msg_buf = encoder::encode(packet).unwrap();
        self.socket.send_to(&msg_buf, self.to_addr)
            .and(Ok(()))
            .map_err(|_| "failed to open socket")
    }

    /// send to a given address
    pub fn send_to(&self, packet: &OscPacket, to_addr: SocketAddrV4) -> Result<(), &'static str> {
        let msg_buf = encoder::encode(packet).unwrap();
        self.socket.send_to(&msg_buf, to_addr)
            .and(Ok(()))
            .map_err(|_| "failed to open socket")
    }
}