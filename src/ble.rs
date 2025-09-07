//! BLE Host 初期化と広告/スキャンの時間多重ユーティリティ

use core::sync::atomic::{AtomicU8, Ordering};

use defmt::info;
use embassy_time::{Duration, Timer};
use embassy_futures::join::join;

use embassy_rp::gpio::Output;

use trouble_host::advertise::{AdStructure, Advertisement, AdvertisementParameters};
use trouble_host::prelude::*;

use pico_w_id_beacon::adv_payload::{build_adv_payload, parse_service_data};
use pico_w_id_beacon::constants::SERVICE_UUID_16;

static RX_PULSES: AtomicU8 = AtomicU8::new(0);

struct RxHandler;

impl EventHandler for RxHandler {
    fn on_adv_reports(&self, mut it: trouble_host::scan::LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            let data = report.data;
            // Service Data の中身を検査
            // ここでは AD 全体のパーサを使いたいので、
            // report.data をそのまま渡す
            if let Some(parsed) = parse_service_data(data) {
                info!("RX CONTACT_ID={:x}", &parsed.contact_id);
                // パルス要求を加算
                let v = RX_PULSES.load(Ordering::Relaxed);
                RX_PULSES.store(v.saturating_add(1), Ordering::Relaxed);
            }
        }
    }

    fn on_ext_adv_reports(&self, mut it: trouble_host::scan::LeExtAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            let data = report.data;
            if let Some(parsed) = parse_service_data(data) {
                info!("RX(ext) CONTACT_ID={:x}", &parsed.contact_id);
                let v = RX_PULSES.load(Ordering::Relaxed);
                RX_PULSES.store(v.saturating_add(1), Ordering::Relaxed);
            }
        }
    }
}

/// 広告用の AD を構築（Flags, Complete 16-bit UUIDs, Service Data）
fn build_advertisement_data<'a>(buf: &'a mut [u8], payload: &'a [u8]) -> &'a [u8] {
    // SERVICE_UUID_16 を LE エンディアンで
    let uuid16 = [(SERVICE_UUID_16 & 0xff) as u8, (SERVICE_UUID_16 >> 8) as u8];
    let mut used = 0usize;
    // Flags (0x01) 0x06
    used += AdStructure::encode_slice(&[AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED)], &mut buf[used..]).unwrap();
    // Complete List of 16-bit UUIDs (0x03)
    used += AdStructure::encode_slice(&[AdStructure::ServiceUuids16(&[uuid16])], &mut buf[used..]).unwrap();
    // Service Data (0x16)
    used += AdStructure::encode_slice(&[AdStructure::ServiceData16 { uuid: uuid16, data: payload }], &mut buf[used..]).unwrap();
    &buf[..used]
}

/// BLE Host を生成し、1秒広告 → 1.5秒スキャンを繰り返す。
/// - 広告: Service Data に TLV フレームを格納
/// - スキャン: 見つかったら RX LED を点滅
pub async fn advertise_and_scan_loop<C>(
    controller: C,
    control: &mut cyw43::Control<'_>,
    rx_led: &mut Output<'_>,
) -> !
where
    C: Controller
        + bt_hci::controller::ControllerCmdSync<bt_hci::cmd::le::LeSetScanParams>
        + bt_hci::controller::ControllerCmdSync<bt_hci::cmd::le::LeSetScanEnable>
        + bt_hci::controller::ControllerCmdSync<bt_hci::cmd::le::LeClearFilterAcceptList>
        + bt_hci::controller::ControllerCmdSync<bt_hci::cmd::le::LeAddDeviceToFilterAcceptList>,
{
    // ランダムアドレス（PoC固定値）
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("BLE address = {:?}", address);

    // Host 準備
    let mut resources: HostResources<DefaultPacketPool, 1, 1> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);

    let Host { mut peripheral, central, mut runner, .. } = stack.build();
    let mut scanner = Scanner::new(central);
    let handler = RxHandler;

    // バッファ
    let mut adv_payload = [0u8; 31];
    let payload_len = build_adv_payload(&mut adv_payload);
    let payload = &adv_payload[..payload_len];
    info!("Built TX payload len={} CONTACT_ID={:x}", payload_len, &pico_w_id_beacon::constants::CONTACT_ID);
    let mut ad_buf = [0u8; 31];

    let _ = join(runner.run_with_handler(&handler), async {
        loop {
            // 送信フェーズ（1秒）: フェーズ中は内蔵LEDを点灯
            info!("TX phase start");
            control.gpio_set(0, true).await;
            let ad = build_advertisement_data(&mut ad_buf, payload);
            info!("TX adv_len={} ad={:x}", ad.len(), ad);
            let mut params = AdvertisementParameters::default();
            // 250ms間隔程度（仕様の目安）
            params.interval_min = Duration::from_millis(250);
            params.interval_max = Duration::from_millis(250);
            let _advertiser = peripheral
                .advertise(
                    &params,
                    Advertisement::NonconnectableNonscannableUndirected { adv_data: ad },
                )
                .await
                .unwrap();
            Timer::after(Duration::from_millis(1000)).await;
            drop(_advertiser);
            info!("TX phase end");
            control.gpio_set(0, false).await;

            // 受信フェーズ（1.5秒）
            info!("Scan phase start");
            let mut cfg = ScanConfig::default();
            cfg.active = false; // passive
            cfg.interval = Duration::from_millis(200);
            cfg.window = Duration::from_millis(150);
            cfg.timeout = Duration::from_millis(0); // 無限（手動で停止）
            let _session = scanner.scan(&cfg).await.unwrap();

            let start = embassy_time::Instant::now();
            while embassy_time::Instant::now() - start < Duration::from_millis(1500) {
                // パルスがあれば点滅
                if RX_PULSES.load(Ordering::Relaxed) > 0 {
                    let v = RX_PULSES.load(Ordering::Relaxed);
                    if v > 0 { RX_PULSES.store(v - 1, Ordering::Relaxed); }
                    crate::leds::blink_rx(rx_led, 120).await;
                } else {
                    // 軽く待つ
                    Timer::after(Duration::from_millis(20)).await;
                }
            }
            // _session drop -> scan 停止
            core::mem::drop(_session);
            info!("Scan phase end");
        }
    }).await;

    // 終了しない
    loop { Timer::after(Duration::from_secs(1)).await; }
}
