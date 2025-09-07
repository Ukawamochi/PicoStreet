//! LED制御（内蔵LED: WL_GPIO0、外付け GPIO18）

use embassy_rp::gpio::Output;
use embassy_time::{Timer, Duration};

/// 送信時の内蔵LED点灯（WL_GPIO0）。
/// cyw43 の GPIO0 を制御する。
pub async fn blink_tx(control: &mut cyw43::Control<'_>, ms: u64) {
    // WL_GPIO0 は 0
    control.gpio_set(0, true).await;
    Timer::after(Duration::from_millis(ms)).await;
    control.gpio_set(0, false).await;
}

/// 受信検知時の外付けLED点灯（GPIO18, Active High）。
pub async fn blink_rx(rx_led: &mut Output<'_>, ms: u64) {
    rx_led.set_high();
    Timer::after(Duration::from_millis(ms)).await;
    rx_led.set_low();
}
