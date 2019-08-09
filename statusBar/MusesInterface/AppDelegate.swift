//
//  AppDelegate.swift
//  MusesInterface
//
//  Created by Benedict Gaster on 08/08/2019.
//  Copyright © 2019 Benedict Gaster. All rights reserved.
//

import Cocoa

@NSApplicationMain
class AppDelegate: NSObject, NSApplicationDelegate {

    let statusBarItem: NSStatusItem = NSStatusBar.system.statusItem(withLength:NSStatusItem.squareLength)
    var connected: Bool = false

    func applicationDidFinishLaunching(_ aNotification: Notification) {
        
        if let statusButton = statusBarItem.button {
            statusButton.image = NSImage(named:NSImage.Name("StatusBarButtonImage"))
            statusButton.action = #selector(nothing(_:))
        }
        
        // initialize the Rust Muses instrument library
        init_rust()
        connectMenu()
    }
    
    @objc func nothing(_ sender: Any?) {
       
    }
    
    @objc func connect(_ sender: Any?) {
        if !connected {
            let quoteText = "Connecting to Muses controller"
            let quoteAuthor = "CSRC"
        
            // connect to the Muses instrument via the Rust library
            connect_rust()
            
            connected = true
        
            // setup disconnect menu
            disconnectMenu()
        
            print("\(quoteText) — \(quoteAuthor)")
        }
    }
    
    @objc func disconnect(_ sender: Any?) {
        if connected {
            let quoteText = "Disconnecting from Muses controller"
            let quoteAuthor = "CSRC"
            
            // disconnect to the Muses instrument via the Rust library
            disconnect_rust();
            
            connected = false
            connectMenu()
            
            print("\(quoteText) — \(quoteAuthor)")
        }
    }

    func applicationWillTerminate(_ aNotification: Notification) {
        // if device connected, then close down before quiting
        if connected {
            
        }
    }

    // menu to be displayed when device connected
    func disconnectMenu() {
        let menu = NSMenu()
        
        menu.addItem(NSMenuItem(title: "Disconnect", action: #selector(AppDelegate.disconnect(_:)), keyEquivalent: "d"))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Quit Muses Connect", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        
        statusBarItem.menu = menu
    }
    
    // menu to be displayed when device not connected
    func connectMenu() {
        let menu = NSMenu()
        
        menu.addItem(NSMenuItem(title: "Connect", action: #selector(AppDelegate.connect(_:)), keyEquivalent: "c"))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Quit Muses Connect", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        
        statusBarItem.menu = menu
    }
}

