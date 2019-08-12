//! Description: 
//!    Simple cmd linetool to run the Muses xyz driver, automatically connecting to xyz. Waits for user to 
//!    enter return, then disconnects and termiates.
//! 
//!    Note this is only intended to test driver during development, the statusBar app for MacOS, is the 
//!    currently supported way to utilize Muses xyz.
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.

extern crate muses_driver;

use std::io::stdin;

pub fn main() {
    // initialize driver
    muses_driver::init_rust();

    // connect to xyz
    muses_driver::connect_rust();

    println!("Press Enter to exit driver");
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    // disconnect from xyz
    muses_driver::disconnect_rust();

    // close down driver
}