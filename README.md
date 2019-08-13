
# <span style="color:#F3B73B">Muses Driver</span>

We are in the process of documenting how to build your own Muses xyz controller, please hold tight while we 
finalize the PCB design for the handling the button, encoders, and Sensel/UART processing.

#  <span style="color:#F3B73B">Dependencies</span>

The native Mac OS component, for the status bar application, is written in Swift 5 and for this you will require Xcode, tested against version 10.3. It is likley to work with Swift 4, but this has not been tested. The driver itself calls our to C compatable library, which itself is written in Rust.

The libary component has been tested with [Rust](https://www.rust-lang.org/) 1.36 (and nightly 1.38.x). To install Rust you need simply to install
[Rustup](https://rustup.rs/) and if you already have Rust installed, then you can update with the command ```rustup update```.

The [Sensel API](https://guide.sensel.com/api/) is required and can be installed from Sensel's [Github](https://github.com/sensel/sensel-api), following the instructions. 

#  <span style="color:#F3B73B">Building</span>

There is no direct dependency enforced between the Xcode project and the Rust
library and so before building the Xcode project the library must be explictly
built. Debug and release builds depend respectively on the corresponding Rust
builds so it is recommended that you build both before continuing. 

From the root directory run the following commands:

```bash
cargo build
```

```bash
cargo build --release
```

Now open the Xcode project (in the directory statusBar) and build the application. Assuming it builds successfully, it can now be run directly from Xcode or exported as an application using menu Project/Archive.

#  <span style="color:#F3B73B">Using it</span>

## <span style="color:#F3B73B">Config file</span>

The driver is configured using a JSON file, which by default is placed in the directory ```~/.muses/config.json```. Importantly it currently defines where
the Sensel SVG IR file can be found, the product ID for the Arduino, and 
the from and to IP addresses (with ports) for out going UDP OSC messages.

An example config file looks like the following:

```json
{
    "svg_ir_path" : "/Users/some-user/muses-examples/farm1.json",
    "arduino_pid": 32822,
    "osc_from_addr": "127.0.0.1:8001",
    "osc_to_addr": "127.0.0.1:8338"
}
```

## <span style="color:#F3B73B">Mac Status Bar</span>

# <span style="color:#F3B73B">Library Interface</span>

It is possible to use the Rust

# <span style="color:#F3B73B">TODO</span>

- [X] Sensel Driver
- [X] Test Sensel Driver
- [X] Add ~/$(HOME)/.muses/driver_init.json
- [ ] Get Sensel presets from config
- [ ] Lightpad Driver
- [X] Non status bar command line interface
- [ ] Document library interface
- [ ] Add error return code to connect_rust()

#  <span style="color:#F3B73B">More Information</span>

Parent project

   - [Muses](https://muses-dmi.github.io/).

Tool and documentation for specification of interfaces as SVGs:

   - [SVG Creator tool](https://github.com/muses-dmi/svg-creator).
   - [SVG Interface Documentation](https://github.com/muses-dmi/svg-creator/blob/master/docs/interfaces.md).

Tools for translating SVG Interfaces to the JSON intermidiate representation and different backends:

   - [SVG Interface to IR tool](https://github.com/muses-dmi/svg_interface).
   - [Interface IR to Littlefoot tool](https://github.com/muses-dmi/svg-littlefoot).
   - [SVG Sensel Driver](https://github.com/muses-dmi/sensel_osc).

#  <span style="color:#F3B73B">License</span>

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 * [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/)

at your option.

Dual MIT/Apache2 is strictly more permissive.