# Rust for microbitv2

The [microbit](https://github.com/nrf-rs/microbit) has a built in bootloader

## Prerequisites

Embassy requires a specific version of nightly. You won't have to do anything though as the rust-toolchain file will use the correct versions when you build.

* Add the target added to your rust toolchain `rustup target add thumbv7em-none-eabi`
* Install any dependencies and [cargo-flash](https://github.com/probe-rs/cargo-flash#prerequisites)
* Install probe-rs-cli `cargo install probe-rs-cli`
* On linux you need the following udev rules saved to somewhere like /etc/udev/rules.d/50-cmsis-dap.rules and then reload your udev rules with something like `sudo udevadm control -R`

Finally the microbit comes with a different softdevice than we want
* Download [SoftDevice S140](https://www.nordicsemi.com/Software-and-tools/Software/S140/Download) from Nordic. Supported versions are 7.x.x
* *`probe-rs-cli download --format hex s140_nrf52_7.2.0_softdevice.hex --chip nRF52833_xxAA`

```bash
# 0d28:0204 DAPLink
SUBSYSTEM=="usb", ATTR{idVendor}=="0d28", ATTR{idProduct}=="0204", MODE:="666"
```

## Debugging with probe-run

For testing and programming while connected to usb, you can get nice logging back from the device from probe-run by calling:
`cargo run --release`

## Flashing

When you want to flash the chip so it can work stand alone we use cargo-flash with:
`cargo flash --release --chip nRF52833_xxAA`

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
