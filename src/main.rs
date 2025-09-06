#![no_std]
#![no_main]

// 日本語ログ出力（RTT経由）
use defmt_rtt as _;
// 日本語パニックハンドラ（defmt連携）
use panic_probe as _;

// BOOT2 は `embassy-rp` 側の機能フラグ（boot2-XXX）で自動配置します

// Embassy 実行環境／時間ドライバ
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

// CYW43（Pico W内蔵LED/無線）制御に必要なクレート
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;

// メイン関数をEmbassy対応（非同期）に変更
// PIO割り込みハンドラをバインド（CYW43用のPIO-SPIで使用）
bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// CYW43のランナーを別タスクで常駐させる
#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // 初期化：RP2040周辺をデフォルト設定で初期化（クロック等）
    let p = embassy_rp::init(Default::default());
    // 重要: println はコンパイル時フィルタの影響を受けにくく、環境依存を減らす
    defmt::println!("初期化完了: Embassy ランタイム起動");

    // ファームウェア（FW/CLM）はリポジトリ直下の `cyw43-firmware/` に配置してください
    // 例: 43439A0.bin, 43439A0_clm.bin（Pico W同梱チップ用）
    let fw: &'static [u8] = include_bytes!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/cyw43-firmware/43439A0.bin")
    );
    let clm: &'static [u8] = include_bytes!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/cyw43-firmware/43439A0_clm.bin")
    );

    // CYW43接続: 電源制御/CSはGPIO、データはPIO-SPI + DMAを使用
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    defmt::println!("PIO/PIO-SPI 初期化");
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    // CYW43ドライバの状態を静的領域に確保
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());

    // ドライバ生成（ネットワークは未使用、LED制御に必要）
    defmt::println!("CYW43 ドライバ生成");
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    // ランナーをバックグラウンドで起動
    defmt::println!("CYW43 ランナー起動");
    spawner.spawn(cyw43_task(runner)).expect("spawn cyw43");

    // 初期化と省電力設定
    defmt::println!("CYW43 初期化開始");
    control.init(clm).await;
    defmt::println!("CYW43 初期化完了");
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // 内蔵LEDを1秒周期で点滅
    let delay = Duration::from_secs(1);
    loop {
        defmt::println!("LED: ON");
        control.gpio_set(0, true).await; // 0番ピンは内蔵LED
        Timer::after(delay).await;

        defmt::println!("LED: OFF");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}
