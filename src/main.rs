//! https://tech.microbit.org/hardware/
//!
//! Blinks 2 leds from two different tasks, and at the same time waits for a
//! button press to advertise a bluetooth led service. If you connect to
//! bluetooth and write a u8 > 0 it will enable the third led, if you write 0 it
//! will disable it.
//!
//! cargo run --release

#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use nrf_softdevice_defmt_rtt as _; // global logger
use panic_probe as _; // print out panic messages
mod ble;

use ble::{bluetooth_task, softdevice_config, softdevice_task};
use defmt::{info, unwrap};
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_nrf::gpio::{self, AnyPin, Pin};
use embassy_nrf::gpiote::{self, Channel};
use embassy_nrf::{interrupt, Peripherals};
use embedded_hal::digital::v2::OutputPin;
use nrf_softdevice::Softdevice;

#[embassy::main(config = "embassy_config()")]
async fn main(spawner: Spawner, dp: Peripherals) {
    // well use these logging macros instead of println to tunnel our logs via the debug chip
    info!("Hello World!");

    // some bluetooth under the covers stuff we need to start up
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

    // tell the executor to start each of our tasks
    unwrap!(spawner.spawn(softdevice_task(sd)));
    // note this unwrap! macro is just like .unwrap() you're used to, but for
    // various reasons has less size for microcontrollers
    unwrap!(spawner.spawn(bluetooth_task(sd, button1, red5)));
    unwrap!(spawner.spawn(blinky_task(red)));

    // we can sneak another 'task' here as well instead of exiting
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

// Configure clocks and interrupt priorities for our microcontroller
// 0 is Highest. Lower prio number can preempt higher prio number
// Softdevice has reserved priorities 0, 1 and 3 so we avoid those
pub fn embassy_config() -> embassy_nrf::config::Config {
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    config.lfclk_source = embassy_nrf::config::LfclkSource::InternalRC;
    config.time_interrupt_priority = interrupt::Priority::P2;
    // if we see button misses lower this
    config.gpiote_interrupt_priority = interrupt::Priority::P7;
    config
}

// Just a bookkeeping function for our logging library
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
