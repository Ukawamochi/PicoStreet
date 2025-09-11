//! 簡易APIクライアント（no_std, 手書きJSON, HTTP/1.1）
use defmt::*;
use heapless::String;

use embassy_net::{Stack, dns::DnsQueryType, IpAddress, IpEndpoint};
use embassy_net::tcp::TcpSocket;
use embedded_io_async::Write;
use embassy_time::{with_timeout, Duration};

use crate::storage::EncounterLog;
use crate::settings;
use pico_w_id_beacon::format::fmt_bytes_colon;

// 送信先設定は settings から取得

/// ペイロード（送信直前に組み立て）
pub struct ApiPayload<'a> {
    pub device_id: [u8; 6],
    pub encounters: &'a [EncounterLog],
    pub reported_at: u64,
}

/// ペイロードをJSONへ（簡易フォーマット、heapless）
pub fn serialize_to_json(payload: &ApiPayload<'_>) -> String<2048> {
    let mut s: String<2048> = String::new();
    let _ = s.push_str("{");

    // device_id
    let id = fmt_bytes_colon(&payload.device_id);
    let _ = s.push_str("\"device_id\":\"");
    let _ = s.push_str(id.as_str());
    let _ = s.push_str("\",");

    // reported_at
    let _ = s.push_str("\"reported_at\":");
    append_u64(&mut s, payload.reported_at);
    let _ = s.push_str(",");

    // encounters
    let _ = s.push_str("\"encounters\":[");
    for (i, e) in payload.encounters.iter().enumerate() {
        if i > 0 { let _ = s.push_str(","); }
        let mac = fmt_bytes_colon(&e.mac_addr);
        let _ = s.push_str("{");
        let _ = s.push_str("\"mac_addr\":\"");
        let _ = s.push_str(mac.as_str());
        let _ = s.push_str("\",");
        let _ = s.push_str("\"timestamp\":");
        append_u64(&mut s, e.timestamp);
        let _ = s.push_str("}");
    }
    let _ = s.push_str("]}");

    s
}

fn append_u64<const N: usize>(s: &mut String<N>, mut v: u64) {
    // 10進数を逆から詰めて反転
    let mut buf = [0u8; 20];
    let mut i = 0;
    if v == 0 { let _ = s.push('0'); return; }
    while v > 0 {
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }
    while i > 0 { i -= 1; let _ = s.push(buf[i] as char); }
}

// rssi は API に含めないポリシーに変更（2025-09）

/// APIへ送信（HTTP/1.1）。成功時は Ok(())
pub async fn send_encounters_to_server(stack: Stack<'static>, payload: &ApiPayload<'_>) -> Result<(), &'static str> {
    // DNS解決
    let host = settings::API_HOST;
    let addrs = with_timeout(Duration::from_secs(3), stack.dns_query(host, DnsQueryType::A))
        .await
        .map_err(|_| "DNS timeout")
        .and_then(|r| r.map_err(|_| "DNS error"))?;
    let server_ip = match addrs.first() { Some(IpAddress::Ipv4(v4)) => *v4, _ => return Err("no ipv4") };
    let ep = IpEndpoint::new(IpAddress::Ipv4(server_ip), settings::API_PORT);

    // TCP
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    let mut sock = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
    info!("API: 接続 {:?}", defmt::Debug2Format(&ep));
    match with_timeout(Duration::from_secs(3), sock.connect(ep)).await {
        Ok(Ok(())) => {}
        Ok(Err(_)) => return Err("connect fail"),
        Err(_) => return Err("connect timeout"),
    }

    // HTTPリクエスト
    let body = serialize_to_json(payload);
    info!(
        "API: POST http://{}:{}{} (encounters={}, body={}B)",
        host,
        settings::API_PORT,
        settings::API_PATH,
        payload.encounters.len() as u32,
        body.len() as u32
    );
    let mut req: String<256> = String::new();
    let _ = req.push_str("POST ");
    let _ = req.push_str(settings::API_PATH);
    let _ = req.push_str(" HTTP/1.1\r\nHost: ");
    let _ = req.push_str(host);
    let _ = req.push_str("\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: ");
    append_u64(&mut req, body.len() as u64);
    let _ = req.push_str("\r\n\r\n");

    match with_timeout(Duration::from_secs(3), sock.write_all(req.as_bytes())).await {
        Ok(Ok(())) => {}
        _ => return Err("write head"),
    }
    match with_timeout(Duration::from_secs(3), sock.write_all(body.as_bytes())).await {
        Ok(Ok(())) => {}
        _ => return Err("write body"),
    }

    // レスポンス先頭を読み、HTTPステータスを表示
    let mut tmp = [0u8; 256];
    let n = match with_timeout(Duration::from_secs(2), sock.read(&mut tmp)).await {
        Ok(Ok(n)) => n,
        Ok(Err(_)) => 0,
        Err(_) => 0,
    };
    if let Some(code) = parse_http_status(&tmp[..n]) {
        info!("API: レスポンス status={}", code as u32);
        if (200..300).contains(&code) {
            info!("API: 送信完了 ({}bytes)", body.len() as u32);
            Ok(())
        } else {
            Err("http status")
        }
    } else {
        info!("API: レスポンス status=不明");
        info!("API: 送信完了 ({}bytes)", body.len() as u32);
        Ok(())
    }
}

fn parse_http_status(buf: &[u8]) -> Option<u16> {
    if buf.len() < 12 { return None; }
    if !buf.starts_with(b"HTTP/1.") { return None; }
    // Find first space
    let mut i = 0;
    while i < buf.len() && buf[i] != b' ' { i += 1; }
    if i + 4 > buf.len() { return None; }
    let d1 = buf[i+1]; let d2 = buf[i+2]; let d3 = buf[i+3];
    if (d1 as char).is_ascii_digit() && (d2 as char).is_ascii_digit() && (d3 as char).is_ascii_digit() {
        let code = ((d1 - b'0') as u16) * 100 + ((d2 - b'0') as u16) * 10 + ((d3 - b'0') as u16);
        Some(code)
    } else {
        None
    }
}
