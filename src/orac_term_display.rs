//! Description: 
//! 
//! Simple terminal (using termion) display for ORAC
//! 
//! Mostly intended for debugging, but can be used to inspect ORAC controls.
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 
extern crate termion;
use termion::terminal_size;

use termion::color;
use termion::raw::IntoRawMode;
use std::io::{Read, Write, stdout, stdin};
use std::io::Stdout;
use termion::async_stdin;

use std::sync::mpsc::{channel, Sender, Receiver};
use rosc::{OscPacket, OscMessage, OscType};

use crate::transport_rec::*;

//------------------------------------------------------------------------------

// background and foreground colour used for interface
const RGB_BACKGROUND: (u8, u8, u8) = (82,82,144);
const RGB_FOREGROUND: (u8, u8, u8) = (32, 29, 128);

// positions for text and controls
const MODULE_NAME_XY: (u16, u16) = (10,3);
const PAGE_NAME_XY: (u16, u16)   = (10, 10);
const P1DESC_XY: (u16, u16)      = (5, 5);
const P2DESC_XY: (u16, u16)      = (30, 5);
const P3DESC_XY: (u16, u16)      = (55, 5);
const P4DESC_XY: (u16, u16)      = (80, 5);
const P5DESC_XY: (u16, u16)      = (5, 10);
const P6DESC_XY: (u16, u16)      = (30, 10);
const P7DESC_XY: (u16, u16)      = (55, 10);
const P8DESC_XY: (u16, u16)      = (80, 10);

// ORAC/MEC currently supports a maximum of 8 controllers
const NUM_CONTROLLERS: usize = 8;

// properties for individual controllers
struct Controller {
    /// text description 
    pub desc: String,
    /// position of controller in terminal display
    pub pos: (u16, u16),
    /// string representation of the controllers value
    pub value: String,
    /// actual controller value, between (0,1]
    pub control: f32,
}

impl Controller {
    /// initalize a new controller
    pub fn new(pos: (u16, u16) ) -> Self {
        // initial values are sent from MEC, so we just choose a default
        Controller {
            desc: "".to_string(),
            pos: pos,
            value: "".to_string(),
            control: 0.0, 
        }
    }

    /// display controller to terminal
    pub fn display<W: Write>(&self, mut stdout : &mut termion::raw::RawTerminal<W>) {
        if self.desc.len() > 0 {
            write!(*stdout,
                "{}{}{}{} {}",
                termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                termion::cursor::Goto(self.pos.0, self.pos.1),
                color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                self.desc,
                self.value)
                .unwrap();
        }
    }

    /// reset controll to initial state
    pub fn reset(&mut self) {
        self.desc    = "".to_string();
        self.value   = "".to_string();
        self.control = 0.0;
    }
}

/// extract a string from 1st OSC argument, fallback ""
pub fn osc_string(msg: &OscMessage) -> String {
    if let Some(vec) = &msg.args {
        if let OscType::String(value) = &vec[0] {
            return value.clone();
        }
    }
    "".to_string()
}

/// extract a float from 1st OSC argument, fallback 0.0
pub fn osc_float(msg: &OscMessage) -> f32 {
    if let Some(vec) = &msg.args {
        if let OscType::Float(value) = &vec[0] {
            return *value;
        }
    }
    0.0
}

/// simple terminal display for ORAC
/// osc_receiver is incoming OSC messages (from MEC)
/// osc_sender is out going OSC messages (to MEC)
pub fn display(osc_receiver: Receiver<(OscPacket, Option<String>)>, osc_sender: Sender<(OscPacket, Option<String>)>) {

    let (x,y) = terminal_size().unwrap();

    let stdout: Stdout = stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();

    let mut stdin = async_stdin().bytes();

    write!(stdout,
           "{}{}{}{}{}{}Muses xyz for OARC{}",
           termion::cursor::Hide,
           termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
           color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
           termion::clear::All,
           termion::cursor::Goto(1, 1),
           termion::style::Bold,
           termion::style::Reset)
            .unwrap();
    stdout.flush().unwrap();

    // setup controllers
    let mut controllers: [Controller; NUM_CONTROLLERS] = [
        Controller::new(P1DESC_XY), Controller::new(P2DESC_XY), Controller::new(P3DESC_XY), Controller::new(P4DESC_XY),
        Controller::new(P5DESC_XY), Controller::new(P6DESC_XY), Controller::new(P7DESC_XY), Controller::new(P8DESC_XY)
    ];

    // set holders for current module and current page
    let mut module: String = "".to_string();
    let mut page: String = "".to_string();

    //let mut bytes = stdin.bytes();
    loop {
        // handle input
        if let Some(Ok(bb)) = stdin.next() {
            match bb {
                    // Quit
                    b'q' => {
                        // write!(stdout, "quitting");
                        // stdout.flush().unwrap();
                        write!(stdout,
                            "{}{}{}{}Good bye",
                        termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                        color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                        termion::clear::All,
                        termion::cursor::Goto(1, 1))
                            .unwrap();
                        stdout.flush().unwrap();
                        return;
                    },
                   
                    // next module
                    b'p' => {
                        let mut packet = OscPacket::Message(OscMessage {
                                    addr: "/ModuleNext".to_string(),
                                    args: Some(vec![OscType::Int(1)]),
                                });
                                osc_sender.send((packet, None)).unwrap();

                        controllers.iter_mut().map(|mut c| c.reset());
                        module.clear();
                        page.clear();
                        write!(stdout,
                            "{}{}{}{}Muses xyz for OARC",
                            termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                            color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                            termion::clear::All,
                            termion::cursor::Goto(1, 1))
                                .unwrap();
                        //stdout.flush().unwrap();
                    },
                    // previous module
                    b'o' => {
                        let mut packet = OscPacket::Message(OscMessage {
                                    addr: "/ModulePrev".to_string(),
                                    args: Some(vec![OscType::Int(1)]),
                                });
                                osc_sender.send((packet, None)).unwrap();

                        controllers.iter_mut().map(|mut c| c.reset());
                        module.clear();
                        page.clear();
                        write!(stdout,
                            "{}{}{}{}Muses xyz for OARC",
                            termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                            color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                            termion::clear::All,
                            termion::cursor::Goto(1, 1))
                                .unwrap();
                        
                        //stdout.flush().unwrap();
                    },

                    // next page
                    b'm' => {
                        let mut packet = OscPacket::Message(OscMessage {
                                    addr: "/PageNext".to_string(),
                                    args: Some(vec![OscType::Int(1)]),
                                });
                                osc_sender.send((packet, None)).unwrap();

                        controllers.iter_mut().map(|mut c| c.reset());
                        page.clear();
                        write!(stdout,
                            "{}{}{}{}Muses xyz for OARC",
                            termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                            color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                            termion::clear::All,
                            termion::cursor::Goto(1, 1))
                                .unwrap();
                        //stdout.flush().unwrap();
                    },
                    // previous page
                    b'n' => {
                        let mut packet = OscPacket::Message(OscMessage {
                                    addr: "/PagePrev".to_string(),
                                    args: Some(vec![OscType::Int(1)]),
                                });
                                osc_sender.send((packet, None)).unwrap();

                        controllers.iter_mut().map(|mut c| c.reset());
                        page.clear();
                        write!(stdout,
                            "{}{}{}{}Muses xyz for OARC",
                            termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                            color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                            termion::clear::All,
                            termion::cursor::Goto(1, 1))
                                .unwrap();
                        
                        //stdout.flush().unwrap();
                    },
        
                    _ => {},
                }
        }

        // handle any OSC input
        loop {
            match osc_receiver.try_recv() {
                Ok ((packet, addr)) => {
                    match packet {
                        OscPacket::Message(msg) => {
                            match &msg.addr[..] {
                                "/module" => module = osc_string(&msg),
                                "/page" =>   page = osc_string(&msg),
                                // handle controllers
                                "/P1Desc" => controllers[0].desc = osc_string(&msg),
                                "/P2Desc" => controllers[1].desc = osc_string(&msg),
                                "/P3Desc" => controllers[2].desc = osc_string(&msg),
                                "/P4Desc" => controllers[3].desc = osc_string(&msg),
                                "/P5Desc" => controllers[4].desc = osc_string(&msg),
                                "/P6Desc" => controllers[5].desc = osc_string(&msg),
                                "/P7Desc" => controllers[6].desc = osc_string(&msg),
                                "/P8Desc" => controllers[7].desc = osc_string(&msg),

                                "/P1Value" => controllers[0].value = osc_string(&msg),
                                "/P2Value" => controllers[1].value = osc_string(&msg),
                                "/P3Value" => controllers[2].value = osc_string(&msg),
                                "/P4Value" => controllers[3].value = osc_string(&msg),
                                "/P5Value" => controllers[4].value = osc_string(&msg),
                                "/P6Value" => controllers[5].value = osc_string(&msg),
                                "/P7Value" => controllers[6].value = osc_string(&msg),
                                "/P8Value" => controllers[7].value = osc_string(&msg),

                                "/P1Ctrl" => controllers[0].control = osc_float(&msg),
                                "/P2Ctrl" => controllers[1].control = osc_float(&msg),
                                "/P3Ctrl" => controllers[2].control = osc_float(&msg),
                                "/P4Ctrl" => controllers[3].control = osc_float(&msg),
                                "/P5Ctrl" => controllers[4].control = osc_float(&msg),
                                "/P6Ctrl" => controllers[5].control = osc_float(&msg),
                                "/P7Ctrl" => controllers[6].control = osc_float(&msg),
                                "/P8Ctrl" => controllers[7].control = osc_float(&msg),
                                _ => {
                                }
                            }
                            
                        },
                        OscPacket::Bundle(_) => {
                            error!("{}", "bundle");
                        }
                    }
                },
                _ => {
                    break;
                }
            }
        }

        // display screen
        write!(stdout,
                "{}{}{}{}{}{}",
                termion::color::Bg(termion::color::Rgb(RGB_BACKGROUND.0, RGB_BACKGROUND.1, RGB_BACKGROUND.2)),
                termion::cursor::Goto(MODULE_NAME_XY.0, MODULE_NAME_XY.1),
                color::Fg(color::Rgb(RGB_FOREGROUND.0, RGB_FOREGROUND.1, RGB_FOREGROUND.2)),
                module,
                termion::cursor::Goto(PAGE_NAME_XY.0, PAGE_NAME_XY.1),
                page)
                .unwrap();

        // display controllers
        for c in controllers.iter() {
            c.display(&mut stdout);
        }

        stdout.flush().unwrap();
    }
}