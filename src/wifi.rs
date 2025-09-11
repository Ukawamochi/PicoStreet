//! WiFi connection and simple connectivity test using CYW43 + Embassy.
//!
//! Notes
//! - WiFi functionality is always enabled in this version
//! - LED patterns indicate connection state as requested.
//! - Includes full network stack with TCP/IP, DHCP, and HTTP connectivity test

use defmt::*;
use embassy_time::{Timer, Duration, Instant};

// Import WiFi config from the library crate

/// Blink pattern: during connection attempt (500ms interval).
pub async fn led_connecting(control: &mut cyw43::Control<'_>, cycles: u32) {
    for _ in 0..cycles {
        let _ = control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(500)).await;
        let _ = control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(500)).await;
    }
}

/// Blink pattern: connection completed (steady 2s ON).
pub async fn led_connected(control: &mut cyw43::Control<'_>) {
    let _ = control.gpio_set(0, true).await;
    Timer::after(Duration::from_secs(2)).await;
    let _ = control.gpio_set(0, false).await;
}

/// Blink pattern: connection failed (fast 100ms blink x5).
pub async fn led_connect_failed(control: &mut cyw43::Control<'_>) {
    for _ in 0..5 {
        let _ = control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(100)).await;
        let _ = control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(100)).await;
    }
}

/// Blink pattern: connectivity test success (3 short blinks).
pub async fn led_test_success(control: &mut cyw43::Control<'_>) {
    for _ in 0..3 {
        let _ = control.gpio_set(0, true).await;
        Timer::after(Duration::from_millis(120)).await;
        let _ = control.gpio_set(0, false).await;
        Timer::after(Duration::from_millis(120)).await;
    }
}

/// Connect to WiFi and run a simple connectivity test.
///
/// Behavior:
/// - Shows status via LED patterns
/// - Logs stages with `info!()`
/// - Measures and logs connection duration
/// - Runs a HTTP GET against example.com for connectivity test
pub async fn connect_and_test(
    spawner: embassy_executor::Spawner,
    mut control: &mut cyw43::Control<'_>,
    net_device: cyw43::NetDriver<'static>,
) {
    use pico_w_id_beacon::wifi_config::{WIFI_PSK, WIFI_SSID};

    info!("WiFi接続開始: SSID='{}'", WIFI_SSID);

    // Power management to save energy once link is up
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Quick visual cue that WiFi connect is in progress.
    led_connecting(&mut control, 2).await; // ~2 seconds (2 cycles)

    let t0 = Instant::now();

    // Attempt to join AP
    let join_res = control
        .join(WIFI_SSID, cyw43::JoinOptions::new(WIFI_PSK.as_bytes()))
        .await;

    match join_res {
        Ok(()) => {
            let ms = (Instant::now() - t0).as_millis();
            info!("WiFi接続成功: '{}' ({}ms)", WIFI_SSID, ms);
            led_connected(&mut control).await;
        }
        Err(e) => {
            warn!("WiFi接続失敗: {}", defmt::Debug2Format(&e));
            led_connect_failed(&mut control).await;
            return;
        }
    }

    // Network stack and connectivity test
    if let Err(e) = net_connectivity_test(spawner, net_device).await {
        warn!("WiFi接続テスト失敗: {}", e);
    } else {
        info!("WiFi接続テスト成功");
        led_test_success(&mut control).await;
    }
}

// ===== Network stack and connectivity tests =====
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

async fn net_connectivity_test(
    spawner: embassy_executor::Spawner,
    net_device: cyw43::NetDriver<'static>,
) -> Result<(), &'static str> {
    use embassy_net::{Config, Stack, StackResources};
    use static_cell::StaticCell;

    // Create DHCPv4 config
    let config = Config::dhcpv4(Default::default());

    // Static resources
    // DHCP (1) + DNS (1) + user TCP socket (1) = 3
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    static STACK: StaticCell<Stack<'static>> = StaticCell::new();

    // Random seed (simple fallback)
    let seed = 0x1357_9bdf_2468_abcdu64;

    // Build stack and spawn runner
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    // SAFETY: `stack` lives in static cell, fine to keep &'static reference.
    let stack: Stack<'static> = *STACK.init(stack);

    // Spawn network task
    // Note: this expects to be called from an Embassy context where spawner is available
    // but we cannot borrow spawner from here; instead, rely on the executor to spawn a task
    // with this function from main if desired. For Phase 1, we run inline in this function
    // by spawning via the global executor.
    spawner
        .spawn(net_task(runner))
        .map_err(|_| "ネットワークタスク起動失敗")
        .ok();

    // IP取得待ち（タイムアウト付きでブロック回避）
    use embassy_time::{with_timeout, Duration};
    if with_timeout(Duration::from_secs(10), stack.wait_config_up())
        .await
        .is_err()
    {
        return Err("DHCPタイムアウト");
    }

    let config = stack.config_v4().ok_or("IPv4設定取得失敗")?;
    info!("IPv4アドレス取得: {}", defmt::Debug2Format(&config.address));

    // Simple HTTP GET test to example.com (fixed IPv4 to avoid DNS here)
    use embassy_net::{IpAddress, IpEndpoint};
    use embassy_net::tcp::TcpSocket;
    use embedded_io_async::Write;

    // example.com (93.184.216.34) port 80
    let ep = IpEndpoint::new(IpAddress::v4(93, 184, 216, 34), 80);

    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);

    info!("TCP接続試行: {:?}", defmt::Debug2Format(&ep));
    if with_timeout(Duration::from_secs(3), socket.connect(ep))
        .await
        .map_err(|_| "TCP接続タイムアウト")?
        .is_err()
    {
        return Err("TCP接続失敗");
    }

    let req = b"GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\nUser-Agent: PicoStreet/0.1\r\n\r\n";
    if with_timeout(Duration::from_secs(2), socket.write_all(req))
        .await
        .map_err(|_| "HTTP送信タイムアウト")?
        .is_err()
    {
        return Err("HTTP送信失敗");
    }

    // Read some bytes
    let mut total = 0usize;
    let mut buf = [0u8; 256];
    loop {
        match with_timeout(Duration::from_secs(2), socket.read(&mut buf)).await {
            Ok(Ok(0)) => break, // closed
            Ok(Ok(n)) => {
                total += n;
                if total > 64 { break; }
            }
            Ok(Err(_)) => return Err("HTTP受信失敗"),
            Err(_) => break, // タイムアウト: データが来ないが接続できたのでOK
        }
    }
    // 明示的にクローズしてソケットを解放
    socket.close();
    info!("HTTP GET応答受信: {}バイト", total);
    Ok(())
}
