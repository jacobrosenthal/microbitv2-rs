use defmt::{info, unwrap};
use embassy_nrf::gpio::{self, AnyPin};
use embassy_nrf::gpiote::{AnyChannel, InputChannel};
use embedded_hal::digital::v2::OutputPin;
use futures::FutureExt;
use nrf_softdevice::ble::{gatt_server, peripheral, Connection};
use nrf_softdevice::{raw, Softdevice};

// Create the gatt server with however many services we've defined
#[nrf_softdevice::gatt_server]
pub struct Server {
    led: LedService,
    dfu: DfuService,
}

// For over the air updating
#[nrf_softdevice::gatt_service(uuid = "fe59")]
pub struct DfuService {
    #[characteristic(uuid = "8ec90003-f315-4f60-9fb8-838830daea50", write, notify, indicate)]
    dfu: heapless::Vec<u8, 16>,
}

// Define a bluetooth service with one characteristic we can write and read to
#[nrf_softdevice::gatt_service(uuid = "9e7312e0-2354-11eb-9f10-fbc30a62cf38")]
pub struct LedService {
    #[characteristic(
        uuid = "9e7312e0-2354-11eb-9f10-fbc30a63cf38",
        read,
        write,
        notify,
        indicate
    )]
    led: u8,
}

#[embassy::task]
pub async fn bluetooth_task(
    sd: &'static Softdevice,
    button1: InputChannel<'static, AnyChannel, AnyPin>,
    mut led5: gpio::Output<'static, AnyPin>,
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
            ServerEvent::Dfu(DfuServiceEvent::DfuWrite(val)) => dfu_task(sd, &server, &conn, val),
            ServerEvent::Dfu(DfuServiceEvent::DfuCccdWrite { .. }) => {}
            ServerEvent::Led(e) => match e {
                LedServiceEvent::LedWrite(val) => {
                    info!("wrote led: {}", val);
                    if let Err(e) = server.led.led_notify(&conn, val + 1) {
                        info!("send notification error: {:?}", e);
                    }

                    if val > 0 {
                        unwrap!(led5.set_low());
                    } else {
                        unwrap!(led5.set_high());
                    }
                }
                LedServiceEvent::LedCccdWrite {
                    indications,
                    notifications,
                } => {
                    info!(
                        "foo indications: {}, notifications: {}",
                        indications, notifications
                    )
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

pub fn dfu_task(
    _sd: &'static Softdevice,
    server: &Server,
    conn: &Connection,
    val: heapless::Vec<u8, 16>,
) {
    info!("wrote dfu instruction: {}", &val[..]);

    match val[0] {
        DFU_OP_ENTER_BOOTLOADER => {
            // enter DFU mode
            unsafe {
                raw::sd_power_gpregret_clr(0, 0);
                let gpregret_mask = (0xB0 | 0x01) as u32;
                raw::sd_power_gpregret_set(0, gpregret_mask);
            }

            let mut resp: heapless::Vec<u8, 16> = heapless::Vec::new();
            resp.push(DFU_OP_RESPONSE_CODE).unwrap();
            resp.push(DFU_OP_ENTER_BOOTLOADER).unwrap();
            resp.push(DFU_RSP_SUCCESS).unwrap();

            // NOTE that indications are not yet supported but we need one for the nrf connect python app to work
            if let Err(e) = server.dfu.dfu_notify(conn, resp) {
                info!("send notification error: {:?}", e);
            }

            // delay(80000000); // not sure if this is required (1 second delay)

            info!("gpregret_mask set. Soft resetting defice...");
            cortex_m::peripheral::SCB::sys_reset();
        }
        DFU_OP_SET_ADV_NAME => {
            // change advertisement name
            // Security Mode 1 Level 1: No security is needed (aka open link).
            let write_perm = raw::ble_gap_conn_sec_mode_t::new_bitfield_1(1, 1);
            let len = val[1] as usize;
            let dev_name = &val[2..len + 2];

            info!(
                "setting adv name to {}",
                core::str::from_utf8(dev_name).unwrap()
            );

            unsafe {
                raw::sd_ble_gap_device_name_set(
                    &write_perm as *const _ as *const raw::ble_gap_conn_sec_mode_t,
                    dev_name as *const _ as *const u8,
                    len as u16,
                );
            }

            let mut resp: heapless::Vec<u8, 16> = heapless::Vec::new();
            resp.push(DFU_OP_RESPONSE_CODE).unwrap();
            resp.push(DFU_OP_SET_ADV_NAME).unwrap();
            resp.push(DFU_RSP_OP_CODE_NOT_SUPPORTED).unwrap();

            if let Err(e) = server.dfu.dfu_set(resp) {
                info!("set error: {:?}", e);
            }

            let mut resp: heapless::Vec<u8, 16> = heapless::Vec::new();
            resp.push(DFU_OP_RESPONSE_CODE).unwrap();
            resp.push(DFU_OP_SET_ADV_NAME).unwrap();
            resp.push(DFU_RSP_OP_CODE_NOT_SUPPORTED).unwrap();

            if let Err(e) = server.dfu.dfu_notify(conn, resp) {
                info!("send notification error: {:?}", e);
            }

            info!("adv name set successfully");
        }
        _ => {
            // TODO: send error response
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
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
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

const DFU_OP_RESPONSE_CODE: u8 = 0x20;
const DFU_OP_ENTER_BOOTLOADER: u8 = 0x01;
const DFU_OP_SET_ADV_NAME: u8 = 0x02;
const DFU_RSP_SUCCESS: u8 = 0x01;
const _DFU_RSP_BUSY: u8 = 0x06;
const _DFU_RSP_OPERATION_FAILED: u8 = 0x04;
const DFU_RSP_OP_CODE_NOT_SUPPORTED: u8 = 0x02;
