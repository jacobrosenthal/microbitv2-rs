//! cargo run --release

#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use defmt::{info, unwrap};
use nrf_softdevice_defmt_rtt as _; // global logger
use panic_probe as _;

use embassy::executor::Spawner;
use embassy_nrf::config::{Config, HfclkSource, LfclkSource};
use embassy_nrf::gpio::{self, AnyPin, Pin};
use embassy_nrf::gpiote::{self, AnyChannel, Channel, InputChannel};
use embassy_nrf::{interrupt, Peripherals};
use futures::FutureExt;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::{raw, Softdevice};

// define a bluetooth service with one characteristic we can write and read to
#[nrf_softdevice::gatt_service(uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf38")]
struct FooService {
    #[characteristic(uuid = "9e7312e0-2354-11eb-9f10-fbc30a63cf38", read, write)]
    foo: u16,
}

#[nrf_softdevice::gatt_server]
struct Server {
    foo: FooService,
}

#[embassy::main(config = "embassy_config()")]
async fn main(spawner: Spawner, dp: Peripherals) {
    info!("Hello World!");

    let config = softdevice_config();
    let sd = Softdevice::enable(&config);

    let button1 = gpiote::InputChannel::new(
        dp.GPIOTE_CH1.degrade(),
        gpio::Input::new(dp.P0_14.degrade(), gpio::Pull::Up),
        gpiote::InputChannelPolarity::HiToLo,
    );

    unwrap!(spawner.spawn(softdevice_task(sd)));
    unwrap!(spawner.spawn(bluetooth_task(sd, button1)));
}

#[embassy::task]
pub async fn bluetooth_task(
    sd: &'static Softdevice,
    button1: InputChannel<'static, AnyChannel, AnyPin>,
) {
    let server: Server = unwrap!(gatt_server::register(sd));

    #[rustfmt::skip]
    let adv_data = &[
        0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        0x03, 0x03, 0x09, 0x18,
        0x0a, 0x09, b'H', b'e', b'l', b'l', b'o', b'R', b'u', b's', b't',
    ];
    #[rustfmt::skip]
    let scan_data = &[
        0x03, 0x03, 0x09, 0x18,
    ];

    let config = peripheral::Config::default();

    info!("Bluetooth is OFF");
    info!("Press microbit-v2 button 1 to enable, press again to disconnect");

    'waiting: loop {
        // wait here until button is pressed
        button1.wait().await;

        info!("advertising!");

        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data,
            scan_data,
        };

        let conn_future = peripheral::advertise_connectable(sd, adv, &config);

        // instead of await to run one future, well select to run both futures until first one returns
        let conn = futures::select_biased! {
            // connection returns if somebody connects
            conn = conn_future.fuse() => unwrap!(conn),
            // button returns if pressed and well go back to top of loop
            _ = button1.wait().fuse() => {info!("stopping"); continue 'waiting;},
        };

        let gatt_future = gatt_server::run(&conn, &server, |e| match e {
            ServerEvent::Foo(e) => match e {
                FooServiceEvent::FooWrite(val) => {
                    info!("wrote foo: {}", val);
                }
            },
        });

        // instead of await to run one future, well select to run both futures until first one returns
        futures::select_biased! {
            // gatt returns if connection drops
            r = gatt_future.fuse() => info!("disconnected {}", r),
            // button returns if pressed
            _ = button1.wait().fuse() => info!("disconnecting"),
        };
    }
}

#[embassy::task]
async fn softdevice_task(sd: &'static Softdevice) {
    sd.run().await;
}

pub fn softdevice_config() -> nrf_softdevice::Config {
    nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 4,
            rc_temp_ctiv: 2,
            accuracy: 7,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 6,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 3,
            central_role_count: 3,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"HelloRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    }
}

// 0 is Highest. Lower prio number can preempt higher prio number
// Softdevice has reserved priorities 0, 1 and 3
pub fn embassy_config() -> Config {
    let mut config = Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    config.lfclk_source = LfclkSource::ExternalXtal;
    // any reason not to run our timer as highest priority?
    config.time_interrupt_priority = interrupt::Priority::P2;
    // if we see button misses lower this
    config.gpiote_interrupt_priority = interrupt::Priority::P7;
    config
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
