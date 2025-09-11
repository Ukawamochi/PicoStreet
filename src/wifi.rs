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

// /// Blink pattern: connectivity test success (3 short blinks).
// pub async fn led_test_success(control: &mut cyw43::Control<'_>) {
//     for _ in 0..3 {
//         let _ = control.gpio_set(0, true).await;
//         Timer::after(Duration::from_millis(120)).await;
//         let _ = control.gpio_set(0, false).await;
//         Timer::after(Duration::from_millis(120)).await;
//     }
// }

// ===== Network stack 永続化 + NTP同期（Phase1） =====

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

/// WiFiへ接続してネットワークスタックを起動し、`Stack`を返す
pub async fn maintain_wifi_connection(
    spawner: embassy_executor::Spawner,
    mut control: &mut cyw43::Control<'_>,
    net_device: cyw43::NetDriver<'static>,
) -> Result<embassy_net::Stack<'static>, &'static str> {
    use crate::settings::{WIFI_PSK, WIFI_SSID};
    use embassy_net::{Config, Stack, StackResources};
    use static_cell::StaticCell;

    info!("WiFi接続開始: SSID='{}'", WIFI_SSID);

    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    led_connecting(&mut control, 2).await;

    let t0 = Instant::now();
    if let Err(e) = control
        .join(WIFI_SSID, cyw43::JoinOptions::new(WIFI_PSK.as_bytes()))
        .await
    {
        warn!("WiFi接続失敗: {}", defmt::Debug2Format(&e));
        led_connect_failed(&mut control).await;
        return Err("AP接続失敗");
    }

    let ms = (Instant::now() - t0).as_millis();
    info!("WiFi接続成功: '{}' ({}ms)", WIFI_SSID, ms);
    led_connected(&mut control).await;

    // DHCPv4でネットワークスタック起動
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new(); // DHCP(1)+DNS(1)+UDP(1)
    static STACK: StaticCell<Stack<'static>> = StaticCell::new();
    let config = Config::dhcpv4(Default::default());
    let seed = 0x1357_9bdf_2468_abcdu64;
    let (stack_tmp, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );
    let stack = *STACK.init(stack_tmp);

    spawner
        .spawn(net_task(runner))
        .map_err(|_| "ネットワークタスク起動失敗")
        .ok();

    // DHCP待ち（タイムアウト付き）
    use embassy_time::with_timeout;
    if with_timeout(Duration::from_secs(10), stack.wait_config_up())
        .await
        .is_err()
    {
        return Err("DHCPタイムアウト");
    }

    if let Some(v4) = stack.config_v4() {
        info!("IPv4取得成功: {}", defmt::Debug2Format(&v4.address));
    }

    Ok(stack)
}

/// 簡易SNTPでNTP時刻同期（JSTでログ）
pub async fn sync_ntp_time(stack: embassy_net::Stack<'static>) -> Result<u64, &'static str> {
    use embassy_net::dns::DnsQueryType;
    use embassy_net::{IpAddress, IpEndpoint};
    use embassy_net::udp::{UdpSocket, PacketMetadata};
    use embassy_time::with_timeout;

    // DNSでNTPサーバを引く（Aレコード）。
    let addrs = with_timeout(Duration::from_secs(3), stack.dns_query("pool.ntp.org", DnsQueryType::A))
        .await
        .map_err(|_| "DNSタイムアウト")
        .and_then(|r| r.map_err(|_| "DNS失敗"))?;
    let server_ip = match addrs.first() {
        Some(IpAddress::Ipv4(v4)) => *v4,
        _ => return Err("IPv4未取得"),
    };

    let server = IpEndpoint::new(IpAddress::Ipv4(server_ip), 123);

    // UDPソケットを一時利用
    let mut rx_meta = [PacketMetadata::EMPTY; 2];
    let mut tx_meta = [PacketMetadata::EMPTY; 2];
    let mut rx_buf = [0u8; 512];
    let mut tx_buf = [0u8; 64];
    let mut udp = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    udp.bind(0).map_err(|_| "UDP bind失敗")?;

    // NTPクライアントパケット（48B）: LI=0,VN=4,Mode=3 -> 0x23
    let mut pkt = [0u8; 48];
    pkt[0] = 0x23;

    with_timeout(Duration::from_secs(2), udp.send_to(&pkt, server))
        .await
        .map_err(|_| "NTP送信タイムアウト")
        .and_then(|r| r.map_err(|_| "NTP送信失敗"))?;

    let mut buf = [0u8; 64];
    let (n, _meta) = with_timeout(Duration::from_secs(2), udp.recv_from(&mut buf))
        .await
        .map_err(|_| "NTP受信タイムアウト")
        .and_then(|r| r.map_err(|_| "NTP受信エラー"))?;
    if n < 48 { return Err("NTP短小応答"); }

    // 送信タイムスタンプ（40..44）をUNIXへ変換
    let secs = u32::from_be_bytes([buf[40], buf[41], buf[42], buf[43]]) as u64;
    const NTP_UNIX_DIFF: u64 = 2_208_988_800; // 1900->1970
    if secs < NTP_UNIX_DIFF { return Err("NTP時刻不正"); }
    let unix = secs - NTP_UNIX_DIFF;

    // JST表示（HH:MM）
    let local = unix + 9 * 3600;
    let sec_day = local % 86_400;
    let hh = sec_day / 3600;
    let mm = (sec_day % 3600) / 60;
    info!("インターネット接続成功！現在時刻: {:02}:{:02} JST", hh as u32, mm as u32);
    // 時刻ベースを設定
    crate::timekeeper::set_unix_time(unix);

    Ok(unix)
}

// （注意）Global Stack 共有は行っていない。必要なら別タスク化/設計見直しで対応。

// 旧HTTPテスト実装は削除（常時接続 + NTP 同期へ移行）
