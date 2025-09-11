#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;
use trouble_host::prelude::ExternalController;
use {defmt_rtt as _, embassy_time as _, panic_probe as _};

//BLEとLEDを別モジュールで制御
mod ble;
mod leds;
mod wifi;
use pico_w_id_beacon::device_id;
use pico_w_id_beacon::format::fmt_bytes_colon;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, cyw43_pio::PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Booting PicoStreet X交換 キーホルダー");

    let p = embassy_rp::init(Default::default());

    #[cfg(feature = "skip-cyw43-firmware")]
    let (fw, clm, btfw) = (&[], &[], &[]);

    #[cfg(not(feature = "skip-cyw43-firmware"))]
    let (fw, clm, btfw) = {
        // cyw43-firmware ディレクトリに配置されたファームウェア
        let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
        let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
        let btfw = include_bytes!("../cyw43-firmware/43439A0_btfw.bin");
        (fw, clm, btfw)
    };

    // CYW43 バス初期化
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = cyw43_pio::PioSpi::new(
        &mut pio.common,
        pio.sm0,
        cyw43_pio::DEFAULT_CLOCK_DIVIDER * 2, // クロック分周を2倍にして通信を安定化
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, bt_device, mut control, runner) = cyw43::new_with_bluetooth(state, pwr, spi, fw, btfw).await;
    spawner.spawn(cyw43_task(runner)).unwrap();
    
    // CYW43の安定化を待つ
    embassy_time::Timer::after_millis(100).await;
    
    control.init(clm).await;
    
    // 初期化後の安定化を待つ
    embassy_time::Timer::after_millis(50).await;
    
    // 初期状態の内蔵LED消灯
    control.gpio_set(0, false).await;
    info!("CYW43チップ初期化完了");
    
    // 自デバイス BD_ADDR 取得（取得失敗時はエラーインジケータを繰り返す）
    let self_bd_addr = device_id::get_bd_addr(&mut control).await;
    if self_bd_addr == [0u8; 6] {
        warn!("BD_ADDR取得失敗: エラー点滅モードに移行");
        leds::error_blink_loop(&mut control).await;
    }
    let bd_str = fmt_bytes_colon(&self_bd_addr);
    info!("自分のBD_ADDR={}", bd_str.as_str());

    // 起動確認: 内蔵LEDを短く点滅（3回）
    for _ in 0..3 {
        control.gpio_set(0, true).await;
        embassy_time::Timer::after_millis(150).await;
        control.gpio_set(0, false).await;
        embassy_time::Timer::after_millis(150).await;
    }

    // BLE Host に接続
    let controller: ExternalController<_, 10> = ExternalController::new(bt_device);

    // まずWiFiへ接続し、DHCP完了後にNTP同期（BLEより優先、ただし短時間で完了）
    info!("WiFi接続とNTP同期を開始...");
    let stack = match wifi::maintain_wifi_connection(spawner, &mut control, net_device).await {
        Ok(s) => s,
        Err(e) => {
            warn!("WiFi初期化に失敗しました: {}", e);
            // WiFi無しで継続（BLE優先）
            ble::advertise_and_scan_loop(controller, &mut control, self_bd_addr).await;
        }
    };

    // NTP時刻同期（失敗しても続行）
    if let Err(e) = wifi::sync_ntp_time(stack).await {
        warn!("NTP同期に失敗: {}", e);
    }

    info!("WiFi/NTP完了。BLE機能を開始します...");
    info!("BLEホスト/コントローラ接続完了");

    // 時分割ループ開始（広告→スキャン）
    ble::advertise_and_scan_loop(controller, &mut control, self_bd_addr).await;
}
