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
    // 高速で3回点滅（50ms on / 50ms off）
    for _ in 0..3 {
        control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(50)).await;
        control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(50)).await;
    }
}
