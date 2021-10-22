//! https://tech.microbit.org/hardware/
//!
//! cargo run --release

#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use nrf_softdevice_defmt_rtt as _; // global logger
use panic_probe as _;
mod ble;

use ble::{bluetooth_task, embassy_config, softdevice_config, softdevice_task};
use defmt::{info, unwrap};
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_nrf::gpio::{self, AnyPin, Pin};
use embassy_nrf::gpiote::{self, Channel};
use embassy_nrf::Peripherals;
use embedded_hal::digital::v2::OutputPin;
use nrf_softdevice::Softdevice;

#[embassy::main(config = "embassy_config()")]
async fn main(spawner: Spawner, dp: Peripherals) {
    info!("Hello World!");

    let config = softdevice_config();
    let sd = Softdevice::enable(&config);

    // button presses will be delivered on LotoHi or when you release the button
    let button1 = gpiote::InputChannel::new(
        // degrade just a typesystem hack to forget which pin it is so we can
        // call it Anypin and make our function calls more generic
        dp.GPIOTE_CH1.degrade(),
        gpio::Input::new(dp.P0_14.degrade(), gpio::Pull::Up),
        gpiote::InputChannelPolarity::LoToHi,
    );

    // microbit dosent have a single led, it has a matrix where you set the
    // column high AND row low for the led you want to turn on.

    // row1 permenantly powered
    let _row1 = gpio::Output::new(
        dp.P0_21.degrade(),
        gpio::Level::High,
        gpio::OutputDrive::Standard,
    );

    // The column pins are active low, start leds high (off)
    let red = gpio::Output::new(
        dp.P0_28.degrade(),
        gpio::Level::High,
        gpio::OutputDrive::Standard,
    );

    let red5 = gpio::Output::new(
        dp.P0_30.degrade(),
        gpio::Level::High,
        gpio::OutputDrive::Standard,
    );

    unwrap!(spawner.spawn(softdevice_task(sd)));
    unwrap!(spawner.spawn(bluetooth_task(sd, button1, red5)));
    unwrap!(spawner.spawn(blinky_task(red)));

    // we can sneak another 'task' here as well
    let mut red2 = gpio::Output::new(
        dp.P0_11.degrade(),
        gpio::Level::High,
        gpio::OutputDrive::Standard,
    );

    loop {
        unwrap!(red2.set_low());
        Timer::after(Duration::from_millis(1000)).await;
        unwrap!(red2.set_high());
        Timer::after(Duration::from_millis(1000)).await;
    }
}

#[embassy::task]
async fn blinky_task(mut red: gpio::Output<'static, AnyPin>) {
    loop {
        unwrap!(red.set_high());
        Timer::after(Duration::from_millis(1000)).await;
        unwrap!(red.set_low());
        Timer::after(Duration::from_millis(1000)).await;
    }
}

// WARNING may overflow and wrap-around in long lived apps
defmt::timestamp! {"{=usize}", {
        use core::sync::atomic::{AtomicUsize, Ordering};

        static COUNT: AtomicUsize = AtomicUsize::new(0);
        // NOTE(no-CAS) `timestamps` runs with interrupts disabled
        let n = COUNT.load(Ordering::Relaxed);
        COUNT.store(n + 1, Ordering::Relaxed);
        n
    }
}
