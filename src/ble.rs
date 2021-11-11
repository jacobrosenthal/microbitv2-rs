use defmt::{info, unwrap};
use embassy::util::Steal;
use embassy_nrf::gpio::{self, AnyPin, Output, Pin};
use embedded_hal::digital::v2::OutputPin;
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::{raw, Softdevice};

// Define a bluetooth service with one characteristic we can write and read to
#[nrf_softdevice::gatt_service(uuid = "bada5555-e91f-1337-a49b-8675309fb099")]
struct BleIo {
    // todo what max, notifications
    #[characteristic(uuid = "2a56", read, write)]
    digital: heapless::Vec<u8, 16>,
}

// Create the gatt server with however many services we've defined
#[nrf_softdevice::gatt_server]
struct Server {
    ble_io: BleIo,
}

#[embassy::task]
pub async fn bluetooth_task(sd: &'static Softdevice) {
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

    // dont want to pass 100 things to the function, just steal what we need and
    // promise not to fuck up
    let dp = unsafe { embassy_nrf::Peripherals::steal() };

    let config = peripheral::Config::default();

    // https://tech.microbit.org/hardware/schematic/
    let p0 = gpio::Output::new(
        dp.P0_02.degrade(),
        gpio::Level::Low,
        gpio::OutputDrive::Standard,
    );

    let p1 = gpio::Output::new(
        dp.P0_03.degrade(),
        gpio::Level::Low,
        gpio::OutputDrive::Standard,
    );

    let p2 = gpio::Output::new(
        dp.P0_04.degrade(),
        gpio::Level::Low,
        gpio::OutputDrive::Standard,
    );

    let mut pins: [Output<AnyPin>; 3] = [p0, p1, p2];

    loop {
        info!("advertising!");

        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data,
            scan_data,
        };

        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        let res = gatt_server::run(&conn, &server, |e| match e {
            ServerEvent::BleIo(e) => match e {
                BleIoEvent::DigitalWrite(val) => {
                    val.array_chunks::<2>().for_each(|a| {
                        // u8 fits in usize always
                        let p = a[0] as usize;

                        // look up the pin the user asked for in our pins array
                        if let Some(pin) = pins.iter_mut().nth(p) {
                            let val = a[1];

                            if val > 0 {
                                info!("setting pin {} high", p);
                                unwrap!(pin.set_high());
                            } else {
                                info!("setting pin {} low", p);
                                unwrap!(pin.set_low());
                            }
                        }
                    });
                }
            },
        })
        .await;

        if let Err(e) = res {
            info!("error {}", e)
        }
    }
}

// This task is an implementation detail of the softdevice. It services the
// softdevice under the hood which ultimately feeds events to our
// gatt_server::run in the blutooth task
#[embassy::task]
pub async fn softdevice_task(sd: &'static Softdevice) {
    sd.run().await;
}

// This function is an implementation detail of the softdevice. It configures
// the underlying softdevice with all the bluetooth settings and buffer sizes.
pub fn softdevice_config() -> nrf_softdevice::Config {
    nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 16,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 1,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 1,
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
