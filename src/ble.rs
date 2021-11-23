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
    digital: [u8; 16],
}

// Create the gatt server with however many services we've defined
#[nrf_softdevice::gatt_server]
struct Server {
    ble_io: BleIo,
}

struct ConfiguredOutput<'a> {
    pin_nbr: usize,
    output: Output<'a, AnyPin>,
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

    let mut mapping: [Option<AnyPin>; 21] = [
        Some(dp.P0_02.degrade()),
        Some(dp.P0_03.degrade()),
        Some(dp.P0_04.degrade()),
        Some(dp.P0_31.degrade()),
        Some(dp.P0_28.degrade()),
        Some(dp.P0_14.degrade()),
        Some(dp.P1_05.degrade()),
        Some(dp.P0_11.degrade()),
        Some(dp.P0_10.degrade()),
        Some(dp.P0_09.degrade()),
        Some(dp.P0_30.degrade()),
        Some(dp.P0_23.degrade()),
        Some(dp.P0_12.degrade()),
        Some(dp.P0_17.degrade()),
        Some(dp.P0_01.degrade()),
        Some(dp.P0_13.degrade()),
        Some(dp.P1_02.degrade()),
        None,
        None,
        Some(dp.P0_26.degrade()),
        Some(dp.P1_00.degrade()),
    ];

    let mut digitals: heapless::Vec<ConfiguredOutput, 21> = heapless::Vec::new();

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
                        let requested_pin = a[0] as usize;

                        // did the user supply a rational pin number?
                        if let Some(possible_pin) = mapping.iter_mut().nth(requested_pin) {
                            // pin configured yet, so its still in the mapping array
                            if let Some(pin) = possible_pin.take() {
                                let value = if a[1] > 0 {
                                    gpio::Level::High
                                } else {
                                    gpio::Level::Low
                                };

                                let output =
                                    gpio::Output::new(pin, value, gpio::OutputDrive::Standard);
                                // how to index it back? put it at the nth position?
                                let _ = digitals.push(ConfiguredOutput {
                                    pin_nbr: requested_pin,
                                    output,
                                });
                            }
                            // else its already configured as a digital? how to check if its configured as something else
                            else {
                                digitals
                                    .iter_mut()
                                    .filter(|a| a.pin_nbr == requested_pin)
                                    .for_each(|configured| {
                                        if a[1] > 0 {
                                            unwrap!(configured.output.set_high());
                                        } else {
                                            unwrap!(configured.output.set_high());
                                        }
                                    })
                            }
                        } else {
                            // bad pin nbr
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
