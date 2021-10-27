# Rust for microbitv2

The [microbit](https://github.com/nrf-rs/microbit) has a built in second microcontroller just for flashing and receiving print debugging which makes it an ideal candidate for learning embedded Rust.

## Prerequisites

Embassy requires a specific version of nightly. You won't have to do anything though as the rust-toolchain file will use the correct versions when you build.

* Install any dependencies and [probe-run](https://github.com/knurling-rs/probe-run#installation) which provides `cargo run` functionality for microcontrollers

## Running

Now you should be able to run `cargo run --release` to build, run and debug the program.

```console
$ cargo run --release
    Finished release [optimized + debuginfo] target(s) in 0.44s
     Running `probe-run --chip nRF52833_xxAA target/thumbv7em-none-eabi/release/microbitv2-embassy`
(HOST) INFO  flashing program (21.57 KiB)
(HOST) INFO  success!
────────────────────────────────────────────────────────────────────────────────
0 INFO  Hello World!
└─ microbitv2_embassy::__embassy_main::task::{generator#0} @ src/main.rs:31
1 INFO  softdevice RAM: 41600 bytes
└─ nrf_softdevice::softdevice::{impl#0}::enable @ /home/jacob/.cargo/git/checkouts/nrf-softdevice-03ef4aef10e777e4/fa369be/nrf-softdevice/src/fmt.rs:138
2 WARN  You're giving more RAM to the softdevice than needed. You can change your app's RAM start address to 2000a280
└─ nrf_softdevice::softdevice::{impl#0}::enable @ /home/jacob/.cargo/git/checkouts/nrf-softdevice-03ef4aef10e777e4/fa369be/nrf-softdevice/src/fmt.rs:151
3 INFO  Bluetooth is OFF
└─ microbitv2_embassy::ble::bluetooth_task::task::{generator#0}::{closure#2} @ src/ble.rs:43
4 INFO  Press microbit-v2 button 1 to enable, press again to disconnect
└─ microbitv2_embassy::ble::bluetooth_task::task::{generator#0}::{closure#2} @ src/ble.rs:44
```

## Troubleshooting

### timed out

If your program uploads but gives time outs, maybe you lost the preinstalled softdevice off your microbit at some point. We can restore it with

* Install probe-rs-cli `cargo install probe-rs-cli`
* Download [SoftDevice S113](https://www.nordicsemi.com/Software-and-tools/Software/S113/Download) from Nordic. Supported versions are 7.x.x and unzip it to get the .hex file
*`probe-rs-cli download --format hex s140_nrf52_7.2.0_softdevice.hex --chip nRF52833_xxAA`

### Error: no probe was found

On linux in order to interact with the usb device you'll need something like following udev rules saved to somewhere like /etc/udev/rules.d/50-cmsis-dap.rules and then reboot or reload with something like `sudo udevadm control -R`

```bash
# 0d28:0204 DAPLink
SUBSYSTEM=="usb", ATTR{idVendor}=="0d28", ATTR{idProduct}=="0204", MODE:="666"
```

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
