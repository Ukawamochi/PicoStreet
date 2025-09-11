//! BLE Host 初期化と広告/スキャンの時間多重ユーティリティ

use core::sync::atomic::{AtomicU8, Ordering};

use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use embassy_futures::join::join;


use trouble_host::advertise::{AdStructure, Advertisement, AdvertisementParameters};
use trouble_host::prelude::*;

use pico_w_id_beacon::adv_payload::{build_adv_payload, parse_service_data};
use pico_w_id_beacon::format::fmt_bytes_colon;
use pico_w_id_beacon::constants::SERVICE_UUID_16;

static RX_PULSES: AtomicU8 = AtomicU8::new(0);

struct RxHandler {
    self_bd_addr: [u8; 6],
}

impl EventHandler for RxHandler {
    fn on_adv_reports(&self, mut it: trouble_host::scan::LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            let data = report.data;
            if let Some(parsed) = parse_service_data(data) {
                // 自分自身のBD_ADDRの場合は「SELF RX」としてログする（LEDは点滅させない）
                if parsed.bd_addr == self.self_bd_addr {
                    let s = fmt_bytes_colon(&parsed.bd_addr);
                    info!("自分の信号受信 bd_addr={} rssi={}", s.as_str(), report.rssi);
                    continue;
                }
                
                let s = fmt_bytes_colon(&parsed.bd_addr);
                info!("他デバイス検出 bd_addr={} rssi={}", s.as_str(), report.rssi);
                // 現在時刻（NTP未同期時は0）
                let now = crate::timekeeper::now_unix().unwrap_or(0);
                let _ = crate::storage::save_encounter(parsed.bd_addr, now, report.rssi);
                let v = RX_PULSES.load(Ordering::Relaxed);
                RX_PULSES.store(v.saturating_add(1), Ordering::Relaxed);
            }
        }
    }

    fn on_ext_adv_reports(&self, mut it: trouble_host::scan::LeExtAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            let data = report.data;
            if let Some(parsed) = parse_service_data(data) {
                // 自分自身のBD_ADDRの場合は「SELF RX」としてログする（LEDは点滅させない）
                if parsed.bd_addr == self.self_bd_addr {
                    let s = fmt_bytes_colon(&parsed.bd_addr);
                    info!("自分の信号受信(拡張) bd_addr={} rssi={}", s.as_str(), report.rssi);
                    continue;
                }
                
                let s = fmt_bytes_colon(&parsed.bd_addr);
                info!("他デバイス検出(拡張) bd_addr={} rssi={}", s.as_str(), report.rssi);
                let now = crate::timekeeper::now_unix().unwrap_or(0);
                let _ = crate::storage::save_encounter(parsed.bd_addr, now, report.rssi);
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
    // NOTE: Service Data に UUID を含めるため、重複する Complete 16-bit UUIDs は省略
    // 省略により全体 31B 制約内に収める（Flags 3B + ServiceData(4B+payload)）
    // Service Data (0x16)
    used += AdStructure::encode_slice(&[AdStructure::ServiceData16 { uuid: uuid16, data: payload }], &mut buf[used..]).unwrap();
    &buf[..used]
}

/// BLE Host を生成し、TXフェーズ → RXフェーズを繰り返す。
/// - TXフェーズ: Service Data に簡素化PicoStreetペイロードを格納して広告
/// - RXフェーズ: スキャンして見つかったら RX LED を点滅
pub async fn advertise_and_scan_loop<C>(
    controller: C,
    control: &mut cyw43::Control<'_>,
    self_bd_addr: [u8; 6],
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
    info!("BLEアドレス = {:?}", address);

    // Host 準備
    let mut resources: HostResources<DefaultPacketPool, 1, 1> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);

    let Host { mut peripheral, central, mut runner, .. } = stack.build();
    let mut scanner = Scanner::new(central);
    let handler = RxHandler { self_bd_addr };

    // バッファ
    let mut adv_payload = [0u8; 8];
    let payload_len = build_adv_payload(&mut adv_payload, &self_bd_addr);
    let payload = &adv_payload[..payload_len];
    let bd_str = fmt_bytes_colon(&self_bd_addr);
    info!("送信ペイロード構築 len={} bd_addr={}", payload_len, bd_str.as_str());
    let mut ad_buf = [0u8; 31];

    let _ = join(runner.run_with_handler(&handler), async {
        // 広告をEnable維持
        let ad = build_advertisement_data(&mut ad_buf, payload);
        info!("BLE送信開始 len={}", ad.len());
        let mut params = AdvertisementParameters::default();
        // 送信頻度: 3秒に1回（min/maxともに3秒）
        params.interval_min = Duration::from_millis(3000);
        params.interval_max = Duration::from_millis(3000);
        let _advertiser = match peripheral
            .advertise(
                &params,
                Advertisement::NonconnectableNonscannableUndirected { adv_data: ad },
            )
            .await
        {
            Ok(h) => h,
            Err(_) => {
                info!("advertise() failed; entering error blink loop");
                crate::leds::error_blink_loop(control).await;
            }
        };

        // スキャン再始動ポンプと送信インジケータのパルスを並列実行
        let scan_pump = async {
            let mut cfg = ScanConfig::default();
            cfg.active = false; // passive
            cfg.interval = Duration::from_millis(200);
            cfg.window = Duration::from_millis(150);
            cfg.timeout = Duration::from_millis(0);
            let mut last_pulse = Instant::now();
            loop {
                // 保存件数が 1000 件を超えたらエラーブリンク
                if crate::storage::total_saved() > 1000 {
                    crate::leds::error_blink_loop(control).await;
                }
                let session = match scanner.scan(&cfg).await {
                    Ok(s) => s,
                    Err(_) => {
                        info!("scan() failed; entering error blink loop");
                        crate::leds::error_blink_loop(control).await;
                    }
                };
                // イベント処理に譲る
                Timer::after(Duration::from_millis(20)).await;
                core::mem::drop(session);
                // 過剰なHCIを避けるため小休止
                Timer::after(Duration::from_millis(5)).await;
                // 受信インジケータ（高速点滅）
                if RX_PULSES.load(Ordering::Relaxed) > 0 {
                    let v = RX_PULSES.load(Ordering::Relaxed);
                    if v > 0 { RX_PULSES.store(v - 1, Ordering::Relaxed); }
                    crate::leds::blink_rx_fast(control).await;
                }
                // 送信インジケータ（100ms点灯を1秒周期）
                if Instant::now() - last_pulse >= Duration::from_millis(1000) {
                    crate::leds::blink_tx(control, 100).await;
                    last_pulse = Instant::now();
                }
            }
        };
        scan_pump.await;
    }).await;

    // 終了しない
    loop { Timer::after(Duration::from_secs(1)).await; }
}
