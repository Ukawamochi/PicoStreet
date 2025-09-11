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
        cyw43_pio::DEFAULT_CLOCK_DIVIDER,
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
    control.init(clm).await;
    // 初期状態の内蔵LED消灯
    control.gpio_set(0, false).await;
    info!("CYW43 initialized");
    
    // 自デバイス BD_ADDR 取得（取得失敗時はエラーインジケータを繰り返す）
    let self_bd_addr = device_id::get_bd_addr(&mut control).await;
    if self_bd_addr == [0u8; 6] {
        warn!("Failed to obtain BD_ADDR; entering error blink loop");
        leds::error_blink_loop(&mut control).await;
    }
    let bd_str = fmt_bytes_colon(&self_bd_addr);
    info!("SELF bd_addr={}", bd_str.as_str());

    // 起動確認: 内蔵LEDを短く点滅（3回）
    for _ in 0..3 {
        control.gpio_set(0, true).await;
        embassy_time::Timer::after_millis(150).await;
        control.gpio_set(0, false).await;
        embassy_time::Timer::after_millis(150).await;
    }

    // BLE Host に接続
    let controller: ExternalController<_, 10> = ExternalController::new(bt_device);

    // WiFi接続を開始しつつ、BLE初期化と並行実行
    // （制御LEDの独占を避けるため、WiFi完了後にBLEのLED制御を開始）
    info!("Starting WiFi connect while BLE initializes...");
    let _ = embassy_futures::join::join(
        wifi::connect_and_test(spawner, &mut control, net_device),
        async {
            info!("BLE host/controller wired");
        },
    ).await;

    // 時分割ループ開始（広告→スキャン）
    ble::advertise_and_scan_loop(controller, &mut control, self_bd_addr).await;
}
