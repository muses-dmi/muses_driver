//
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//

use std::str::FromStr;

use std::{thread, time};

use rosc::{OscPacket, OscMessage, OscType};

use std::sync::mpsc::{Receiver};

use std::io::stdin;
use std::thread::spawn;
use std::sync::mpsc::channel;

use std::sync::atomic::{AtomicBool, Ordering};

use super::transport::*;

//------------------------------------------------------------------------------

// Shared AtomicBool, when true listening threads should shutdown
use crate::DISCONNECT;

//------------------------------------------------------------------------------
