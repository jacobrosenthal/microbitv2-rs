# Rust for microbitv2

The [microbit](https://github.com/nrf-rs/microbit) has a built in second microcontroller just for flashing and receiving print debugging which makes it an ideal candidate for learning embedded Rust.

## Prerequisites

Embassy requires a specific version of nightly. You won't have to do anything though as the rust-toolchain file will use the correct versions when you build.

* Install any dependencies and [probe-run](https://github.com/knurling-rs/probe-run#installation) which provides `cargo run` functionality for microcontrollers

## Running

Now you should be able to run `DEFMT_LOG=trace cargo run --release` to build, run and debug the program.

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
└─ nrf_softdevice::softdevice::{impl#0}::enable @ /home/j/.cargo/git/checkouts/nrf-softdevice-03ef4aef10e777e4/fa369be/nrf-softdevice/src/fmt.rs:138
2 WARN  You're giving more RAM to the softdevice than needed. You can change your app's RAM start address to 2000a280
└─ nrf_softdevice::softdevice::{impl#0}::enable @ /home/j/.cargo/git/checkouts/nrf-softdevice-03ef4aef10e777e4/fa369be/nrf-softdevice/src/fmt.rs:151
3 INFO  Bluetooth is OFF
└─ microbitv2_embassy::ble::bluetooth_task::task::{generator#0}::{closure#2} @ src/ble.rs:43
4 INFO  Press microbit-v2 button 1 to enable, press again to disconnect
└─ microbitv2_embassy::ble::bluetooth_task::task::{generator#0}::{closure#2} @ src/ble.rs:44
```

## Over the air bootloader

Using the bootloader means we can't use our probe-run workflow anymore so you'd probably not use this method until you need it. When that time comes you'll need to flash a secure bootloader and sign your compiled files with a few more dependencies:

* `pip3 install nrfutil`
* `cargo install cargo-make`

There's a private.key (which thus isn't particuarly private but good enough for testing) as well as the corresponding bootloader, with the public key, included in this directory. We can upload the softdevice and the secure bootloader to a connected device with:

* `cargo make first`

With the secure bootloader we can no longer use probe-run but we can still load our code via command line with: (Make sure to increase APP value EVERY time you upload)

* `cargo make --env APP=1 flash`

When you're done testing you can create a package at target/app_dfu_package.zip to use yourself or distribute to your users with:

* `cargo make --env APP=2 pkg`

This signed package can be upload to the device via the [nRF Connect](https://www.nordicsemi.com/Products/Development-tools/nRF-Connect-for-mobile). Connect to the device, and choose the DFU icon, and select your package to upload.

Finally for distributing for real you'll need to generate your own private key (and keep it private) and recreate the bootloader for your new public key.

Note if you want to go back to using probe-run you'll need to go back to prerequisites and reupload just the softdevice which will erase and remove the secure bootloader.

## Troubleshooting

### timed out

If your program uploads but then times out, maybe you lost the preinstalled softdevice off your microbit at some point. We can restore it with

* Install probe-rs-cli `cargo install probe-rs-cli`
* Download [SoftDevice S113](https://www.nordicsemi.com/Software-and-tools/Software/S113/Download) from Nordic. Supported versions are 7.x.x and unzip it to get the .hex file
* `probe-rs-cli download --format hex s113_nrf52_7.2.0_softdevice.hex --chip nRF52833_xxAA --chip-erase`

### Error: no probe was found

On linux in order to interact with the usb device you'll need something like following udev rules saved to somewhere like /etc/udev/rules.d/50-cmsis-dap.rules and then reboot or reload with something like `sudo udevadm control -R`

```bash
# 0d28:0204 DAPLink
SUBSYSTEM=="usb", ATTR{idVendor}=="0d28", ATTR{idProduct}=="0204", MODE:="666"
```

## Advanced

## Generating a private key

* `pip install nrfutil`
* `nrfutil keys generate private.key`
* `nrfutil keys display --key pk --format code private.key --out_file dfu_public_key.c`

We've published this private.key just as an example. Obviously dont publish your private.key but make sure to save it or you can't update the device in the future.

## Building a secure bootloader

Theres a softdevice and bootloader prebuilt and included here, but you may want to customize. This repo uses an s113 and nrf52833 so well use the bootloader for the nrf52833 dev kit

* Download [nrf5 SDK 17](https://www.nordicsemi.com/Products/Development-software/nRF5-SDK/Download#infotabs) and unzip it somewhere
* `cp public_key.c ~/nRF5_SDK_17.0.2_d674dde/examples/dfu/dfu_public_key.c`
* change to that directory, for instance `cd ~/nRF5_SDK_17.0.2_d674dde`
* `git clone https://github.com/kmackay/micro-ecc.git external/micro-ecc/micro-ecc`
* `cd external/micro-ecc/nrf52hf_armgcc/armgcc`
* `make`

* edit `examples/dfu/secure_bootloader/pca10100_s113_ble/config/sdk_config.h` and change these existing defines

```cpp
#define NRF_BL_DFU_ENTER_METHOD_BUTTON 0
#define NRF_BL_DFU_ENTER_METHOD_BUTTONLESS 1
#define NRF_SDH_CLOCK_LF_SRC 0
#define NRF_SDH_CLOCK_LF_RC_CTIV 16
#define NRF_SDH_CLOCK_LF_RC_TEMP_CTIV 2
#define NRF_SDH_CLOCK_LF_ACCURACY 1
```

* `cd ~/nRF5_SDK_17.0.2_d674dde/examples/dfu/secure_bootloader/pca10100_s113_ble/armgcc`
* `make`

That last make will fail because it cant find your gcc compiler

```console
make: /usr/local/gcc-arm-none-eabi-7-2018-q2-update/bin/arm-none-eabi-gcc: No such file or directory
Cannot find: '/usr/local/gcc-arm-none-eabi-7-2018-q2-update/bin/arm-none-eabi-gcc'.
Please set values in: "/home/j/Downloads/nRF5_SDK_17.0.2_d674dde/components/toolchain/gcc/Makefile.posix"
according to the actual configuration of your system.
../../../../../components/toolchain/gcc/Makefile.common:129: *** Cannot continue.  Stop.
```

You might already have it installed

```bash
$ arm-none-eabi-gcc --version
arm-none-eabi-gcc (GNU Arm Embedded Toolchain 9-2020-q2-update) 9.3.1 20200408 (release)
```

If that doesnt work, install [armgcc](https://developer.arm.com/tools-and-software/open-source-software/developer-tools/gnu-toolchain/gnu-rm/downloads) and find your version

And then we can use -print-sysroot will get you close to your path dir, you can strip out the ../arm-none-eabi

```bash
$ arm-none-eabi-gcc -print-sysroot
/usr/share/gcc-arm-none-eabi-9-2020-q2-update/bin/../arm-none-eabi
```

Now place those in the makefile it told you to edit

```make
GNU_INSTALL_ROOT ?= /usr/share/gcc-arm-none-eabi-9-2020-q2-update/bin/
GNU_VERSION ?= 9.3.1
GNU_PREFIX ?= arm-none-eabi
```

now `make` should succeed

```console
Linking target: _build/nrf52833_xxaa_s113.out
   text    data     bss     dec     hex filename
  22572     184   17872   40628    9eb4 _build/nrf52833_xxaa_s113.out
Preparing: _build/nrf52833_xxaa_s113.hex
Preparing: _build/nrf52833_xxaa_s113.bin
DONE nrf52833_xxaa_s113
```

And you can copy `_build/nrf52833_xxaa_s113.hex` into this directory

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
