//! LED制御（内蔵LED: WL_GPIO0 のみ使用）
use embassy_time::{Timer, Duration};

/// TXフェーズ時の内蔵LED点灯（WL_GPIO0）。
/// cyw43 の GPIO0 を制御する。
pub async fn blink_tx(control: &mut cyw43::Control<'_>, ms: u64) {
    // WL_GPIO0 は 0
    control.gpio_set(0, true).await;
    Timer::after(Duration::from_millis(ms)).await;
    control.gpio_set(0, false).await;
}

/// 受信検知時の内蔵LED高速点滅（視認性のため短い点滅を複数回）。
pub async fn blink_rx_fast(control: &mut cyw43::Control<'_>) {
    // 高速で5回点滅（50ms on / 50ms off）
    for _ in 0..5 {
        control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(50)).await;
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(50)).await;
    }
}

/// 異常通知用パターン（500ms, 500ms, 100ms の長さで点滅を繰り返す）
/// - ユーザに無線初期化などのエラー発生を知らせる
pub async fn error_blink_loop(control: &mut cyw43::Control<'_>) -> ! {
    loop {
        // 500ms 点灯 / 500ms 消灯
        control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(500)).await;
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(500)).await;

        // 500ms 点灯 / 500ms 消灯（2回目）
        control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(500)).await;
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(500)).await;

        // 100ms 点灯 / 500ms 消灯（短い点滅）
        control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(100)).await;
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(500)).await;
    }
}
